"""Stage native icon assets with a built binary; does not install or sign."""
import argparse
from pathlib import Path
import plistlib
import shutil

ROOT = Path(__file__).resolve().parents[1]


def stage(binary: Path, destination: Path, platform: str) -> None:
    """Copy the ZSR executable and native icons into a new platform staging tree."""
    if not binary.is_file():
        raise FileNotFoundError(binary)
    destination.mkdir(parents=True, exist_ok=False)
    if platform == "macos":
        contents = destination / "ZSR.app" / "Contents"
        executable = contents / "MacOS" / "ZSR"
        resources = contents / "Resources"
        executable.parent.mkdir(parents=True)
        resources.mkdir()
        shutil.copy2(binary, executable)
        executable.chmod(0o755)
        shutil.copy2(ROOT / "logo/macos/zsr.icns", resources / "zsr.icns")
        with (contents / "Info.plist").open("wb") as stream:
            plistlib.dump({
                "CFBundleName": "ZSR",
                "CFBundleDisplayName": "ZSR",
                "CFBundleIdentifier": "app.zsr.ZSR",
                "CFBundleExecutable": "ZSR",
                "CFBundleIconFile": "zsr.icns",
                "CFBundlePackageType": "APPL",
                "CFBundleShortVersionString": "0.1.0",
                "CFBundleVersion": "1",
                "NSHighResolutionCapable": True,
                "NSMicrophoneUsageDescription": "Record microphone audio with your screen recording.",
            }, stream)
    else:
        prefix = destination / "usr"
        (prefix / "bin").mkdir(parents=True)
        (prefix / "share/applications").mkdir(parents=True)
        shutil.copy2(binary, prefix / "bin/ZSR")
        (prefix / "bin/ZSR").chmod(0o755)
        shutil.copy2(ROOT / "packaging/linux/zsr.desktop", prefix / "share/applications/zsr.desktop")
        shutil.copytree(ROOT / "logo/linux/hicolor", prefix / "share/icons/hicolor")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=["macos", "linux"], required=True)
    parser.add_argument("--binary", type=Path, required=True, help="binary built for the selected platform")
    parser.add_argument("--destination", type=Path, required=True, help="new staging directory")
    args = parser.parse_args()
    stage(args.binary.resolve(), args.destination.resolve(), args.platform)
    print(args.destination.resolve())
