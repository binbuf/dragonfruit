#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7j Dock Tahoe visual language capture.
#
# Runs the nested demo with the compositor's synthetic-input harness and
# captures the Dock at rest, hovered (Trash, no magnification), magnified, and
# pressed under the light and dark schemes, then the `Minimal` degrade tier
# with reduced motion. The crops are stacked into:
#
#   * docs/captures/t14-dock-tahoe-light.png
#   * docs/captures/t14-dock-tahoe-dark.png
#   * docs/captures/t14-dock-tahoe-reduced.png
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-tahoe.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-tahoe}"
mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-tahoe: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-tahoe: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-tahoe: python3 Pillow not found" >&2
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

echo "capture-dock-tahoe: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!

for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-dock-tahoe: synthetic-input socket never appeared; demo log:" >&2
    tail -30 "$LOG" >&2
    exit 1
fi
sleep "$SETTLE"

python3 scripts/capture-dock-tahoe-driver.py \
    --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH"

echo "capture-dock-tahoe: done"