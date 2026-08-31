from __future__ import annotations

import subprocess
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class ReferenceRuntimeContractTests(unittest.TestCase):
    def test_reference_runtime_is_wayland_first_without_inner_desktop(self) -> None:
        containerfile = (ROOT / "Containerfile").read_text(encoding="utf-8")

        self.assertIn("PIXELFLUX_WAYLAND=true", containerfile)
        self.assertIn("SELKIES_DESKTOP=false", containerfile)
        self.assertIn("PELORUS=false", containerfile)
        self.assertIn("ARG VERSION=0.1.0", containerfile)
        self.assertIn('org.opencontainers.image.version="${VERSION}"', containerfile)
        self.assertLess(
            containerfile.index("ARG SELKIES_BASE_IMAGE="),
            containerfile.index("FROM ${RUST_IMAGE} AS build"),
        )
        self.assertIn("COPY labwc/rc.xml /defaults/labwc.xml", containerfile)
        self.assertIn(
            'COPY ["labwc/theme/Pelagian Shell/", "/usr/share/themes/Pelagian Shell/"]',
            containerfile,
        )
        self.assertIn("COPY session/autostart_wayland /defaults/autostart_wayland", containerfile)
        self.assertIn("/init", (ROOT / "tests/container-smoke.sh").read_text(encoding="utf-8"))
        self.assertNotIn("labwc -i", containerfile)

    def test_labwc_regions_cover_every_current_planner_region(self) -> None:
        root = ET.parse(ROOT / "labwc/rc.xml").getroot()
        regions = {region.attrib["name"] for region in root.findall("./regions/region")}
        expected = {
            "auto-2-left",
            "auto-2-right",
            "auto-3-left",
            "auto-3-right-top",
            "auto-3-right-bottom",
            "auto-4-top-left",
            "auto-4-top-right",
            "auto-4-bottom-left",
            "auto-4-bottom-right",
            "auto-5-r0-c0",
            "auto-5-r0-c1",
            "auto-5-r0-c2",
            "auto-5-r1-c0",
            "auto-5-r1-c1",
            "auto-6-r0-c0",
            "auto-6-r0-c1",
            "auto-6-r0-c2",
            "auto-6-r1-c0",
            "auto-6-r1-c1",
            "auto-6-r1-c2",
        }
        self.assertEqual(expected, regions)
        self.assertEqual("2", root.findtext("./theme/cornerRadius"))
        self.assertEqual("none", root.findtext("./theme/maximizedDecoration"))
        normal_rule = root.find("./windowRules/windowRule")
        if normal_rule is None:
            self.fail("the generic normal window rule is missing")
        self.assertEqual("no", normal_rule.attrib["serverDecoration"])

    def test_session_scripts_are_posix_parseable_and_refresh_shell_owned_files(self) -> None:
        for relative in (
            "session/autostart",
            "session/autostart_wayland",
            "session/20-pelagian-shell-config",
            "wine/apply-defaults.sh",
        ):
            subprocess.run(["sh", "-n", str(ROOT / relative)], check=True)

        init = (ROOT / "session/20-pelagian-shell-config").read_text(encoding="utf-8")
        self.assertIn("/config/.config/labwc/rc.xml", init)
        self.assertIn("/config/.config/labwc/autostart", init)
        self.assertIn("gtk-3.0/settings.ini", init)
        self.assertIn("gtk-4.0/settings.ini", init)

    def test_wine_defaults_are_explicit_and_do_not_require_msstyles(self) -> None:
        apply_defaults = (ROOT / "wine/apply-defaults.sh").read_text(encoding="utf-8")
        registry = (ROOT / "wine/pelagian-shell.reg").read_text(encoding="utf-8")

        self.assertIn("pelagian-shellctl capability wine", apply_defaults)
        self.assertIn("wine regedit", apply_defaults)
        self.assertIn("Control Panel\\Colors", registry)
        self.assertNotIn("VisualStyles", registry)
        self.assertNotIn("uxtheme", registry.lower())

    def test_workload_profiles_and_capabilities_are_separate_data(self) -> None:
        defaults = (ROOT / "config/defaults.toml").read_text(encoding="utf-8")
        browser = (ROOT / "config/profiles/browser.toml").read_text(encoding="utf-8")
        legacy_apps = (ROOT / "config/profiles/legacy-apps.toml").read_text(encoding="utf-8")
        pbs = (ROOT / "examples/legacy-apps/profile.d/80-pbs.toml").read_text(encoding="utf-8")

        self.assertIn("[capabilities]", defaults)
        self.assertIn("wine = false", defaults)
        self.assertNotIn("[capabilities]", browser)
        self.assertIn("[capabilities]", legacy_apps)
        self.assertIn("wine = true", legacy_apps)
        self.assertIn("[[window_rules]]", pbs)
        self.assertNotIn("launch", pbs.lower())
        self.assertFalse((ROOT / "config/profiles/wine.toml").exists())

    def test_docs_and_ci_expose_the_real_runtime_gate(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        workflow = ROOT / ".github/workflows/ci.yml"

        self.assertIn("docs/reference-runtime.md", readme)
        self.assertTrue(workflow.is_file())
        workflow_text = workflow.read_text(encoding="utf-8")
        self.assertIn("make check", workflow_text)
        self.assertIn("make container-smoke", workflow_text)


if __name__ == "__main__":
    unittest.main()
