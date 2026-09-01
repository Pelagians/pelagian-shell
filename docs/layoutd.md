# `pelagian-layoutd`

`pelagian-layoutd` is deliberately a planner and reconciler, not a window manager.

## Implemented

- Session-local ordered toplevel model with upsert/remove events.
- Pure classification into `managed`, `floating`, and `ignored`:
  - normal root toplevels are managed;
  - dialogs, utilities, and transient normals float;
  - desktop/other surfaces are ignored;
  - ordered rules override the default; the last matching `app_id`/`title` rule wins.
- Pure deterministic plans for one through six managed windows:
  - 1: maximize, not fullscreen;
  - 2: equal left/right halves;
  - 3: primary left half with two stacked right windows;
  - 4: 2×2;
  - 5: three top plus two bottom tiles;
  - 6: 3×2.
- A configured managed-window ceiling moves overflow to floating rather than silently dropping it.
- An explicit `CompositorAdapter` trait and live `xwayland-ewmh` implementation.
- Generic X11 window observation through `wmctrl` plus type, transient, class, and state metadata from `xprop`.
- Idempotent maximize/unmaximize and geometry mutation through `wmctrl`; snapping removes maximized state before applying geometry.
- A continuous daemon loop started by the Shell-owned Labwc autostart hook.
- Direct consumption of the same resolved profile/drop-in configuration as `pelagian-shellctl`.

All planner/model behavior is unit tested without a Wayland server.

## Supported scope

The live adapter deliberately controls only X11/XWayland toplevels exposed through EWMH. A missing XID fails closed; layoutd never substitutes an app/title guess. `layout.mode = "float"` observes the workspace but performs no automatic placement.

Native Wayland geometry control is unsupported. Dynamic per-window decoration mutation is also unavailable through the EWMH adapter; Labwc's static generic normal/dialog/utility decoration rules remain authoritative.

`pelagian-layoutd status` reports adapter capability and verifies the daemon state file's PID before claiming `running`.
