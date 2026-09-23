#!/usr/bin/env python3
"""Regression tests for the real-window smoke gate."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("shell_check", Path(__file__).with_name("verify-shell-session.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class GeometryGateTests(unittest.TestCase):
    def setUp(self):
        self.area = dict(x=0, y=0, width=1920, height=1080)
        self.view = dict(self.area, id=1, type="normal", parent_id=None, app_id="test-app",
                         title="Test App", decoration="full", titlebar_visible=True,
                         maximized=True, fullscreen=False, minimized=False, tiled=False, region="")
        self.state = dict(views=[self.view], outputs=[dict(usable_area=self.area)])
        self.health = dict(adapter_connected=True, layoutd="healthy", reconciliation="healthy", managed_windows=1)

    def test_accepts_observed_maximized_app(self):
        module.verify(self.state, self.health, "test-app")

    def test_rejects_green_status_with_stale_geometry(self):
        self.view["width"] = 800
        with self.assertRaises(AssertionError):
            module.verify(self.state, self.health, "test-app")

    def test_rejects_fullscreen_and_missing_titlebar(self):
        for changes in (dict(fullscreen=True), dict(titlebar_visible=False)):
            with self.subTest(changes=changes):
                view = self.view | changes
                with self.assertRaises(AssertionError):
                    module.verify(self.state | dict(views=[view]), self.health, "test-app")

    def test_rejects_wrong_application(self):
        with self.assertRaises(AssertionError):
            module.verify(self.state, self.health, "unrelated")

    def test_two_windows_use_the_left_right_split(self):
        left = self.view | dict(
            x=0, y=0, width=960, height=1080, maximized=False, tiled=True,
            region="auto-2-left", title="Fixture"
        )
        right = self.view | dict(
            x=960, y=0, width=960, height=1080, maximized=False, tiled=True,
            region="auto-2-right", title="Grotto App"
        )
        state = self.state | dict(views=[left, right])
        health = self.health | dict(managed_windows=2)
        module.verify(state, health, "Grotto App", managed_count=2)

    def test_dialog_is_floating_and_excluded_from_managed_count(self):
        dialog = self.view | dict(
            id=2, type="dialog", parent_id=1, title="Login", maximized=False,
            tiled=False, region=""
        )
        state = self.state | dict(views=[self.view, dialog])
        health = self.health | dict(floating_windows=1)
        module.verify(state, health, "test-app", floating_count=1)


class OpaqueBinarySessionTests(unittest.TestCase):
    def test_opaque_client_uses_live_shell_sockets_and_rejects_conflicts(self):
        with tempfile.TemporaryDirectory(prefix="pelagian-bus-") as directory:
            env = {"XDG_RUNTIME_DIR": directory}
            with self.assertRaises(AssertionError):
                module.verify_app_bus(env)
            session = {"XDG_RUNTIME_DIR": "/run/pelagian-shell"}
            cmdlines = [b"/usr/bin/Xwayland\0:1\0-rootless\0"]
            with self.assertRaises(AssertionError):
                module.opaque_binary_session_env({}, session, cmdlines)
            with patch.object(Path, "is_socket", return_value=True):
                derived = module.opaque_binary_session_env({}, session, cmdlines)
                self.assertEqual(derived["WAYLAND_DISPLAY"], "wayland-1")
                self.assertEqual(derived["DISPLAY"], ":1")
                self.assertEqual(derived["DBUS_SESSION_BUS_ADDRESS"],
                                 "unix:path=/run/pelagian-shell/bus")
                with self.assertRaises(AssertionError):
                    module.opaque_binary_session_env({"WAYLAND_DISPLAY": "wayland-0"},
                                                     session, cmdlines)
                with self.assertRaises(AssertionError):
                    module.opaque_binary_session_env({}, session, cmdlines * 2)


if __name__ == "__main__":
    unittest.main()
