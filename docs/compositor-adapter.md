# Compositor adapter boundary

`pelagian-layoutd` plans layout from observed toplevel metadata but does not own compositor geometry.

The initial Rust implementation therefore has two separate contracts:

- pure model/classification/planning functions, fully unit-testable without Wayland; and
- a small, testable adapter trait that observes toplevel lifecycle and receives explicit commands: `maximize`, `unmaximize`, `snap(region)`, `unsnap`, and optional decoration changes.

Standard Wayland client protocols do not grant a normal client authority to resize or place arbitrary other toplevels. The reference runtime therefore carries `labwc/ipc-control.patch` against the exact pinned Labwc source commit. This is a Pelagian downstream interface, not an upstream-supported Labwc API.

The interface is intentionally session-private and narrow: `LIST` returns compositor-owned view/output identity, type/parent, state, outer geometry, decoration, and titlebar data; `ACTION <creation-id>` permits only maximize, named-region snap, float, and decoration changes. It cannot execute arbitrary compositor commands. The socket is mode `0600`, safely replaces only a same-user socket path, caps accepted clients, expires incomplete clients after one second, caps responses at 1 MiB, and uses SIGPIPE-safe nonblocking writes. The Rust client bounds each request to 250 ms and 1 MiB.

Labwc performs maximize and named-region placement through its existing view/SSD geometry path, so tile bounds include the titlebar and border. The adapter observes state and reconciles drift; it does not implement a second geometry engine.
