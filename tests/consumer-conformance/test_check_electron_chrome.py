import importlib.util
from pathlib import Path
import unittest

TOOL = Path(__file__).with_name("check-electron-chrome.py")
SPEC = importlib.util.spec_from_file_location("check_electron_chrome", TOOL)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ElectronChromeCheckerTests(unittest.TestCase):
    def test_accepts_adapted_ordinary_windows_and_dialogs(self):
        source = """
new BrowserWindow(applyPelagianShellWindowChrome({ frame: false, titleBarStyle: 'hidden' }))
new BrowserWindow(applyPelagianShellWindowChrome({ modal: true }, { role: 'dialog' }))
"""
        self.assertEqual([], MODULE.inspect_source(source))

    def test_requires_explicit_reason_for_special_surfaces(self):
        source = """
// pelagian-shell-chrome-exception: overlay -- transparent always-on-top HUD surface
new BrowserWindow({ frame: false, transparent: true })
"""
        self.assertEqual([], MODULE.inspect_source(source))

    def test_rejects_ordinary_constructor_that_bypasses_adapter(self):
        source = "new BrowserWindow({ frame: false, titleBarOverlay: true })"
        problems = MODULE.inspect_source(source)
        self.assertEqual(1, len(problems))
        self.assertIn("raw client chrome option bypasses the Shell adapter", problems[0])

    def test_default_framed_window_is_allowed(self):
        self.assertEqual([], MODULE.inspect_source("new BrowserWindow({ width: 800 })"))


if __name__ == "__main__":
    unittest.main()
