"""Exercise release gates and publication ordering without contacting GitHub."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import release


class ReleaseTests(unittest.TestCase):
    """Prevent incomplete, mismatched or prematurely published releases."""
    def test_tag_matches_version(self):
        """Accept matching stable/prerelease tags and reject mismatched versions."""
        release.validate_tag("v0.1.0", "0.1.0")
        release.validate_tag("v0.2.0-rc.1", "0.2.0-rc.1")
        with self.assertRaises(ValueError):
            release.validate_tag("v9.0.0", "0.1.0")

    def test_assets_require_all_editions_and_generate_checksums(self):
        """Do not publish when either platform/edition is missing or empty."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaises(FileNotFoundError):
                release.prepare_assets(root, "0.1.0")
            for edition in ("Full", "Lite"):
                (root / f"ZSR-0.1.0-Windows-x64-{edition}-Setup.exe").write_bytes(b"abc")
                (root / f"ZSR-0.1.0-Linux-amd64-{edition}.deb").write_bytes(b"abc")
            assets = release.prepare_assets(root, "0.1.0")
            self.assertEqual(len(assets), 5)
            self.assertEqual(len(assets[-1].read_text().splitlines()), 4)
            self.assertIn("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", assets[-1].read_text())
            assets[0].write_bytes(b"")
            with self.assertRaises(FileNotFoundError):
                release.prepare_assets(root, "0.1.0")

    def test_upload_precedes_publication_and_marks_prerelease(self):
        """Create a draft and upload every asset before making a prerelease public."""
        with patch.object(release.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "")) as run:
            release.publish("v0.2.0-rc.1", [Path("asset")])
            commands = [call.args[0] for call in run.call_args_list]
            self.assertIn("--draft", commands[1])
            self.assertIn("--verify-tag", commands[1])
            self.assertEqual(commands[2][:3], ["gh", "release", "upload"])
            self.assertIn("--draft=false", commands[3])
            self.assertIn("--prerelease=true", commands[3])
            self.assertIn("--latest=false", commands[3])

    def test_failed_upload_leaves_draft(self):
        """An upload failure must not publish an incomplete release."""
        with patch.object(release.subprocess, "run", side_effect=[subprocess.CompletedProcess([], 0, '{"isDraft":true}'), subprocess.CalledProcessError(1, "upload")]) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                release.publish("v0.1.0", [Path("asset")])
            self.assertEqual(run.call_count, 2)

    def test_published_release_is_not_overwritten(self):
        """Reruns may repair drafts but cannot replace publicly released downloads."""
        with patch.object(release.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, '{"isDraft":false}')) as run:
            with self.assertRaises(RuntimeError):
                release.publish("v0.1.0", [Path("asset")])
            self.assertEqual(run.call_count, 1)
