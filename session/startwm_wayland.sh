#!/bin/sh
set -eu

ulimit -c 0
export XCURSOR_THEME=breeze_cursors
export XCURSOR_SIZE=24
export XKB_DEFAULT_LAYOUT=us
export XKB_DEFAULT_RULES=evdev
export WAYLAND_DISPLAY=wayland-1

exec labwc -i >/dev/null 2>&1
