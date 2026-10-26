# ZSR

A Rust desktop screen recorder with a native C display adapter and a Chinese/English interface. This repository contains the first implementation milestone, not a production-ready release.

## Run

Install a Rust toolchain (1.88 or newer), a platform C compiler, and an FFmpeg build with `libx264` and the relevant capture input enabled for development. Full packages include a private FFmpeg runtime; Lite packages use an external FFmpeg installation. ZSR does not download FFmpeg automatically.

```sh
cargo run --locked
```

Select a display or region, choose an output directory, and press **Start recording**. Selecting **Region** opens an on-screen frame on the chosen display. Drag outside the frame to draw a new area, drag inside to move it, or drag a corner to resize. Confirm with **Use area** / Enter, or cancel with Escape. **Select area on screen…** reopens the picker; the selection controls close after confirmation, and a click-through border remains around the chosen area before and during recording. Press **Stop and save** to finalize the MP4. The language selector switches between **English** and **简体中文** immediately; preferences are saved in the OS application configuration directory. Settings and device identifiers should be treated as local user data.

- **Windows:** Install MSVC Build Tools with the Desktop development with C++ workload. FFmpeg needs `gdigrab`; microphone inputs are discovered automatically through DirectShow.
- **macOS:** Install Xcode Command Line Tools. FFmpeg needs `avfoundation`. Grant screen-recording permission to the launched application or terminal as required by macOS, then recheck setup. Microphone inputs are discovered automatically through AVFoundation and require microphone permission. The permission behavior of a packaged, signed app still needs qualification.
- **Linux:** Install a C compiler and desktop development dependencies. For Debian/Ubuntu, start with `build-essential pkg-config libxkbcommon-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev`. Recording currently requires an X11 session and an FFmpeg build with `x11grab`; microphone discovery uses `pactl --format=json list sources` (install `pulseaudio-utils` on Debian/Ubuntu) with PulseAudio or PipeWire’s PulseAudio compatibility server. Wayland capture is explicitly unavailable in this milestone.

Select **Application window** to record a specific open window. Choose a target from the automatically refreshed list and press **Start recording**. Targets are identified by window ID and process ID, so duplicate or changing titles do not switch the target. Windows requires an FFmpeg build with the `gfxcapture` filter and its HWND option (Windows Graphics Capture). Linux/X11 requires `wmctrl` for discovery and FFmpeg `x11grab`. macOS and Wayland window capture are not yet implemented and show an explanation. Keep the target visible, open, and at a fixed size; protected content may be blank. This records one window, not all windows of an application or its system audio.

Put `ffmpeg` on PATH or enter its full executable path in the app and click **Recheck setup**. The recording source picker is populated only after display discovery and FFmpeg checks succeed. Microphones appear automatically in the audio selector. Enable **Record microphone**, then choose a detected input. The list loads at startup, supports manual refresh, and refreshes every 15 seconds while microphone capture is enabled and the recorder is idle. A saved device that disappears must be explicitly replaced; ZSR does not silently select another microphone.

For Chinese glyphs, ZSR loads a system CJK font: Microsoft YaHei on Windows, PingFang or Heiti on macOS, or Noto Sans CJK/WenQuanYi Micro Hei at common Linux paths. Install one of these fonts if Chinese characters do not display. Fonts are not redistributed with this project. Initial language uses `LC_ALL`, `LC_MESSAGES`, or `LANG` when set; otherwise it defaults to English until changed in the UI.

Use **Recordings / 录制文件** in the header to browse up to 200 completed ZSR recordings in the current save folder, sorted by modification time. The list shows filename, modification date, and file size, with actions to open a video or the folder. It refreshes when opened, after a recording is saved, and when the save directory changes; use **Refresh list** for external changes. In-progress `.partial.mp4` files are excluded.

## Implemented

- Local display recording and visual region selection with a draggable, resizable frame. Windows/X11 capture the selected coordinates directly; macOS crops the selected display input.
- Original, 480p, 720p, 1080p, 1440p, or 4K output resolution; 30/60 FPS; three compression quality presets; and cursor inclusion. Resolution presets preserve proportions, add black bars when needed, and upscale smaller sources.
- Automatically discovered microphone inputs with friendly names, persisted selections, refresh, and unavailable-device handling.
- H.264 MP4 output with fragmented intermediate files, unique names, and collision-safe finalization.
- A worker thread for capture, progress updates, stop/finalization, and retained incomplete files on recording failures.
- Immediate English/Simplified Chinese switching, matching translation catalogs, persistent preferences, and system CJK font fallback.
- Output-file opening, folder selection, and setup diagnostics.

System audio, visual previews, macOS/Wayland window capture, pause/resume, hardware encoding, disk-space monitoring, Wayland portal capture, and automatic recovery are still pending. Fragmented MP4 may preserve playable data after interruption, but recovery is not guaranteed or automated. Linux currently exposes the complete default X11 screen as a single source, which may include several physical displays.

## Build and verify

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo test --test recording_pipeline -- --ignored
cargo build --release --locked
```

The opt-in integration test requires `ffmpeg` and `ffprobe` on PATH. It records a generated test pattern, stops the same worker used by the UI, verifies H.264 metadata, decodes the result, and removes its own temporary output. It never captures the desktop or microphone. Unit tests cover translation parity, input validation, capture-coordinate handling, device parsing, and shell-free argument construction.

The CI matrix builds and runs unit tests on Windows, macOS, and Linux. CI compilation does not qualify live capture on those platforms. Real desktop permissions, scaled multi-monitor setups, audio synchronization, resource use, and packaged releases still require platform testing. See [implementation status](docs/implementation.md) and [requirements](docs/requirements.md).

## Package Full and Lite editions

打包命令：[Windows](#windows-powershell) · [macOS](#macos) · [Linux](#linux-debianubuntu)。请在项目根目录执行；每个平台的打包脚本都会同时生成 **Full（内置 FFmpeg）** 和 **Lite（不内置 FFmpeg）** 两个版本。

Run commands from the repository root. `cargo build --release` builds the executable only; the packaging script produces **both Full and Lite** packages. Full includes FFmpeg and automatically uses it. Lite excludes FFmpeg and needs an external installation. Both editions contain the same ZSR application.

Packaging requires Python 3.11+ and a complete FFmpeg distribution for the target OS and architecture, including its license notices, runtime libraries, and redistribution materials. Supply the distribution root containing `bin/ffmpeg.exe` (Windows) or `bin/ffmpeg` (macOS/Linux). The scripts omit unused `ffplay` and `ffprobe` companion programs while retaining FFmpeg and its supporting files.

### Windows (PowerShell)

Install Inno Setup on the build machine if it is missing:

```powershell
winget install --id JRSoftware.InnoSetup --exact --scope user
```

Build and package:

```powershell
cargo build --release --locked
py scripts/package_windows.py
```

The script discovers FFmpeg from PATH and Inno Setup from standard installation locations. To supply paths explicitly, replace the example paths below with actual locations on your machine:

```powershell
py scripts/package_windows.py --ffmpeg-dir "D:\dependencies\ffmpeg"
# Optional: explicitly specify the Inno Setup compiler
py scripts/package_windows.py --ffmpeg-dir "D:\dependencies\ffmpeg" --iscc "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
# Show discovered inputs without compiling installers
py scripts/package_windows.py --check
```

Outputs in `dist/windows/`:

- `ZSR-<version>-Windows-x64-Full-Setup.exe`
- `ZSR-<version>-Windows-x64-Lite-Setup.exe`

Python and Inno Setup are build tools; users of the installers do not need them. See [Windows installer details](docs/windows-installers.md).

### macOS

在 **Mac 上执行**，需要 Rust、Python 3.11+、Xcode Command Line Tools 和 `hdiutil`。将下方 `/path/to/ffmpeg-distribution` 替换为真实的 FFmpeg 发行目录；目录内需包含 `bin/ffmpeg`、依赖库和许可证文件，且架构需与 ZSR 一致。

Run on macOS with Xcode Command Line Tools and `hdiutil`. Replace `/path/to/ffmpeg-distribution` with the actual distribution root:

```sh
cargo build --release --locked
python3 scripts/package_unix.py --platform macos --ffmpeg-dir /path/to/ffmpeg-distribution
```

一次执行生成 Full 和 Lite 两个 `.dmg` 安装镜像，保存在 `dist/`。

Outputs in `dist/`:

- `ZSR-<version>-macOS-<arch>-Full.dmg`
- `ZSR-<version>-macOS-<arch>-Lite.dmg`

### Linux (Debian/Ubuntu)

在 **Linux 上执行**，需要 Rust、Python 3.11+、上文列出的开发依赖和 `dpkg-deb`。将下方 `/path/to/ffmpeg-distribution` 替换为真实的 FFmpeg 发行目录；目录内需包含 `bin/ffmpeg`、依赖库和许可证文件，且架构需与 ZSR 一致。

Run on Linux with the build dependencies listed above and `dpkg-deb`. Replace `/path/to/ffmpeg-distribution` with the actual distribution root:

```sh
cargo build --release --locked
python3 scripts/package_unix.py --platform linux --ffmpeg-dir /path/to/ffmpeg-distribution
```

一次执行生成 Full 和 Lite 两个 `.deb` 安装包，保存在 `dist/`。

Outputs in `dist/`:

- `ZSR-<version>-Linux-<arch>-Full.deb`
- `ZSR-<version>-Linux-<arch>-Lite.deb`

版本号读取自 `Cargo.toml`，`<arch>` 为 `amd64` 或 `arm64`。这些脚本不能在 Windows 上直接生成 macOS/Linux 安装包；需要在对应系统上构建。macOS 包目前未签名、未公证；macOS/Linux 打包流程尚待在对应系统上验证。

Version comes from `Cargo.toml`; Unix architecture labels are `amd64` or `arm64`. Build on the target OS with matching binaries: these scripts do not cross-compile. macOS packages are unsigned/unnotarized, and native macOS/Linux packages still require platform validation. See [macOS/Linux packaging details](docs/unix-installers.md).

## Structure

| Path | Responsibility |
| --- | --- |
| `src/app/` | Application controller, separate UI panels, appearance, and preferences |
| `src/audio/` | Shared device inventory, asynchronous discovery, and platform-specific parsers |
| `src/recording/` | Recording API, validation, command construction, session lifecycle, and worker |
| `src/platform.rs` | Capability checks, native display boundary, subprocess setup |
| `src/settings.rs` | Preferences and validation |
| `src/i18n.rs`, `locales/` | Translation lookup and English/Chinese catalogs |
| `native/` | C ABI display discovery using platform system APIs |
| `tests/` | Opt-in synthetic media pipeline verification |

Capture commands are passed as argument arrays without a shell. No network service, account, or telemetry is required. FFmpeg errors can include device identifiers and local paths; review diagnostics before sharing.

The `obs-studio/` source is available and its monitor capture and preview-cropping implementation has been reviewed; see [OBS reference review](docs/obs-reference.md). No OBS code was copied or linked. Before shipping installers, settle the project license and review the licenses of the selected media dependencies and any bundled binaries.

Reference documentation: [eframe 0.33.3](https://docs.rs/eframe/0.33.3/eframe/), [FFmpeg capture devices](https://ffmpeg.org/ffmpeg-devices.html).

See [module boundaries](docs/architecture.md) for responsibilities and extension points.
