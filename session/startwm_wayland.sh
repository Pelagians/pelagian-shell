#!/bin/sh
set -eu

ulimit -c 0
export XCURSOR_THEME=breeze_cursors
export XCURSOR_SIZE=24
export XKB_DEFAULT_LAYOUT=us
export XKB_DEFAULT_RULES=evdev
export WAYLAND_DISPLAY=wayland-1

state_dir=${XDG_STATE_HOME:-/config/.local/state}/pelagian-shell
umask 077
mkdir -p "$state_dir"
if [ "${PELAGIAN_SHELL_LABWC_VERBOSE:-false}" = true ]; then
    exec labwc -i -V > "$state_dir/labwc.log" 2>&1
fi
exec labwc -i > "$state_dir/labwc.log" 2>&1
