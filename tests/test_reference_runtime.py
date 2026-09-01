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
        smoke = (ROOT / "tests/container-smoke.sh").read_text(encoding="utf-8")
        self.assertIn("pelagian-shellctl status", smoke)
        self.assertIn("pelagian-shellctl config show", smoke)

    def test_readme_states_the_v0_1_0_boundary(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")

        for capability in (
            "Selkies/Labwc reference GUI workspace",
            "Pelagian visual/session defaults",
            "strict profiles/drop-ins",
            "optional Wine appearance capability",
            "deterministic layout planner",
            "compositor adapter seam",
            "planner-only layoutd",
            "shellctl/status/config tooling",
        ):
            self.assertIn(capability, readme)
        self.assertIn("does not yet provide live automatic tiling", readme)

    def test_configuration_docs_separate_operative_and_planned_behavior(self) -> None:
        configuration = (ROOT / "docs/configuration.md").read_text(encoding="utf-8")
        schema = (ROOT / "crates/shellctl/src/lib.rs").read_text(encoding="utf-8")

        self.assertIn("## Operative in v0.1.0", configuration)
        self.assertIn("## Resolved but not dynamically applied in v0.1.0", configuration)
        for field in (
            "layout.mode",
            "decorations.solo",
            "decorations.tiled",
            "decorations.floating",
            "window_rules",
        ):
            self.assertIn(field, configuration)
        self.assertIn("planner_only", configuration)
        self.assertIn("compositor_adapter = unavailable", configuration)
        self.assertIn("does not dynamically maximize one window", configuration)
        self.assertIn("tile multiple windows", configuration)
        self.assertIn('theme.variant = "dark"', configuration)
        self.assertIn("`light` is rejected", configuration)
        self.assertNotIn("Light,", schema)

    def test_canonical_image_publication_contract(self) -> None:
        containerfile = (ROOT / "Containerfile").read_text(encoding="utf-8")
        makefile = (ROOT / "Makefile").read_text(encoding="utf-8")
        workflow = (ROOT / ".github/workflows/publish.yml").read_text(encoding="utf-8")
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        architecture = (ROOT / "docs/architecture.md").read_text(encoding="utf-8")
        runtime = (ROOT / "docs/reference-runtime.md").read_text(encoding="utf-8")

        self.assertIn("ARG REVISION=unknown", containerfile)
        self.assertIn('org.opencontainers.image.revision="${REVISION}"', containerfile)
        self.assertIn("--build-arg REVISION=$(REVISION)", makefile)
        self.assertIn("ghcr.io/pelagians/pelagian-shell", workflow)
        self.assertIn("DOCKER_METADATA_SHORT_SHA_LENGTH: 40", workflow)
        self.assertIn(
            "type=sha,prefix=sha-,enable=${{ github.ref_type != 'tag' }}", workflow
        )
        self.assertIn("type=ref,event=tag", workflow)
        self.assertIn("type=raw,value=latest,enable={{is_default_branch}}", workflow)
        self.assertIn("platforms: linux/amd64", workflow)
        self.assertIn("provenance: mode=max", workflow)
        self.assertIn("sbom: true", workflow)
        self.assertIn("make container-smoke", workflow)
        self.assertIn("packages: write", workflow)
        self.assertIn(
            '{{ index .Config.Labels "org.opencontainers.image.revision" }}', workflow
        )
        self.assertNotIn(r'\"org.opencontainers.image.revision\"', workflow)
        self.assertIn("VERSION=${{ github.ref_type == 'tag'", workflow)
        self.assertIn("ghcr.io/pelagians/pelagian-shell", readme)
        self.assertIn("LinuxServer Selkies → Pelagian Shell → consumer", architecture)
        self.assertIn("digest is the canonical immutable identity", runtime)


if __name__ == "__main__":
    unittest.main()
