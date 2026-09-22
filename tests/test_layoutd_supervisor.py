from __future__ import annotations

import os
import signal
import subprocess
import tempfile
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class LayoutdSupervisorTests(unittest.TestCase):
    def start_supervisor(self, daemon: str, fast_backoff: bool = False):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        binary = root / "layoutd"
        binary.write_text("#!/bin/sh\n" + daemon)
        binary.chmod(0o755)
        supervisor = root / "supervisor"
        supervisor.write_text(
            (ROOT / "session/supervise-layoutd").read_text().replace(
                "/usr/local/bin/pelagian-layoutd", str(binary)
            )
        )
        environment = dict(os.environ, XDG_STATE_HOME=str(root), TEST_ROOT=str(root))
        if fast_backoff:
            sleep = root / "sleep"
            sleep.write_text('#!/bin/sh\nprintf "%s\\n" "$1" >> "$TEST_ROOT/delays"\n')
            sleep.chmod(0o755)
            environment["PATH"] = str(root) + ":" + environment["PATH"]
        process = subprocess.Popen(
            ["sh", str(supervisor)], env=environment,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )

        def stop():
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)

        self.addCleanup(stop)
        return root, root / "pelagian-shell", process

    def wait_for(self, predicate):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if predicate():
                return
            time.sleep(0.02)
        self.fail("supervisor did not reach expected state")

    def test_killed_daemon_is_restarted_and_shutdown_stops_replacement(self):
        _, state, supervisor = self.start_supervisor("exec sleep 60\n")
        pid_file = state / "layoutd.pid"
        self.wait_for(lambda: pid_file.exists() and pid_file.read_text().strip())
        old_pid = int(pid_file.read_text())
        os.kill(old_pid, signal.SIGKILL)
        self.wait_for(lambda: pid_file.read_text().strip() and int(pid_file.read_text()) != old_pid)
        replacement = int(pid_file.read_text())
        os.kill(replacement, 0)
        self.assertEqual("1", (state / "layoutd.restarts").read_text().strip())
        supervisor.terminate()
        supervisor.wait(timeout=5)
        with self.assertRaises(ProcessLookupError):
            os.kill(replacement, 0)
        self.assertEqual("stopped", (state / "layoutd-supervisor.status").read_text().strip())

    def test_crash_loop_has_bounded_exponential_backoff_and_preserves_exit(self):
        root, state, supervisor = self.start_supervisor(
            'echo attempt >> "$TEST_ROOT/attempts"\nexit 7\n', fast_backoff=True
        )
        self.assertEqual(1, supervisor.wait(timeout=5))
        self.assertEqual(6, len((root / "attempts").read_text().splitlines()))
        self.assertEqual(["1", "2", "4", "8", "16"], (root / "delays").read_text().splitlines())
        self.assertEqual("7", (state / "layoutd.exit").read_text().strip())
        self.assertEqual("failed", (state / "layoutd-supervisor.status").read_text().strip())


if __name__ == "__main__":
    unittest.main()
