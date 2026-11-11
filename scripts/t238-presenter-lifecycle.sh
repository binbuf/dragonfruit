#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 presenter lifecycle stress test.
#
#   * EGL fault injection on a direct nested child (`bad-surface`, `other`)
#   * repeated minimize/restore in a controllable nested host
#   * repeated output resize through the host's edge drag
#   * host compositor disconnection
#   * shutdown while the presenter is blocked in a host swap
#
# The child must recover from a surface loss, stay responsive while minimized,
# keep rendering after a resize, exit on host loss, and exit cleanly (with the
# documented leak-and-exit path) when a swap is permanently blocked.
#
# Overrides: OUT_DIR, PROFILE, COMPOSITOR_BIN.
set -u

cd "$(dirname "$0")/.."

OUT_DIR="${OUT_DIR:-/tmp/t238-presenter-lifecycle}"
PROFILE="${PROFILE:-release}"
BIN="${COMPOSITOR_BIN:-$PWD/target/$PROFILE/dragonfruit-compositor}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DRAGONFRUIT_REFRESH_HZ="${DRAGONFRUIT_REFRESH_HZ:-144}"

[ -x "$BIN" ] || {
    echo "t238-presenter-lifecycle: missing $BIN (run make build)" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "t238-presenter-lifecycle: python3 not found" >&2
    exit 1
}
command -v xmessage >/dev/null 2>&1 || {
    echo "t238-presenter-lifecycle: xmessage not found" >&2
    exit 1
}

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
PASS=0
FAIL=0
check() {
    local label="$1" ok="$2"
    if [ "$ok" = "1" ]; then
        echo "PASS $label"
        PASS=$((PASS + 1))
    else
        echo "FAIL $label"
        FAIL=$((FAIL + 1))
    fi
}

# Send a line to a synthetic socket.
send_synth() {
    python3 - "$1" "$2" <<'PY'
import socket
import sys
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.sendto((sys.argv[2] + "\n").encode(), sys.argv[1])
PY
}

# Query a synthetic socket; prints the joined reply or an empty string.
query_synth() {
    python3 - "$1" "$2" <<'PY'
import os
import socket
import sys

synth, command = sys.argv[1], sys.argv[2]
local = synth + ".q"
try:
    os.unlink(local)
except OSError:
    pass
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.bind(local)
sock.settimeout(1.2)
sock.sendto((command + "\n").encode(), synth)
lines = []
try:
    while True:
        data, _ = sock.recvfrom(65535)
        for line in data.decode(errors="replace").splitlines():
            if line == "end":
                print(" ".join(lines))
                raise SystemExit(0)
            lines.append(line)
except Exception:
    print("")
PY
}

field() {
    python3 -c 'import re,sys; m=re.search(sys.argv[1]+r"=(\d+)", sys.argv[2]); print(int(m.group(1)) if m else -1)' "$1" "$2"
}

# A direct nested child on the real session: reliable presenter/fault test.
start_direct() {
    local tag="$1"
    local sock="direct-$tag-$$"
    local synth="$XDG_RUNTIME_DIR/$sock.synth"
    rm -f "$synth" "$XDG_RUNTIME_DIR/$sock" "$XDG_RUNTIME_DIR/$sock.lock"
    DRAGONFRUIT_SYNTHETIC_INPUT="$synth" DRAGONFRUIT_NESTED_SIZE=1280x800 \
        "$BIN" --backend nested --socket-name "$sock" >"$OUT_DIR/direct-$tag.txt" 2>&1 &
    DIRECT_PID=$!
    for _ in $(seq 1 100); do [ -S "$synth" ] && break; sleep 0.1; done
    local display_file="$XDG_RUNTIME_DIR/$sock.x11-display"
    for _ in $(seq 1 100); do [ -s "$display_file" ] && break; sleep 0.1; done
    local display
    display="$(cat "$display_file" 2>/dev/null || true)"
    if [ -n "$display" ]; then
        DISPLAY="$display" xmessage -geometry 500x300+120+120 -title lifecycle lifecycle \
            >/dev/null 2>&1 &
        DIRECT_XMSG=$!
    fi
    DIRECT_SYNTH="$synth"
    DIRECT_SOCK="$sock"
    sleep 1.2
}

stop_direct() {
    kill -9 "${DIRECT_PID:-}" "${DIRECT_XMSG:-}" 2>/dev/null || true
    rm -f "${DIRECT_SYNTH:-}" "$XDG_RUNTIME_DIR/${DIRECT_SOCK:-}.launch-token" \
          "$XDG_RUNTIME_DIR/${DIRECT_SOCK:-}.desktop-launch-token" 2>/dev/null || true
}

echo "t238-presenter-lifecycle: direct child EGL faults"
start_direct faults
before="$(query_synth "$DIRECT_SYNTH" "query frames")"
send_synth "$DIRECT_SYNTH" "set present-fault bad-surface"
send_synth "$DIRECT_SYNTH" "motion-abs 0.5 0.5"
sleep 0.6
after_bad="$(query_synth "$DIRECT_SYNTH" "query frames")"
send_synth "$DIRECT_SYNTH" "set present-fault other"
send_synth "$DIRECT_SYNTH" "motion-abs 0.6 0.5"
sleep 0.6
after_other="$(query_synth "$DIRECT_SYNTH" "query frames")"
send_synth "$DIRECT_SYNTH" "set present-fault none"
send_synth "$DIRECT_SYNTH" "motion-abs 0.4 0.5"
sleep 0.6
after_none="$(query_synth "$DIRECT_SYNTH" "query frames")"
echo "  before: $before"
echo "  bad:    $after_bad"
echo "  other:  $after_other"
echo "  none:   $after_none"
check "bad-surface fault recorded" "$([ "$(field failed "$after_bad")" -ge 1 ] && echo 1 || echo 0)"
check "other fault recorded" "$([ "$(field failed "$after_other")" -ge 2 ] && echo 1 || echo 0)"
check "fault cleared and frames continue" \
    "$([ "$(field completed "$after_none")" -ge "$(field completed "$after_other")" ] && echo 1 || echo 0)"
stop_direct

# Controllable nested host for minimize/restore/resize.
H_SOCK="lc-host-$$"
C_SOCK="lc-child-$$"
H_SYNTH="$XDG_RUNTIME_DIR/$H_SOCK.synth"
C_SYNTH="$XDG_RUNTIME_DIR/$C_SOCK.synth"
ANIM_PID=""
H_PID=""
C_PID=""
cleanup_pair() {
    for pid in "${C_PID:-}" "${H_PID:-}" "${ANIM_PID:-}"; do
        [ -n "$pid" ] && kill -9 "$pid" 2>/dev/null || true
    done
    rm -f "$H_SYNTH" "$C_SYNTH" \
          "$XDG_RUNTIME_DIR/$H_SOCK" "$XDG_RUNTIME_DIR/$H_SOCK.lock" \
          "$XDG_RUNTIME_DIR/$C_SOCK" "$XDG_RUNTIME_DIR/$C_SOCK.lock"
}
trap cleanup_pair EXIT

start_pair() {
    local tag="$1"
    rm -f "$H_SYNTH" "$C_SYNTH" \
          "$XDG_RUNTIME_DIR/$H_SOCK" "$XDG_RUNTIME_DIR/$H_SOCK.lock" \
          "$XDG_RUNTIME_DIR/$C_SOCK" "$XDG_RUNTIME_DIR/$C_SOCK.lock"
    DRAGONFRUIT_SYNTHETIC_INPUT="$H_SYNTH" DRAGONFRUIT_NESTED_SIZE=1200x800 \
        "$BIN" --backend nested --socket-name "$H_SOCK" >"$OUT_DIR/host-$tag.txt" 2>&1 &
    H_PID=$!
    for _ in $(seq 1 100); do [ -S "$H_SYNTH" ] && break; sleep 0.1; done
    WAYLAND_DISPLAY="$H_SOCK" DRAGONFRUIT_SYNTHETIC_INPUT="$C_SYNTH" \
        DRAGONFRUIT_NESTED_SIZE=640x480 \
        "$BIN" --backend nested --socket-name "$C_SOCK" >"$OUT_DIR/child-$tag.txt" 2>&1 &
    C_PID=$!
    for _ in $(seq 1 100); do [ -S "$C_SYNTH" ] && break; sleep 0.1; done
}

start_animation() {
    python3 - "$C_SYNTH" 2>/dev/null <<'PY' &
import socket
import sys
import time

sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
index = 0
while True:
    x = 0.2 + 0.3 * ((index % 50) / 50.0)
    sock.sendto(("motion-abs %.3f 0.5\n" % x).encode(), sys.argv[1])
    index += 1
    time.sleep(0.008)
PY
    ANIM_PID=$!
}

host_child_content() {
    python3 - "$H_SYNTH" <<'PY'
import os
import re
import socket
import sys

synth = sys.argv[1]
local = synth + ".q"
try:
    os.unlink(local)
except OSError:
    pass
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.bind(local)
sock.settimeout(1.5)


def query(command):
    sock.sendto((command + "\n").encode(), synth)
    lines = []
    try:
        while True:
            data, _ = sock.recvfrom(65535)
            for line in data.decode(errors="replace").splitlines():
                if line == "end":
                    return lines
                lines.append(line)
    except Exception:
        return lines


logical = (1200, 800)
for line in query("query scale"):
    match = re.search(r"logical=(\d+)x(\d+)", line)
    if match:
        logical = (int(match.group(1)), int(match.group(2)))
child = None
for line in query("query decorations"):
    parts = line.split()
    if parts and parts[0] == "decoration" and len(parts) >= 11:
        child = parts
        break
if child is None:
    print("")
    raise SystemExit(1)
print(logical[0], logical[1], " ".join(child[7:11]))
PY
}

echo "t238-presenter-lifecycle: minimize/restore + resize"
start_pair main
start_animation
sleep 2
start_completed=$(field completed "$(query_synth "$C_SYNTH" "query frames")")

ok=1
for cycle in $(seq 1 5); do
    send_synth "$H_SYNTH" "minimize 0"
    sleep 0.6
    query_synth "$C_SYNTH" "query frames" >/dev/null || ok=0
    send_synth "$H_SYNTH" "restore 0"
    sleep 2.0
    frames="$(query_synth "$C_SYNTH" "query frames")"
    echo "  cycle $cycle: $frames"
    [ -n "$frames" ] || ok=0
done
send_synth "$H_SYNTH" "restore 0"
sleep 1.0
end_completed=$(field completed "$(query_synth "$C_SYNTH" "query frames")")
check "minimize/restore responsive" "$ok"
check "minimize/restore progresses ($start_completed -> $end_completed)" \
    "$([ "$end_completed" -gt "$start_completed" ] && echo 1 || echo 0)"

ok=1
for cycle in $(seq 1 2); do
    read -r width height cx cy cw ch <<<"$(host_child_content)"
    if [ -z "${width:-}" ]; then
        ok=0
        continue
    fi
    before_scale="$(query_synth "$C_SYNTH" "query scale")"
    python3 - "$H_SYNTH" "$width" "$height" "$cx" "$cy" "$cw" "$ch" <<'PY'
import socket
import sys
import time

synth, width, height, cx, cy, cw, ch = sys.argv[1:]
width, height, cx, cy, cw, ch = map(int, (width, height, cx, cy, cw, ch))
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
x = (cx + cw - 3) / width
y = (cy + ch - 3) / height
sock.sendto(("motion-abs %.6f %.6f\n" % (x, y)).encode(), synth)
time.sleep(0.05)
sock.sendto(b"button 272 down\n", synth)
direction = 4 if int(time.time()) % 2 == 0 else -4
for _ in range(40):
    sock.sendto(("motion %d %d\n" % (direction, direction)).encode(), synth)
    time.sleep(1.0 / 60.0)
sock.sendto(b"button 272 up\n", synth)
PY
    sleep 0.8
    after_scale="$(query_synth "$C_SYNTH" "query scale")"
    echo "  resize $cycle: $before_scale -> $after_scale"
    [ -n "$after_scale" ] || ok=0
done
check "resize responsive" "$ok"

echo "t238-presenter-lifecycle: host disconnect"
kill -9 "$H_PID"
start_ms=$(date +%s%3N)
for _ in $(seq 1 40); do
    kill -0 "$C_PID" 2>/dev/null || break
    sleep 0.1
done
exit_ms=$(( $(date +%s%3N) - start_ms ))
if kill -0 "$C_PID" 2>/dev/null; then
    check "host disconnect exits child" 0
    kill -9 "$C_PID"
else
    check "host disconnect exits child (${exit_ms}ms)" 1
fi
grep -q "host Wayland connection closed" "$OUT_DIR/child-main.txt" && \
    grep -q "clean exit" "$OUT_DIR/child-main.txt" && \
    check "host disconnect clean exit log" 1 || check "host disconnect clean exit log" 0
kill -9 "$ANIM_PID" 2>/dev/null || true
ANIM_PID=""

echo "t238-presenter-lifecycle: blocked shutdown"
start_pair blocked
start_animation
sleep 2
send_synth "$H_SYNTH" "minimize 0"
sleep 0.5
kill -TERM "$C_PID"
start_ms=$(date +%s%3N)
for _ in $(seq 1 50); do
    kill -0 "$C_PID" 2>/dev/null || break
    sleep 0.1
done
exit_ms=$(( $(date +%s%3N) - start_ms ))
if kill -0 "$C_PID" 2>/dev/null; then
    check "blocked shutdown exits" 0
    kill -9 "$C_PID"
else
    check "blocked shutdown exits (${exit_ms}ms)" 1
fi
grep -q "present thread still blocked at shutdown" "$OUT_DIR/child-blocked.txt" && \
    grep -q "clean exit" "$OUT_DIR/child-blocked.txt" && \
    check "blocked shutdown clean exit log" 1 || check "blocked shutdown clean exit log" 0

echo "t238-presenter-lifecycle: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
