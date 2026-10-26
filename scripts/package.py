"""One-command native Full/Lite packaging for Windows and Linux."""
import argparse
import os
from pathlib import Path
import platform
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def host_platform() -> str:
    """Select a supported native host; this command does not cross-compile installers."""
    name = {"Windows": "windows", "Linux": "linux"}.get(platform.system())
    if not name:
        raise RuntimeError("This entry point supports Windows and Linux")
    return name


def package(obs_dir: Path | None, output: Path | None, iscc: str | None, skip_build: bool) -> None:
    """Prepare OBS, build both Rust binaries, then invoke the existing native packager."""
    target = host_platform()
    if obs_dir is None:
        if target == "windows":
            from fetch_obs_windows import fetch, ROOT as CACHE, VERSION
            fetch()
            obs_dir = CACHE / VERSION
        else:
            from fetch_obs_linux import fetch
            obs_dir = fetch()
    if not skip_build:
        subprocess.run(["cargo", "build", "--locked", "--release", "--bins"], cwd=ROOT, check=True)
    build = Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))
    if not build.is_absolute():
        build = ROOT / build
    binary = build / "release" / ("ZSR.exe" if target == "windows" else "ZSR")
    output = (output or ROOT / "dist" / target).resolve()
    if target == "windows":
        from package_windows import package as native_package, find_compiler
        native_package(binary, obs_dir.resolve(), output, find_compiler(iscc))
    else:
        from package_unix import package as native_package
        native_package(binary, obs_dir.resolve(), output, "linux")


def main() -> None:
    """Expose optional runtime/output overrides while keeping the default command argument-free."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--obs-dir", type=Path, help="Use an existing normalized runtime instead of downloading")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--iscc", help="Windows Inno Setup compiler path")
    parser.add_argument("--skip-build", action="store_true", help="Package existing release binaries")
    args = parser.parse_args()
    try:
        package(args.obs_dir, args.output, args.iscc, args.skip_build)
    except (OSError, RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Packaging failed: {error}\n")


if __name__ == "__main__":
    main()
