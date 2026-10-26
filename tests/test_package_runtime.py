"""Verify compact bundles preserve executable dependencies and distribution notices."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from package_runtime import stage_runtime


class RuntimeTests(unittest.TestCase):
    """Exercise both native executable naming conventions and retained payloads."""

    def test_runtime_keeps_libraries_and_notices(self):
        """Drop unused tools without dropping DLLs, libraries, licenses or nested resources."""
        for suffix in ("", ".exe"):
            with self.subTest(suffix=suffix), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                source = root / "source"
                retained = [f"bin/ffmpeg{suffix}", "bin/avcodec.dll", "lib/libavcodec.so.1",
                            "LICENSE", "doc/COPYING", "resources/ffprobe"]
                removed = [f"bin/ffprobe{suffix}", f"bin/ffplay{suffix}"]
                for name in retained + removed:
                    path = source / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(name.encode())
                stage_runtime(source, root / "staged")
                for name in retained:
                    self.assertEqual((root / "staged" / name).read_bytes(), name.encode())
                for name in removed:
                    self.assertFalse((root / "staged" / name).exists())
                    self.assertTrue((source / name).is_file())


if __name__ == "__main__":
    unittest.main()
