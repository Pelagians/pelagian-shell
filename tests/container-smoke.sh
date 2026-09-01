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
# Invoked indirectly by the trap below.
# shellcheck disable=SC2317,SC2329
cleanup() {
    rc=$?
    if [ "$rc" -ne 0 ]; then
        "$engine" logs "$name" >&2 || true
        "$engine" exec "$name" cat /config/.local/state/pelagian-shell/layoutd.log >&2 || true
        "$engine" exec "$name" pelagian-layoutd status >&2 || true
        "$engine" exec "$name" ps aux >&2 || true
    fi
    "$engine" rm -f "$name" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

exec_x11() {
    "$engine" exec --user abc --env DISPLAY=:0 "$name" "$@"
}

window_id() {
    title=$1
    exec_x11 wmctrl -l | awk -v title="$title" 'index($0, title) { print $1; exit }'
}

wait_for_window() {
    title=$1
    count=0
    while [ "$count" -lt 40 ]; do
        id=$(window_id "$title")
        if [ -n "$id" ]; then
            printf '%s\n' "$id"
            return 0
        fi
        count=$((count + 1))
        sleep 1
    done
    return 1
}

wait_for_maximized() {
    id=$1
    count=0
    while [ "$count" -lt 40 ]; do
        state=$(exec_x11 xprop -id "$id" _NET_WM_STATE 2>/dev/null || true)
        if printf '%s' "$state" | grep -q _NET_WM_STATE_MAXIMIZED_VERT \
            && printf '%s' "$state" | grep -q _NET_WM_STATE_MAXIMIZED_HORZ; then
            return 0
        fi
        count=$((count + 1))
        sleep 1
    done
    return 1
}

wait_for_halves() {
    left=$1
    right=$2
    count=0
    while [ "$count" -lt 40 ]; do
        dimensions=$(exec_x11 wmctrl -d | awk '/\*/ { for (i=1; i<=NF; i++) if ($i == "DG:") { print $(i+1); exit } }')
        width=${dimensions%x*}
        height=${dimensions#*x}
        half=$((width / 2))
        left_geometry=$(exec_x11 wmctrl -lG | awk -v id="$left" '$1 == id { print $3, $4, $5, $6; exit }')
        right_geometry=$(exec_x11 wmctrl -lG | awk -v id="$right" '$1 == id { print $3, $4, $5, $6; exit }')
        if [ "$left_geometry" = "0 0 $half $height" ] \
            && [ "$right_geometry" = "$half 0 $((width - half)) $height" ]; then
            return 0
        fi
        count=$((count + 1))
        sleep 1
    done
    return 1
}

start_window() {
    title=$1
    exec_x11 sh -c "xmessage -title '$title' -name '$title' -buttons OK:0 -geometry 320x200+40+40 '$title' >/tmp/$title.log 2>&1 & echo \$!"
}

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
        shell_status=$("$engine" exec "$name" pelagian-shellctl status 2>/dev/null || true)
        layout_status=$("$engine" exec "$name" pelagian-layoutd status 2>/dev/null || true)
        if printf '%s' "$shell_status" | grep -q '"layoutd":"running"' \
            && printf '%s' "$layout_status" | grep -q '"compositor_adapter":"xwayland-ewmh"'; then
            "$engine" exec "$name" pelagian-shellctl config show >/dev/null
            break
        fi
    fi
    attempt=$((attempt + 1))
    sleep 1
done

if [ "$attempt" -ge 60 ]; then
    "$engine" logs "$name" >&2 || true
    echo "pelagian-shell smoke: Labwc, layoutd, session autostart, or Selkies HTTPS did not become ready" >&2
    exit 1
fi

first_title=pelagian-layout-primary
first_pid=$(start_window "$first_title")
first_xid=$(wait_for_window "$first_title")
wait_for_maximized "$first_xid"
layoutd_pid=$("$engine" exec "$name" pgrep -fo '[p]elagian-layoutd')
fd_before=$("$engine" exec "$name" sh -c "find /proc/$layoutd_pid/fd -mindepth 1 -maxdepth 1 | wc -l")

second_title=pelagian-layout-secondary
second_pid=$(start_window "$second_title")
second_xid=$(wait_for_window "$second_title")
wait_for_halves "$first_xid" "$second_xid"
test "$(window_id "$first_title")" = "$first_xid"

dialog_title=pelagian-layout-dialog
dialog_pid=$(start_window "$dialog_title")
dialog_xid=$(wait_for_window "$dialog_title")
exec_x11 xprop -id "$dialog_xid" -f _NET_WM_WINDOW_TYPE 32a \
    -set _NET_WM_WINDOW_TYPE _NET_WM_WINDOW_TYPE_DIALOG >/dev/null
exec_x11 xprop -id "$dialog_xid" -f WM_TRANSIENT_FOR 32x \
    -set WM_TRANSIENT_FOR "$first_xid" >/dev/null
sleep 1
wait_for_halves "$first_xid" "$second_xid"
exec_x11 xprop -id "$dialog_xid" _NET_WM_WINDOW_TYPE WM_TRANSIENT_FOR | grep -q _NET_WM_WINDOW_TYPE_DIALOG
exec_x11 kill "$dialog_pid"

exec_x11 kill "$second_pid"
wait_for_maximized "$first_xid"
test "$(window_id "$first_title")" = "$first_xid"

cycle=1
while [ "$cycle" -le 5 ]; do
    title="pelagian-layout-cycle-$cycle"
    pid=$(start_window "$title")
    xid=$(wait_for_window "$title")
    wait_for_halves "$first_xid" "$xid"
    test "$(window_id "$first_title")" = "$first_xid"
    exec_x11 kill "$pid"
    wait_for_maximized "$first_xid"
    cycle=$((cycle + 1))
done

test "$("$engine" exec "$name" pgrep -fc '[p]elagian-layoutd')" = 1
fd_after=$("$engine" exec "$name" sh -c "find /proc/$layoutd_pid/fd -mindepth 1 -maxdepth 1 | wc -l")
test "$fd_after" -le "$((fd_before + 2))"
exec_x11 xwininfo -id "$first_xid" >/dev/null
exec_x11 kill "$first_pid"

echo "pelagian-shell smoke: PASS image=$image engine=$engine first_xid=$first_xid cycles=5 fd_before=$fd_before fd_after=$fd_after"
exit 0
