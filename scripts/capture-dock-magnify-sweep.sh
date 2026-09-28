#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7y Dock magnification slow-sweep capture.
#
# Runs the nested demo with the compositor's synthetic-input harness and sweeps
# the pointer slowly across the Dock under the light and dark schemes, taking a
# crop at evenly spaced positions and stacking them into one filmstrip per
# scheme:
#
#   * docs/captures/t14-dock-magnify-sweep-light.png
#   * docs/captures/t14-dock-magnify-sweep-dark.png
#
# The strips show the plate top tracking the peak across a sweep. A settled
# crop cannot show the temporal ringing the fix removes (that is pinned by
# `tst_dock.qml` and the measured oscillation in PROGRESS.md), so the committed
# strips carry a "before" row captured from the pre-fix tree above the "after"
# row when `sweep-row-<scheme>-before.png` is present in the output directory.
#
# Env knobs: LABEL (row label, default `after`), COMPOSE (0 to skip the stacked
# strip, default 1).
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
LABEL="${LABEL:-after}"
COMPOSE="${COMPOSE:-1}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-sweep.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-sweep}"
mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-magnify-sweep: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-magnify-sweep: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-magnify-sweep: python3 Pillow not found" >&2
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

echo "capture-dock-magnify-sweep: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!

for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-dock-magnify-sweep: synthetic-input socket never appeared; demo log:" >&2
    tail -30 "$LOG" >&2
    exit 1
fi
sleep "$SETTLE"

for scheme in light dark; do
    python3 scripts/capture-dock-magnify-sweep-driver.py \
        --capture --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH" \
        --scheme "$scheme" --label "$LABEL"
done

if [ "$COMPOSE" = "1" ]; then
    python3 scripts/capture-dock-magnify-sweep-driver.py \
        --compose --outdir "$OUTDIR"
fi

echo "capture-dock-magnify-sweep: done"