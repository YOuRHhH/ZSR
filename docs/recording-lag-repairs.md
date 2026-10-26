# Minimized recording: ordered investigation and repairs

Date: 2026-10-26. Based on `ZSR_最小化后录制卡顿：源码分析.md`.

The source review correctly identifies missing diagnostics and a CPU encoding path. It does not establish CPU starvation as the cause of the user's one-frame-per-second recordings. Static desktop content and FFmpeg's output frame count alone cannot establish the rate of newly captured images.

## Changes in the document's order

| Step | Query / finding | Repair | Validation |
| --- | --- | --- | --- |
| 1. Progress and power diagnostics | Worker previously consumed only `frame` and `out_time_us`; GUI builds lost `eprintln!` messages. | New `recording/diagnostics.rs` saves JSON lines beside the MP4: wall time, speed, frames, duplicates, drops, backend, encoder options, PID, power-policy results and final stderr tail. Each log is capped at 4 MiB. | Parser test covers unavailable speed, numerical counters and the final batch. Real recordings retain the counters even without UI consumption. |
| 2. Recording priority | FFmpeg previously ran at Normal priority. | Native `SetPriorityClass` requests Above Normal only for the owned recording child and logs success/error. General setup commands remain unchanged. Parent priority and system power plan remain unchanged. | Windows real-capture tests exercise the native path; live process inspection confirmed AboveNormal. |
| 3. Software preset | Software encoding used veryfast at both frame rates. | 60 fps uses ultrafast; 30 fps retains veryfast. | On this Ryzen 5 5500U, a single synthetic 1080p60/180-frame run took 1.09 s with veryfast and 0.58 s with ultrafast. This is an encoding-headroom comparison, not proof of minimized performance. Larger files are a tradeoff. |
| 4. Actual capture backend | The selected graph was not retained in logs. | Log actual ddagrab/gfxcapture/gdigrab or platform input, selected DXGI device and screen graph. | Both full-display and region tests selected ddagrab on this machine. |
| 5. Hardware encoding | All recording used libx264. | Windows automatically tries AMF, NVENC and QSV with a real two-frame encode before selection, bounded to five seconds per probe. If probes fail, use software. Settings includes a bilingual hardware preference switch. | AMF initialized and produced decodable recordings on AMD Radeon Graphics. NVENC/QSV argument construction is checked, but those GPUs are not present here and their runtime behavior remains unverified. |

## Pipeline limits

- Hardware encoding still uses GPU download plus CPU scale/pad/color conversion. This is not a zero-copy pipeline and does not eliminate all CPU work. A direct GPU path requires separate handling of resizing, padding and encoder/device compatibility.
- AMF CQP, NVENC CQ and QSV global quality are approximate mappings of the existing compression choices; their sizes and visual quality are not identical to x264 CRF.
- Probes run in the recorder worker, not the UI. A driver/device failure after preflight is reported with the partial movie retained when nonempty; the recording is not silently restarted or overwritten.
- Automatic hardware encoding is Windows-only. macOS/Linux retain the current software path.
- No microphone was recorded during these tests. Audio-enabled and prolonged foreground-load reproduction still require validation.

## Checks

- All-target/all-feature Rust tests and Clippy with warnings denied pass.
- Synthetic recording finalized and decoded successfully; pausing UI event consumption for four seconds still produced changing video frames.
- Region capture at 60 fps passed with automatic AMF and explicit software selection, including output dimensions and decode validation.
- Full display at 1920×1080, 60 fps, with UI progress consumption paused for five seconds:
  - AMF: 341 frames, 60.13 fps, speed 0.999×, 28 duplicates, zero drops.
  - Software ultrafast: 341 frames, 60.09 fps, speed 1.000×, 34 duplicates, zero drops.
- The duplicate counts above include startup/capture cadence and are not a count of visually identical desktop frames.

Run the relevant automated checks:

```powershell
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --test recording_pipeline -- --ignored --test-threads=1
cargo test --test desktop_capture -- --ignored --nocapture --test-threads=1
```

For user reproduction, use the newly built executable, record active screen content before and after minimizing, then compare progress intervals in the same-name `.log`. Sustained speed below 1× with increasing lag warrants investigation; duplicate frames alone do not prove CPU starvation.

Priority reference: [Microsoft SetPriorityClass](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setpriorityclass). Encoder options were also checked against the installed FFmpeg's `-h encoder=h264_amf`, `h264_nvenc` and `h264_qsv` help.


## Follow-up: microphone-enabled capture starvation

The user's next recording, with ZSR visible and microphone enabled, contained 638 output frames but 570 FFmpeg-generated duplicates. Thus the nominal 60 fps did not represent a healthy capture cadence. The earlier microphone-disabled tests missed this condition.

Controlled six-second captures reproduced it with the same DirectShow microphone: the old lavfi-demuxer video path produced 251 duplicates. A smaller audio buffer alone did not resolve it. Moving the Windows compositor source into `-filter_complex` reduced duplicates to 44; combining it with a 50 ms DirectShow audio buffer reduced them to 34. This evidence implicates audio/video input scheduling, rather than minimizing alone or insufficient encoder speed.

`command.rs` now builds compositor capture as a source filter, applies timestamp normalization and output scaling in that graph, maps `[zsr_video]`, and maps microphone input `0:a:0`. Without a microphone, no demuxer input is required. GDI/macOS/X11 compatibility inputs retain their existing input layout. DirectShow audio uses a 50 ms requested buffer. This also applies to WGC application capture.

The worker-level microphone regression initially recorded 398 frames with only 28 duplicates, speed 0.996×, and no dropped frames. It verifies real audio/video streams, similar track durations and successful decode. It now exercises both hardware and software encoders:

```powershell
cargo test --test microphone_cadence -- --ignored --nocapture
```

This opt-in test uses the configured microphone and removes its temporary recordings. Duplicate counts measure FFmpeg's CFR insertion, not visual uniqueness; prolonged real-world motion and playback remain relevant checks.

Windows build note: a running `target\debug\ZSR.exe` locks that executable. Close ZSR before `cargo run`, or build into a different target directory. The repaired executable was built at `target\region-verify\debug\ZSR.exe` without terminating the user's running instance.
