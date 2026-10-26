"""Build Full/Lite DMG packages on macOS or DEB packages on Linux."""
import argparse
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import tomllib
from package_app import ROOT, stage
from package_runtime import stage_runtime


def package(binary: Path, ffmpeg: Path, output: Path, target: str) -> None:
    """Stage both editions, preserving FFmpeg distribution files, then invoke the native packager."""
    expected_os = "Darwin" if target == "macos" else "Linux"
    if platform.system() != expected_os:
        raise RuntimeError(f"Build {target} packages on {expected_os} with a binary for that OS")
    tool = shutil.which("hdiutil" if target == "macos" else "dpkg-deb")
    if not tool:
        raise RuntimeError("Native packager missing: hdiutil (macOS) or dpkg-deb (Linux)")
    if not binary.is_file() or not (ffmpeg / "bin/ffmpeg").is_file():
        raise FileNotFoundError("Supply ZSR and an FFmpeg distribution containing bin/ffmpeg")
    if not any(ffmpeg.glob("*LICENSE*")) and not any(ffmpeg.glob("*COPYING*")):
        raise ValueError("FFmpeg distribution must include its license notices")
    machine = platform.machine().lower()
    architectures = {"x86_64": "amd64", "amd64": "amd64", "aarch64": "arm64", "arm64": "arm64"}
    if machine not in architectures:
        raise ValueError(f"Unsupported package architecture: {machine}")
    architecture = architectures[machine]
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    output.mkdir(parents=True, exist_ok=True)
    for edition in ("Full", "Lite"):
        with tempfile.TemporaryDirectory(prefix="zsr-package-") as temporary:
            folder = Path(temporary) / "payload"
            stage(binary, folder, target)
            if edition == "Full":
                runtime = folder / ("ZSR.app/Contents/MacOS/ffmpeg" if target == "macos" else "usr/lib/zsr/ffmpeg")
                stage_runtime(ffmpeg, runtime)
                (runtime / "bin/ffmpeg").chmod(0o755)
            if target == "macos":
                # Let users install the application by dragging it into Applications.
                (folder / "Applications").symlink_to("/Applications")
                artifact = output / f"ZSR-{version}-macOS-{architecture}-{edition}.dmg"
                subprocess.run([tool, "create", "-volname", f"ZSR {edition}", "-srcfolder", str(folder),
                                "-format", "UDZO", "-ov", str(artifact)], check=True)
            else:
                control = folder / "DEBIAN"
                control.mkdir()
                dependencies = "libc6, libgcc-s1, libx11-6, libxcb1, libxkbcommon0, libwayland-client0, libegl1, libgl1, wmctrl"
                if edition == "Lite":
                    dependencies += ", ffmpeg"
                (control / "control").write_text(
                    f"Package: zsr\nVersion: {version}\nArchitecture: {architecture}\n"
                    "Maintainer: ZSR maintainers\nSection: video\nPriority: optional\n"
                    f"Depends: {dependencies}\nDescription: ZSR screen recorder ({edition} edition)\n",
                    encoding="utf-8")
                artifact = output / f"ZSR-{version}-Linux-{architecture}-{edition}.deb"
                subprocess.run([tool, "--build", "--root-owner-group", str(folder), str(artifact)], check=True)
            print(artifact)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=["macos", "linux"], required=True)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/ZSR")
    parser.add_argument("--ffmpeg-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    package(args.binary.resolve(), args.ffmpeg_dir.resolve(), args.output.resolve(), args.platform)
