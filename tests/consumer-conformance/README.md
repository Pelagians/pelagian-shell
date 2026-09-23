# Consumer session conformance

These files are test tooling, not part of the production Shell image. Downstream
repos check out one immutable Shell commit and run the scripts from that tree.

- `start-shell-stream.sh <container> <selkies-smoke-client.py>` starts a real
  Selkies viewer and waits for a decoded 1920x1080 frame. Set
  `CONTAINER_ENGINE=podman` for a rootless runtime.
- `verify-shell-session.py <app-pattern> [--native] [--keyring store|lookup] [--opaque-binary-env]`
  runs as the desktop user inside the container. It checks the live Labwc
  inventory, healthy layoutd status, usable-area geometry, decoration, and
  optionally the native Wayland and libsecret contracts.

`--opaque-binary-env` is only valid with `--native` and without `--keyring`.
Use it only for an inspected binary-only package whose window process hides
its environment after the launcher inherits the Shell session. It requires
Shell's private runtime path, live bus and Wayland sockets, a single live
XWayland display, and a Wayland toplevel absent from the X11 client list.
Any session coordinate the binary does expose must match Shell's coordinates.
This exception does not prove the vendor binary uses D-Bus or qualify keyring
persistence. The ordinary and keyring checks require the application process
itself to expose the `/run` runtime and correct bus address.

The caller owns the container lifecycle, persistent volumes, and any application
checks. Pin the Shell commit used to fetch these files, and check out the whole
tree so the viewer and its checksum travel together. Changes to these scripts
require downstream runtime qualification before updating that pin.

`test_verify_shell_session.py` is the shared geometry regression suite. It runs
as part of Shell's source checks so consumers do not need to copy these tests.
