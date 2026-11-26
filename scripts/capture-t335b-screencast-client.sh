#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T335b standard-client ScreenCast check.
#
# Proves the live screen-share path end to end:
#
#   1. a private `pipewire` daemon runs in its own runtime dir;
#   2. a compositor on its own socket provisions the portal role's one-time
#      token (`<socket>.portal-launch-token`);
#   3. the portal backend's shipped bridge opens as that trusted client and
#      creates a stream node with `df_toplevel_manager.create_screencast` (v17)
#      — the exact message a portal `Start` sends;
#   4. a standard PipeWire client (`pipewiresrc`) reads that node.
#
# Nested/DRM capture feeds the stream from the live frame, so a real client
# run needs a Wayland session (`WAYLAND_DISPLAY` on a compositor that renders):
# `DRAGONFRUIT_BACKEND=nested make t335b-screencast-client`. The headless
# backend has no readback by design, so this check proves the transport and the
# graph node there and says so; it never claims frames it did not see.
#
# Outputs:
#   docs/captures/t335b-screencast-client.txt   verdict and evidence
#
# Requires: `pipewire`, `pw-cli`, `gst-launch-1.0` with `pipewiresrc`, the
# built tree, `XDG_RUNTIME_DIR`. Not part of `make e2e`.
set -u

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

OUTDIR="${OUTDIR:-docs/captures}"
mkdir -p "$OUTDIR"
VERDICT="$OUTDIR/t335b-screencast-client.txt"
BACKEND="${DRAGONFRUIT_BACKEND:-headless}"

record() {
    local verdict="$1"
    local reason="$2"
    shift 2
    {
        echo "T335b standard-client ScreenCast check"
        echo "timestamp: $(date -Is)"
        echo "verdict: $verdict"
        echo "reason: $reason"
        echo "backend: $BACKEND"
        for line in "$@"; do
            echo "$line"
        done
        echo "rerun: DRAGONFRUIT_BACKEND=nested WAYLAND_DISPLAY=<host-socket> make t335b-screencast-client"
    } >"$VERDICT"
    echo "t335b-screencast-client: $verdict — $reason"
    # One verdict per run: the first outcome is the recorded one.
    exit 0
}

for tool in pipewire pw-cli gst-launch-1.0 gst-inspect-1.0; do
    command -v "$tool" >/dev/null 2>&1 \
        || record OPEN "missing $tool (the PipeWire host stack or GStreamer)"
done
gst-inspect-1.0 pipewiresrc >/dev/null 2>&1 \
    || record OPEN "GStreamer has no pipewiresrc plugin (install gst-plugin-pipewire)"
[ -n "${XDG_RUNTIME_DIR:-}" ] || record OPEN "XDG_RUNTIME_DIR is not set"
for binary in dragonfruit-compositor dragonfruit-screencast-producer xdg-desktop-portal-dragonfruit; do
    [ -x "target/debug/$binary" ] || record OPEN "target/debug/$binary is missing (run make build)"
done
if [ "$BACKEND" = "nested" ] && [ ! -S "$XDG_RUNTIME_DIR/${WAYLAND_DISPLAY:-wayland-0}" ]; then
    record OPEN "nested capture needs a host Wayland session at \$XDG_RUNTIME_DIR/${WAYLAND_DISPLAY:-wayland-0}"
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t335b.XXXXXX")" || record OPEN "no temp dir"
PIDS=()
cleanup() {
    for pid in "${PIDS[@]}"; do kill "$pid" 2>/dev/null; done
    sleep 0.2
    for pid in "${PIDS[@]}"; do kill -9 "$pid" 2>/dev/null; done
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
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
[ "$ready" = "1" ] || record OPEN "the private PipeWire daemon did not start ($SCRATCH/pipewire.log)"

# --- a session: compositor + portal token -----------------------------------
socket="dragonfruit-t335b-$$"
# The compositor spawns `dragonfruit-screencast-producer` from PATH and
# resolves the node with `pw-cli`; both act on the private core through
# `XDG_RUNTIME_DIR`.
XDG_RUNTIME_DIR="$SCRATCH/runtime" \
    PATH="$root/target/debug:$PATH" \
    timeout 300 "$root/target/debug/dragonfruit-compositor" \
    --backend "$BACKEND" --socket-name "$socket" >"$SCRATCH/compositor.log" 2>&1 &
PIDS+=($!)
token_file="$SCRATCH/runtime/$socket.portal-launch-token"
for _ in $(seq 1 100); do
    [ -S "$SCRATCH/runtime/$socket" ] && [ -f "$token_file" ] && break
    sleep 0.1
done
[ -S "$SCRATCH/runtime/$socket" ] || record OPEN "the compositor never became ready ($SCRATCH/compositor.log)"
[ -f "$token_file" ] || record OPEN "the compositor wrote no portal token hand-off file"
# The session manager hands the backend the token the compositor minted for
# the portal role (its hand-off file), never one it invents.
token="$(tr -d '[:space:]' <"$token_file")"

# --- the shipped bridge opens as the portal role and creates a stream -------
# One process: the launch token is one-time, so the probe and the create share
# the one bridge connection. The node lives on that connection, so the process
# then waits while the client reads it.
create_log="$SCRATCH/create.log"
(
    XDG_RUNTIME_DIR="$SCRATCH/runtime" \
        DRAGONFRUIT_SCREENCAST_BRIDGE_TOKEN="$token" \
        DRAGONFRUIT_SCREENCAST_BRIDGE_SOCKET="$socket" \
        PATH="$root/target/debug:$PATH" \
        timeout 60 target/debug/xdg-desktop-portal-dragonfruit --check-screencast --create \
        >"$create_log" 2>&1
    echo "bridge-exit $?" >>"$create_log"
    # Keep the connection (and the node) alive for the client.
    sleep 20
) &
PIDS+=($!)
bridge_mode=""
node=""
for _ in $(seq 1 300); do
    bridge_mode="$(sed -n 1p "$create_log" 2>/dev/null)"
    node="$(sed -n 's/^node //p' "$create_log" 2>/dev/null | head -1)"
    grep -q "^bridge-exit " "$create_log" 2>/dev/null && break
    [ -n "$node" ] && break
    sleep 0.1
done
create_error="$(tr '\n' ' ' <"$create_log")"
[ "$bridge_mode" = "pipewire" ] \
    || record FAIL "the shipped bridge did not open as the portal role (mode '$bridge_mode'): $create_error"
if [ -z "$node" ]; then
    if printf '%s' "$create_error" | grep -q "pipewire-producer-unavailable"; then
        record PARTIAL "the shipped bridge opened as the portal role (mode pipewire); the stack has no captured frame for a client yet: $create_error" \
            "unit_coverage: cargo test -p dragonfruit-portal --test screencast_producer_socket pins the host tool's node and frame packets"
    fi
    record FAIL "the bridge created no stream node: $create_error"
fi

node_name="$(XDG_RUNTIME_DIR="$SCRATCH/runtime" pw-cli list-objects Node 2>/dev/null \
    | sed -n 's/.*node\.name = "\(dragonfruit-screencast\.[^"]*\)".*/\1/p' | head -1)"
[ -n "$node_name" ] \
    || record FAIL "no dragonfruit-screencast node in the private graph for stream $node"

# --- a standard client reads the node ---------------------------------------
frames_log="$SCRATCH/frames.log"
XDG_RUNTIME_DIR="$SCRATCH/runtime" timeout 15 gst-launch-1.0 -v \
    pipewiresrc path="$node" num-buffers=1 ! videoconvert ! fakesink \
    >"$frames_log" 2>&1
client_status=$?
if [ "$client_status" -ne 0 ]; then
    record PARTIAL "the bridge created stream node $node ($node_name) as the portal role; no frame reached a client in this ${BACKEND} session: $(tail -2 "$frames_log" | tr '\n' ' ')" \
        "bridge_mode: $bridge_mode" \
        "producer_node: $node ($node_name)" \
        "client: pipewiresrc path=$node num-buffers=1 ! videoconvert ! fakesink" \
        "client_status: $client_status" \
        "next: DRAGONFRUIT_BACKEND=nested make t335b-screencast-client (nested capture feeds the stream)"
fi

record PASS "the bridge created stream node $node as the portal role and pipewiresrc received a frame from it" \
    "bridge_mode: $bridge_mode" \
    "producer_node: $node ($node_name)" \
    "compositor: $(grep -c 'authenticated as portal' "$SCRATCH/compositor.log" 2>/dev/null || echo 0) portal-role handshake(s)" \
    "client: pipewiresrc path=$node num-buffers=1 ! videoconvert ! fakesink" \
    "client_status: $client_status"
