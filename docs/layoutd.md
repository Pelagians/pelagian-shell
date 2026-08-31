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
- An explicit `CompositorAdapter` trait: lifecycle observation plus a small `CompositorCommand` set (`maximize`, `unmaximize`, `snap`, `unsnap`, optional decoration state). Pure planner output translates to those commands and is tested with a recording adapter.

All planner/model behavior is unit tested without a Wayland server.

## Not implemented yet

No live Wayland observer, Labwc control adapter, IPC, geometry mutation, or daemon loop is shipped. Standard Wayland does not give an ordinary client authority to place arbitrary other toplevels. The future adapter must demonstrate supported Labwc-side maximize, snap, unsnap, and optional-decoration operations before it can reconcile these plans.

`pelagian-layoutd status` intentionally reports `planner_only` and `compositor_adapter: unavailable` rather than pretending a live layout controller exists.
