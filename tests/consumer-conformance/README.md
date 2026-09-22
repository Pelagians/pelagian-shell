# Consumer session conformance

These files are test tooling, not part of the production Shell image. Downstream
repos check out one immutable Shell commit and run the scripts from that tree.

- `start-shell-stream.sh <container> <selkies-smoke-client.py>` starts a real
  Selkies viewer and waits for a decoded 1920x1080 frame. Set
  `CONTAINER_ENGINE=podman` for a rootless runtime.
- `verify-shell-session.py <app-pattern> [--native] [--keyring store|lookup]`
  runs as the desktop user inside the container. It checks the live Labwc
  inventory, healthy layoutd status, usable-area geometry, decoration, and
  optionally the native Wayland and libsecret contracts.

The caller owns the container lifecycle, persistent volumes, and any application
checks. Pin the Shell commit used to fetch these files, and check out the whole
tree so the viewer and its checksum travel together. Changes to these scripts
require downstream runtime qualification before updating that pin.

`test_verify_shell_session.py` is the shared geometry regression suite. It runs
as part of Shell's source checks so consumers do not need to copy these tests.
