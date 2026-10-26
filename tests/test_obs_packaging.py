"""Check OBS staging preserves its runtime and installs the separate helper."""
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from stage_obs import stage_obs


class ObsStagingTests(unittest.TestCase):
    """Keep runtime data and notices while rejecting incomplete inputs."""
    def test_staging_preserves_runtime_and_adds_helper(self):
        """Copy modules/data/licenses, omit debug symbols and install the matching helper."""
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            runtime = base / "source"
            for file in ["bin/64bit/obs.dll", "data/obs-studio/license/gpl.txt", "obs-plugins/64bit/win-capture.dll"]:
                path = runtime / file
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            (runtime / "debug.pdb").write_bytes(b"symbols")
            helper = base / "zsr-obs.exe"
            helper.write_bytes(b"helper")
            destination = base / "output"
            stage_obs(runtime, helper, destination)
            self.assertEqual((destination / "bin/64bit/zsr-obs.exe").read_bytes(), b"helper")
            self.assertTrue((destination / "data/obs-studio/license/gpl.txt").is_file())
            self.assertFalse((destination / "debug.pdb").exists())
            with self.assertRaises(FileNotFoundError):
                stage_obs(base / "missing", helper, base / "invalid")
