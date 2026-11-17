import os
import tempfile
import unittest
from pathlib import Path

from install import install


class InstallTest(unittest.TestCase):
    def test_install_and_update_keep_the_same_desktop_command(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "original"
            binary.write_bytes(b"first version")
            bin_dir = root / "local bin"
            data_home = root / "shared data"

            command, desktop = install(binary, bin_dir, data_home)
            entry = desktop.read_text(encoding="utf-8")
            self.assertIn(f'Exec="{command}" gui\n', entry)
            self.assertIn(f"TryExec={command}\n", entry)
            self.assertTrue(os.access(command, os.X_OK))
            self.assertFalse(command.is_symlink())
            self.assertEqual(command.read_bytes(), b"first version")

            updated = root / "replacement"
            updated.write_bytes(b"second version")
            self.assertEqual(install(updated, bin_dir, data_home), (command, desktop))
            binary.unlink()
            updated.unlink()
            self.assertEqual(desktop.read_text(encoding="utf-8"), entry)
            self.assertEqual(command.read_bytes(), b"second version")
