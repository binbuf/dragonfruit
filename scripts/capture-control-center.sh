#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T-11.3a live capture of the Control Center panel (Wi-Fi/Sound/Display
# tiles), opened through the real Control-Option-C shortcut. Writes
# docs/captures/t11-control-center.png. Requires a host Wayland session and
# `spectacle`; the panel region is cropped from a full-screen still.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

SOCKET="dragonfruit-t74-capture"
SYNTH="${XDG_RUNTIME_DIR:?}/$SOCKET.synth"
LOG=/tmp/opencode/t74-demo.log
rm -f "$SYNTH" "$LOG" /tmp/opencode/t74-*.png

DF_STATUS_FIXTURE=1 DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" >"$LOG" 2>&1 &
PGID=$!
cleanup() {
    kill -TERM -"$PGID" 2>/dev/null || true
    sleep 1
    kill -KILL -"$PGID" 2>/dev/null || true
    rm -f "$SYNTH"
}
trap cleanup EXIT

for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
[ -e "$SYNTH" ] || { echo "no synthetic socket"; tail -30 "$LOG"; exit 1; }
sleep 10
T74_SYNTH="$SYNTH" T74_OUT="$root/docs/captures" \
    python3 "$root/scripts/capture-control-center-driver.py"