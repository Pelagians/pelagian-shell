# Pelagian Shell

`pelagian-shell` is the small Selkies/Labwc workspace substrate used inside Pelagian workload pods. It is not a desktop environment and it does not provide product navigation, a panel, desktop icons, Wi-Fi controls, or application launch policy. The outer Pelagian web UI owns product chrome and workspace controls.

The first implementation provides strict, inspectable TOML profile resolution, a Rust workspace model/layout planner, and a reference Selkies/Labwc container. `layoutd` remains planner-only until a supported compositor-control path exists; application launch and app-specific quirks stay with consumers.

Quick check:

```bash
cargo test --workspace --locked
PELAGIAN_SHELL_PROFILE=legacy-apps pelagian-shellctl config show
pelagian-shellctl status
pelagian-layoutd status
```

See [`docs/migration-inventory.md`](docs/migration-inventory.md) for the source inventory, [`docs/architecture.md`](docs/architecture.md) for the boundary, [`docs/configuration.md`](docs/configuration.md) for workload profiles and capabilities, [`docs/layoutd.md`](docs/layoutd.md) for implemented versus blocked layout behavior, and [`docs/reference-runtime.md`](docs/reference-runtime.md) for the image/run gate.
