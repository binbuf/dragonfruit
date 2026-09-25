#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T-11.4b live capture of the OSD card and the menu-bar Focus/DND reflection.
# Runs the nested demo with the status/notification fixtures (notification
# fixture started in DND), drives a real Control Center volume drag to present
# the OSD over the synthetic-input harness, and writes
# docs/captures/t11-osd.png, t11-osd-context.png, and t11-dnd.png. Requires a
# host Wayland session and `spectacle`.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

SOCKET="dragonfruit-t77-capture"
SYNTH="${XDG_RUNTIME_DIR:?}/$SOCKET.synth"
LOG=/tmp/opencode/t11-demo.log
rm -f "$SYNTH" "$LOG" /tmp/opencode/t11-*.png

DF_STATUS_FIXTURE=1 DF_NOTIFY_FIXTURE=1 DF_FOCUS_FIXTURE=dnd \
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
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
T77_SYNTH="$SYNTH" T77_OUT="$root/docs/captures" \
    python3 "$root/scripts/capture-osd-dnd-driver.py"