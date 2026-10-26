# Application icons

ZSR uses the supplied assets in `logo/` without modifying them.

| Platform | Runtime window icon | Packaged application icon |
| --- | --- | --- |
| Windows | `logo/windows/zsr-256.png` | `logo/windows/zsr.ico`, embedded in the executable by `build.rs` |
| macOS | `logo/macos/zsr.iconset/icon_128x128@2x.png` | `logo/macos/zsr.icns`, referenced by the app bundle's Info.plist |
| Linux | `logo/linux/hicolor/256x256/apps/zsr.png` | All supplied hicolor sizes and scalable SVG, referenced by `Icon=zsr` in the desktop entry |

Runtime PNGs are embedded in the binary, so launching from another directory does not require a separate logo folder. Windows resource compilation requires the platform resource compiler supplied with the Windows SDK or cross-compilation toolchain.

## macOS and Linux staging

Build on the target OS using `cargo build --release --locked`, then run one of:

```sh
python3 scripts/package_app.py --platform macos --binary target/release/ZSR --destination dist/macos
python3 scripts/package_app.py --platform linux --binary target/release/ZSR --destination dist/linux
```

Use a new destination directory for each invocation. The script refuses to overwrite an existing directory. It creates `ZSR.app` for macOS or a `usr/` staging tree for Linux. The macOS bundle supplies the Dock/Finder icon; a bare command-line binary does not have bundle metadata. The Linux desktop file expects `ZSR` on PATH and the icon files installed under the corresponding shared icon theme directory.

Staging does not install files, bundle FFmpeg, sign or notarize the macOS app, or create an installer. Use a binary built for the selected OS. Windows executable icon presence was verified locally through the shell icon API. macOS/Linux staging metadata and copied icons were checked, but their native Dock/launcher appearance has not been tested here.
