# macOS and Linux packages

Both platforms provide Full (bundled FFmpeg) and Lite (external FFmpeg) editions. Run on the target OS with Python 3.11+, a native release build, and a complete FFmpeg distribution for the same architecture. `bin/ffmpeg` must be executable with bundled dependencies resolved relative to the distribution; do not copy a dynamically linked binary whose libraries exist only on the build machine. Preserve the distribution's license and matching source/provenance materials.

```sh
cargo build --release --locked
# On macOS:
python3 scripts/package_unix.py --platform macos --ffmpeg-dir /path/to/ffmpeg-distribution
# On Debian/Ubuntu Linux, with dpkg-deb installed:
python3 scripts/package_unix.py --platform linux --ffmpeg-dir /path/to/ffmpeg-distribution
```

Outputs in `dist/`:

| Platform | Full | Lite |
| --- | --- | --- |
| macOS | `ZSR-<version>-macOS-<arch>-Full.dmg` | `ZSR-<version>-macOS-<arch>-Lite.dmg` |
| Linux | `ZSR-<version>-Linux-<arch>-Full.deb` | `ZSR-<version>-Linux-<arch>-Lite.deb` |

macOS DMGs contain ZSR.app and an Applications shortcut. Full embeds FFmpeg under Contents/MacOS/ffmpeg. Lite uses the configured external executable or PATH; Finder launches may need an explicit external executable path. These bundles are unsigned and unnotarized; public macOS distribution requires signing the nested binaries and app, then notarizing the release.

Linux Full installs its private engine under /usr/lib/zsr/ffmpeg. Lite declares a dependency on the distribution's ffmpeg package, which the package manager can install. Both editions use the same Debian package name and upgrade/replace each other. Install with `sudo apt install ./ZSR-*.deb`, specifying exactly one edition. Native runtime dependencies must be qualified on the supported distribution baseline; RPM/AppImage packaging is not implemented.

These scripts do not cross-compile. Build for the host architecture (amd64 or arm64); do not pass a binary for a different architecture. The current Windows development environment can validate Python syntax and bundle lookup but cannot generate or qualify native DMG/DEB installers. macOS/Linux capture support remains as described in implementation.md.

## Size optimization

Release builds use size optimization, whole-program LTO, one code generation unit, and stripped symbols. This increases build time but reduces the shipped executable. Full packages omit only the unused `bin/ffplay` and `bin/ffprobe` companion tools (including `.exe` variants); FFmpeg itself, shared libraries, resources, license notices, and source/provenance material are retained. The original distribution is never modified. Lite continues to exclude the entire FFmpeg bundle.
