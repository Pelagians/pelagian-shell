# Configuration

`pelagian-shell` resolves one **workload profile** and optional **capabilities** independently:

1. `/usr/share/pelagian-shell/defaults.toml`
2. one workload profile selected by `PELAGIAN_SHELL_PROFILE` (default: `default`)
3. lexically ordered consumer drop-ins in `/etc/pelagian-shell/profile.d/*.toml`

A profile is a workload choice such as `browser` or `legacy-apps`; it is not a capability bundle language. The profile and every drop-in are strict, versioned TOML. They cannot import profiles, inherit recursively, template values, or run commands.

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

## One window is maximized, not fullscreen

The default `layout.solo = "maximized"` means borderless compositor maximization. It intentionally is **not** true fullscreen, so dialogs remain functional and normal compositor behavior is retained.

Grotto's ChatGPT desktop runtime is a consumer-side exception: its Electron window resets bounds after mapping, so Grotto keeps its app-specific true-fullscreen repair. That rule must not enter a generic profile or the shell baseline.
