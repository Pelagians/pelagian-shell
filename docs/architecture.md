# Architecture

## Scope

The shell supplies a quiet native GUI substrate inside a pod:

- Selkies session convention;
- upstream Labwc baseline, theme, and XWayland-ready configuration;
- declarative profile resolution;
- optional Wine visual defaults;
- a tiny status/config command; and
- `pelagian-layoutd`, a future deterministic layout reconciler.

The outer Pelagian web UI remains the product shell. This repository deliberately does not create a panel, launcher, wallpaper manager, desktop icons, network controls, or a plugin system.

## Ownership

Labwc remains the compositor and authority for protocol, output, decoration, XWayland, and actual window geometry. `pelagian-layoutd` is not a second window manager. It owns only an in-memory model, classification, deterministic planning, and a narrow reconciliation request through a replaceable compositor adapter.

Consumers own application installation and launch, business logic, credentials, task execution, app-specific quirks, and acceptance tests.

## Configuration order

The resolver applies exactly three layers:

1. `/usr/share/pelagian-shell/defaults.toml`
2. one named profile selected by `PELAGIAN_SHELL_PROFILE` (default `default`), from `/etc/pelagian-shell/profiles/<name>.toml` when present, otherwise `/usr/share/pelagian-shell/profiles/<name>.toml`
3. lexicographically ordered `/etc/pelagian-shell/profile.d/*.toml`

Every layer has `schema_version = 1`. No profile executes code, imports another profile, inherits recursively, or templates values. Scalar values use last-layer-wins; `window_rules` append in layer order so their matching order is inspectable.

## Control seam

A future Labwc-side adapter needs only targeted operations: maximize/unmaximize a toplevel, snap/unsnap a toplevel to a named region, and optionally change its decoration state. Standard Wayland client protocols do not provide all of these geometry operations, so the adapter is intentionally not assumed to exist yet. See [`compositor-adapter.md`](compositor-adapter.md).
