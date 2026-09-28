#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7z Dock plate corner and frost-alignment capture.
#
# Runs the nested demo with the synthetic-input harness, parks the pointer off
# the Dock to catch the resting plate, and writes:
#
#   * docs/captures/t14-dock-plate-corners-{light,dark}.png — a 6x close-up of
#     the plate's top-left and top-right corners (the rim follows the arcs; no
#     straight hairline pokes past them).
#   * docs/captures/t14-dock-plate-magnified-{light,dark}.png — a 2x strip of
#     the plate mid-magnification (the QML fill and the compositor frost share
#     one rounded edge).
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-corners.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-corners}"
mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-plate-corners: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-plate-corners: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-plate-corners: python3 Pillow not found" >&2
    exit 1
}

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
rm -f "$SYNTH"
LOG="$SCRATCH/demo.log"

cleanup() {
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

echo "capture-dock-plate-corners: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!

for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-dock-plate-corners: synthetic-input socket never appeared; demo log:" >&2
    tail -30 "$LOG" >&2
    exit 1
fi
sleep "$SETTLE"

COMPOSITOR_PID="$(pgrep -n -f 'dragonfruit-compositor' || true)"
echo "capture-dock-plate-corners: compositor pid ${COMPOSITOR_PID:-unknown}"

python3 scripts/capture-dock-plate-corners-driver.py \
    --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH" \
    --demo-log "$LOG" --compositor-pid "${COMPOSITOR_PID:-0}"

echo "capture-dock-plate-corners: done"