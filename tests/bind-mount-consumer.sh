#!/bin/sh
set -eu

state_dir=${XDG_STATE_HOME:-/config/.local/state}/pelagian-shell
mkdir -p "$state_dir"
{
    printf 'XDG_RUNTIME_DIR=%s\n' "${XDG_RUNTIME_DIR:-}"
    printf 'PELAGIAN_SHELL_WINDOW_CHROME=%s\n' "${PELAGIAN_SHELL_WINDOW_CHROME:-}"
    printf 'DBUS_SESSION_BUS_ADDRESS=%s\n' "${DBUS_SESSION_BUS_ADDRESS:-}"
} > "$state_dir/bind-consumer-environment"

exec sleep infinity
