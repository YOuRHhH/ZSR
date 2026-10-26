"""Verify the unified packaging command dispatches safely without building installers."""
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import package


class UnifiedPackagingTests(unittest.TestCase):
    """Cover host selection, build failure propagation, and runtime overrides."""
    def test_windows_builds_both_binaries_and_uses_native_packager(self):
        """Use one release build and pass explicit runtime/compiler arguments through."""
        with patch.object(package, "host_platform", return_value="windows"), patch.object(package.subprocess, "run") as run, patch("package_windows.find_compiler", return_value="ISCC.exe"), patch("package_windows.package") as native:
            package.package(Path("runtime"), Path("output"), "compiler", False)
            self.assertEqual(run.call_args.args[0], ["cargo", "build", "--locked", "--release", "--bins"])
            self.assertEqual(native.call_args.args[1], Path("runtime").resolve())
            self.assertEqual(native.call_args.args[3], "ISCC.exe")

    def test_linux_skip_build(self):
        """Allow existing release binaries while still selecting the Linux packager."""
        with patch.object(package, "host_platform", return_value="linux"), patch.object(package.subprocess, "run") as run, patch("package_unix.package") as native:
            package.package(Path("runtime"), Path("output"), None, True)
            run.assert_not_called()
            self.assertEqual(native.call_args.args[3], "linux")

    def test_build_failure_does_not_package_stale_files(self):
        """Never proceed to packaging after Cargo fails."""
        error = package.subprocess.CalledProcessError(1, "cargo")
        with patch.object(package, "host_platform", return_value="windows"), patch.object(package.subprocess, "run", side_effect=error), patch("package_windows.package") as native:
            with self.assertRaises(package.subprocess.CalledProcessError):
                package.package(Path("runtime"), None, None, False)
            native.assert_not_called()

    def test_unsupported_host(self):
        """Do not silently choose Linux packaging on an unsupported operating system."""
        with patch.object(package.platform, "system", return_value="Darwin"):
            with self.assertRaises(RuntimeError):
                package.host_platform()
