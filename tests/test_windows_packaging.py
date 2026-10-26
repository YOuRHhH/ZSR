"""Check packaging input discovery without installing or invoking build tools."""
import importlib.util
import os
from pathlib import Path
import tempfile
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))

SPEC = importlib.util.spec_from_file_location(
    "package_windows", Path(__file__).resolve().parents[1] / "scripts/package_windows.py")
PACKAGING = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKAGING)


class DiscoveryTests(unittest.TestCase):
    """Cover actual folder layouts and actionable failures for missing paths."""

    def test_explicit_compiler_directory_and_invalid_override(self):
        """Accept a compiler folder but never silently ignore an invalid explicit path."""
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            compiler = folder / "ISCC.exe"
            compiler.touch()
            self.assertEqual(PACKAGING.find_compiler(str(folder)), str(compiler.resolve()))
            with self.assertRaises(FileNotFoundError):
                PACKAGING.find_compiler(str(folder / "missing.exe"))

    def test_per_user_versioned_compiler(self):
        """Find versioned per-user installations outside PATH."""
        with tempfile.TemporaryDirectory() as temporary:
            compiler = Path(temporary) / "Programs/Inno Setup 7/ISCC.exe"
            compiler.parent.mkdir(parents=True)
            compiler.touch()
            with patch.dict(os.environ, {"LOCALAPPDATA": temporary}, clear=True), patch.object(PACKAGING.shutil, "which", return_value=None):
                self.assertEqual(PACKAGING.find_compiler(None), str(compiler.resolve()))

    def test_obs_requires_an_explicit_complete_runtime(self):
        """Never mistake an FFmpeg installation for the OBS distribution."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            library = root / "bin/64bit/obs.dll"
            library.parent.mkdir(parents=True)
            library.touch()
            self.assertEqual(PACKAGING.find_obs(root), root.resolve())
            with self.assertRaises(FileNotFoundError):
                PACKAGING.find_obs(None)
            with self.assertRaises(FileNotFoundError):
                PACKAGING.find_obs(root / "missing")


if __name__ == "__main__":
    unittest.main()
