import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class PlannerOnlyBoundaryTests(unittest.TestCase):
    def test_shell_does_not_embed_a_live_xwayland_controller(self) -> None:
        containerfile = (ROOT / "Containerfile").read_text(encoding="utf-8")
        autostart = (ROOT / "session/autostart_wayland").read_text(encoding="utf-8")

        self.assertNotRegex(autostart, r"(?m)^(?!\s*#).*?\bpelagian-layoutd\b")
        self.assertNotIn("wmctrl", containerfile)
        self.assertFalse((ROOT / "crates/layoutd/src/runtime.rs").exists())
        self.assertFalse((ROOT / "crates/layoutd/src/xwayland.rs").exists())

    def test_runtime_contract_includes_boundary_guard(self) -> None:
        makefile = (ROOT / "Makefile").read_text(encoding="utf-8")

        self.assertIn("tests.test_planner_only_boundary", makefile)


if __name__ == "__main__":
    unittest.main()
