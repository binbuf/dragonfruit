#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 wake-only-swap audit.
#
# A no-damage compositor frame still hands a swap to the presenter. On the
# nested rail that is `eglSwapBuffers` on a Wayland EGL window surface, which
# attaches a buffer and commits the surface. This script puts a small Wayland
# proxy between a direct nested child and the host, drives the child's
# post-input frame cadence, and reports what each commit actually contained:
#
#   * buffer attach vs a commit with no buffer
#   * `wl_surface.damage` / `damage_buffer` count (0 means the host must
#     assume the entire surface was damaged)
#   * `wl_surface.frame` request present or not
#
# It also reports the child's own `query frames` accounting so the protocol
# commits can be joined to damage and wake-only swaps.
#
# Overrides: OUT_DIR, PROFILE, COMPOSITOR_BIN, PULSE_SECONDS.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT_DIR="${OUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/t238-wake-audit.XXXXXX")}"
PROFILE="${PROFILE:-release}"
BIN="${COMPOSITOR_BIN:-$PWD/target/$PROFILE/dragonfruit-compositor}"
PULSE_SECONDS="${PULSE_SECONDS:-2}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"

[ -x "$BIN" ] || {
    echo "t238-wake-swap-audit: missing $BIN (run make build)" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "t238-wake-swap-audit: python3 not found" >&2
    exit 1
}

upstream="${WAYLAND_DISPLAY:-}"
case "$upstream" in
    /*) ;;
    "") echo "t238-wake-swap-audit: WAYLAND_DISPLAY is not set" >&2; exit 1 ;;
    *) upstream="$XDG_RUNTIME_DIR/$upstream" ;;
esac
[ -S "$upstream" ] || {
    echo "t238-wake-swap-audit: host socket $upstream is not a socket" >&2
    exit 1
}

find_host_pid() {
    local fd link pid
    for fd in /proc/[0-9]*/fd/*; do
        link="$(readlink "$fd" 2>/dev/null || true)"
        [ "$link" = "$upstream" ] || continue
        pid="$(basename "$(dirname "$(dirname "$fd")")")"
        echo "$pid"
        return 0
    done
    if command -v fuser >/dev/null 2>&1; then
        pid="$(fuser "$upstream" 2>/dev/null | tr -s ' ' '\n' | grep -E '^[0-9]+$' | head -1)"
        [ -n "$pid" ] && { echo "$pid"; return 0; }
    fi
    return 1
}
host_pid="${HOST_PID:-$(find_host_pid || true)}"
cpu_ticks() {
    [ -n "${1:-}" ] && [ -r "/proc/$1/stat" ] || { echo 0; return; }
    awk '{print $14 + $15}' "/proc/$1/stat"
}

mkdir -p "$OUT_DIR"
proxy_socket="$XDG_RUNTIME_DIR/t238-trace-$$"
proxy_log="$OUT_DIR/proxy.log"
child_socket="t238-wake-$$"
child_synth="$XDG_RUNTIME_DIR/$child_socket.synth"
child_log="$OUT_DIR/child.txt"

cleanup() {
    for pid in "${CHILD_PID:-}" "${PROXY_PID:-}" "${PULSE_PID:-}"; do
        [ -n "$pid" ] && kill -9 "$pid" 2>/dev/null || true
    done
    rm -f "$child_synth" "$proxy_socket" \
          "$XDG_RUNTIME_DIR/$child_socket" "$XDG_RUNTIME_DIR/$child_socket.lock"
}
trap cleanup EXIT

python3 "$PWD/scripts/t238-wl-trace-proxy.py" \
    "$upstream" "$proxy_socket" "$proxy_log" >"$OUT_DIR/proxy.out" 2>&1 &
PROXY_PID=$!
for _ in $(seq 1 50); do [ -S "$proxy_socket" ] && break; sleep 0.1; done

WAYLAND_DISPLAY="$proxy_socket" \
DRAGONFRUIT_SYNTHETIC_INPUT="$child_synth" \
DRAGONFRUIT_NESTED_SIZE="${DRAGONFRUIT_NESTED_SIZE:-1280x800}" \
    "$BIN" --backend nested --socket-name "$child_socket" >"$child_log" 2>&1 &
CHILD_PID=$!
for _ in $(seq 1 100); do [ -S "$child_synth" ] && break; sleep 0.1; done

query_frames() {
    python3 - "$child_synth" <<'PY'
import os
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
sock.settimeout(1.0)
sock.sendto(b"query frames\n", synth)
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

sleep 1.5
before="$(query_frames)"
# Baseline host cost with the child present but idle (no wake-only cadence).
idle_cpu_before="$(cpu_ticks "$host_pid")"
idle_start="$(date +%s%3N)"
sleep 2
idle_ms=$(( $(date +%s%3N) - idle_start ))
idle_cpu_after="$(cpu_ticks "$host_pid")"
# Hold the post-input frame cadence open without moving the pointer: this is
# the path that produces wake-only swaps (a pending frame callback with no
# scene damage).
host_cpu_before="$(cpu_ticks "$host_pid")"
pulse_start="$(date +%s%3N)"
python3 - "$child_synth" "$PULSE_SECONDS" <<'PY' &
import socket
import sys
import time

sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
end = time.monotonic() + float(sys.argv[2])
while time.monotonic() < end:
    sock.sendto(b"motion-abs 0.5 0.5\n", sys.argv[1])
    time.sleep(1.0 / 60.0)
PY
PULSE_PID=$!
wait "$PULSE_PID" 2>/dev/null || true
PULSE_PID=""
pulse_ms=$(( $(date +%s%3N) - pulse_start ))
host_cpu_after="$(cpu_ticks "$host_pid")"
sleep 0.5
after="$(query_frames)"

kill -TERM "$CHILD_PID" 2>/dev/null || true
for _ in $(seq 1 50); do kill -0 "$CHILD_PID" 2>/dev/null || break; sleep 0.1; done
kill -TERM "$PROXY_PID" 2>/dev/null || true
sleep 0.3

echo "t238-wake-swap-audit: before"
echo "  $before"
echo "t238-wake-swap-audit: after"
echo "  $after"
python3 - "$proxy_log" <<'PY'
import sys

lines = [line for line in open(sys.argv[1], errors="replace") if line.startswith("CLIENT commit")]
commits = len(lines)
attach = sum("attach=buffer" in line for line in lines)
no_damage = sum("damage_calls=0" in line for line in lines)
full = sum("full_surface_assumed=1" in line for line in lines)
frame = sum("frame=yes" in line for line in lines)
print("t238-wake-swap-audit: protocol commits=%d attach_buffer=%d "
      "zero_damage=%d full_surface_assumed=%d frame_request=%d"
      % (commits, attach, no_damage, full, frame))
if commits:
    print("t238-wake-swap-audit: full-surface share=%.1f%%"
          % (100.0 * full / commits))
for line in lines[:5]:
    print("  " + line.rstrip())
PY
if [ -n "${host_pid:-}" ]; then
    clock_ticks="$(getconf CLK_TCK 2>/dev/null || echo 100)"
    idle_ticks=$((idle_cpu_after - idle_cpu_before))
    host_ticks=$((host_cpu_after - host_cpu_before))
    echo "t238-wake-swap-audit: host pid=$host_pid idle_ticks=$idle_ticks over ${idle_ms}ms pulse_ticks=$host_ticks over ${pulse_ms}ms"
    python3 - "$idle_ticks" "$idle_ms" "$host_ticks" "$pulse_ms" "$clock_ticks" <<'PY'
import sys
idle_ticks, idle_ms, pulse_ticks, pulse_ms, hz = map(float, sys.argv[1:6])
def pct(ticks, millis):
    return 100.0 * ticks / hz / (millis / 1000.0) if millis > 0 and hz > 0 else 0.0
print("t238-wake-swap-audit: host cpu idle=%.2f%% pulse=%.2f%% of one core"
      % (pct(idle_ticks, idle_ms), pct(pulse_ticks, pulse_ms)))
PY
fi
echo "t238-wake-swap-audit: artifacts in $OUT_DIR"
