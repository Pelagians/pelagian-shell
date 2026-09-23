# Architecture

## Scope

The shell supplies a quiet native GUI substrate inside a pod:

- Selkies session convention;
- upstream Labwc baseline, theme, and XWayland-ready configuration;
- declarative workload-profile resolution and optional capabilities;
- optional Wine visual defaults;
- a tiny status/config command; and
- `pelagian-layoutd`, a live deterministic layout reconciler.

The outer Pelagian web UI remains the product shell. This repository deliberately does not create a panel, launcher, wallpaper manager, desktop icons, network controls, or a plugin system.

## Ownership

Labwc remains the compositor and authority for protocol, output, decoration, XWayland, and actual window geometry. It owns Shell window chrome: ordinary applications use server decorations with title and close only, and Shell owns maximization and layout state. Consumers inherit `PELAGIAN_SHELL_WINDOW_CHROME=server`; Electron consumers can use the reference adapter in `integrations/electron/`. `pelagian-layoutd` is not a second window manager. It owns only an in-memory model, classification, deterministic planning, and a narrow reconciliation request through a replaceable compositor adapter.

Consumers own application installation and launch, business logic, credentials, task execution, app-specific quirks, and acceptance tests. A consumer may install exactly one executable hook at `/usr/local/bin/pelagian-shell-consumer`; Shell keeps the complete Labwc autostart, starts layoutd once, then waits for that hook as a child. If a consumer image enables LinuxServer's `RESTART_APP` watchdog, an exited hook lets the watchdog rerun the hook without spawning a second layoutd supervisor. Session IPC belongs under `/run/pelagian-shell`; persistent application state belongs under `/config`.

The image dependency is one-way: **LinuxServer Selkies → Pelagian Shell → consumer**. Pelagian Shell never imports consumer code, and its release does not depend on Grotto, Cage, or any other downstream build. Consumers add their own runtime and application layers from an immutable Pelagian Shell image reference.

## Configuration order

The resolver applies exactly three layers:

1. `/usr/share/pelagian-shell/defaults.toml`
2. one named **workload profile** selected by `PELAGIAN_SHELL_PROFILE` (default `default`), from `/etc/pelagian-shell/profiles/<name>.toml` when present, otherwise `/usr/share/pelagian-shell/profiles/<name>.toml`
3. lexicographically ordered `/etc/pelagian-shell/profile.d/*.toml`

Every layer has `schema_version = 1`. Profiles choose workload behavior (`browser`, `legacy-apps`); optional features are resolved data under `[capabilities]` (`wine` is the first). No profile executes code, imports another profile, inherits recursively, or templates values. Scalar values use last-layer-wins; `window_rules` append in layer order so their matching order is inspectable. See [`configuration.md`](configuration.md).

## Control seam

The pinned Labwc adapter exposes only targeted operations: maximize/unmaximize a toplevel, snap/unsnap a toplevel to a named region, change its decoration state, and clear Shell interaction ownership. Plans use names such as `auto-2-left`, `auto-3-right-top`, and `auto-5-r1-c1`; the adapter maps those identifiers to Labwc regions. Standard Wayland client protocols do not provide all of these geometry operations, so the control socket remains session-private, bounded, and deliberately narrow. See [`compositor-adapter.md`](compositor-adapter.md) and [`layoutd.md`](layoutd.md).
