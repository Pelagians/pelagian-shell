#!/bin/sh
set -eu

image=${1:-${IMAGE:-pelagian-shell:local}}
engine=${ENGINE:-}

if [ -z "$engine" ]; then
    for candidate in docker podman; do
        if command -v "$candidate" >/dev/null 2>&1; then
            engine=$candidate
            break
        fi
    done
fi

if [ -z "$engine" ]; then
    echo "pelagian-shell smoke: Docker or Podman is required" >&2
    exit 2
fi
if ! "$engine" info >/dev/null 2>&1; then
    echo "pelagian-shell smoke: $engine daemon is unavailable" >&2
    exit 2
fi

name="pelagian-shell-smoke-$$"
port=${PELAGIAN_SHELL_SMOKE_PORT:-13001}
sentinel=/tmp/pelagian-shell-session-smoke
cleanup() {
    "$engine" rm -f "$name" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

# Start the image's real /init entrypoint; do not override it.
"$engine" run -d --name "$name" --shm-size=1g \
    --publish "127.0.0.1:${port}:3001" \
    --env "PUID=$(id -u)" \
    --env "PGID=$(id -g)" \
    --env PIXELFLUX_WAYLAND=true \
    --env PELAGIAN_SHELL_SESSION_SENTINEL="$sentinel" \
    "$image" >/dev/null

attempt=0
while [ "$attempt" -lt 60 ]; do
    if "$engine" exec "$name" pgrep -x labwc >/dev/null 2>&1 \
        && "$engine" exec "$name" test -f "$sentinel" \
        && curl --fail --silent --show-error --insecure --max-time 3 "https://127.0.0.1:${port}/" >/dev/null 2>&1; then
        "$engine" exec "$name" pelagian-shellctl status >/dev/null
        "$engine" exec "$name" pelagian-shellctl config show >/dev/null
        "$engine" exec "$name" pelagian-layoutd status >/dev/null
        echo "pelagian-shell smoke: PASS image=$image engine=$engine"
        exit 0
    fi
    attempt=$((attempt + 1))
    sleep 1
done

"$engine" logs "$name" >&2 || true
echo "pelagian-shell smoke: Labwc, session autostart, or Selkies HTTPS did not become ready" >&2
exit 1
