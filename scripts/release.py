"""Validate version-tagged packages and optionally publish them as a GitHub Release."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def validate_tag(tag: str, version: str) -> None:
    """Require the release tag to match Cargo's package version exactly."""
    if tag != f"v{version}":
        raise ValueError(f"Release tag {tag!r} must match Cargo.toml version: v{version}")


def prepare_assets(folder: Path, version: str) -> list[Path]:
    """Require both editions for both platforms and write reproducible SHA-256 checksums."""
    names = [f"ZSR-{version}-Windows-x64-{edition}-Setup.exe" for edition in ("Full", "Lite")]
    names += [f"ZSR-{version}-Linux-amd64-{edition}.deb" for edition in ("Full", "Lite")]
    files = [folder / name for name in sorted(names)]
    for file in files:
        if not file.is_file() or file.stat().st_size == 0:
            raise FileNotFoundError(f"Release package missing or empty: {file}")
    lines = []
    for file in files:
        with file.open("rb") as stream:
            lines.append(f"{hashlib.file_digest(stream, 'sha256').hexdigest()}  {file.name}\n")
    checksum = folder / "SHA256SUMS.txt"
    checksum.write_text("".join(lines), encoding="utf-8", newline="\n")
    return files + [checksum]


def publish(tag: str, assets: list[Path]) -> None:
    """Upload to a draft first; publish only after all uploads succeed, never overwrite public releases."""
    existing = subprocess.run(["gh", "release", "view", tag, "--json", "isDraft"], capture_output=True, text=True)
    if existing.returncode == 0:
        if not json.loads(existing.stdout)["isDraft"]:
            raise RuntimeError(f"Release {tag} is already published; use a new version/tag")
    else:
        command = ["gh", "release", "create", tag, "--verify-tag", "--draft", "--title", f"ZSR {tag}", "--generate-notes"]
        if "-" in tag:
            command.append("--prerelease")
        subprocess.run(command, check=True)
    # --clobber permits recovery of an interrupted draft upload, never a public release.
    subprocess.run(["gh", "release", "upload", tag, *map(str, assets), "--clobber"], check=True)
    command = ["gh", "release", "edit", tag, "--draft=false", f"--prerelease={'true' if '-' in tag else 'false'}"]
    if "-" in tag:
        command.append("--latest=false")
    subprocess.run(command, check=True)


def main() -> None:
    """Default to validation only; publishing requires an explicit flag and complete packages."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--assets", type=Path)
    parser.add_argument("--publish", action="store_true")
    args = parser.parse_args()
    try:
        version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
        validate_tag(args.tag, version)
        if args.publish and args.assets is None:
            raise ValueError("--publish requires --assets")
        if args.assets is not None:
            assets = prepare_assets(args.assets.resolve(), version)
            if args.publish:
                publish(args.tag, assets)
        print(f"Release validation passed: {args.tag}")
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Release failed: {error}\n")


if __name__ == "__main__":
    main()
