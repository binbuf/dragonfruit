#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 P0 regression: a nested compositor must keep servicing its synthetic
# socket when the host minimizes, backgrounds, or fully covers its window.
#
# Runs a nested Dragonfruit compositor (H) as the host, a second nested
# compositor (C) inside it, and an Xwayland xmessage window for the
# background/occlude cases. It probes C with `query scene` and checks the
# synthetic socket's receive queue. The old main-thread swap blocked in
# `eglSwapBuffers` and left the queue non-empty; the presenter keeps C in
# `ep_poll`.
#
# Overrides: OUT_DIR, SIZE, CHILD_SIZE, BACKEND, PROFILE.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT_DIR="${OUT_DIR:-/tmp/t238-nested-responsiveness}"
SIZE="${SIZE:-1200x800}"
CHILD_SIZE="${CHILD_SIZE:-640x480}"
PROFILE="${PROFILE:-release}"
BIN="${COMPOSITOR_BIN:-$PWD/target/$PROFILE/dragonfruit-compositor}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DRAGONFRUIT_REFRESH_HZ="${DRAGONFRUIT_REFRESH_HZ:-144}"

[ -x "$BIN" ] || {
    echo "t238-nested-responsiveness: missing $BIN (run make build)" >&2
    exit 1
}
command -v xmessage >/dev/null 2>&1 || {
    echo "t238-nested-responsiveness: xmessage not found" >&2
    exit 1
}

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
H_SOCK="t238-host-$$"
C_SOCK="t238-child-$$"
H_SYNTH="$XDG_RUNTIME_DIR/$H_SOCK.synth"
C_SYNTH="$XDG_RUNTIME_DIR/$C_SOCK.synth"
cleanup() {
    for pid in "${H_PID:-}" "${C_PID:-}" "${SIDE_PID:-}" "${OCC_PID:-}"; do
        [ -n "$pid" ] && kill -9 "$pid" 2>/dev/null || true
    done
    rm -f "$H_SYNTH" "$C_SYNTH" \
          "$XDG_RUNTIME_DIR/$H_SOCK" "$XDG_RUNTIME_DIR/$H_SOCK.lock" \
          "$XDG_RUNTIME_DIR/$C_SOCK" "$XDG_RUNTIME_DIR/$C_SOCK.lock"
}
trap cleanup EXIT

DRAGONFRUIT_SYNTHETIC_INPUT="$H_SYNTH" DRAGONFRUIT_NESTED_SIZE="$SIZE" \
    "$BIN" --backend nested --socket-name "$H_SOCK" >"$OUT_DIR/host.txt" 2>&1 &
H_PID=$!
for _ in $(seq 1 100); do [ -S "$H_SYNTH" ] && break; sleep 0.1; done

WAYLAND_DISPLAY="$H_SOCK" DRAGONFRUIT_SYNTHETIC_INPUT="$C_SYNTH" \
    DRAGONFRUIT_NESTED_SIZE="$CHILD_SIZE" \
    "$BIN" --backend nested --socket-name "$C_SOCK" >"$OUT_DIR/child.txt" 2>&1 &
C_PID=$!
for _ in $(seq 1 100); do [ -S "$C_SYNTH" ] && break; sleep 0.1; done

H_DISPLAY_FILE="$XDG_RUNTIME_DIR/$H_SOCK.x11-display"
for _ in $(seq 1 100); do [ -s "$H_DISPLAY_FILE" ] && break; sleep 0.1; done
H_DISPLAY="$(cat "$H_DISPLAY_FILE" 2>/dev/null || true)"

probe() {
    local label="$1"
    python3 - "$C_SYNTH" "$label" <<'PY'
import os
import socket
import sys
import time

synth, label = sys.argv[1], sys.argv[2]
local = synth + ".probe"
try:
    os.unlink(local)
except OSError:
    pass
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.bind(local)
sock.settimeout(1.0)
start = time.monotonic()
sock.sendto(b"query scene\n", synth)
try:
    while True:
        data, _ = sock.recvfrom(65535)
        if data.strip().endswith(b"end"):
            elapsed = time.monotonic() - start
            print(f"{label}: responsive in {elapsed:.3f}s")
            raise SystemExit(0)
except Exception as exc:  # noqa: BLE001 - the probe is the diagnostic
    print(f"{label}: NOT responsive ({exc})")
    raise SystemExit(1)
PY
}

# Focus the topmost host decoration (the side window once it exists).
focus_top() {
    python3 - "$H_SYNTH" <<'PY'
import os
import re
import socket
import sys
import time

synth = sys.argv[1]
local = synth + ".focus"
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
decorations = [
    line.split() for line in query("query decorations")
    if line.startswith("decoration ")
]
if not decorations:
    raise SystemExit("no host decorations")
x, y, w, h = (int(value) for value in decorations[-1][3:7])
for state in ("down", "up"):
    sock.sendto(
        ("motion-abs %.6f %.6f\n" % ((x + w / 2) / logical[0], (y + h / 2) / logical[1])).encode(),
        synth,
    )
    time.sleep(0.15)
    sock.sendto(("button 272 %s\n" % state).encode(), synth)
    time.sleep(0.15)
PY
}

sleep 1
probe "baseline"

if [ -n "$H_DISPLAY" ]; then
    DISPLAY="$H_DISPLAY" xmessage -geometry 300x200+20+120 -title bg bg \
        >/dev/null 2>&1 &
    SIDE_PID=$!
    sleep 1
    focus_top
    sleep 0.5
    probe "backgrounded"

    DISPLAY="$H_DISPLAY" xmessage -geometry 700x600+500+220 -title occlude occlude \
        >/dev/null 2>&1 &
    OCC_PID=$!
    sleep 1
    focus_top
    sleep 0.5
    probe "occluded"
fi

# Minimize the child through the host's own traffic light.
python3 - "$H_SYNTH" <<'PY'
import os
import re
import socket
import sys
import time

synth = sys.argv[1]
local = synth + ".min"
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
decorations = [
    line.split() for line in query("query decorations")
    if line.startswith("decoration ")
]
child = next((parts for parts in decorations if parts[0] == "decoration"), None)
if child is None:
    raise SystemExit("no child decoration")
x, y = int(child[3]), int(child[4])
# The minimize light is the second circle (close is first, +23 logical px).
for state in ("down", "up"):
    sock.sendto(
        ("motion-abs %.6f %.6f\n" % ((x + 39) / logical[0], (y + 16) / logical[1])).encode(),
        synth,
    )
    time.sleep(0.15)
    sock.sendto(("button 272 %s\n" % state).encode(), synth)
    time.sleep(0.15)
PY
sleep 1
probe "minimized"

queue="$(ss -x -a 2>/dev/null | grep -F "$C_SYNTH" | awk '{print $3}' | head -1)"
queue="${queue:-0}"
echo "child synthetic Recv-Q: $queue"
[ "$queue" = "0" ] || {
    echo "t238-nested-responsiveness: synthetic queue did not drain" >&2
    exit 1
}
echo "t238-nested-responsiveness: all probes passed"
