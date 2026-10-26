"""Fetch the checksum-pinned official OBS Windows runtime for ZSR development."""
import hashlib
from pathlib import Path
import urllib.request
import zipfile

VERSION = "32.2.2"
SHA256 = "4d6e40e3ab155f56b30de517380566a206d74b63cdf5ad49aa596924768f97e1"
URL = f"https://github.com/obsproject/obs-studio/releases/download/{VERSION}/OBS-Studio-{VERSION}-Windows-x64.zip"
ROOT = Path(__file__).resolve().parents[1] / "target/obs-runtime"


def fetch() -> None:
    """Verify the complete archive before extracting anything into the development runtime."""
    ROOT.mkdir(parents=True, exist_ok=True)
    archive = ROOT / "obs.zip"
    if not archive.is_file():
        urllib.request.urlretrieve(URL, archive)
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if digest != SHA256:
        raise ValueError(f"OBS checksum mismatch; remove {archive} and retry")
    destination = ROOT / VERSION
    with zipfile.ZipFile(archive) as bundle:
        for entry in bundle.infolist():
            if not (destination / entry.filename).resolve().is_relative_to(destination.resolve()):
                raise ValueError("Archive member escapes the destination")
        bundle.extractall(destination)
    print(destination)


if __name__ == "__main__":
    fetch()
