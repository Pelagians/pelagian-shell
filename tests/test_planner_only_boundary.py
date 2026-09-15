import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class LiveRuntimeBoundaryTests(unittest.TestCase):
    def test_shell_embeds_and_starts_the_live_labwc_adapter(self) -> None:
        containerfile = (ROOT / "Containerfile").read_text(encoding="utf-8")
        autostart = (ROOT / "session/autostart_wayland").read_text(encoding="utf-8")

        self.assertRegex(autostart, r"(?m)^(?!\s*#).*?\bpelagian-layoutd\b")
        self.assertIn("COPY --from=labwc-builder /usr/bin/labwc /usr/bin/labwc", containerfile)
        self.assertIn("COPY labwc/ipc-control.patch", containerfile)
        self.assertNotIn("wmctrl", containerfile)
        self.assertTrue((ROOT / "crates/layoutd/src/labwc.rs").is_file())
        self.assertTrue((ROOT / "crates/layoutd/src/runtime.rs").is_file())

    def test_acceptance_rejects_planner_only_status(self) -> None:
        smoke = (ROOT / "tests/container-smoke.sh").read_text(encoding="utf-8")
        shellctl = (ROOT / "crates/shellctl/src/main.rs").read_text(encoding="utf-8")

        self.assertNotIn("pgrep -x pelagian-layoutd", smoke)
        self.assertIn('state["layoutd"] == "healthy"', smoke)
        self.assertIn("planner_only", smoke)
        self.assertRegex(smoke, r"grep\s+-q.*planner_only")
        self.assertNotIn("planner_only", shellctl)
        self.assertIn('compositor_adapter\\\":\\\"labwc-ipc', shellctl)

    def test_labwc_control_socket_is_session_private(self) -> None:
        ipc_patch = (ROOT / "labwc/ipc-control.patch").read_text(encoding="utf-8")

        self.assertIn("chmod(socket_path, 0600)", ipc_patch)
        self.assertIn("wl_event_loop_add_timer", ipc_patch)
        self.assertIn("IPC_CLIENT_TIMEOUT_MS", ipc_patch)
        self.assertIn("IPC_MAX_CLIENTS", ipc_patch)
        self.assertIn("IPC_MAX_RESPONSE_BYTES", ipc_patch)
        self.assertIn("ipc_array_append", ipc_patch)
        self.assertIn("send(client->fd,", ipc_patch)
        self.assertIn("MSG_NOSIGNAL", ipc_patch)
        self.assertIn("WL_EVENT_WRITABLE", ipc_patch)
        self.assertIn("wl_event_source_fd_update", ipc_patch)
        self.assertIn("response_offset", ipc_patch)
        self.assertIn("FD_CLOEXEC", ipc_patch)
        self.assertIn("SOCK_CLOEXEC", ipc_patch)
        self.assertIn("connect(probe_fd", ipc_patch)
        self.assertIn("static bool socket_owned;", ipc_patch)
        self.assertIn("if (socket_owned)", ipc_patch)
        self.assertEqual(
            1,
            ipc_patch.count("wl_event_source_timer_update(client->timer,"),
            "the one-second client deadline must not restart after parsing",
        )
        self.assertIn('#include "common/border.h"', ipc_patch)
        for command in ("GET_WINDOWS", "GET_STATE", "GET_WINDOW_BY_PID", "GET_FOCUSED_WINDOW"):
            self.assertNotIn(command, ipc_patch)
        self.assertIn("view_minimize(view, false)", ipc_patch)
        self.assertIn("view_set_fullscreen(view, false)", ipc_patch)

        adapter = (ROOT / "crates/layoutd/src/labwc.rs").read_text(encoding="utf-8")
        self.assertIn("connect_timeout", adapter)
        self.assertLess(
            adapter.index("let deadline = Instant::now() + IPC_TIMEOUT;"),
            adapter.index(".connect_timeout(&address,"),
        )
        self.assertIn("connect_timeout(&address, remaining)", adapter)
        self.assertIn("set_write_timeout(Some(remaining))", adapter)
        self.assertNotIn("UnixStream::connect(&self.socket)", adapter)
        self.assertNotIn('PathBuf::from("/tmp")', adapter)


if __name__ == "__main__":
    unittest.main()
