#!/bin/sh
set -eu

image=${1:-${IMAGE:-pelagian-shell:local}}
engine=${ENGINE:-}
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

if [ -z "$engine" ]; then
    for candidate in docker podman; do
        if command -v "$candidate" >/dev/null 2>&1 && "$candidate" info >/dev/null 2>&1; then
            engine=$candidate
            break
        fi
    done
fi

if [ -z "$engine" ]; then
    echo "pelagian-shell smoke: an available Docker or Podman daemon is required" >&2
    exit 2
fi
if ! "$engine" info >/dev/null 2>&1; then
    echo "pelagian-shell smoke: $engine daemon is unavailable" >&2
    exit 2
fi

if [ "$#" -lt 3 ]; then
    ENGINE="$engine" sh "$0" "$image" 1920 1080
    ENGINE="$engine" sh "$0" "$image" 1366 768
    echo "pelagian-shell smoke: PASS image=$image engine=$engine resolutions=1920x1080,1366x768"
    exit 0
fi

width=$2
height=$3
case "$width" in
    ''|*[!0-9]*|0)
        echo "pelagian-shell smoke: resolution must contain positive integers" >&2
        exit 2
        ;;
esac
case "$height" in
    ''|*[!0-9]*|0)
        echo "pelagian-shell smoke: resolution must contain positive integers" >&2
        exit 2
        ;;
esac

name="pelagian-shell-smoke-$$"
config_volume="$name-config"
port=${PELAGIAN_SHELL_SMOKE_PORT:-13001}
sentinel=/tmp/pelagian-shell-session-smoke

labwc_state() {
    "$engine" exec --user abc \
        --env XDG_RUNTIME_DIR=/config/.XDG \
        "$name" python3 -c '
import os, socket
s = socket.socket(socket.AF_UNIX)
s.settimeout(2)
s.connect(os.path.join(os.environ["XDG_RUNTIME_DIR"], "labwc.sock"))
s.sendall(b"LIST\n")
data = b""
while True:
    chunk = s.recv(65536)
    if not chunk:
        break
    data += chunk
print(data.decode(), end="")
'
}

dump_failure() {
    echo "pelagian-shell smoke: failure diagnostics" >&2
    "$engine" logs "$name" >&2 2>/dev/null || true
    labwc_state >&2 2>/dev/null || true
    "$engine" exec "$name" pelagian-layoutd status >&2 2>/dev/null || true
    "$engine" exec "$name" sh -c \
        'for file in /config/.local/state/pelagian-shell/output-mode.status /config/.local/state/pelagian-shell/output-mode.log /config/.local/state/pelagian-shell/labwc.log /config/.local/state/pelagian-shell/layoutd.log /config/.local/state/pelagian-shell/consumer.log; do test ! -f "$file" || { echo "--- $file"; tail -n 100 "$file"; }; done' \
        >&2 2>/dev/null || true
}

finish() {
    rc=$?
    trap - EXIT INT TERM
    if [ "$rc" -ne 0 ]; then
        dump_failure
    fi
    "$engine" rm -f "$name" >/dev/null 2>&1 || true
    "$engine" volume rm -f "$config_volume" >/dev/null 2>&1 || true
    exit "$rc"
}
trap finish EXIT
trap 'exit 130' INT TERM

stage_matches() {
    python3 - "$1" "$2" "$3" "$4" "$5" "$6" <<'PY'
import json
import sys

count, width, height = map(int, sys.argv[1:4])
focused, expect_dialog, raw = sys.argv[4:]
state = json.loads(raw)
titles = [
    "Pelagian Fixture One",
    "Pelagian Fixture Two",
    "Pelagian Fixture Three",
    "Pelagian Fixture Four",
    "Pelagian Fixture Five",
    "Pelagian Fixture Six",
]
regions = {
    2: ["auto-2-left", "auto-2-right"],
    3: ["auto-3-left", "auto-3-right-top", "auto-3-right-bottom"],
    4: ["auto-4-top-left", "auto-4-top-right", "auto-4-bottom-left", "auto-4-bottom-right"],
    5: ["auto-5-r0-c0", "auto-5-r0-c1", "auto-5-r0-c2", "auto-5-r1-c0", "auto-5-r1-c1"],
    6: ["auto-6-r0-c0", "auto-6-r0-c1", "auto-6-r0-c2", "auto-6-r1-c0", "auto-6-r1-c1", "auto-6-r1-c2"],
}
expected_geometry = {
    2: [(0, 0, 50, 100), (50, 0, 50, 100)],
    3: [(0, 0, 50, 100), (50, 0, 50, 50), (50, 50, 50, 50)],
    4: [(0, 0, 50, 50), (50, 0, 50, 50), (0, 50, 50, 50), (50, 50, 50, 50)],
    5: [(0, 0, 33, 50), (33, 0, 34, 50), (67, 0, 33, 50), (0, 50, 50, 50), (50, 50, 50, 50)],
    6: [(0, 0, 33, 50), (33, 0, 34, 50), (67, 0, 33, 50), (0, 50, 33, 50), (33, 50, 34, 50), (67, 50, 33, 50)],
}
views = {view["title"]: view for view in state["views"]}
assert not any(title in views for title in titles[count:])
normals = [views[title] for title in titles[:count]]
area = state["outputs"][0]["usable_area"]
assert (area["width"], area["height"]) == (width, height)

for view in normals:
    assert view["type"] == "normal" and view["parent_id"] is None
    assert view["decoration"] == "full" and view["titlebar_visible"]
    assert not view["minimized"] and not view["fullscreen"]
    assert view["client_width"] >= 320 and view["client_height"] >= 200
    assert view["x"] >= area["x"] and view["y"] >= area["y"]
    assert view["x"] + view["width"] <= area["x"] + area["width"]
    assert view["y"] + view["height"] <= area["y"] + area["height"]

if count == 1:
    view = normals[0]
    assert view["maximized"] and not view["tiled"] and view["region"] == ""
    assert all(view[key] == area[key] for key in ("x", "y", "width", "height"))
else:
    assert [view["region"] for view in normals] == regions[count]
    assert all(view["tiled"] and not view["maximized"] for view in normals)
    for view, (x, y, region_width, region_height) in zip(normals, expected_geometry[count]):
        left = area["width"] * x // 100
        right = area["width"] * (x + region_width) // 100
        top = area["height"] * y // 100
        bottom = area["height"] * (y + region_height) // 100
        expected = (area["x"] + left, area["y"] + top, right - left, bottom - top)
        actual = tuple(view[key] for key in ("x", "y", "width", "height"))
        assert actual == expected
    assert sum(view["width"] * view["height"] for view in normals) == area["width"] * area["height"]
    for index, first in enumerate(normals):
        for second in normals[index + 1:]:
            overlap_width = min(first["x"] + first["width"], second["x"] + second["width"]) - max(first["x"], second["x"])
            overlap_height = min(first["y"] + first["height"], second["y"] + second["height"]) - max(first["y"], second["y"])
            assert overlap_width <= 0 or overlap_height <= 0

assert len({view["app_id"] for view in normals}) == 1
assert normals[0]["app_id"] != ""
assert len({view["pid"] for view in normals}) == count
focused_normals = [view for view in normals if view["focused"]]
if focused == "any":
    assert len(focused_normals) == 1
elif focused != "-":
    assert len(focused_normals) == 1
    assert views[f"Pelagian Fixture {focused}"]["focused"]

dialog = views.get("Pelagian Fixture Dialog")
if expect_dialog == "1":
    assert dialog and dialog["type"] == "dialog"
    assert dialog["parent_id"] == normals[0]["id"]
    assert dialog["decoration"] == "full" and dialog["titlebar_visible"]
    assert not dialog["minimized"] and not dialog["fullscreen"]
    assert not dialog["maximized"] and not dialog["tiled"] and dialog["region"] == ""
    assert dialog["client_width"] >= 320 and dialog["client_height"] >= 200
    assert dialog["x"] >= area["x"] and dialog["y"] >= area["y"]
    assert dialog["x"] + dialog["width"] <= area["x"] + area["width"]
    assert dialog["y"] + dialog["height"] <= area["y"] + area["height"]
else:
    assert dialog is None
PY
}

wait_layout() {
    expected_count=$1
    expected_width=$2
    expected_height=$3
    expected_focus=$4
    expected_dialog=$5
    attempt=0
    while [ "$attempt" -lt 200 ]; do
        state=$(labwc_state 2>/dev/null || true)
        if [ -n "$state" ] && stage_matches \
            "$expected_count" "$expected_width" "$expected_height" \
            "$expected_focus" "$expected_dialog" "$state" 2>/dev/null; then
            printf 'pelagian-shell layout: windows=%s resolution=%sx%s focus=%s dialog=%s\n' \
                "$expected_count" "$expected_width" "$expected_height" \
                "$expected_focus" "$expected_dialog"
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 0.1
    done
    echo "pelagian-shell smoke: layout did not converge" >&2
    return 1
}

wait_counts() {
    expected_managed=$1
    expected_floating=$2
    attempt=0
    status=
    while [ "$attempt" -lt 200 ]; do
        status=$("$engine" exec "$name" pelagian-layoutd status 2>/dev/null || true)
        if printf '%s\n' "$status" | python3 -c \
            'import json, sys; state=json.load(sys.stdin); assert type(state["managed_windows"]) is int and type(state["floating_windows"]) is int; assert state["managed_windows"] == int(sys.argv[1]) and state["floating_windows"] == int(sys.argv[2])' \
            "$expected_managed" "$expected_floating" 2>/dev/null; then
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 0.1
    done
    echo "pelagian-shell smoke: layoutd counts did not converge" >&2
    printf '%s\n' "$status" >&2
    return 1
}

launch_fixture() {
    "$engine" exec -d --user abc \
        --env GDK_BACKEND=wayland \
        --env XDG_RUNTIME_DIR=/config/.XDG \
        --env WAYLAND_DISPLAY=wayland-1 \
        "$name" /usr/local/bin/pelagian-shell-consumer "$1"
}

send_command() {
    fixture_mode=$1
    fixture_command=$2
    "$engine" exec --user abc "$name" sh -c \
        'rm -f "/tmp/pelagian-layout-$1.ack"; printf %s "$2" > "/tmp/pelagian-layout-$1.command"' \
        sh "$fixture_mode" "$fixture_command"
    attempt=0
    while [ "$attempt" -lt 100 ]; do
        ack=$("$engine" exec --user abc "$name" sh -c \
            'test ! -f "/tmp/pelagian-layout-$1.ack" || cat "/tmp/pelagian-layout-$1.ack"' \
            sh "$fixture_mode" 2>/dev/null || true)
        if [ "$ack" = "$fixture_command" ]; then
            sleep 0.5
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 0.1
    done
    echo "pelagian-shell smoke: fixture $fixture_mode did not acknowledge $fixture_command" >&2
    return 1
}

pause_layoutd() {
    status=$("$engine" exec "$name" pelagian-layoutd status)
    layoutd_pid=$(printf '%s\n' "$status" | python3 -c 'import json, sys; print(json.load(sys.stdin)["pid"])')
    "$engine" exec "$name" kill -STOP "$layoutd_pid"
}

resume_layoutd() {
    "$engine" exec "$name" kill -CONT "$layoutd_pid"
}

labwc_action() {
    title=$1
    action=$2
    state=$(labwc_state)
    toplevel_id=$(python3 -c \
        'import json, sys; state=json.loads(sys.argv[1]); print(next(view["id"] for view in state["views"] if view["title"] == sys.argv[2]))' \
        "$state" "$title")
    "$engine" exec -i --user abc --env XDG_RUNTIME_DIR=/config/.XDG \
        "$name" python3 - "$toplevel_id" "$action" <<'PY'
import json
import os
import socket
import sys

request = f"ACTION {sys.argv[1]} {sys.argv[2]}\n".encode()
with socket.socket(socket.AF_UNIX) as client:
    client.settimeout(2)
    client.connect(os.path.join(os.environ["XDG_RUNTIME_DIR"], "labwc.sock"))
    client.sendall(request)
    response = b""
    while True:
        chunk = client.recv(65536)
        if not chunk:
            break
        response += chunk
result = json.loads(response)
assert result.get("ok") is True, result
PY
}

wait_disrupted() {
    title=$1
    disruption=$2
    attempt=0
    while [ "$attempt" -lt 100 ]; do
        state=$(labwc_state 2>/dev/null || true)
        if [ -n "$state" ] && python3 - "$title" "$disruption" "$state" <<'PY' 2>/dev/null
import json
import sys

title, disruption, raw = sys.argv[1:]
view = next(view for view in json.loads(raw)["views"] if view["title"] == title)
if disruption == "resize":
    assert not view["tiled"] and not view["maximized"] and view["region"] == ""
    assert view["client_width"] <= 400 and view["client_height"] <= 320
elif disruption == "fullscreen":
    assert view["fullscreen"]
elif disruption == "minimize":
    assert view["minimized"]
else:
    raise AssertionError(disruption)
PY
        then
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 0.1
    done
    echo "pelagian-shell smoke: $disruption disruption was not observed for $title" >&2
    return 1
}

restart_layoutd() {
    status=$("$engine" exec "$name" pelagian-layoutd status)
    old_pid=$(printf '%s\n' "$status" | python3 -c 'import json, sys; print(json.load(sys.stdin)["pid"])')
    "$engine" exec "$name" kill "$old_pid"
    attempt=0
    while [ "$attempt" -lt 100 ]; do
        if ! "$engine" exec "$name" kill -0 "$old_pid" >/dev/null 2>&1; then
            break
        fi
        attempt=$((attempt + 1))
        sleep 0.1
    done
    [ "$attempt" -lt 100 ]
    "$engine" exec -d --user abc \
        --env XDG_RUNTIME_DIR=/config/.XDG \
        --env XDG_STATE_HOME=/config/.local/state \
        "$name" sh -c \
        'exec pelagian-layoutd >> /config/.local/state/pelagian-shell/layoutd.log 2>&1'
    attempt=0
    while [ "$attempt" -lt 200 ]; do
        status=$("$engine" exec "$name" pelagian-layoutd status 2>/dev/null || true)
        if printf '%s\n' "$status" | python3 -c \
            'import json, sys; state=json.load(sys.stdin); assert state["layoutd"] == "healthy" and state["adapter_connected"] and state["reconciliation"] == "healthy" and state["pid"] != int(sys.argv[1])' \
            "$old_pid" 2>/dev/null; then
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 0.1
    done
    echo "pelagian-shell smoke: layoutd did not recover after restart" >&2
    return 1
}

assert_native_wayland() {
    xclients=$("$engine" exec --user abc --env DISPLAY=:0 "$name" xlsclients -l 2>/dev/null)
    if printf '%s\n' "$xclients" | grep -Eqi \
        'pelagian-layout-fixture|pelagian-shell-consumer|Pelagian Fixture (One|Two|Three|Four|Five|Six)'; then
        echo "pelagian-shell smoke: GTK fixture unexpectedly appeared in xlsclients" >&2
        printf '%s\n' "$xclients" >&2
        return 1
    fi
}

"$engine" volume create "$config_volume" >/dev/null
"$engine" run --rm --entrypoint /bin/sh \
    --volume "$config_volume:/config" \
    "$image" -c '
set -eu
mkdir -p /config/.config/labwc
printf "%s\n" preserve-me > /config/pelagian-shell-smoke.sentinel
printf "%s\n" "<stale />" > /config/.config/labwc/rc.xml
'

# Start the image's real /init entrypoint; do not override it.
mount_mode=ro
[ "$engine" = podman ] && mount_mode=ro,z
"$engine" run -d --name "$name" --shm-size=1g \
    --publish "127.0.0.1:${port}:3001" \
    --env "PUID=$(id -u)" \
    --env "PGID=$(id -g)" \
    --env PIXELFLUX_WAYLAND=true \
    --env SELKIES_MANUAL_WIDTH="$width" \
    --env SELKIES_MANUAL_HEIGHT="$height" \
    --env PELAGIAN_SHELL_SESSION_SENTINEL="$sentinel" \
    --volume "$config_volume:/config" \
    --volume "$root/tests/layout-fixture.py:/usr/local/bin/pelagian-shell-consumer:$mount_mode" \
    "$image" >/dev/null

ready=
attempt=0
while [ "$attempt" -lt 60 ]; do
    status=$("$engine" exec "$name" pelagian-layoutd status 2>/dev/null || true)
    if "$engine" exec "$name" pgrep -x labwc >/dev/null 2>&1 \
        && printf '%s\n' "$status" | python3 -c \
            'import json, sys; state=json.load(sys.stdin); assert state["layoutd"] == "healthy" and state["adapter_connected"] and state["reconciliation"] == "healthy"' \
            2>/dev/null \
        && "$engine" exec "$name" test -f "$sentinel" \
        && curl --fail --silent --show-error --insecure --max-time 3 \
            "https://127.0.0.1:${port}/" >/dev/null 2>&1; then
        ready=1
        break
    fi
    attempt=$((attempt + 1))
    sleep 1
done
if [ -z "$ready" ]; then
    echo "pelagian-shell smoke: Labwc, session autostart, or Selkies HTTPS did not become ready" >&2
    exit 1
fi

"$engine" exec "$name" pelagian-shellctl status >/dev/null
"$engine" exec "$name" pelagian-shellctl config show >/dev/null
status=$("$engine" exec "$name" pelagian-layoutd status)
if printf '%s\n' "$status" | grep -q planner_only; then
    echo "pelagian-shell smoke: planner-only layoutd is not accepted" >&2
    exit 1
fi
printf '%s\n' "$status" | grep -q '"layoutd":"healthy"'
printf '%s\n' "$status" | grep -q '"compositor_adapter":"labwc-ipc"'
printf '%s\n' "$status" | grep -q '"adapter_connected":true'
printf '%s\n' "$status" | grep -q '"reconciliation":"healthy"'
"$engine" exec "$name" sh -c '
test "$(cat /config/pelagian-shell-smoke.sentinel)" = preserve-me
cmp -s /defaults/labwc.xml /config/.config/labwc/rc.xml
grep -Fq "<layout>:close</layout>" /config/.config/labwc/rc.xml
grep -Fqx "gtk-decoration-layout=:close" /config/.config/gtk-3.0/settings.ini
grep -Fqx "gtk-decoration-layout=:close" /config/.config/gtk-4.0/settings.ini
'

wait_layout 1 "$width" "$height" One 0
wait_counts 1 0

count=1
for fixture_mode in second third fourth fifth sixth; do
    count=$((count + 1))
    launch_fixture "$fixture_mode"
    case "$fixture_mode" in
        second) focused=Two ;;
        third) focused=Three ;;
        fourth) focused=Four ;;
        fifth) focused=Five ;;
        sixth) focused=Six ;;
    esac
    wait_layout "$count" "$width" "$height" "$focused" 0
    wait_counts "$count" 0
done
assert_native_wayland

send_command first dialog
wait_layout 6 "$width" "$height" - 1
wait_counts 6 1
send_command first dialog-close
wait_layout 6 "$width" "$height" - 0
wait_counts 6 0

send_command first focus
wait_layout 6 "$width" "$height" One 0
send_command fourth focus
wait_layout 6 "$width" "$height" Four 0

pause_layoutd
labwc_action "Pelagian Fixture Three" FLOAT
send_command third resize
wait_disrupted "Pelagian Fixture Three" resize
send_command fourth focus
resume_layoutd
wait_layout 6 "$width" "$height" Four 0

pause_layoutd
send_command fifth fullscreen
wait_disrupted "Pelagian Fixture Five" fullscreen
send_command fourth focus
resume_layoutd
wait_layout 6 "$width" "$height" Four 0

pause_layoutd
send_command second minimize
wait_disrupted "Pelagian Fixture Two" minimize
send_command fourth focus
resume_layoutd
wait_layout 6 "$width" "$height" Four 0

wait_counts 6 0
restart_layoutd
wait_layout 6 "$width" "$height" Four 0
wait_counts 6 0

count=6
for fixture_mode in sixth fifth fourth third second; do
    fixture_pid=$("$engine" exec "$name" cat "/tmp/pelagian-layout-$fixture_mode.pid")
    "$engine" exec "$name" kill "$fixture_pid"
    count=$((count - 1))
    case "$fixture_mode" in
        sixth|fifth) focused=Four ;;
        *) focused=any ;;
    esac
    wait_layout "$count" "$width" "$height" "$focused" 0
    wait_counts "$count" 0
done

echo "pelagian-shell smoke: PASS image=$image engine=$engine resolution=${width}x${height}"
