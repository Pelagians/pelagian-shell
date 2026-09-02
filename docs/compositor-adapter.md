# Compositor adapter boundary

`pelagian-layoutd` plans layout from observed toplevel metadata but does not own compositor geometry.

The initial Rust implementation therefore has two separate contracts:

- pure model/classification/planning functions, fully unit-testable without Wayland; and
- a small, testable adapter trait that observes toplevel lifecycle and receives explicit commands: `maximize`, `unmaximize`, `snap(region)`, `unsnap`, and optional decoration changes.

The blocker is intentional: standard Wayland client protocols do not grant a normal client authority to resize or place arbitrary other toplevels. A foreign-toplevel protocol can be compositor-specific and is not a portable geometry-control solution. We will choose a tiny supported Labwc extension, upstream mechanism, or another supported adapter only after its behavior is demonstrated. No experimental Labwc IPC is introduced here.

Until then, the reference Labwc policy handles quiet decorations and transient dialog behavior. Dynamic tiling is planned and tested but not applied to live windows.
