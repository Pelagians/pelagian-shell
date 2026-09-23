#!/usr/bin/env python3
"""Check a real consumer window through Labwc IPC, not process existence."""
import argparse
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import time


def command(*args, env=None, input=None):
    return subprocess.check_output(args, env=env, input=input, text=True, timeout=15)


def verify(state, health, pattern, managed_count=1, floating_count=None):
    assert health["adapter_connected"] and health["layoutd"] == "healthy", health
    assert health["reconciliation"] == "healthy", health
    normals = [v for v in state["views"] if v["type"] == "normal" and v["parent_id"] is None]
    assert len(normals) == managed_count, state
    assert health["managed_windows"] == managed_count, health
    matching = [v for v in normals if re.search(pattern, v["app_id"] + " " + v["title"], re.I)]
    assert matching, (pattern, normals)
    for view in normals:
        assert view["decoration"] == "full" and view["titlebar_visible"], view
        assert not view["fullscreen"] and not view["minimized"], view
    area = state["outputs"][0]["usable_area"]
    assert (area["width"], area["height"]) == (1920, 1080), area
    if managed_count == 1:
        view = normals[0]
        assert view["maximized"] and not view["tiled"] and view["region"] == "", view
        assert all(view[k] == area[k] for k in ("x", "y", "width", "height")), (view, area)
    elif managed_count == 2:
        by_region = {view["region"]: view for view in normals}
        assert set(by_region) == {"auto-2-left", "auto-2-right"}, normals
        left, right = by_region["auto-2-left"], by_region["auto-2-right"]
        assert left["tiled"] and not left["maximized"], left
        assert right["tiled"] and not right["maximized"], right
        assert (left["x"], left["y"], left["width"], left["height"]) == (
            area["x"], area["y"], area["width"] // 2, area["height"]
        ), (left, area)
        assert (right["x"], right["y"], right["width"], right["height"]) == (
            area["x"] + area["width"] // 2,
            area["y"],
            area["width"] - area["width"] // 2,
            area["height"],
        ), (right, area)
    else:
        raise AssertionError(f"consumer conformance supports one or two live windows, got {managed_count}")

    if floating_count is not None:
        floating = [v for v in state["views"] if v["type"] == "dialog" or v["parent_id"] is not None]
        assert len(floating) == floating_count, state
        if floating_count:
            managed_ids = {view["id"] for view in normals}
            for view in floating:
                assert view["type"] == "dialog" and view["parent_id"] in managed_ids, view
                assert view["decoration"] == "full" and view["titlebar_visible"], view
                assert not view["maximized"] and not view["fullscreen"] and not view["minimized"], view
                assert not view["tiled"] and view["region"] == "", view
        assert health["floating_windows"] == floating_count, health
    return matching[0]


def snapshot():
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR", "")
    assert runtime_dir.startswith("/run/"), runtime_dir
    with socket.socket(socket.AF_UNIX) as client:
        client.settimeout(3)
        client.connect(os.path.join(runtime_dir, "labwc.sock"))
        client.sendall(b"LIST\n")
        chunks = []
        while chunk := client.recv(65536):
            chunks.append(chunk)
    return json.loads(b"".join(chunks))


def verify_app_bus(env):
    runtime = env.get("XDG_RUNTIME_DIR", "")
    assert runtime, runtime
    expected = f"unix:path={runtime}/bus"
    address = env.get("DBUS_SESSION_BUS_ADDRESS")
    assert address == expected, address


def select_compositor_display(runtime, pattern, session_env):
    matches = []
    for path in sorted(Path(runtime).glob("wayland-[0-9]*")):
        if not path.is_socket():
            continue
        env = {**session_env, "XDG_RUNTIME_DIR": runtime, "WAYLAND_DISPLAY": path.name}
        try:
            toplevels = command("wlrctl", "toplevel", "list", env=env)
        except (OSError, subprocess.SubprocessError):
            continue
        if re.search(pattern, toplevels, re.I):
            matches.append((path.name, toplevels))
    assert len(matches) == 1, matches
    return matches[0]


def opaque_binary_session_env(process_env, session_env, xwayland_cmdlines, wayland_display):
    """Use Shell's live sockets when a packaged binary hides its own environment."""
    runtime = session_env.get("XDG_RUNTIME_DIR", "")
    assert runtime.startswith("/run/"), runtime
    assert Path(runtime, wayland_display).is_socket(), runtime
    assert Path(runtime, "bus").is_socket(), runtime
    displays = [arg.decode() for args in xwayland_cmdlines for arg in args.split(b"\0")
                if re.fullmatch(rb":[0-9]+", arg)]
    assert len(displays) == 1, displays
    expected = {"XDG_RUNTIME_DIR": runtime, "WAYLAND_DISPLAY": wayland_display,
                "DISPLAY": displays[0], "DBUS_SESSION_BUS_ADDRESS": f"unix:path={runtime}/bus"}
    for key, value in process_env.items():
        if key in expected:
            assert value == expected[key], (key, value, expected[key])
    return {**session_env, **expected}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pattern")
    parser.add_argument("--native", action="store_true")
    parser.add_argument("--keyring", choices=("store", "lookup"))
    parser.add_argument("--opaque-binary-env", action="store_true",
                        help="verify an inspected binary client through Shell's sockets when it hides process environment")
    parser.add_argument("--managed-count", type=int, default=1)
    parser.add_argument("--floating-count", type=int)
    args = parser.parse_args()
    if args.opaque_binary_env and (not args.native or args.keyring):
        parser.error("--opaque-binary-env requires --native and cannot be used with --keyring")
    deadline = time.monotonic() + 180
    state = None
    while True:
        try:
            state = snapshot()
            health = json.loads(command("pelagian-layoutd", "status"))
            view = verify(state, health, args.pattern, args.managed_count, args.floating_count)
            shell = json.loads(command("pelagian-shellctl", "status"))
            assert shell["window_chrome_policy"] == "server", shell
            break
        except (AssertionError, OSError, ValueError, subprocess.SubprocessError) as error:
            if time.monotonic() >= deadline:
                raise RuntimeError(f"consumer layout did not converge: {error}; observed={state}") from error
            time.sleep(1)
    if args.native or args.keyring:
        # Read only the session coordinates needed for child commands. Never log
        # the process environment, which can contain application credentials.
        entries = Path(f"/proc/{view['pid']}/environ").read_bytes().split(b"\0")
        selected = {"DISPLAY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"}
        process_env = {}
        for entry in entries:
            key, sep, value = entry.partition(b"=")
            if sep and key.decode() in selected:
                process_env[key.decode()] = value.decode()
        if args.opaque_binary_env:
            runtime = os.environ.get("XDG_RUNTIME_DIR", "")
            assert runtime.startswith("/run/"), runtime
            compositor_display, wayland = select_compositor_display(runtime, args.pattern, os.environ)
            pids = command("pgrep", "-x", "Xwayland").splitlines()
            cmdlines = [Path(f"/proc/{pid}/cmdline").read_bytes() for pid in pids]
            env = opaque_binary_session_env(process_env, os.environ, cmdlines, compositor_display)
        else:
            env = {**os.environ, **process_env}
            assert process_env.get("XDG_RUNTIME_DIR", "").startswith("/run/"), process_env.get("XDG_RUNTIME_DIR")
            verify_app_bus(process_env)
        if args.native:
            assert env.get("WAYLAND_DISPLAY") and env.get("DISPLAY"), "missing application display coordinates"
            if not args.opaque_binary_env:
                wayland = command("wlrctl", "toplevel", "list", env=env)
            x11 = command("xlsclients", "-display", env["DISPLAY"], "-l", env=env)
            assert re.search(args.pattern, wayland, re.I), wayland
            assert not re.search(args.pattern, x11, re.I), x11
        if args.keyring:
            assert env.get("DBUS_SESSION_BUS_ADDRESS"), "missing application session bus"
            attributes = ("application", "grotto-shell-ci", "purpose", "restart-persistence")
            secret = "ephemeral-ci-secret"
            if args.keyring == "store":
                command("secret-tool", "store", "--label=Grotto CI restart check", *attributes, env=env, input=secret)
            assert command("secret-tool", "lookup", *attributes, env=env).strip() == secret, "keyring value did not survive"
    print(
        f"consumer layout: {args.pattern}, managed={args.managed_count}, "
        f"floating={args.floating_count if args.floating_count is not None else 'unchecked'}, "
        f"visible titlebar, 1920x1080, healthy{', opaque binary environment' if args.opaque_binary_env else ''}"
    )


if __name__ == "__main__":
    main()
