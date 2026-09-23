#!/bin/sh
set -eu

image=${1:-${IMAGE:-pelagian-shell:local}}
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

if [ "$(podman info --format '{{.Host.Security.Rootless}}')" != true ]; then
    echo "pelagian-shell bind-mount smoke: rootless Podman is required" >&2
    exit 2
fi

name="pelagian-shell-bind-smoke-$$"
host_config=$(mktemp -d)
port=3001

cleanup() {
    rc=$?
    trap - EXIT INT TERM
    if [ "$rc" -ne 0 ]; then
        podman logs "$name" >&2 2>/dev/null || true
        podman exec "$name" sh -c \
            'for file in /config/.local/state/pelagian-shell/*.log; do test ! -f "$file" || { echo "--- $file"; tail -n 100 "$file"; }; done' \
            >&2 2>/dev/null || true
        podman exec "$name" sh -c '
            echo "--- s6 active services"; s6-rc -a list 2>&1 || true
            echo "--- Shell s6 definitions"
            find /etc/s6-overlay/s6-rc.d/init-pelagian-runtime \
                /etc/s6-overlay/s6-rc.d/user/contents.d \
                /etc/s6-overlay/s6-rc.d/svc-de/dependencies.d \
                /etc/s6-overlay/s6-rc.d/svc-pulseaudio/dependencies.d \
                /etc/s6-overlay/s6-rc.d/svc-selkies/dependencies.d \
                -maxdepth 2 -type f \( -name '*pelagian*' -o -path '*/init-pelagian-runtime/*' \) \
                -print 2>&1 || true
            echo "--- compiled Shell s6 graph"
            if command -v s6-rc-db >/dev/null 2>&1; then
                s6-rc-db list all | grep -E "^(init-pelagian-runtime|init-selkies-config|svc-de|svc-pulseaudio|svc-selkies|user)$" || true
                for service in init-pelagian-runtime svc-de svc-pulseaudio svc-selkies; do
                    echo "$service dependencies"; s6-rc-db dependencies "$service" 2>&1 || true
                done
                echo "user bundle"; s6-rc-db contents user 2>&1 || true
            fi
            echo "--- s6 runtime directories"; find /run/s6-rc -maxdepth 3 -type d -print 2>&1 || true
            echo "--- session environment"
            for key in XDG_RUNTIME_DIR WAYLAND_DISPLAY PIXELFLUX_WAYLAND CUSTOM_WS_PORT LD_PRELOAD; do
                if test -r "/run/s6/container_environment/$key"; then
                    printf "%s=" "$key"; cat "/run/s6/container_environment/$key"; echo
                fi
            done
            echo "--- runtime directory"; ls -ld /run/pelagian-shell 2>&1 || true
            ls -la /run/pelagian-shell 2>&1 || true
            echo "--- input setup"; ls -la /dev/input /tmp/selkies* 2>&1 || true
            echo "--- processes"
            for proc in /proc/[0-9]*/comm; do
                test -r "$proc" || continue
                pid=${proc#/proc/}; pid=${pid%/comm}
                IFS= read -r command < "$proc" || true
                printf "%s %s " "$pid" "$command"
                cat "/proc/$pid/wchan" 2>/dev/null || true
            done
        ' >&2 2>/dev/null || true
    fi
    podman rm -f "$name" >/dev/null 2>&1 || true
    rm -rf "$host_config"
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

mkdir -p "$host_config/.XDG" "$host_config/.local/share/keyrings"
printf '%s\n' old-runtime-state > "$host_config/.XDG/legacy-sentinel"
printf '%s\n' persistent-keyring > "$host_config/.local/share/keyrings/keyring.sentinel"

assert_no_wayland_permission_error() {
    if podman logs "$name" 2>&1 | grep -Fq 'Could not connect to remote display: Permission denied'; then
        echo "pelagian-shell bind-mount smoke: Wayland runtime permission failure" >&2
        return 1
    fi
    if podman exec "$name" sh -c \
        'grep -R -Fq "Could not connect to remote display: Permission denied" /config/.local/state/pelagian-shell 2>/dev/null'; then
        echo "pelagian-shell bind-mount smoke: Labwc recorded a Wayland runtime permission failure" >&2
        return 1
    fi
}

assert_process_runtime() {
    process=$1
    pid=$2
    runtime=$(podman exec "$name" sh -c \
        'tr "\000" "\n" < "/proc/$1/environ" | sed -n "s/^XDG_RUNTIME_DIR=//p"' sh "$pid")
    [ "$runtime" = /run/pelagian-shell ] || {
        echo "pelagian-shell bind-mount smoke: $process PID $pid has XDG_RUNTIME_DIR=$runtime" >&2
        return 1
    }
}

start_container() {
    podman run -d --name "$name" --shm-size=1g --publish "127.0.0.1::3001" \
        --env "PUID=$(id -u)" --env "PGID=$(id -g)" \
        --env PIXELFLUX_WAYLAND=true \
        --env RUST_BACKTRACE=1 \
        --env SELKIES_MANUAL_WIDTH=1920 --env SELKIES_MANUAL_HEIGHT=1080 \
        --env PELAGIAN_SHELL_LABWC_VERBOSE=true \
        --volume "$host_config:/config:Z" \
        --volume "$root/tests/layout-fixture.py:/usr/local/bin/pelagian-shell-consumer:ro,z" \
        --volume "$root/tests/selkies-smoke-client.py:/tmp/selkies-smoke-client.py:ro,z" \
        "$image" >/dev/null
    port=$(podman port "$name" 3001/tcp | sed 's/.*://')
    [ -n "$port" ]
}

wait_for_session() {
    attempt=0
    ready=false
    status=
    while [ "$attempt" -lt 120 ]; do
        running=$(podman inspect --format '{{.State.Running}}' "$name" 2>/dev/null || true)
        [ "$running" = true ] || break
        status=$(podman exec "$name" pelagian-layoutd status 2>/dev/null || true)
        if podman exec "$name" pgrep -x labwc >/dev/null 2>&1 \
            && podman exec "$name" test -S /run/pelagian-shell/labwc.sock \
            && printf '%s\n' "$status" | python3 -c \
                'import json,sys; s=json.load(sys.stdin); assert s["adapter_connected"] and s["layoutd"] == "healthy" and s["reconciliation"] == "healthy"' \
                2>/dev/null \
            && curl --fail --silent --insecure --max-time 3 "https://127.0.0.1:${port}/" >/dev/null 2>&1; then
            ready=true
            break
        fi
        attempt=$((attempt + 1))
        sleep 1
    done
    [ "$ready" = true ] || {
        echo "pelagian-shell bind-mount smoke: /init session did not become healthy" >&2
        printf '%s\n' "$status" >&2
        return 1
    }

    podman exec "$name" sh -c '
set -eu
test "$(env | sed -n "s/^XDG_RUNTIME_DIR=//p")" = /run/pelagian-shell
test "$(env | sed -n "s/^PELAGIAN_SHELL_WINDOW_CHROME=//p")" = server
test "$(stat -c %u:%g:%a /run/pelagian-shell)" = "$(id -u abc):$(id -g abc):700"
test -S /run/pelagian-shell/labwc.sock
test -S /run/pelagian-shell/bus
set -- /run/pelagian-shell/wayland-*
found=false
for socket in "$@"; do
    if test -S "$socket"; then found=true; fi
done
test "$found" = true
test ! -e /config/.XDG/legacy-sentinel
test "$(cat /config/.local/share/keyrings/keyring.sentinel)" = persistent-keyring
'
    podman exec "$name" pelagian-shellctl status | grep -Fq '"window_chrome_policy":"server"'
    assert_process_runtime Labwc "$(podman exec "$name" pgrep -xo labwc)"
    assert_process_runtime PulseAudio "$(podman exec "$name" pgrep -xo pulseaudio)"
    assert_process_runtime Selkies "$(podman exec "$name" pgrep -o -f '[s]elkies --addr=localhost')"
    assert_process_runtime layoutd "$(podman exec "$name" pgrep -xo pelagian-layoutd)"
    assert_process_runtime session-D-Bus "$(podman exec "$name" pgrep -f '[d]bus-daemon --session --address=unix:path=/run/pelagian-shell/bus')"
    podman exec "$name" sh -c \
        'printf "%s\n" persisted-after-recreate > /config/bind-mount-persistence.sentinel'
    assert_no_wayland_permission_error
}

qualify_stream_and_window() {
    CONTAINER_ENGINE=podman "$root/tests/consumer-conformance/start-shell-stream.sh" \
        "$name" "$root/tests/selkies-smoke-client.py"
    podman cp "$root/tests/consumer-conformance/verify-shell-session.py" \
        "$name:/tmp/verify-shell-session.py"
    podman exec --user abc "$name" python3 /tmp/verify-shell-session.py \
        'Pelagian Fixture One' --native
    assert_process_runtime consumer \
        "$(podman exec "$name" cat /config/.local/state/pelagian-shell/consumer.pid)"
    podman exec "$name" test "$(cat /config/bind-mount-persistence.sentinel)" = persisted-after-recreate
    podman exec "$name" test -S /run/pelagian-shell/wayland-1
    assert_no_wayland_permission_error
}

start_container
wait_for_session
qualify_stream_and_window
podman rm -f "$name" >/dev/null
start_container
wait_for_session
qualify_stream_and_window

printf 'pelagian-shell bind-mount smoke: PASS image=%s rootless-podman config=%s\n' "$image" "$host_config"
