# Pelagian Shell

`pelagian-shell` is the small Selkies/Labwc workspace substrate used inside Pelagian workload pods. It is not a desktop environment and it does not provide product navigation, a panel, desktop icons, Wi-Fi controls, or application launch policy. The outer Pelagian web UI owns product chrome and workspace controls.

The canonical downstream base image is `ghcr.io/pelagians/pelagian-shell`. Consumers should pin its published digest or full `sha-<commit>` tag rather than copy shell files or derive directly from Selkies.

The runtime provides strict TOML profile resolution, a deterministic layout planner, a live Labwc compositor adapter, and daemonized layoutd. Consumers install an executable `/usr/local/bin/pelagian-shell-consumer`; Shell-owned autostart starts layoutd once, launches the consumer as a child, waits for it, and records diagnostics under `${XDG_STATE_HOME:-/config/.local/state}/pelagian-shell/`. This lets LinuxServer's optional `RESTART_APP` watchdog restart the consumer after it exits without starting another layoutd supervisor.

Session IPC lives under `/run/pelagian-shell`; persistent application state remains under `/config`. Shell exports `PELAGIAN_SHELL_WINDOW_CHROME=server` to consumer processes. See [`docs/window-chrome.md`](docs/window-chrome.md) for the v0 contract and Electron reference adapter.

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
- a consumer-facing server-decoration policy and Electron reference adapter.

v0.1.0 provides live automatic tiling for one through six normal windows while dialogs and transient windows float.

Quick check:

```bash
cargo test --workspace --locked
PELAGIAN_SHELL_PROFILE=legacy-apps pelagian-shellctl config show
pelagian-shellctl status
pelagian-layoutd status
```

See [`docs/migration-inventory.md`](docs/migration-inventory.md) for the source inventory, [`docs/architecture.md`](docs/architecture.md) for the boundary, [`docs/configuration.md`](docs/configuration.md) for workload profiles and capabilities, [`docs/layoutd.md`](docs/layoutd.md) for layout behavior, and [`docs/reference-runtime.md`](docs/reference-runtime.md) for the image/run gate.
