from __future__ import annotations

import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
AUTOSTART = ROOT / "session/autostart_wayland"


class ConsumerSessionContractTests(unittest.TestCase):
    def run_autostart(self, hook_body: str) -> tuple[Path, Path, tempfile.TemporaryDirectory[str]]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        hook = root / "consumer"
        marker = root / "marker"
        autostart = root / "autostart"
        hook.write_text(f"#!/bin/sh\n{hook_body}\n", encoding="utf-8")
        hook.chmod(0o755)
        source = AUTOSTART.read_text(encoding="utf-8")
        self.assertEqual(1, source.count("/usr/local/bin/pelagian-shell-consumer"))
        autostart.write_text(
            source.replace("/usr/local/bin/pelagian-shell-consumer", str(hook)),
            encoding="utf-8",
        )
        environment = dict(os.environ)
        environment.pop("PELAGIAN_SHELL_CONSUMER_HOOK", None)
        environment.update(
            {
                "PELAGIAN_SHELL_SESSION_SENTINEL": str(root / "sentinel"),
                "TEST_CONSUMER_MARKER": str(marker),
                "XDG_STATE_HOME": str(root / "state"),
            }
        )
        subprocess.run(["sh", str(autostart)], check=True, env=environment)
        return root, marker, temporary

    @staticmethod
    def wait_for(path: Path) -> None:
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            if path.exists():
                return
            time.sleep(0.01)
        raise AssertionError(f"timed out waiting for {path}")

    def test_shell_runs_consumer_hook_without_replacing_shell_autostart(self) -> None:
        root, marker, temporary = self.run_autostart('printf launched > "$TEST_CONSUMER_MARKER"')
        with temporary:
            status = root / "state/pelagian-shell/consumer.status"
            self.wait_for(marker)
            self.wait_for(status)
            self.assertEqual("launched", marker.read_text(encoding="utf-8"))
            self.assertEqual("0\n", status.read_text(encoding="utf-8"))
            self.assertTrue((root / "sentinel").is_file())

    def test_consumer_failure_is_recorded_without_failing_shell_autostart(self) -> None:
        root, _, temporary = self.run_autostart("echo consumer-failed; exit 7")
        with temporary:
            status = root / "state/pelagian-shell/consumer.status"
            log = root / "state/pelagian-shell/consumer.log"
            self.wait_for(status)
            self.assertEqual("7\n", status.read_text(encoding="utf-8"))
            self.assertIn("consumer-failed", log.read_text(encoding="utf-8"))

    def test_consumer_pid_identifies_the_hook_process(self) -> None:
        root, marker, temporary = self.run_autostart(
            'printf "%s\\n" "$$" > "$TEST_CONSUMER_MARKER"; sleep 0.1'
        )
        with temporary:
            pid = root / "state/pelagian-shell/consumer.pid"
            status = root / "state/pelagian-shell/consumer.status"
            self.wait_for(marker)
            self.wait_for(pid)
            self.wait_for(status)
            self.assertEqual(
                marker.read_text(encoding="utf-8"),
                pid.read_text(encoding="utf-8"),
            )

    def test_consumer_hook_path_is_fixed(self) -> None:
        source = AUTOSTART.read_text(encoding="utf-8")

        self.assertNotIn("PELAGIAN_SHELL_CONSUMER_HOOK", source)
        self.assertIn(
            "consumer_hook=/usr/local/bin/pelagian-shell-consumer",
            source,
        )


if __name__ == "__main__":
    unittest.main()
