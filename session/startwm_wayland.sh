#!/bin/sh
set -eu

ulimit -c 0
export XCURSOR_THEME=breeze_cursors
export XCURSOR_SIZE=24
export XKB_DEFAULT_LAYOUT=us
export XKB_DEFAULT_RULES=evdev
export XDG_RUNTIME_DIR=/run/pelagian-shell
export PELAGIAN_SHELL_WINDOW_CHROME=server
export WAYLAND_DISPLAY=wayland-1

# Keep the session bus in the same private, ephemeral runtime directory as the
# Wayland and Labwc sockets. The Grotto consumer may unlock a persistent
# keyring later, but its daemon socket also resolves beneath XDG_RUNTIME_DIR.
bus_address="unix:path=${XDG_RUNTIME_DIR}/bus"
if [ ! -S "${XDG_RUNTIME_DIR}/bus" ]; then
    dbus-daemon --session --address="$bus_address" --fork --nopidfile >/dev/null
fi
export DBUS_SESSION_BUS_ADDRESS=$bus_address

state_dir=${XDG_STATE_HOME:-/config/.local/state}/pelagian-shell
umask 077
mkdir -p "$state_dir"
if [ "${PELAGIAN_SHELL_LABWC_VERBOSE:-false}" = true ]; then
    exec labwc -i -V > "$state_dir/labwc.log" 2>&1
fi
exec labwc -i > "$state_dir/labwc.log" 2>&1
