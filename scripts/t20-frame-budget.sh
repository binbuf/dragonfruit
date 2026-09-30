#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-20.4 frame-budget trace (nested).
#
# Runs the nested demo (shell + the demo's live windows) with the material
# pinned to `Full`, drives a burst of real animation frames, then cleanly stops
# the compositor so its `exit` dump lands in the log. The report records the
# GPU material counters (blur passes/panels, the largest downsample), the
# degrade-ladder counters (per-tier frames, downgrades/upgrades), and the
# frame-timing samples judged against the 16 ms budget. It lands in
# docs/captures/t20-frame-budget.txt.
#
# This is the honest host trace: the material's blur/glass pass runs on the
# dev iGPU. The trace also records the recommended-GPU run as OPEN (the T-16
# hardware rail owns the baseline GPU), matching the T-03 pattern.
#
# Requires a host Wayland session and the built tree (`make build`); not part of
# `make e2e`. Overrides: SETTLE (default 12), ANIMATE_MS (default 1200),
# OUT (report path), BUDGET_US (default 16000), KEEP_SCRATCH.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="${OUT:-docs/captures/t20-frame-budget.txt}"
SETTLE="${SETTLE:-12}"
ANIMATE_MS="${ANIMATE_MS:-1200}"
BUDGET_US="${BUDGET_US:-16000}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"

[ -n "${XDG_RUNTIME_DIR:-}" ] || {
    echo "t20-frame-budget: XDG_RUNTIME_DIR is not set" >&2
    exit 1
}
[ -n "${WAYLAND_DISPLAY:-}" ] || {
    echo "t20-frame-budget: no WAYLAND_DISPLAY — the nested trace needs a host session" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "t20-frame-budget: python3 not found" >&2
    exit 1
}

socket="dragonfruit-t20-budget-$$"
synth="$XDG_RUNTIME_DIR/$socket.synth"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t20-budget.XXXXXX")"
log="$scratch/demo.log"
rm -f "$synth"

echo "t20-frame-budget: starting the nested demo ($socket)"
DRAGONFRUIT_SYNTHETIC_INPUT="$synth" DRAGONFRUIT_FRAME_TRACE=1 \
    setsid ./target/debug/dragonfruit dev --demo --socket-name "$socket" \
    >"$log" 2>&1 &
pgid=$!

cleanup() {
    # Stop the compositor directly so it dumps stats on exit, then reap the
    # demo process group. A leftover socket is removed here regardless.
    local comp
    comp="$(pgrep -f "dragonfruit-compositor --backend nested --socket-name $socket" || true)"
    if [ -n "$comp" ]; then
        kill -TERM "$comp" 2>/dev/null || true
        for _ in $(seq 1 100); do [ -e "$XDG_RUNTIME_DIR/$socket" ] || break; sleep 0.1; done
    fi
    kill -TERM -"$pgid" 2>/dev/null || true
    sleep 1
    kill -KILL -"$pgid" 2>/dev/null || true
    rm -f "$synth"
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$scratch"
}
trap cleanup EXIT

for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$socket" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$synth" ] && break; sleep 0.1; done
if [ ! -e "$synth" ]; then
    echo "t20-frame-budget: synthetic-input socket never appeared; demo log:" >&2
    tail -30 "$log" >&2
    exit 1
fi
sleep "$SETTLE"

python3 scripts/t20-frame-budget-driver.py --synth "$synth" \
    --animate-ms "$ANIMATE_MS" --settle 2.0

# Stop the compositor so its `exit` dump_stats block lands in the log.
comp="$(pgrep -f "dragonfruit-compositor --backend nested --socket-name $socket" || true)"
if [ -z "$comp" ]; then
    echo "t20-frame-budget: compositor pid not found" >&2
    exit 1
fi
kill -TERM "$comp" 2>/dev/null || true
for _ in $(seq 1 150); do [ -e "$XDG_RUNTIME_DIR/$socket" ] || break; sleep 0.1; done
sleep 1

mkdir -p "$(dirname "$OUT")"
{
    echo "# T-20.4 frame-budget trace (nested, material Full)"
    echo "# host: $(uname -srm)"
    echo "# gpu: $(grep -m1 -E 'Multi-GPU:|renderer' "$log" 2>/dev/null || echo 'nested GL renderer (no DRM device scan)')"
    echo "# budget_us: $BUDGET_US"
    echo "# command: make build && scripts/t20-frame-budget.sh"
    echo "#"
    echo "# The dev-iGPU run below drives the material at Full. The recommended"
    echo "# GPU baseline run (GLES 3.x / Vulkan, dedicated) is OPEN — the T-16"
    echo "# hardware rail owns the baseline machine."
    echo "#"
    grep -E "Multi-GPU: SCENE|material stats \((exit|SIGUSR1)\)|degrade stats \((exit|SIGUSR1)\)|render stats \((exit|SIGUSR1)\)|latency stats \((exit|SIGUSR1)\)|frame timing \((exit|SIGUSR1)\)" "$log" \
        | sed 's/^dragonfruit-compositor: //' || true
} >"$OUT"

echo "t20-frame-budget: report written to $OUT"
grep -E "material stats \(exit\)|degrade stats \(exit\)|frame timing \(exit\)" "$log" \
    | sed 's/^dragonfruit-compositor: //' || true