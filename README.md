# Pelagian Shell

`pelagian-shell` is the small Selkies/Labwc workspace substrate used inside Pelagian workload pods. It is not a desktop environment and it does not provide product navigation, a panel, desktop icons, Wi-Fi controls, or application launch policy. The outer Pelagian web UI owns product chrome and workspace controls.

The first implementation provides strict, inspectable TOML profile resolution, a Rust workspace model/layout planner, and an explicit planner-only status surface. A reference container and a compositor adapter arrive with the runtime slice; application launch and app-specific quirks stay with consumers.

Quick check:

```bash
cargo test --workspace --locked
PELAGIAN_SHELL_PROFILE=wine pelagian-shellctl config show
pelagian-shellctl status
pelagian-layoutd status
```

See [`docs/migration-inventory.md`](docs/migration-inventory.md) for the source inventory, [`docs/architecture.md`](docs/architecture.md) for the boundary, and [`docs/layoutd.md`](docs/layoutd.md) for implemented versus blocked layout behavior.
