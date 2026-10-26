"""Build Full (bundled FFmpeg) and Lite Windows installers with Inno Setup 6."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tomllib
import tempfile
from package_runtime import stage_runtime

ROOT = Path(__file__).resolve().parents[1]


def find_compiler(explicit: str | None) -> str:
    """Find the developer's Inno Setup compiler without installing anything."""
    if explicit:
        path = Path(explicit)
        if path.is_dir():
            path /= "ISCC.exe"
        if not path.is_file():
            raise FileNotFoundError(f"指定的 Inno Setup 编译器不存在：{path}")
        return str(path.resolve())
    candidates = [os.environ.get("ISCC"), shutil.which("ISCC.exe")]
    for variable in ("ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"):
        base = os.environ.get(variable)
        if base:
            candidates += [str(Path(base) / "Inno Setup 6/ISCC.exe"),
                           str(Path(base) / "Programs/Inno Setup 6/ISCC.exe")]
            candidates += [str(path) for pattern in ("Inno Setup */ISCC.exe", "Programs/Inno Setup */ISCC.exe")
                           for path in Path(base).glob(pattern)]
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return str(Path(candidate).resolve())
    raise FileNotFoundError(
        "找不到 Inno Setup 编译器（与 Python 版本无关）。\n"
        "请运行：winget install --id JRSoftware.InnoSetup --exact --scope user\n"
        "或指定：--iscc \"实际安装目录\\ISCC.exe\"")


def find_ffmpeg(explicit: Path | None) -> Path:
    """Resolve a complete distribution from an explicit root, bin folder, executable, or PATH."""
    path = explicit
    if path is None:
        executable = shutil.which("ffmpeg.exe")
        if not executable:
            raise FileNotFoundError("找不到 FFmpeg，请用 --ffmpeg-dir 指定完整发行目录，而不是示例路径。")
        path = Path(executable)
    if not path.exists():
        raise FileNotFoundError(f"FFmpeg 路径不存在：{path}\n可省略 --ffmpeg-dir，让脚本从 PATH 自动查找。")
    if path.is_file():
        path = path.parent
    if path.name.lower() == "bin":
        path = path.parent
    return path.resolve()


def package(binary: Path, ffmpeg: Path, output: Path, compiler: str) -> None:
    """Validate inputs, then compile both editions from the same application binary."""
    if not binary.is_file():
        raise FileNotFoundError(f"Build ZSR first: {binary}")
    if not (ffmpeg / "bin/ffmpeg.exe").is_file():
        raise FileNotFoundError("--ffmpeg-dir must contain bin/ffmpeg.exe and its distribution files")
    if not any(ffmpeg.glob("*LICENSE*")) and not any(ffmpeg.glob("*COPYING*")):
        raise ValueError("Supply the complete FFmpeg distribution including its license notices")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="zsr-runtime-") as temporary:
        runtime = Path(temporary) / "ffmpeg"
        stage_runtime(ffmpeg, runtime)
        for edition in ("Full", "Lite"):
            subprocess.run([compiler, f"/DEdition={edition}", f"/DAppVersion={version}",
                            f"/DSourceRoot={ROOT}", f"/DAppBinary={binary}",
                            f"/DFFmpegRoot={runtime}", f"/DOutputPath={output}",
                            str(ROOT / "packaging/windows/ZSR.iss")], check=True)
            expected = output / f"ZSR-{version}-Windows-x64-{edition}-Setup.exe"
            if not expected.is_file():
                raise RuntimeError(f"Compiler did not produce {expected}")
            print(expected)



if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/ZSR.exe")
    parser.add_argument("--ffmpeg-dir", type=Path, help="FFmpeg distribution; defaults to discovery from PATH")
    parser.add_argument("--output", type=Path, default=ROOT / "dist/windows")
    parser.add_argument("--iscc")
    parser.add_argument("--check", action="store_true", help="show resolved build inputs without packaging")
    args = parser.parse_args()
    try:
        compiler = find_compiler(args.iscc)
        ffmpeg = find_ffmpeg(args.ffmpeg_dir)
        print(f"Inno Setup: {compiler}\nZSR: {args.binary.resolve()}\nFFmpeg: {ffmpeg}", flush=True)
        if not args.check:
            package(args.binary.resolve(), ffmpeg, args.output.resolve(), compiler)
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"打包失败：{error}\n")
