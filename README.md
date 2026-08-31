# Pelagian Shell

`pelagian-shell` is the small Selkies/Labwc workspace substrate used inside Pelagian workload pods. It is not a desktop environment and it does not provide product navigation, a panel, desktop icons, Wi-Fi controls, or application launch policy. The outer Pelagian web UI owns product chrome and workspace controls.

The first implementation provides strict, inspectable TOML profile resolution and a Rust layout-planning foundation. A reference container and a compositor adapter arrive with the runtime slice; application launch and app-specific quirks stay with consumers.

See [`docs/migration-inventory.md`](docs/migration-inventory.md) for the source inventory and [`docs/architecture.md`](docs/architecture.md) for the boundary.
