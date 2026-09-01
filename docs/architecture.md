# Architecture

## Scope

The shell supplies a quiet native GUI substrate inside a pod:

- Selkies session convention;
- upstream Labwc baseline, theme, and XWayland-ready configuration;
- declarative workload-profile resolution and optional capabilities;
- optional Wine visual defaults;
- a tiny status/config command; and
- `pelagian-layoutd`, the generic deterministic XWayland layout reconciler.

The outer Pelagian web UI remains the product shell. This repository deliberately does not create a panel, launcher, wallpaper manager, desktop icons, network controls, or a plugin system.

## Ownership

Labwc remains the compositor and authority for protocol, output, decoration, XWayland, and actual window geometry. `pelagian-layoutd` is not a second window manager. It owns the generic in-memory model, classification, deterministic planning, and narrow XWayland/EWMH reconciliation.

Consumers own application installation and launch, business logic, credentials, task execution, app-specific quirks, and acceptance tests.

The image dependency is one-way: **LinuxServer Selkies → Pelagian Shell → consumer**. Pelagian Shell never imports consumer code, and its release does not depend on Grotto, Cage, or any other downstream build. Consumers add their own runtime and application layers from an immutable Pelagian Shell image reference.

## Configuration order

The resolver applies exactly three layers:

1. `/usr/share/pelagian-shell/defaults.toml`
2. one named **workload profile** selected by `PELAGIAN_SHELL_PROFILE` (default `default`), from `/etc/pelagian-shell/profiles/<name>.toml` when present, otherwise `/usr/share/pelagian-shell/profiles/<name>.toml`
3. lexicographically ordered `/etc/pelagian-shell/profile.d/*.toml`

Every layer has `schema_version = 1`. Profiles choose workload behavior (`browser`, `legacy-apps`); optional features are resolved data under `[capabilities]` (`wine` is the first). No profile executes code, imports another profile, inherits recursively, or templates values. Scalar values use last-layer-wins; `window_rules` append in layer order so their matching order is inspectable. See [`configuration.md`](configuration.md).

## Control seam

The `xwayland-ewmh` adapter observes X11 identities and metadata with `wmctrl`/`xprop`, then applies maximize and geometry changes with `wmctrl`. Native Wayland placement and dynamic EWMH decoration control are deliberately unsupported. See [`compositor-adapter.md`](compositor-adapter.md) and [`layoutd.md`](layoutd.md).
