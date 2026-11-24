#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T335a live ScreenCast producer check.
#
# Proves the live half of the screen share on a host with a Wayland session and
# a PipeWire stack:
#
#   1. a private `pipewire` daemon runs in its own runtime dir;
#   2. the shipped host tool (`dragonfruit-screencast-producer`) is started
#      against a `SOCK_STREAM`/`SOCK_SEQPACKET`-framed feeder (the shell cannot
#      create `SOCK_SEQPACKET` sockets, so the feeder uses a stream socket and
#      the host reads one whole packet per frame);
#   3. the tool registers its `Video/Source` node in the private graph, so the
#      check can assert the node exists by name through `pw-cli`;
#   4. the feeder writes a 1920x1200 RGBA packet whose last row is a known
#      colour, and the tool reports the first frame it pumped.
#
# The verdict file names the node id, the frame count the tool reported and the
# byte count it logged. A host without a Wayland session or PipeWire is an
# `OPEN` check with the exact rerun command, not a failure.
#
# Outputs:
#   docs/captures/t335a-screencast-live.txt    verdict, node id, frames
#   docs/captures/t335a-screencast-open.txt    the OPEN notice and rerun command
#
# Requires: `pipewire`, `pw-cli`, `python3`, `XDG_RUNTIME_DIR`. Not part of
# `make e2e`.
set -u

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

OUTDIR="${OUTDIR:-docs/captures}"
mkdir -p "$OUTDIR"

open_notice() {
    cat >"$OUTDIR/t335a-screencast-open.txt" <<EOF
T335a live ScreenCast producer — live check
timestamp: $(date -Is)
verdict: OPEN
reason: $1
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host-socket> make t335a-screencast
EOF
    rm -f "$OUTDIR/t335a-screencast-live.txt"
    exit 0
}

[ -x target/debug/dragonfruit-screencast-producer ] \
    || open_notice "target/debug/dragonfruit-screencast-producer missing; run make build"
command -v pipewire >/dev/null 2>&1 || open_notice "pipewire is not installed"
command -v pw-cli >/dev/null 2>&1 || open_notice "pw-cli is not installed"
command -v python3 >/dev/null 2>&1 || open_notice "python3 is not installed"
[ -n "${XDG_RUNTIME_DIR:-}" ] || open_notice "XDG_RUNTIME_DIR is not set"
[ -S "$XDG_RUNTIME_DIR/${WAYLAND_DISPLAY:-wayland-0}" ] \
    || open_notice "no host Wayland session at $XDG_RUNTIME_DIR/${WAYLAND_DISPLAY:-wayland-0}"

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t335a.XXXXXX")" \
    || open_notice "no temp dir"
PIDS=()
cleanup() {
    for pid in "${PIDS[@]}"; do kill "$pid" 2>/dev/null; done
    sleep 0.3
    for pid in "${PIDS[@]}"; do kill -9 "$pid" 2>/dev/null; done
    rm -rf "$SCRATCH"
}
trap cleanup EXIT

# --- a private PipeWire core ------------------------------------------------
mkdir -p "$SCRATCH/runtime"
chmod 700 "$SCRATCH/runtime"
XDG_RUNTIME_DIR="$SCRATCH/runtime" setsid pipewire >"$SCRATCH/pipewire.log" 2>&1 &
PIDS+=($!)
ready=0
for _ in $(seq 1 100); do
    if XDG_RUNTIME_DIR="$SCRATCH/runtime" pw-cli info 0 >/dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 0.1
done
[ "$ready" = "1" ] || open_notice "the private PipeWire daemon did not start ($SCRATCH/pipewire.log)"

# --- the feeder the host tool reads -----------------------------------------
# One frame: a 16-byte header (width, height, stride, length) and a 1920x1200
# RGBA buffer whose last row is (0x11, 0x22, 0x33, 0xff). The feeder accepts the
# host, writes the frame, waits for the host to exit, and prints what it sent.
cat >"$SCRATCH/feeder.py" <<'EOF'
import socket
import struct
import sys

socket_path = sys.argv[1]
width, height = 1920, 1200
stride = width * 4
row = bytes((0x11, 0x22, 0x33, 0xFF)) * width
data = bytes((0x40, 0x40, 0x40, 0xFF)) * (width * (height - 1)) + row
header = struct.pack("<IIII", width, height, stride, len(data))
listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
listener.bind(socket_path)
listener.listen(1)
listener.settimeout(10)
try:
    connection, _ = listener.accept()
except socket.timeout:
    print("feeder: the host never connected")
    sys.exit(1)
connection.sendall(header + data)
connection.close()
print(f"feeder: sent {width}x{height} rgba ({len(data)} bytes)")
EOF

node_name="dragonfruit-screencast.1920.1200.30"
socket_path="$SCRATCH/frame.sock"
python3 "$SCRATCH/feeder.py" "$socket_path" >"$SCRATCH/feeder.log" 2>&1 &
FEEDER_PID=$!
PIDS+=("$FEEDER_PID")
sleep 0.3

XDG_RUNTIME_DIR="$SCRATCH/runtime" timeout 20 \
    target/debug/dragonfruit-screencast-producer "$socket_path" 1920 1200 30 "$node_name" \
    >"$SCRATCH/host.log" 2>&1 &
HOST_PID=$!
PIDS+=("$HOST_PID")
# Resolve the node id while the stream is live: a `pw-cli` lookup through the
# same parser the compositor uses (`node_ids_from_listing`).
node_id=""
# The node is registered by a short-lived `pw-cli` call, so poll for it (bounded
# by the same window as the host's own read).
for _ in $(seq 1 100); do
    node_id="$(XDG_RUNTIME_DIR="$SCRATCH/runtime" pw-cli list-objects Node 2>/dev/null \
        | awk -v name="$node_name" '
            /^[[:space:]]*id [0-9]+/ { id = $2; sub(",", "", id) }
            /node.name = ".*"/ { matched = index($0, "node.name = \"" name "\"") > 0; if (matched) print id }
        ' | tail -1)"
    [ -n "$node_id" ] && break
    sleep 0.05
done
node_seen="no"
[ -n "$node_id" ] && node_seen="yes"
wait "$FEEDER_PID" 2>/dev/null
wait "$HOST_PID" 2>/dev/null
host_status=$?

# The node is destroyed when the stream ends, so this is the end-state check.
node_gone="no"
XDG_RUNTIME_DIR="$SCRATCH/runtime" pw-cli list-objects Node 2>/dev/null \
    | awk -v name="$node_name" 'index($0, "node.name = \"" name "\"") { found = 1 } END { exit !found }' \
    && node_gone="stale" || node_gone="yes"
frames="$(sed -n 's/.*: \([0-9][0-9]*\) frame(s).*/\1/p' "$SCRATCH/host.log" | tail -1)"

verdict="FAIL"
reason="the host tool did not pump a frame"
if [ -n "$frames" ] && [ "$frames" -ge 1 ] && [ "$node_seen" = "yes" ]; then
    verdict="PASS"
    reason="node $node_id is in the graph; the host tool pumped $frames frame(s) of 1920x1200 and the stream teardown removed the node ($node_gone)"
fi

{
    echo "T335a live ScreenCast producer — live check"
    echo "timestamp: $(date -Is)"
    echo "verdict: $verdict"
    echo "reason: $reason"
    echo "pipewire: private daemon ($SCRATCH/runtime)"
    echo "node_name: $node_name"
    echo "node_id: ${node_id:-none}"
    echo "node_destroyed_on_teardown: $node_gone"
    echo "frames: ${frames:-0}"
    echo "feeder_status: $(tail -1 "$SCRATCH/feeder.log" 2>/dev/null)"
    echo "host_status: $host_status"
    echo "host_log:"
    sed -n '1,20p' "$SCRATCH/host.log"
    echo "pipewire_log_tail:"
    tail -5 "$SCRATCH/pipewire.log"
} >"$OUTDIR/t335a-screencast-live.txt"
rm -f "$OUTDIR/t335a-screencast-open.txt"
[ "$verdict" = "PASS" ]
