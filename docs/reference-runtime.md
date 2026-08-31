# Reference runtime

The reference image is a thin overlay on the exact LinuxServer Selkies base currently proven by Grotto. It uses `PIXELFLUX_WAYLAND=true`, disables `SELKIES_DESKTOP` and `PELORUS`, and leaves the Labwc session empty when no consumer application runs. That gives the outer Pelagian web UI a dark, quiet workspace rather than an inner desktop panel.

```bash
make container-build VERSION=0.1.0
make container-smoke VERSION=0.1.0
```

`container-smoke` builds the image, starts its real `/init`, waits for Labwc and the session autostart sentinel, verifies Selkies HTTPS, and reads both status commands. It needs a running Docker or Podman daemon.

The reference ships GTK dark defaults. It deliberately does not install `qt5ct` or `qt6ct`: selecting Qt appearance without the matching consumer runtime is inert configuration. Consumers that include one may add a small data-only drop-in and its matching Qt settings file.

Wine is not installed by the reference image. A Wine consumer selects a workload profile such as `PELAGIAN_SHELL_PROFILE=legacy-apps`; that profile enables `[capabilities].wine = true`. After prefix initialization, it explicitly runs `pelagian-shell-apply-wine-defaults`. The initial registry only sets broadly portable colors; DPI and font defaults remain unconfigured until a Wine/XWayland matrix proves them. No `msstyles` dependency is introduced.

The first reference artifact intentionally retains Grotto's pinned Selkies digest. It does not call `labwc -i`, enable Pelorus, or consume the downstream Labwc IPC patch as an API. LinuxServer documents that its base currently builds Labwc with that read-only patch; rebasing onto a clean upstream-Labwc runtime is future qualification work, not an unverified claim for this artifact.
