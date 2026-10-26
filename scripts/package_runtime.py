"""Stage FFmpeg runtime files while omitting unused companion applications."""
from pathlib import Path
import shutil


def stage_runtime(source: Path, destination: Path) -> None:
    """Keep libraries, resources and notices; omit only unused bin/ffplay and bin/ffprobe."""
    source = source.resolve()
    # Preserve a companion if an unusual distribution uses it as ffmpeg's target.
    engines = {(source / "bin" / name).resolve() for name in ("ffmpeg", "ffmpeg.exe")}

    def unused_tools(directory: str, names: list[str]) -> list[str]:
        """Exclude known companion tools only in bin, never similarly named libraries."""
        folder = Path(directory)
        if folder != source / "bin":
            return []
        return [name for name in names
                if name.lower() in {"ffplay", "ffplay.exe", "ffprobe", "ffprobe.exe"}
                and not (folder / name).is_dir()
                and (folder / name).resolve() not in engines]

    shutil.copytree(source, destination, symlinks=True, ignore=unused_tools)
