# Pelagian Shell

`pelagian-shell` is the small Selkies/Labwc workspace substrate used inside Pelagian workload pods. It is not a desktop environment and it does not provide product navigation, a panel, desktop icons, Wi-Fi controls, or application launch policy. The outer Pelagian web UI owns product chrome and workspace controls.

The canonical downstream base image is `ghcr.io/pelagians/pelagian-shell`. Consumers should pin its published digest or full `sha-<commit>` tag rather than copy shell files or derive directly from Selkies.

The runtime provides strict TOML profile resolution, a deterministic layout planner, a live Labwc compositor adapter, and daemonized layoutd. Consumers install an executable `/usr/local/bin/pelagian-shell-consumer`; Shell-owned autostart starts layoutd first, launches the consumer independently, and records diagnostics under `${XDG_STATE_HOME:-/config/.local/state}/pelagian-shell/`.

## v0.1.0

Provides:

- Selkies/Labwc reference GUI workspace;
- Pelagian visual/session defaults;
- strict profiles/drop-ins;
- optional Wine appearance capability;
- deterministic layout planner;
- live Labwc compositor adapter;
- daemonized layoutd; and
- shellctl/status/config tooling.

v0.1.0 provides live automatic tiling for one through six normal windows while dialogs and transient windows float.

Quick check:

```bash
cargo test --workspace --locked
PELAGIAN_SHELL_PROFILE=legacy-apps pelagian-shellctl config show
pelagian-shellctl status
pelagian-layoutd status
```

See [`docs/migration-inventory.md`](docs/migration-inventory.md) for the source inventory, [`docs/architecture.md`](docs/architecture.md) for the boundary, [`docs/configuration.md`](docs/configuration.md) for workload profiles and capabilities, [`docs/layoutd.md`](docs/layoutd.md) for layout behavior, and [`docs/reference-runtime.md`](docs/reference-runtime.md) for the image/run gate.
