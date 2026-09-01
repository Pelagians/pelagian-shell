# Compositor adapter boundary

`pelagian-layoutd` plans layout from observed toplevel metadata but does not own compositor geometry.

The Rust implementation has two separate contracts:

- pure model/classification/planning functions, fully unit-testable without Wayland; and
- a small, testable adapter trait implemented by the live `xwayland-ewmh` adapter, which observes X11 lifecycle and receives explicit `maximize`, `unmaximize`, `snap`, and `unsnap` commands.

The adapter uses `wmctrl` for EWMH inventory, state, and geometry mutation and `xprop` for authoritative class/type/transient metadata. It never selects windows by a consumer-specific identity. A disappeared XID fails closed.

Standard Wayland client protocols still do not grant a normal client authority to resize or place arbitrary native Wayland toplevels, so native Wayland geometry control remains unsupported. The reference Labwc policy handles generic decorations because EWMH has no reliable dynamic `none`/`border`/`full` decoration operation.
