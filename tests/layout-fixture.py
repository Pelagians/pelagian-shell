#!/usr/bin/python3
import os
import signal
import sys
from pathlib import Path

os.environ.setdefault("GDK_BACKEND", "wayland")

import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk

mode = sys.argv[1] if len(sys.argv) > 1 else "first"
titles = {
    "first": "Pelagian Fixture One",
    "second": "Pelagian Fixture Two",
    "third": "Pelagian Fixture Three",
    "fourth": "Pelagian Fixture Four",
    "fifth": "Pelagian Fixture Five",
    "sixth": "Pelagian Fixture Six",
}
if mode not in titles:
    raise SystemExit(f"unknown fixture mode: {mode}")

# Every process deliberately identifies as the same native Wayland application.
GLib.set_prgname("pelagian-layout-fixture")
window = Gtk.Window(title=titles[mode])
window.set_default_size(800, 600)
window.connect("destroy", Gtk.main_quit)
window.show_all()
Path(f"/tmp/pelagian-layout-{mode}.pid").write_text(str(os.getpid()))
# The first fixture inherits Labwc's child display from its autostart hook.
# Container exec otherwise inherits the outer Pixelflux display instead.
Path(f"/tmp/pelagian-layout-{mode}.display").write_text(os.environ["WAYLAND_DISPLAY"])

dialog = None
command_path = Path(f"/tmp/pelagian-layout-{mode}.command")
ack_path = Path(f"/tmp/pelagian-layout-{mode}.ack")


def read_command() -> bool:
    global dialog
    if not command_path.exists():
        return True
    command = command_path.read_text().strip()
    if not command:
        return True
    command_path.unlink()
    if command == "dialog" and dialog is None:
        dialog = Gtk.Dialog(
            title="Pelagian Fixture Dialog",
            transient_for=window,
            modal=True,
        )
        dialog.set_default_size(480, 320)
        dialog.add_button("Close", Gtk.ResponseType.CLOSE)
        dialog.connect("response", lambda current, _response: current.destroy())
        dialog.show_all()
    elif command == "dialog-close" and dialog is not None:
        dialog.destroy()
        dialog = None
    elif command == "focus":
        window.present()
    elif command == "resize":
        window.unmaximize()
        window.resize(320, 240)
    elif command == "fullscreen":
        window.fullscreen()
    elif command == "minimize":
        window.iconify()
    else:
        raise ValueError(f"unknown fixture command: {command}")
    ack_path.write_text(command)
    return True


GLib.timeout_add(100, read_command)
signal.signal(signal.SIGTERM, lambda *_args: Gtk.main_quit())
Gtk.main()
