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
- the reference Selkies/Labwc runtime;
- generic ordered `window_rules`; and
- live XWayland automatic layout.

Schema v1 accepts only `theme.variant = "dark"` because the runtime installs only dark GTK defaults. `light` is rejected as unsupported. `layout.max_managed_windows` is constrained to `1..=6`, matching the layouts and static Labwc regions shipped in v0.1.0.

## Operative automatic layout

`pelagian-layoutd` consumes the same resolved `layout.mode`, `layout.solo`, `layout.multiple`, `layout.dialogs`, `layout.max_managed_windows`, `decorations.solo`, `decorations.tiled`, `decorations.floating`, and profile-derived `window_rules` as `pelagian-shellctl`.

With `layout.mode = "auto"`, layoutd maximizes one managed window and tiles multiple managed windows while dialogs, utilities, transients, and overflow remain floating. With `layout.mode = "float"`, it performs no placement. Ordered window rules remain last-match-wins overrides.

```text
layoutd = running
compositor_adapter = xwayland-ewmh
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

Consumers add small data overrides under `/etc/pelagian-shell/profile.d` without forking shell code. Application identities, startup, authentication, and task behavior remain in the consumer repository.

## One-window behavior

The resolved target `layout.solo = "maximized"` means compositor maximization, intentionally **not** true fullscreen, so dialogs remain functional and normal compositor behavior is retained.

The adapter is X11/XWayland-only. Placement of native Wayland windows is unsupported. Dynamic decoration changes cannot be applied safely through EWMH, so the resolved decoration values remain diagnostics while Labwc's generic static rules supply the current decoration behavior.
