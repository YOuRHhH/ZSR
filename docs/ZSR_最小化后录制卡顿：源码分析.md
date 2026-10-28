# ZSR 最小化后录制卡顿：源码分析

Oct 26, 2026 · @BIGASS

## 结论与范围

最可能的原因是：ffmpeg 子进程的 CPU 编码管线在最小化后抢不过前台程序，而代码里没有任何指标能证明这一点。UI 依赖和 EcoQoS 这两类常见原因，源码里已经处理好了。

本文是对 `src/recording/`、`native/recording_power.c`、`src/app/` 相关部分和 `docs/implementation.md` 的静态分析。没有在 Windows 上编译或运行过，也没有 ffmpeg 的 `speed`、`dup_frames`、`drop_frames` 实测数据，所以下面的排序是推断，不是复现后的定论。

## 已排除的原因

- **采集不依赖 UI。** ffmpeg 是独立子进程，worker 线程用 `try_wait` 每 50ms 轮询，stdout 和 stderr 各有单独的读取线程。最小化后 egui 不再调用 `update()`，只影响界面，不会堵住 ffmpeg 的管道。
- **EcoQoS 已有处理。** `power.rs` 和 `recording_power.c` 会对 ffmpeg 子进程关闭定时器和执行速度节流，并在录制结束后恢复父进程原有策略。`docs/implementation.md` 注明这是对“最小化调度”的疑似原因的修复，尚未确认是否真正解决。

## 剩下最可疑的原因

1. **全 CPU 管线，且优先级为 Normal。** 当前链路是 `ddagrab` 或 `gfxcapture` → `hwdownload,format=bgra` → CPU 上的 `scale`/`pad`/格式转换 → `libx264 -preset veryfast`。1080p60 已经很吃 CPU，4K 基本跑不动。`recording_power.c` 的注释写明不提高进程优先级，ZSR 最小化后，前台程序在调度上占优，ffmpeg 编码跟不上就会重复帧或丢帧，表现为卡顿。
2. **看不到真实编码速度。** 命令里已有 `-progress pipe:1`，会输出 `speed=`、`dup_frames=`、`drop_frames=`，但 `worker.rs` 只解析 `frame=` 和 `out_time_us=`。没有这三个值，就分不清是编码慢，还是采集不出帧。
3. **节流设置失败是无声的。** 失败只调用 `eprintln!`，而 ZSR 是 GUI 程序，没有控制台，opt-out 没生效也看不到。
4. **实际采集路径可能是 WGC。** 文档记录过一次实机录制走的是 `gfxcapture`，不是 `ddagrab`。WGC 按画面变化出帧，静止画面帧很少，再用 `-r` 补成恒定帧率，节奏会不均匀。走哪条路径取决于 ffmpeg 是否带 `ddagrab`，以及 DXGI 输出能否匹配所选显示器。

## 建议的改法

按性价比从高到低排列，前两项改动小，建议先做。

| 顺序 | 改法 | 涉及文件 | 目的 |
| --- | --- | --- | --- |
| 1 | 解析 `speed=`、`dup_frames=`、`drop_frames=` 并写入日志；`power.rs` 的失败信息也写入日志 | `worker.rs`、`power.rs` | 先判断是编码慢、采集断帧，还是节流设置未生效 |
| 2 | 对录制用的 ffmpeg 子进程设置 `ABOVE_NORMAL_PRIORITY_CLASS`（`0x00008000`），只用于录制，不加到 `platform::command` | `worker.rs` | 最小化后不被前台程序抢走 CPU |
| 3 | 60fps 下先把 `-preset` 改为 `ultrafast` 试验 | `command.rs` | 降低 CPU 编码负载，验证是否为编码瓶颈 |
| 4 | 记录实际使用的采集输入是 `ddagrab`、`gfxcapture` 还是 `gdigrab` | `worker.rs`、`desktop.rs` | 确认走的是哪条采集路径 |
| 5 | 支持硬件编码（NVENC、QSV、AMF）；ffmpeg 官方文档有 `ddagrab` 直接接 `h264_nvenc` 的示例，可省掉 `hwdownload` 和 CPU 色彩转换 | `command.rs`、`desktop/duplication.rs` | 从根本上消除 CPU 瓶颈 |

改动需要在 Windows 本机编译和验证。

## 需要补充的信息

- [ ] 录制时任务管理器里 ffmpeg 的 CPU 占用，以及 CPU 型号（Intel 12 代以上要留意 P 核和 E 核调度）
- [ ] 分辨率、帧率，是否开启麦克风
- [ ] ffmpeg 是否带 `ddagrab`（运行 `ffmpeg -h filter=ddagrab` 确认）
- [ ] 不最小化时是否同样卡顿
- [ ] 被录制的是整个显示器、区域，还是应用窗口
