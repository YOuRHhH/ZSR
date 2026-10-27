# ZSR

[English](README.md) | 简体中文

ZSR 是使用 Rust 和 C 平台适配层开发的桌面录屏工具，提供中文和英文界面。当前录制引擎通过独立的 `zsr-obs` 辅助进程调用 **OBS libobs**，不再使用 FFmpeg 命令行程序进行采集、编码或设备发现。OBS 自身仍包含 FFmpeg 等编解码库。

## 功能与平台状态

- 支持显示器、区域和应用窗口录制。区域选择时会临时最小化主窗口，确认后恢复，并保留录制边框。
- 支持系统默认播放设备的桌面音频和指定麦克风。桌面音频默认关闭；从旧版 FFmpeg 后端迁移后，需要重新选择麦克风，因为 OBS 使用不同的原生设备标识。
- 音频可选择混合单轨，或在同一个 MP4 中将桌面音频保存到轨道 1、麦克风保存到轨道 2。仅启用一个音源时保存单轨。多数播放器一次只播放一个音轨，需要同时听到两个音源时请选择混合模式。
- 支持原始尺寸、480p、720p、1080p、1440p、2160p，以及 30/60 fps；画面按比例适配，不拉伸。
- Windows 优先尝试 AMF、NVENC、QSV 硬件 H.264 编码，失败后回退到 x264；设置中也可强制软件编码。
- 停止录制后生成 OBS 原生混合式 MP4；中断文件保留为 `.partial.mp4`，不会作为成功录制显示。录制列表可打开文件或所在目录。
- 支持语言切换、输出偏好和全局快捷键设置。OBS 日志保存在录像旁，仅保留在本地，可能包含设备名称和路径。

Windows 使用 WASAPI 采集音频，并已有端到端录制验证。Linux 当前要求 X11，窗口发现还需要 `wmctrl`；Ubuntu 24.04 x86_64 提供自动打包和 CI 启动检查，但实际桌面采集仍需原生验证。macOS 已有平台映射和打包脚本，运行时部署与采集尚未在原生设备验证。暂停/继续和 Wayland portal 采集尚未实现。两个音源都关闭时，当前 MP4 输出仍包含一条静音 AAC 音轨。

## 技术细节与选型

| 技术 | 用途与选择理由 |
| --- | --- |
| Rust 2024 / Rust 1.88+ | 实现应用状态、配置、进程管理及录制流程，结合类型检查和资源所有权管理桌面应用生命周期。 |
| eframe/egui 0.33.3 + glow | 使用 Rust 即时模式界面和 OpenGL 渲染构建桌面 UI，无需启动 OBS 的 Qt 前端。 |
| OBS libobs 32 / `libloading` | 动态加载录制运行时，由成熟的媒体引擎负责采集时序、GPU 渲染、音画同步、编码和封装。项目固定使用 OBS 32.2.2 运行时，并检查 libobs 主版本。 |
| 独立 `zsr-obs` 进程 + JSON Lines | 主界面发送序列化请求，辅助进程返回进度和事件；将 OBS 初始化及原生录制生命周期与 UI 分离。停止命令或管道关闭会请求停止录制。 |
| C/C++ 适配层 + `cc` | 对接原生窗口、显示器、区域选择、录制边框及 OBS 日志回调等平台能力。 |
| Serde / JSON | 序列化配置和进程通信数据；`locales/zh-CN.json` 与 `locales/en.json` 保存界面翻译。 |
| Python 3.11+ / Inno Setup / dpkg-deb | 下载并校验 OBS 运行时、部署辅助程序，以及生成 Windows 安装程序和 Linux DEB。 |

UI 不向录制引擎逐帧提交画面。Windows 显示器采集使用 OBS `monitor_capture`，窗口采集使用 `window_capture`，区域录制使用场景项裁剪；GPU 完成输出尺寸适配。窗口选择会验证标题、类名和可执行文件匹配，拒绝有歧义的选择，录制时应保持目标窗口打开且可见。

主要目录：`src/app/` 为桌面界面与交互，`src/recording/` 为录制会话与进程监督，`src/obs/` 为 libobs 绑定和录制实现，`native/` 为平台适配，`scripts/` 为运行时和打包工具，`tests/` 为 Rust 集成测试与 Python 打包测试。当前后端详见 [OBS 后端说明](docs/obs-backend.md)；旧 FFmpeg 排障文档保留为历史记录。

## Windows 构建与运行

在项目根目录执行。需要 Rust 1.88+、MSVC 的“使用 C++ 的桌面开发”工具链、Python 3.11+，以及匹配架构的 OBS 运行时。项目已验证的运行时是官方 OBS Studio **32.2.2 x64**。

```powershell
# 下载官方 OBS 运行时并校验 SHA-256。
py scripts/fetch_obs_windows.py

# 同时构建 UI 和录制辅助程序，再部署辅助程序。
cargo build --bins
py scripts/stage_obs.py --obs-dir target/obs-runtime/32.2.2 --helper target/debug/zsr-obs.exe --output target/obs-runtime/32.2.2
cargo run --bin ZSR
```

重新构建前请关闭正在运行的 ZSR，Windows 无法替换运行中的可执行文件。修改辅助程序后需重新执行部署命令。也可使用独立构建目录：

```powershell
cargo build --bins --target-dir target/region-verify
py scripts/stage_obs.py --obs-dir target/obs-runtime/32.2.2 --helper target/region-verify/debug/zsr-obs.exe --output target/obs-runtime/32.2.2
.\target\region-verify\debug\ZSR.exe
```

可通过 `ZSR_OBS_RUNTIME` 指定默认运行时，或在“设置 → 高级 → OBS 运行时目录”中选择。未显式覆盖时，可执行文件旁的 `obs` 目录优先于开发运行时。

启动后选择显示器、区域或应用窗口，按需启用桌面音频并选择麦克风，设置音轨模式、分辨率和帧率，然后点击“开始录制”。完成后点击“停止并保存”，在录制列表中打开成品。

## 构建安装包

Full 包含 OBS 运行时和辅助程序，并保留运行时许可证与源码引用；Lite 包含辅助程序，需要用户在设置中指定兼容的外部 OBS 运行时。这两个版本取代旧版 FFmpeg 的捆绑/非捆绑版本。

在目标平台运行统一打包命令；这是本机打包，不是跨平台编译。Windows 构建机还需安装 Inno Setup 6：

```powershell
py scripts/package.py
```

Ubuntu 24.04 x86_64：

```sh
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libxkbcommon-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev wmctrl
python3 scripts/package.py
sudo apt install ./dist/linux/*Full.deb
```

脚本下载并校验 OBS 32.2.2，使用 release 模式构建两个 Rust 程序，在 `dist/windows/` 或 `dist/linux/` 生成 Full 和 Lite 安装包。它不会自动安装生成的安装包或修改系统依赖。支持 `--obs-dir PATH`、`--output PATH`、`--iscc PATH`（Windows）、`--skip-build`，并遵循 `CARGO_TARGET_DIR`。

Linux Lite 可通过 `python3 scripts/fetch_obs_linux.py` 准备规范化运行时，再配置目录并安装所需依赖；发行版提供的任意 `obs-studio` 版本不保证兼容。其他 Linux 发行版或架构需要自行提供匹配的 `--obs-dir`，其中应包含用于 DEB 依赖元数据的 `linux-dependencies.txt`。

直接调用 Windows 打包器：

```powershell
cargo build --release --bins
py scripts/package_windows.py --obs-dir target/obs-runtime/32.2.2
# 可选：指定 Inno Setup 编译器。
py scripts/package_windows.py --obs-dir target/obs-runtime/32.2.2 --iscc "C:\Path\To\Inno Setup 6\ISCC.exe"
```

当前输出为 `dist/windows/ZSR-0.1.0-Windows-x64-Full-Setup.exe` 和对应的 `Lite-Setup.exe`。单独执行 `cargo build` 不会生成安装包；源码变更后，`dist` 中已有的安装包不会自动更新。

macOS/Linux 的直接打包方式如下，需在对应系统构建。Unix 运行时应包含 `lib/libobs.so`（Linux）或 `lib/libobs.dylib`（macOS）、`lib` 下的图形后端、`lib/obs-plugins` 下的插件，以及 `data/libobs` 和 `data/obs-plugins` 下的数据。原始 macOS OBS `.app` 不符合此布局，需先完成运行时重定位和签名。

```sh
cargo build --release --bins
# macOS 需要 hdiutil 和已准备好的运行时。
python3 scripts/package_unix.py --platform macos --obs-dir /path/to/obs-runtime
# Linux 需要 dpkg-deb 和适配目标发行版的运行时。
python3 scripts/package_unix.py --platform linux --obs-dir /path/to/obs-runtime
```

输出位于 `dist/`。Windows 测试结果不代表 macOS/Linux 实际采集已经通过验证。

## CI 与发布

[打包工作流](.github/workflows/package.yml) 仅在推送 `v*` 版本标签时，分别构建 Windows 和 Ubuntu 安装包。标签必须与 Cargo 包版本匹配；两个平台成功后，工作流发布 GitHub Release。完成后可在 Actions 的 Artifacts 中下载 `ZSR-windows-Full-Lite` 和 `ZSR-linux-Full-Lite`，保留 14 天。普通分支推送和 PR 运行独立的 Rust 桌面检查，不构建安装包。安装包未进行代码签名。Linux CI 安装 Full 包并在 Xvfb 下检查 OBS 启动，实际桌面采集仍需单独测试。

发布时先修改 `Cargo.toml` 版本并通过 Cargo 更新 `Cargo.lock`，提交并推送，再创建与版本完全匹配的标签，例如当前版本：

```sh
git tag -a v0.1.0 -m "ZSR 0.1.0"
git push github v0.1.0
```

若远程名为 `origin`，使用 `git push origin v0.1.0`。两个平台成功后，CI 将四个安装包和 `SHA256SUMS.txt` 上传到草稿 Release，全部上传成功才发布，并自动生成发布说明。带预发布后缀的标签（如 `v0.2.0-rc.1`）会生成预发布版。仅发布任务获得 `GITHUB_TOKEN` 的 `contents: write` 权限，无需额外 token secret。

上传失败可重新运行失败的发布任务以完成草稿；已发布版本不会被覆盖，应发布新版本。打包工作流不提供手动触发入口；提交工作流本身不会创建标签或 Release。

## 验证与排障

```powershell
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
py -m unittest discover -s tests -p "test_*.py"
# 按需执行：实际录制屏幕/音频，并在 Windows 播放低音量测试音。
cargo test --test obs_recording -- --ignored --nocapture
# 按需执行：打开并录制临时动画窗口。
cargo test --test obs_window -- --ignored --nocapture
```

媒体集成测试使用 `ffprobe`/`ffmpeg` 检查成品，终端用户无需安装这两个程序。历史 FFmpeg 命令测试仅在启用 `legacy-ffmpeg` 特性时可用，该特性不会切换回旧录制引擎。运行时缺失时请检查设置中的路径、OBS 架构与版本，以及辅助程序是否已部署；录制失败可查看录像旁的 OBS 日志。

本地 `obs-studio` 参考源码、下载的运行时和构建产物不纳入 Git。运行时与源码来源：[OBS Studio 32.2.2](https://github.com/obsproject/obs-studio/releases/tag/32.2.2)。
