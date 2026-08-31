# Configuration

`pelagian-shell` resolves one **workload profile** and optional **capabilities** independently:

1. `/usr/share/pelagian-shell/defaults.toml`
2. one workload profile selected by `PELAGIAN_SHELL_PROFILE` (default: `default`)
3. lexically ordered consumer drop-ins in `/etc/pelagian-shell/profile.d/*.toml`

A profile is a workload choice such as `browser` or `legacy-apps`; it is not a capability bundle language. The profile and every drop-in are strict, versioned TOML. They cannot import profiles, inherit recursively, template values, or run commands.

## Operative in v0.1.0

The following resolved configuration and static runtime behavior are operative:

- workload profile selection and lexically layered TOML resolution;
- capabilities such as `capabilities.wine` and the Wine helper's capability gate;
- static Labwc configuration and the Pelagian Shell theme;
- GTK 3 and GTK 4 dark defaults; and
- the reference Selkies/Labwc runtime.

Schema v1 accepts only `theme.variant = "dark"` because the runtime installs only dark GTK defaults. `light` is rejected as unsupported. `layout.max_managed_windows` is constrained to `1..=6`, matching the layouts and static Labwc regions shipped in v0.1.0.

## Resolved but not dynamically applied in v0.1.0

The resolver accepts and reports the target policy fields `layout.mode`, `layout.solo`, `layout.multiple`, `layout.dialogs`, `decorations.solo`, `decorations.tiled`, `decorations.floating`, and profile-derived `window_rules`. They are planned inputs, not live compositor behavior in v0.1.0.

In particular, v0.1.0 does not dynamically maximize one window, tile multiple windows, change decorations from resolved profiles, or reconcile live compositor state. `pelagian-layoutd` remains a pure planner and reports:

```text
layoutd = planner_only
compositor_adapter = unavailable
```

## Capabilities

Capabilities are ordinary resolved data under `[capabilities]`. The first capability is `wine`:

```toml
[capabilities]
wine = true
```

`browser` leaves the default `wine = false`. The `legacy-apps` workload profile enables it. Consumers inspect the fully resolved data with:

```bash
pelagian-shellctl config show
pelagian-shellctl capability wine
```

The Wine registry helper requires that resolved capability; it still performs no Wine or application launch itself.

## Consumer drop-ins

Consumers add small data overrides without forking shell code. For example, a PBS consumer may install [`examples/legacy-apps/profile.d/80-pbs.toml`](../examples/legacy-apps/profile.d/80-pbs.toml) after validating its real identifiers. It adds a floating authentication-dialog rule only. Application startup, authentication, and task behavior remain in the consumer.

## Planned one-window behavior

The resolved target `layout.solo = "maximized"` means borderless compositor maximization once a supported live adapter exists. It intentionally is **not** true fullscreen, so dialogs remain functional and normal compositor behavior is retained. This is not dynamically applied in v0.1.0.

Grotto's ChatGPT desktop runtime is a consumer-side exception: its Electron window resets bounds after mapping, so Grotto keeps its app-specific true-fullscreen repair. That rule must not enter a generic profile or the shell baseline.
