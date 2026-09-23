from __future__ import annotations

import os
import subprocess
import tempfile
import tomllib
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
        self.assertIn("COPY session/startwm_wayland.sh /defaults/startwm_wayland.sh", containerfile)
        self.assertIn("wlr-randr", containerfile)
        self.assertIn("command -v wlr-randr", containerfile)
        startwm = (ROOT / "session/startwm_wayland.sh").read_text(encoding="utf-8")
        self.assertIn("exec labwc -i", startwm)
        self.assertIn("labwc.log", startwm)
        self.assertIn("PELAGIAN_SHELL_LABWC_VERBOSE", startwm)
        self.assertIn("-V", startwm)
        self.assertNotIn("/dev/null 2>&1", startwm)
        autostart = (ROOT / "session/autostart_wayland").read_text(encoding="utf-8")
        mode_command = 'wlr-randr --output WL-1 --custom-mode "${width}x${height}"'
        self.assertIn(mode_command, autostart)
        self.assertLess(autostart.index(mode_command), autostart.index('layoutd=/usr/local/bin/pelagian-layoutd'))
        self.assertIn("/init", (ROOT / "tests/container-smoke.sh").read_text(encoding="utf-8"))

    def test_labwc_output_mode_validation_and_failure_state(self) -> None:
        autostart = (ROOT / "session/autostart_wayland").read_text(encoding="utf-8")
        mode_setup, separator, _ = autostart.partition("\nlayoutd=")
        self.assertTrue(separator)

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            wlr_randr = fake_bin / "wlr-randr"
            capture = root / "capture"
            state_home = root / "state"
            env = os.environ | {
                "PATH": f"{fake_bin}:{os.environ['PATH']}",
                "CAPTURE": str(capture),
                "XDG_STATE_HOME": str(state_home),
            }

            wlr_randr.write_text(
                '#!/bin/sh\nprintf "%s\\n" "$*" > "$CAPTURE"\n', encoding="utf-8"
            )
            wlr_randr.chmod(0o755)
            configured = subprocess.run(
                ["sh", "-c", mode_setup],
                env=env | {"SELKIES_MANUAL_WIDTH": "1920", "SELKIES_MANUAL_HEIGHT": "1080"},
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(0, configured.returncode, configured.stderr)
            self.assertEqual("--output WL-1 --custom-mode 1920x1080\n", capture.read_text())
            status = state_home / "pelagian-shell/output-mode.status"
            self.assertEqual("configured 1920x1080\n", status.read_text())

            for width, height in (("1:2", "3"), ("1920", ""), ("00", "1080")):
                capture.unlink(missing_ok=True)
                rejected = subprocess.run(
                    ["sh", "-c", mode_setup],
                    env=env | {"SELKIES_MANUAL_WIDTH": width, "SELKIES_MANUAL_HEIGHT": height},
                    check=False,
                    capture_output=True,
                    text=True,
                )
                self.assertNotEqual(0, rejected.returncode, (width, height))
                self.assertFalse(capture.exists(), (width, height))

            wlr_randr.write_text('#!/bin/sh\necho mode-failed\nexit 7\n', encoding="utf-8")
            failed = subprocess.run(
                ["sh", "-c", mode_setup],
                env=env | {"SELKIES_MANUAL_WIDTH": "1366", "SELKIES_MANUAL_HEIGHT": "768"},
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(7, failed.returncode)
            self.assertEqual("failed 1366x768 rc=7\n", status.read_text())
            self.assertEqual(
                "mode-failed\n",
                (state_home / "pelagian-shell/output-mode.log").read_text(),
            )

    def test_labwc_regions_and_minimal_single_workspace_policy(self) -> None:
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
        expected_thirds = {
            "auto-5-r0-c0": ("0%", "33%"),
            "auto-5-r0-c1": ("33%", "34%"),
            "auto-5-r0-c2": ("67%", "33%"),
            "auto-6-r0-c0": ("0%", "33%"),
            "auto-6-r0-c1": ("33%", "34%"),
            "auto-6-r0-c2": ("67%", "33%"),
            "auto-6-r1-c0": ("0%", "33%"),
            "auto-6-r1-c1": ("33%", "34%"),
            "auto-6-r1-c2": ("67%", "33%"),
        }
        region_elements = {
            region.attrib["name"]: region for region in root.findall("./regions/region")
        }
        for name, (x, width) in expected_thirds.items():
            self.assertEqual((x, width), (region_elements[name].attrib["x"], region_elements[name].attrib["width"]))
        self.assertEqual("2", root.findtext("./theme/cornerRadius"))
        self.assertEqual(":close", root.findtext("./theme/titlebar/layout"))
        self.assertEqual("yes", root.findtext("./theme/titlebar/showTitle"))
        self.assertEqual("titlebar", root.findtext("./theme/maximizedDecoration"))
        for rule in root.findall("./windowRules/windowRule"):
            for selector in ("app_id", "appId", "identifier", "title", "class"):
                self.assertNotIn(selector, rule.attrib)

        self.assertEqual(
            ["Workspace 1"],
            [name.text for name in root.findall("./desktops/names/name")],
        )
        self.assertEqual("Workspace 1", root.findtext("./desktops/initial"))

        keyboard = root.find("./keyboard")
        mouse = root.find("./mouse")
        if keyboard is None or mouse is None:
            self.fail("explicit keyboard and mouse policy is required")
        self.assertIsNone(keyboard.find("./default"))
        self.assertIsNone(mouse.find("./default"))
        self.assertEqual(
            {"A-Tab", "A-S-Tab", "A-F4", "W-Return"},
            {binding.attrib["key"] for binding in keyboard.findall("./keybind")},
        )
        key_actions = {
            action.attrib["name"]
            for action in keyboard.findall(".//action")
        }
        self.assertEqual(
            {"NextWindow", "PreviousWindow", "Close", "Execute"}, key_actions
        )
        mouse_actions = {
            action.attrib["name"] for action in mouse.findall(".//action")
        }
        self.assertEqual({"Focus", "Raise", "Close", "Move", "Resize"}, mouse_actions)
        drag_actions = {
            (context.attrib["name"], binding.attrib["button"], action.attrib["name"])
            for context in mouse.findall("./context")
            for binding in context.findall("./mousebind")
            if binding.attrib.get("action") == "Drag"
            for action in binding.findall("./action")
        }
        self.assertEqual(
            {("Title", "Left", "Move"), ("Border", "Left", "Resize")},
            drag_actions,
        )

        rules = root.findall("./windowRules/windowRule")
        self.assertFalse(
            any(
                rule.attrib.get("type") == "normal"
                and rule.attrib.get("serverDecoration") == "no"
                for rule in rules
            )
        )

    def test_default_decorations_are_full_and_gtk_is_close_only(self) -> None:
        defaults = tomllib.loads(
            (ROOT / "config/defaults.toml").read_text(encoding="utf-8")
        )
        self.assertEqual(
            {"solo": "full", "tiled": "full", "floating": "full"},
            defaults["decorations"],
        )
        self.assertFalse(
            any("title" in rule for rule in defaults.get("window_rules", [])),
            "default floating classification must use compositor type/parent, not title guesses",
        )
        for version in ("gtk-3.0", "gtk-4.0"):
            settings = (
                ROOT / f"theme/{version}/settings.ini"
            ).read_text(encoding="utf-8")
            self.assertIn("gtk-decoration-layout=:close", settings)

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

    def test_session_ipc_and_window_chrome_are_shell_owned(self) -> None:
        containerfile = (ROOT / "Containerfile").read_text(encoding="utf-8")
        runtime_service = (ROOT / "session/s6-rc.d/init-pelagian-runtime/run").read_text(
            encoding="utf-8"
        )
        runtime_up = (ROOT / "session/s6-rc.d/init-pelagian-runtime/up").read_text(
            encoding="utf-8"
        ).strip()
        runtime_dependencies = ROOT / "session/s6-rc.d/init-pelagian-runtime/dependencies.d/legacy-cont-init"
        selkies_config_dependency = ROOT / "session/s6-rc.d/init-pelagian-runtime/dependencies.d/init-selkies-config"
        pulseaudio_runtime_dependency = ROOT / "session/s6-rc.d/svc-pulseaudio/dependencies.d/init-pelagian-runtime"
        startwm = (ROOT / "session/startwm_wayland.sh").read_text(encoding="utf-8")
        autostart = (ROOT / "session/autostart_wayland").read_text(encoding="utf-8")
        bind_smoke = (ROOT / "tests/container-bind-mount-smoke.sh").read_text(
            encoding="utf-8"
        )
        self.assertIn("XDG_RUNTIME_DIR=/run/pelagian-shell", containerfile)
        self.assertIn("PELAGIAN_SHELL_WINDOW_CHROME=server", containerfile)
        self.assertEqual("/etc/s6-overlay/s6-rc.d/init-pelagian-runtime/run", runtime_up)
        self.assertIn("check-electron-chrome.py /usr/share/pelagian-shell/consumer-conformance/check-electron-chrome.py", containerfile)
        self.assertTrue(runtime_dependencies.is_file())
        self.assertTrue(selkies_config_dependency.is_file())
        self.assertTrue(pulseaudio_runtime_dependency.is_file())
        self.assertIn("/run/s6/container_environment", runtime_service)
        self.assertIn("init-selkies-config", runtime_service)
        self.assertIn('"$environment_dir/XDG_RUNTIME_DIR"', runtime_service)
        self.assertIn('"$environment_dir/PELAGIAN_SHELL_WINDOW_CHROME"', runtime_service)
        self.assertIn("printf '%s' \"$runtime_dir\"", runtime_service)
        self.assertIn("printf '%s' server", runtime_service)
        self.assertIn("chmod 0700", runtime_service)
        self.assertIn("rm -rf -- /config/.XDG", runtime_service)
        self.assertIn("export XDG_RUNTIME_DIR=/run/pelagian-shell", startwm)
        self.assertIn("export PELAGIAN_SHELL_WINDOW_CHROME=server", autostart)
        self.assertIn("unix:path=${XDG_RUNTIME_DIR}/bus", startwm)
        self.assertIn("$host_config:/config:Z", bind_smoke)
        self.assertIn("/run/pelagian-shell/labwc.sock", bind_smoke)
        self.assertIn("bind-mount-persistence.sentinel", bind_smoke)
        self.assertIn("/config/.XDG", runtime_service)
        self.assertIn("test ! -e /config/.XDG", bind_smoke)
        self.assertIn("assert_runtime_writable_as_abc", bind_smoke)

    def test_electron_adapter_and_bind_smoke_scripts_parse(self) -> None:
        subprocess.run(
            ["bash", "-n", str(ROOT / "session/s6-rc.d/init-pelagian-runtime/run")],
            check=True,
        )
        subprocess.run(["sh", "-n", str(ROOT / "tests/container-bind-mount-smoke.sh")], check=True)
        subprocess.run(["node", "--check", str(ROOT / "integrations/electron/window-chrome.mjs")], check=True)

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
        runtime_docs = (ROOT / "docs/reference-runtime.md").read_text(encoding="utf-8")

        self.assertIn("docs/reference-runtime.md", readme)
        self.assertTrue(workflow.is_file())
        workflow_text = workflow.read_text(encoding="utf-8")
        self.assertIn("make check", workflow_text)
        self.assertIn("make container-smoke", workflow_text)
        self.assertEqual(
            5,
            workflow_text.count(
                "ref: ${{ github.event.pull_request.head.sha || github.sha }}"
            ),
        )
        self.assertIn("rootless-podman", workflow_text)
        self.assertIn("ENGINE=podman make container-smoke", workflow_text)
        self.assertIn("one through six normal windows", runtime_docs)
        self.assertIn("a transient dialog floats", runtime_docs)
        architecture = (ROOT / "docs/architecture.md").read_text(encoding="utf-8")
        library = (ROOT / "crates/layoutd/src/lib.rs").read_text(encoding="utf-8")
        self.assertNotIn("future deterministic layout reconciler", architecture)
        self.assertNotIn("A future Labwc-side adapter", architecture)
        self.assertNotIn("not assumed to exist yet", architecture)
        self.assertNotIn("No adapter is implemented yet", library)

        smoke = (ROOT / "tests/container-smoke.sh").read_text(encoding="utf-8")
        self.assertNotIn("stream_resolution()", smoke)
        self.assertIn('sh "$0" "$fixture_image" 1920 1080', smoke)
        self.assertIn('sh "$0" "$fixture_image" 1366 768', smoke)
        self.assertIn("SELKIES_MANUAL_WIDTH", smoke)
        self.assertIn("SELKIES_MANUAL_HEIGHT", smoke)
        self.assertIn("PELAGIAN_SHELL_LABWC_VERBOSE", smoke)
        self.assertIn("output-mode.status", smoke)
        self.assertIn("output-mode.log", smoke)
        self.assertIn("labwc.log", smoke)
        self.assertIn("assert_native_wayland", smoke)
        self.assertIn("xlsclients -l", smoke)
        self.assertIn("pelagian-shell-consumer", smoke)
        self.assertIn("pause_layoutd", smoke)
        self.assertIn("wait_disrupted", smoke)
        self.assertIn("kill -STOP", smoke)
        self.assertIn("kill -CONT", smoke)
        self.assertIn('len({view["pid"] for view in normals}) == count', smoke)
        self.assertIn('dialog["x"] >= area["x"]', smoke)
        self.assertIn('wait_layout 6 "$width" "$height" Four 0', smoke)
        self.assertIn('state["managed_windows"] == int(sys.argv[1])', smoke)
        self.assertIn('state["floating_windows"] == int(sys.argv[2])', smoke)
        self.assertIn('type(state["managed_windows"]) is int', smoke)
        self.assertIn('type(state["floating_windows"]) is int', smoke)
        self.assertIn("expected_geometry = {", smoke)
        self.assertIn("assert actual == expected", smoke)
        self.assertIn('dialog["client_width"] >= 320', smoke)
        self.assertIn('if focused == "any":', smoke)
        self.assertIn("*) focused=any", smoke)
        self.assertIn("'\"layoutd\":\"healthy\"'", smoke)
        self.assertIn("'\"adapter_connected\":true'", smoke)
        self.assertIn("'\"reconciliation\":\"healthy\"'", smoke)
        self.assertNotIn("'\"layoutd\":\"running\"'", smoke)

        fixture = (ROOT / "tests/layout-fixture.py").read_text(encoding="utf-8")
        self.assertIn("dialog.set_default_size(480, 320)", fixture)

    def test_readme_states_the_v0_1_0_boundary(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")

        for capability in (
            "Selkies/Labwc reference GUI workspace",
            "Pelagian visual/session defaults",
            "strict profiles/drop-ins",
            "optional Wine appearance capability",
            "deterministic layout planner",
            "live Labwc compositor adapter",
            "daemonized layoutd",
            "shellctl/status/config tooling",
        ):
            self.assertIn(capability, readme)
        self.assertIn("live automatic tiling", readme)
        self.assertNotIn("planner-only", readme)

    def test_configuration_docs_describe_operative_behavior(self) -> None:
        configuration = (ROOT / "docs/configuration.md").read_text(encoding="utf-8")
        schema = (ROOT / "crates/shellctl/src/lib.rs").read_text(encoding="utf-8")

        self.assertIn("## Operative in v0.1.0", configuration)
        self.assertNotIn("Resolved but not dynamically applied", configuration)
        self.assertIn("Layoutd reapplies full decoration", configuration)
        self.assertIn("Floating views retain titlebar drag and border resize", configuration)
        for field in (
            "layout.mode",
            "decorations.solo",
            "decorations.tiled",
            "decorations.floating",
            "window_rules",
        ):
            self.assertIn(field, configuration)
        self.assertNotIn("planner_only", configuration)
        self.assertIn("compositor_adapter = labwc-ipc", configuration)
        self.assertIn("maximizes one window", configuration)
        self.assertIn("tiles multiple windows", configuration)
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
        self.assertIn('docker pull "$reference"', workflow)
        self.assertIn(
            'ENGINE=docker sh tests/container-smoke.sh "$reference"', workflow
        )
        self.assertLess(
            workflow.index('docker pull "$reference"'),
            workflow.index('ENGINE=docker sh tests/container-smoke.sh "$reference"'),
        )
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
