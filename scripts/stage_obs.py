"""Stage ZSR's helper inside a complete, matching libobs runtime distribution."""
import argparse
from pathlib import Path
import shutil


def stage_obs(runtime: Path, helper: Path, destination: Path, target: str = "windows") -> None:
    """Preserve runtime dependencies and licenses and install the matching helper."""
    library = {"windows": "bin/64bit/obs.dll", "linux": "lib/libobs.so", "macos": "lib/libobs.dylib"}[target]
    if not (runtime / library).is_file() or not helper.is_file():
        raise FileNotFoundError(f"Supply a libobs 32 runtime containing {library} and the built ZSR helper")
    if runtime.resolve() != destination.resolve():
        shutil.copytree(runtime, destination, dirs_exist_ok=True, ignore=shutil.ignore_patterns("*.pdb"))
    bin_dir = destination / ("bin/64bit" if target == "windows" else "bin")
    bin_dir.mkdir(parents=True, exist_ok=True)
    output = bin_dir / ("zsr-obs.exe" if target == "windows" else "zsr-obs")
    if helper.resolve() != output.resolve():
        shutil.copy2(helper, output)
    if target != "windows":
        output.chmod(0o755)
    (destination / "ZSR-OBS-SOURCE.txt").write_text(
        "OBS Studio / libobs 32.2.2\nhttps://github.com/obsproject/obs-studio/tree/32.2.2\n"
        "Runtime modules retain their upstream licenses. ZSR helper source is in the ZSR repository.\n",
        encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--obs-dir", required=True, type=Path)
    parser.add_argument("--helper", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--platform", choices=["windows", "linux", "macos"], default="windows")
    args = parser.parse_args()
    stage_obs(args.obs_dir, args.helper, args.output, args.platform)
