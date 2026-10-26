# Windows installer editions

One packaging command produces two installers from the same application build:

| Edition | Output | FFmpeg behavior |
| --- | --- | --- |
| Full | `ZSR-0.1.0-Windows-x64-Full-Setup.exe` | Includes FFmpeg and automatically uses the private bundled executable |
| Lite | `ZSR-0.1.0-Windows-x64-Lite-Setup.exe` | Excludes FFmpeg; uses PATH or the executable configured in ZSR |

## Build

The build machine needs Rust/MSVC, Python 3.11+, and Inno Setup 6. End users do not need these tools. Prepare a complete Windows x64 FFmpeg distribution with `bin/ffmpeg.exe` and its license notices; include the matching source/provenance material needed for redistribution. Use a build with libx264, gdigrab, dshow, and gfxcapture with HWND support.

```powershell
cargo build --release --locked
py scripts/package_windows.py
```

Omit `--ffmpeg-dir` to discover the FFmpeg distribution from PATH, or pass its real location explicitly. `py scripts/package_windows.py --check` displays resolved paths. Do not copy placeholder paths from examples. The script discovers `ISCC.exe` or accepts `--iscc "C:\path\ISCC.exe"`. Optional `--binary` and `--output` override the release executable and `dist/windows` directory. Versioned filenames follow Cargo.toml. It validates inputs before compiling both editions, stages the supplied FFmpeg runtime into Full, and never downloads software or modifies PATH.

Both editions install per-user and share an application ID so switching editions upgrades ZSR. The installer replaces its own `ffmpeg` directory; Full-to-Lite removes the bundled engine. User recordings and preferences remain outside that directory. ZSR resolves the bundled engine relative to its executable; Lite retains external-engine configuration. A missing absolute backend path resets to PATH lookup on startup.

The installer is currently unsigned. Runtime bundle lookup has automated coverage. Both Full and Lite installers were successfully compiled locally with Inno Setup 6.7.3 on 2026-10-26. Installation, edition switching, and uninstall behavior still need end-to-end validation; compilation alone does not validate those operations.

## Size optimization

Release builds use size optimization, whole-program LTO, one code generation unit, and stripped symbols. This increases build time but reduces the shipped executable. Full packages omit only the unused `bin/ffplay` and `bin/ffprobe` companion tools (including `.exe` variants); FFmpeg itself, shared libraries, resources, license notices, and source/provenance material are retained. The original distribution is never modified. Lite continues to exclude the entire FFmpeg bundle.
