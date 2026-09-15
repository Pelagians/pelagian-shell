# Compositor adapter boundary

`pelagian-layoutd` plans layout from observed toplevel metadata but does not own compositor geometry.

The initial Rust implementation therefore has two separate contracts:

- pure model/classification/planning functions, fully unit-testable without Wayland; and
- a small, testable adapter trait that observes toplevel lifecycle and receives explicit commands: `maximize`, `unmaximize`, `snap(region)`, `unsnap`, interaction-ownership release, and optional decoration changes.

Standard Wayland client protocols do not grant a normal client authority to resize or place arbitrary other toplevels. The reference runtime therefore carries `labwc/ipc-control.patch` against the exact pinned Labwc source commit. This is a Pelagian downstream interface, not an upstream-supported Labwc API.

The interface is intentionally session-private and narrow: `LIST` returns compositor-owned view/output identity, type/parent, state, outer geometry, decoration, and titlebar data; `ACTION <creation-id>` permits only maximize, named-region snap, float, interaction-ownership release, and decoration changes. It cannot execute arbitrary compositor commands. The socket is mode `0600`, replaces only a stale same-user socket after refusing a live endpoint, marks listener/client descriptors close-on-exec, caps accepted clients, expires incomplete or backpressured clients after one second, caps responses at 1 MiB, and drains SIGPIPE-safe nonblocking writes through writable event-loop callbacks. The Rust client applies one 250 ms end-to-end connect/write/read deadline and a 1 MiB response cap.

Labwc performs maximize and named-region placement through its existing view/SSD geometry path, so tile bounds include the titlebar and border. The adapter observes state and reconciles drift; it does not implement a second geometry engine.
