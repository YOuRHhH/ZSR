# ZSR

ZSR is a Rust desktop screen recorder with C platform adapters and an English/Chinese interface. Recording now uses **OBS libobs**, through the isolated `zsr-obs` helper. ZSR no longer runs the FFmpeg command-line application for capture, encoding or device discovery. OBS itself includes codec libraries, including FFmpeg libraries.

## Windows development

Requires Rust 1.88+, MSVC Desktop development with C++, Python 3.11+, and a matching OBS runtime. The tested runtime is official OBS Studio **32.2.2 x64**; the helper checks the libobs major version before use.

```powershell
# Download and verify the official OBS runtime once.
py scripts/fetch_obs_windows.py

# Build BOTH the UI and the native recording helper.
cargo build --bins
py scripts/stage_obs.py --obs-dir target/obs-runtime/32.2.2 --helper target/debug/zsr-obs.exe --output target/obs-runtime/32.2.2
cargo run --bin ZSR
```

Close a running ZSR before rebuilding its executable. Windows cannot replace an executable that is still running. An isolated build is also possible:

```powershell
cargo build --bins --target-dir target/region-verify
py scripts/stage_obs.py --obs-dir target/obs-runtime/32.2.2 --helper target/region-verify/debug/zsr-obs.exe --output target/obs-runtime/32.2.2
.\target\region-verify\debug\ZSR.exe
```

Re-stage the helper after changing its code. You can override runtime discovery with `ZSR_OBS_RUNTIME`, or choose **Settings → Advanced → OBS runtime folder**. A bundled `obs` directory beside ZSR takes precedence over the development runtime when no explicit override is supplied.

**从 FFmpeg 迁移后，请重新选择麦克风。** OBS 使用系统原生设备标识，与原来的 DirectShow 标识不同。软件不会擅自替换失效的设备选择。

## Recording

- Choose **Display**, **Region**, or **Application window**, then **Start recording**.
- Region selection shows your desktop, temporarily minimizes ZSR, and restores it after confirmation. The confirmed border stays visible.
- Enable desktop audio to capture the system default playback device, and/or select a microphone from the automatically detected list. Windows audio uses OBS WASAPI loopback and microphone capture.
- Choose **Mixed single track** to combine enabled sources, or **Separate audio tracks** for desktop audio on track 1 and microphone on track 2 in the same MP4. With only one source enabled, one track is saved. Most players play one track at a time; choose mixed mode for playback of both together. Desktop capture starts disabled so existing users do not unexpectedly record system sound.
- Output sizes include Original, 480p, 720p, 1080p, 1440p and 2160p, at 30 or 60 fps. Content fits within the output without stretching.
- Windows tries OBS AMF/NVENC/QSV hardware H.264 encoders and falls back to x264. The settings checkbox can force software encoding.
- **Stop and save** finalizes a native OBS hybrid MP4. Interrupted output is retained as `.partial.mp4`; it is not presented as a successful recording.
- The **Recordings** list opens completed files or their folder. Language, output preferences and press-to-enter global shortcuts remain in Settings.
- Native OBS logs are saved beside recordings. They stay local and may contain device names and paths.

OBS window capture uses its title/class/executable matching scheme. ZSR validates the selected window and rejects ambiguous title/executable choices; keep the target open and visible. Pause/resume and Wayland portal capture remain unimplemented. With both audio sources disabled, OBS's native MP4 output currently contains a silent AAC track.

## Build and package

Full packages include the OBS runtime and helper. Lite packages include the helper and require a compatible external OBS runtime configured in Settings. These editions replace the former bundled/unbundled FFmpeg editions. Runtime licenses and source references are retained in the Full package.

### One-command Windows / Linux packaging

Run the same script on the platform you want to package:

```powershell
# Windows x64 (Rust/MSVC, Python 3.11+, Inno Setup 6 required)
py scripts/package.py
```

```sh
# Ubuntu 24.04 x86_64 (Rust, Python 3.11+, native build dependencies required)
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libxkbcommon-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev
python3 scripts/package.py
```

The command downloads and SHA-256-verifies OBS 32.2.2, builds both Rust binaries in release mode, and creates **Full and Lite** packages in `dist/windows/` or `dist/linux/`. It never installs the generated packages or changes system dependencies automatically. Optional arguments: `--obs-dir PATH`, `--output PATH`, `--iscc PATH` (Windows), and `--skip-build` for existing release binaries. `CARGO_TARGET_DIR` is respected. This is native packaging, not cross-compilation: a Windows invocation does not also build a Linux binary.

The automatic Linux runtime targets **Ubuntu 24.04 x86_64**. Full DEBs bundle OBS and declare its required system libraries; install using `sudo apt install ./dist/linux/*Full.deb` so apt resolves those dependencies. Lite omits OBS: prepare a compatible normalized runtime using `python3 scripts/fetch_obs_linux.py`, configure that folder in ZSR, and install its dependencies. A distribution's arbitrary `obs-studio` version is not guaranteed compatible. Other Linux distributions/architectures require a matching `--obs-dir` runtime containing `linux-dependencies.txt` for DEB dependency metadata.

### GitHub Actions

[Packaging workflow](.github/workflows/package.yml) runs Windows and Ubuntu jobs on pushes to `main`, `v*` tags, pull requests, or manual **Actions → Package Windows and Linux → Run workflow**. Each runner uses `python scripts/package.py`. Download **ZSR-windows-Full-Lite** and **ZSR-linux-Full-Lite** from the completed run's Artifacts section (retained for 14 days). Branch/PR/manual runs upload workflow artifacts only. Pushing a matching version tag also publishes a GitHub Release after both platform jobs pass. Installers are not code-signed. Linux CI installs the Full package and checks OBS startup under Xvfb; this does not replace testing capture on a real desktop.

### Publishing a GitHub Release

Set the package version in `Cargo.toml`, update `Cargo.lock` with Cargo, and commit/push the version change. Then push a matching tag (example for the current version):

```sh
git tag -a v0.1.0 -m "ZSR 0.1.0"
git push github v0.1.0
```

In a fresh GitHub clone the remote is usually `origin`; use `git push origin v0.1.0` instead. The tag must exactly match `v` plus the Cargo package version. CI builds and verifies both platforms, then uploads all four installers and `SHA256SUMS.txt` to a draft release with generated release notes. It publishes only after every upload succeeds. Tags containing a prerelease suffix (for example `v0.2.0-rc.1`) produce prereleases. No extra token secret is needed: only the release job receives `contents: write` on `GITHUB_TOKEN`.

If an upload fails, rerun the failed release job to complete the draft. An already published release is not overwritten; publish a new version instead. Manual workflow runs remain build-only—even when a tag is selected. No version tag or release is created by committing the workflow itself.

### Windows (direct packager)

Install Inno Setup 6 on the **build machine**, then:

```powershell
cargo build --release --bins
py scripts/package_windows.py --obs-dir target/obs-runtime/32.2.2
# Optional compiler override:
py scripts/package_windows.py --obs-dir target/obs-runtime/32.2.2 --iscc "C:\Path\To\Inno Setup 6\ISCC.exe"
```

Outputs: `dist/windows/ZSR-0.1.0-Windows-x64-Full-Setup.exe` and `...-Lite-Setup.exe`. `cargo build` alone does not create installers. Existing installers in `dist` are not automatically rebuilt when source changes.

### macOS / Linux

Build on the destination OS. Linux currently requires X11. Install a C compiler and desktop development dependencies; on Debian/Ubuntu these include `build-essential pkg-config libxkbcommon-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev`. Window discovery additionally uses `wmctrl`.

The new OBS backend includes platform source/audio mappings, but **macOS and Linux runtime packaging and capture have not been verified on native machines**. Their packaging scripts expect a relocatable OBS SDK/runtime layout: `lib/libobs.so` (Linux) or `lib/libobs.dylib` (macOS), graphics backend libraries in `lib`, capture/encoder/output plugins in `lib/obs-plugins`, and data in `data/libobs` and `data/obs-plugins`. A stock macOS OBS `.app` is not directly this layout; prepare the matching runtime first.

```sh
cargo build --release --bins
# macOS: requires hdiutil and a correctly relocated/signed OBS runtime.
python3 scripts/package_unix.py --platform macos --obs-dir /path/to/obs-runtime
# Linux: requires dpkg-deb and a runtime built for the target distribution.
python3 scripts/package_unix.py --platform linux --obs-dir /path/to/obs-runtime
```

输出在 `dist/`。请在对应系统构建和验证；Windows 上的成功测试不代表 macOS/Linux 已通过验证。

## Verification

```powershell
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
py -m unittest discover -s tests -p "test_*.py"
# Opt-in: records display/audio and plays a quiet desktop test tone on Windows.
cargo test --test obs_recording -- --ignored --nocapture
# Opt-in: opens and records a disposable animated window.
cargo test --test obs_window -- --ignored --nocapture
```

The media integration test uses `ffprobe`/`ffmpeg` only to verify generated files; these tools are not needed by ZSR users. Historical FFmpeg command tests are available only with the `legacy-ffmpeg` feature; this feature does not select the old recording engine.

Architecture and validation: [OBS backend](docs/obs-backend.md). The local `obs-studio` reference checkout and downloaded runtime/build outputs are excluded from Git. Official runtime/source: [OBS Studio 32.2.2](https://github.com/obsproject/obs-studio/releases/tag/32.2.2).
