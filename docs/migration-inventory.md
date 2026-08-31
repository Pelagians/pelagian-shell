# Migration inventory

Inspected on 2026-08-31 against:

- `Pelagians/Grotto` `378b011db1b97ed13c7eda678f064172cc1c76f2`
- `Pelagians/legacy-apps` `3e409392e208cfd27ce28f6340bab3e3ad0127ae`

This is an inventory, not a migration of either consumer.

## Grotto

| Source | Disposition | Reason |
| --- | --- | --- |
| `Containerfile.chatgpt-desktop` lines 10, 32–36 | Generalize | The pinned LinuxServer Selkies base, `PIXELFLUX_WAYLAND=true`, `AUTO_GPU=true`, and `/init` process model are the proven reference session substrate. Application package, Codex, tool paths, authentication, and ChatGPT labels stay in Grotto. |
| `runtimes/chatgpt-desktop/root/defaults/labwc.xml` lines 4–16 | Generalize selectively | Zero gap and server-side decoration are reusable. The Clearlooks theme, buttons, and radius are not the target aesthetic; replace with a quiet Pelagian theme. |
| `labwc.xml` lines 45–50, 72–86 | Generalize | The normal/dialog/utility distinction, server decorations, unmaximize, raise, and focus behavior inform the generic baseline. Shell-owned configuration must not include ChatGPT identifiers. |
| `labwc.xml` lines 52–70 | Remain Grotto-specific | `chatgpt-desktop`, true fullscreen, lower/bottom layer, and focus handling exist for one Electron application's resize race. They are not default shell policy. |
| `root/defaults/autostart` and `autostart_wayland` | Split | Directory preparation is consumer state management; device authentication, ChatGPT launch, and the fullscreen helper stay in Grotto. Shell provides only a POSIX-safe Labwc session hook. The current Grotto hook is Bash but Labwc invokes its hook through `sh`; do not copy it until its shell-boundary behavior is separately proven. |
| `grotto-chatgpt-fullscreen` | Keep in Grotto | It waits for `app_id:chatgpt-desktop` and retries `wlrctl toplevel fullscreen` to repair an observed ChatGPT-only mapping race. |
| `grotto-configure-openbox` | Do not migrate | It maintains an X11/Openbox compatibility lane. `pelagian-shell` is Wayland/Labwc-first and must not make Openbox a core dependency. |
| `10-grotto-chatgpt-permissions` lines 35–60 | Generalize pattern only | Refreshing shell-managed persistent Labwc configuration during LinuxServer init is useful. Grotto's Codex, tools, cache, workspace, and authentication ownership rules remain workload-specific. |
| `tests/test_window_manager_config.py` | Generalize test style | XML policy and launcher/config identity assertions are useful. The ChatGPT fullscreen and Openbox assertions stay with Grotto. |
| `tests/chatgpt-desktop-runtime.sh` | Split | Session executable/path checks inform a shell smoke test. ChatGPT package, Codex policy, and doctor checks remain Grotto tests. |

## legacy-apps

| Source | Disposition | Reason |
| --- | --- | --- |
| `components/wine-runtime/theme/vic-legacy-modern/theme.reg` | Generalize and replace | It proves registry-based color defaults are viable, but it only sets three light colors and carries the old VIC name. `pelagian-shell` owns neutral reusable Wine registry appearance defaults. |
| `components/wine-runtime/docs/theme.md` | Replace | It states the right boundary (readability without general desktop controls) but has no reusable implementation detail. |
| `components/wine-runtime/runtime/start-wine-desktop.sh` | Replace after migration | It starts Openbox plus a Wine virtual desktop. That is an X11-specific runtime session and conflicts with the Wayland/Labwc target. Keep it until a Wine/XWayland consumer smoke proves the replacement. |
| `components/wine-runtime/Containerfile` | Remain workload runtime | It installs Wine, Xvfb, x11vnc, Openbox, xdotool, and wmctrl for a specific VIC image. Wine must stay optional in the shell; task runner, shims, and artifact directories do not belong in the shell. |
| `runtime/start-novnc.sh` | Delete/avoid as a shell concern | It is an explicit placeholder. Selkies is the human-session transport for the new shell; noVNC must not be recreated as a second permanent desktop transport. |
| `runtime/entrypoint.sh`, `run-skill.sh`, `install-shim-bundle.sh`, document-capture registry scripts | Remain legacy-apps-specific | These implement VIC task execution, shims, application integration, and artifacts—not GUI substrate policy. |

## Target boundary

`pelagian-shell` owns: Selkies integration convention, Labwc baseline/theme, XWayland-ready session defaults, deterministic window classification/layout planning, declarative configuration resolution, generic optional Wine appearance defaults, and a small status/config CLI.

Grotto and legacy-apps own: application launch, package installation, app identities and quirks, authentication, task execution, app-specific profiles/drop-ins, persistent workload state, and workload acceptance tests.

The present extraction must not delete or overwrite Grotto's working ChatGPT desktop overlay. A consumer switches only after it has a versioned shell artifact and passes its own session smoke.

## Migration prerequisites

1. Publish a versioned shell image or package artifact; consumers must not copy from shell `main`.
2. Add a compositor adapter only when a supported Labwc mechanism can perform targeted maximize/snap/unsnap operations. Standard Wayland observation/control is insufficient for the requested layout reconciliation.
3. Qualify a Grotto browser/XWayland matrix and a legacy Wine/XWayland matrix separately.
4. Retire Grotto's Openbox and legacy-apps' Xvfb/Openbox session paths only after those consumer-specific proofs pass.
