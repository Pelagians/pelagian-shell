# Configuration

`pelagian-shell` resolves one **workload profile** and optional **capabilities** independently:

1. `/usr/share/pelagian-shell/defaults.toml`
2. one workload profile selected by `PELAGIAN_SHELL_PROFILE` (default: `default`)
3. lexically ordered consumer drop-ins in `/etc/pelagian-shell/profile.d/*.toml`

A profile is a workload choice such as `browser` or `legacy-apps`; it is not a capability bundle language. Profiles and drop-ins are strict, versioned TOML. They cannot import profiles, inherit recursively, template values, or run commands.

## Operative in v0.1.0

The runtime applies:

- workload profile selection and lexically layered TOML resolution;
- `layout.mode`, `layout.solo`, `layout.multiple`, `layout.dialogs`, and profile-derived `window_rules`;
- maximizes one window and tiles multiple windows through named Labwc regions;
- floating dialogs, utilities, transients, matching rules, and managed-window overflow;
- full server decoration for solo, tiled, floating, and overflow windows, styled as a slim titlebar with title and close only;
- capabilities such as `capabilities.wine` and the Wine helper's capability gate;
- static Labwc configuration, GTK 3/4 dark defaults, and the Shell theme; and
- the optional `/usr/local/bin/pelagian-shell-consumer`, with logs, PID, and exit status under `${XDG_STATE_HOME:-/config/.local/state}/pelagian-shell/`.

The Shell session uses `/run/pelagian-shell` for `XDG_RUNTIME_DIR`, Labwc IPC, Wayland sockets, and the session D-Bus socket. It recreates this directory at container start with owner `abc` and mode `0700`. LinuxServer clears its obsolete `/config/.XDG` runtime directory during initialization. Other persistent files under `/config`, including keyrings, remain application data.

Shell exports `PELAGIAN_SHELL_WINDOW_CHROME=server` to the Wayland session and consumers. This v0 policy means ordinary top-level application windows yield chrome to Labwc, which owns their visible titlebar and close-only controls.

Schema v1 accepts only `theme.variant = "dark"`; `light` is rejected. `layout.max_managed_windows` is constrained to `1..=6`, matching the planner and installed Labwc regions.

`pelagian-layoutd status` reports process, connection, and reconciliation health rather than binary presence. Command acknowledgement leaves reconciliation `pending`; only a subsequent compositor observation matching planned outer geometry, state, and decoration makes it `healthy`. An unchanged plan that fails to converge within five seconds becomes `degraded` with an error. Retries remain bounded to once per second, and later convergence restores health:

```text
layoutd = starting | healthy | degraded | stopped
compositor_adapter = labwc-ipc
adapter_connected = true | false
reconciliation = pending | healthy | error | stopped
managed_windows = <count>
floating_windows = <count>
last_error = <message or null>
```

`pelagian-shellctl status` embeds this live result under `runtime`.

Schema v1 requires `decorations.solo`, `decorations.tiled`, and `decorations.floating` to be `full`. Unsupported `none` and `border` values are rejected in defaults, profiles, and drop-ins. Layoutd reapplies full decoration during every reconciliation; the Labwc theme and `<layout>:close</layout>` make that decoration minimal.

## Capabilities

Capabilities are ordinary resolved data under `[capabilities]`. The first capability is `wine`:

```toml
[capabilities]
wine = true
```

`browser` leaves the default `wine = false`. The `legacy-apps` workload profile enables it. Consumers inspect resolved data with:

```bash
pelagian-shellctl config show
pelagian-shellctl capability wine
```

The Wine registry helper requires that resolved capability; it performs no Wine or application launch itself.

## Consumer drop-ins

Consumers add small data overrides without forking Shell code. For example, [`examples/legacy-apps/profile.d/80-pbs.toml`](../examples/legacy-apps/profile.d/80-pbs.toml) adds a floating authentication-dialog rule. Application startup, authentication, and task behavior remain in the consumer.

## One-window behavior

`layout.solo = "maximized"` means compositor maximization, not true fullscreen. Dialogs remain functional and normal compositor behavior is retained.

## Workspace and interaction policy

The shipped Labwc configuration creates exactly one workspace. Its explicit bindings are click-to-focus, `Alt+Tab` / `Shift+Alt+Tab`, `Alt+F4`, titlebar close, and `Super+Enter` for recovery. Workspace movement, minimize, manual maximize/fullscreen, shade, show-desktop, and manual snapping are not bound. Managed maximized/region-tiled views reject interactive move/resize. Floating views retain titlebar drag and border resize.
