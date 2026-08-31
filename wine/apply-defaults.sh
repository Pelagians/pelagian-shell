#!/bin/sh
set -eu

if ! command -v pelagian-shellctl >/dev/null 2>&1; then
    echo "pelagian-shell: pelagian-shellctl is required to resolve capabilities" >&2
    exit 127
fi

if [ "$(pelagian-shellctl capability wine)" != "true" ]; then
    echo "pelagian-shell: Wine defaults are disabled; select a workload profile with capabilities.wine = true" >&2
    exit 2
fi

if ! command -v wine >/dev/null 2>&1; then
    echo "pelagian-shell: wine is not installed in this consumer image" >&2
    exit 127
fi

registry="${PELAGIAN_SHELL_WINE_REGISTRY:-/usr/share/pelagian-shell/wine/pelagian-shell.reg}"
[ -r "$registry" ] || {
    echo "pelagian-shell: registry defaults not readable: $registry" >&2
    exit 1
}

wine regedit "$registry"
