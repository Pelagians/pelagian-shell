# `pelagian-layoutd`

`pelagian-layoutd` is a planner and reconciler, not a window manager. Labwc remains the compositor.

## Implemented

- Session-local ordered toplevel model with upsert/remove events.
- Classification into `managed`, `floating`, and `ignored`:
  - normal root toplevels are managed;
  - dialogs, utilities, and transient normals float;
  - desktop/other surfaces are ignored;
  - the last matching `app_id`/`title` rule wins.
- Deterministic plans for one through six managed windows:
  - 1: maximize, not fullscreen;
  - 2: equal left/right halves;
  - 3: primary left half with two stacked right windows;
  - 4: 2×2;
  - 5: three top plus two bottom tiles;
  - 6: 3×2.
- Managed-window overflow floats instead of disappearing.
- A live `labwc-ipc` adapter using session-local stable view IDs and explicit Labwc `MAXIMIZE`, `SNAP`, `FLOAT`, `UNMANAGE`, and `DECORATION` actions.
- A daemon loop started by Shell's Wayland autostart before the consumer hook.
- Reconciliation after lifecycle, output-size, geometry, decoration, minimized, maximized, fullscreen, tiled-state, and daemon-restart drift; focus alone is ignored and never changes opening order.
- Managed maximize/snap actions normalize stale minimized/fullscreen state and lock interactive movement until the view floats again.
- Atomic mode-0600 runtime state with PID identity, adapter connection, reconciliation health, counts, and the last error.

The planner/model and adapter protocol are unit tested without a Wayland server. `tests/container-smoke.sh` is the real runtime gate: it starts `/init`, proves layoutd stays alive, and checks maximize → 50/50 → floating dialog → maximize through Labwc's independent state inventory.

## Deliberate limit

Six normal windows are the supported tiled ceiling. Additional normal windows remain visible as floating overflow; they are not hidden, closed, or treated as a seventh layout mode. Closing managed windows promotes overflow in stable creation order. Application-specific client-side decorations remain application behavior; Shell does not crop or patch client chrome.
