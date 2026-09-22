# Reference runtime

The reference image overlays the exact LinuxServer Selkies base currently proven by Grotto. It uses `PIXELFLUX_WAYLAND=true`, disables `SELKIES_DESKTOP` and `PELORUS`, and starts a Shell-owned `labwc -i` session without an inner panel or desktop.

```bash
make container-build VERSION=0.1.0
make container-smoke VERSION=0.1.0
```

`container-smoke` builds the image, starts its real `/init`, waits for Labwc, layoutd adapter connectivity, the session sentinel, and Selkies HTTPS, then connects a WebSocket video client through nginx. The client negotiates JPEG streaming, decodes the received stripes, and acknowledges frames for the duration of the test. Native GTK/Wayland fixtures then prove:

1. one through six normal windows use the documented layouts, including two windows from one application;
2. outer geometry is bounded, non-overlapping, and covers the usable workspace while client content remains usable;
3. every managed and floating view reports a visible full titlebar and no managed view remains minimized/fullscreen;
4. a transient dialog floats without disturbing managed layout;
5. focus changes, application resize/state requests, closing windows, and automatic layoutd restart reconcile without reordering; and
6. an existing `/config` volume keeps unrelated application data while Shell-owned configuration is refreshed; and
7. a Wayland client that commits stale geometry while maximized first reports degraded health, then recovers when it starts accepting resize requests. The test-only image adds that client binary without changing the runtime's dependencies. CI also verifies that the previously published image fails this recovery check.

Each engine runs fresh sessions at 1920×1080 and 1366×768. In-session output resizing and viewer disconnect/reconnect remain follow-up work. The restart test kills layoutd and waits for the session supervisor to replace it, without launching a daemon from the test.

The Hermes consumer CI job builds the pinned Grotto source against the current Shell candidate, then exercises its real `/init` under Docker and rootless Podman. It requires observed native Wayland geometry, streamed frames, and a libsecret round trip across container restart. This is pre-publication qualification; Grotto's production base pin advances only after the Shell fix is merged and its new digest is published. Cage retains its own runtime catalog and Wine qualification gates.

The assertions read Labwc's own window state and geometry rather than trusting layoutd's status file. The gate needs a running Docker or Podman daemon.

Focus changes use the configured Alt+Tab binding through streamed keyboard input. They do not depend on an application being allowed to activate itself without a user-input token. Layout recovery preserves the previously active window when restoring minimized tiles.

The first fixture inherits Labwc's child Wayland display through autostart; subsequent fixtures use that same socket. The outer Pixelflux socket is for Labwc itself. An HTTPS GET alone does not start video capture, and the pinned outer compositor can stall nested frame callbacks without a connected viewer. This smoke therefore qualifies an active streaming session, not operation while disconnected.

The image builds Labwc commit `f0dbad27fcf6e388cad1aa32448a1d548da68990` with `labwc/ipc-control.patch`. This is a narrow Pelagian downstream patch, not an upstream Labwc interface. It exposes stable compositor IDs, type/parent/output/state and decoration-aware outer geometry through `LIST`, plus targeted idempotent `ACTION` operations. The mode-0600 same-user socket bounds client count and lifetime. It does not add a second geometry engine; Labwc executes its existing maximize and named-region operations.

The reference ships GTK dark defaults, and schema v1 accepts only `theme.variant = "dark"`. Wine is not installed. A Wine consumer selects `PELAGIAN_SHELL_PROFILE=legacy-apps`, initializes its prefix, then explicitly runs `pelagian-shell-apply-wine-defaults`.

## Canonical OCI publication

Default-branch and version-tag workflows publish `ghcr.io/pelagians/pelagian-shell` after `make check` and the real `/init` runtime smoke pass. Published tags are:

- `sha-<full commit>` for source-bound consumption;
- the Git version tag, such as `v0.1.0`; and
- `latest` for the current default branch.

The digest is the canonical immutable identity. Downstream images should pin `ghcr.io/pelagians/pelagian-shell@sha256:...` or the full commit tag and record that identity in their provenance. Publication includes OCI source, revision, and version labels plus BuildKit SBOM and provenance attestations.

v0.1.0 publishes `linux/amd64` only because that is the architecture exercised by the Selkies/Labwc runtime smoke. Additional architectures require the same behavioral qualification.
