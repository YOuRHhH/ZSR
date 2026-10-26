"""Fetch and normalize the official OBS runtime for Ubuntu 24.04 x86_64."""
import hashlib
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
VERSION = "32.2.2"
SHA256 = "b6557ca2059287210332accc94c267094050489cffffc5c832e746e3a418dab0"
URL = f"https://github.com/obsproject/obs-studio/releases/download/{VERSION}/OBS-Studio-{VERSION}-Ubuntu-24.04-x86_64.deb"


def fetch() -> Path:
    """Verify the official DEB, preserve runtime libraries/data/licenses and record dependencies."""
    if platform.system() != "Linux" or platform.machine().lower() not in ("x86_64", "amd64"):
        raise RuntimeError("Automatic OBS Linux downloads require Ubuntu 24.04 x86_64; use --obs-dir elsewhere")
    cache = ROOT / "target/obs-runtime"
    cache.mkdir(parents=True, exist_ok=True)
    archive = cache / "obs-linux.deb"
    if not archive.is_file():
        temporary = archive.with_suffix(".download")
        urllib.request.urlretrieve(URL, temporary)
        temporary.replace(archive)
    with archive.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != SHA256:
            raise ValueError(f"OBS checksum mismatch; remove {archive} and retry")
    destination = cache / f"{VERSION}-linux"
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="zsr-obs-deb-") as temporary:
        extracted = Path(temporary)
        subprocess.run(["dpkg-deb", "-x", str(archive), str(extracted)], check=True)
        prefix = extracted / "usr/local" if (extracted / "usr/local/share/obs").is_dir() else extracted / "usr"
        library = destination / "lib"
        library.mkdir(exist_ok=True)
        # Copy OBS shared libraries without Debian's architecture directory nesting.
        for parent in [prefix / "lib", prefix / "lib/x86_64-linux-gnu"]:
            for file in parent.glob("*.so*"):
                if file.is_file():
                    shutil.copy2(file, library / file.name)
        for name in ("libobs", "libobs-opengl"):
            if not (library / f"{name}.so").exists():
                candidates = sorted(library.glob(f"{name}.so.*"))
                if not candidates:
                    raise FileNotFoundError(f"OBS package is missing {name}")
                shutil.copy2(candidates[0], library / f"{name}.so")
        modules = library / "obs-plugins"
        modules.mkdir(exist_ok=True)
        for file in (prefix / "lib").rglob("*.so"):
            if "obs-plugins" in file.parts or "obs-modules" in file.parts:
                shutil.copy2(file, modules / file.name)
        shutil.copytree(prefix / "share/obs", destination / "data", dirs_exist_ok=True)
        for document_root in (prefix / "share/doc", extracted / "usr/share/doc"):
            if document_root.is_dir():
                shutil.copytree(document_root, destination / "licenses", dirs_exist_ok=True)
        required = ["linux-capture", "linux-pulseaudio", "obs-x264", "obs-ffmpeg", "obs-outputs", "obs-filters"]
        for name in required:
            if not (modules / f"{name}.so").is_file():
                raise FileNotFoundError(f"OBS package is missing required module {name}")
    dependencies = subprocess.check_output(["dpkg-deb", "-f", str(archive), "Depends"], text=True).strip()
    (destination / "linux-dependencies.txt").write_text(dependencies + "\n", encoding="utf-8")
    (destination / "ZSR-OBS-SOURCE.txt").write_text(f"OBS {VERSION}\nhttps://github.com/obsproject/obs-studio/tree/{VERSION}\n", encoding="utf-8")
    return destination


if __name__ == "__main__":
    print(fetch())
