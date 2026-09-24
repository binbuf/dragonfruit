#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-01.6b loop walkthrough capture.
#
# Launches the T-01 demo on the nested backend with the compositor's
# synthetic-input harness enabled, drives the demo checklist against the live
# session (focus, window menu, zoom, minimize, restore-from-Dock, close) via
# scripts/capture-demo-driver.py, screenshots each step with Spectacle, and
# assembles a short walkthrough clip. The stills land in docs/captures/.
#
# Requires: a host Wayland session, `spectacle` (a screenshot tool that
# supports the host session), ffmpeg, python3 with
# Pillow, and the Qt/CMake tree built (`make build`). This is deliberately
# not part of `make e2e`: CI has no host session and no screenshot tool.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-capture.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t01-capture}"
SETTLE="${SETTLE:-9}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"
mkdir -p "$SCRATCH"

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

for tool in spectacle ffmpeg python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-demo: $tool not found — the walkthrough capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-demo: python3 Pillow not found" >&2
    exit 1
}

rm -f "$SYNTH"
LOG="$SCRATCH/demo.log"

echo "capture-demo: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!

for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-demo: synthetic-input socket never appeared; demo log:" >&2
    tail -30 "$LOG" >&2
    exit 1
fi
sleep "$SETTLE"

echo "capture-demo: driving the walkthrough"
python3 scripts/capture-demo-driver.py --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH"

# A short clip from the captured states (T-02 adds real motion; this slice is
# static, so the clip is the ordered walkthrough states).
STILLS=(
    "$OUTDIR/t01-loop-v0.png"
    "$OUTDIR/t01-loop-v0-wayland.png"
    "$OUTDIR/t01-loop-v0-menu.png"
    "$OUTDIR/t01-loop-v0-zoomed.png"
    "$OUTDIR/t01-loop-v0-minimized.png"
    "$OUTDIR/t01-loop-v0-restored.png"
    "$OUTDIR/t01-loop-v0-closed.png"
)
LIST="$SCRATCH/concat.txt"
: >"$LIST"
for still in "${STILLS[@]}"; do
    [ -f "$still" ] || continue
    printf "file '%s'\nduration 1.4\n" "$(realpath "$still")" >>"$LIST"
done
if [ -s "$LIST" ]; then
    # The concat demuxer needs the final frame repeated once.
    last=$(grep "^file " "$LIST" | tail -1 | sed "s/^file '//; s/'$//")
    printf "file '%s'\n" "$last" >>"$LIST"
    # Pick any H.264 encoder this ffmpeg has; fall back to mpeg4.
    encoder="mpeg4"
    for candidate in libx264 libopenh264; do
        if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q " $candidate "; then
            encoder="$candidate"
            break
        fi
    done
    ffmpeg -y -loglevel error -f concat -safe 0 -i "$LIST" \
        -vf "scale=1280:-2,format=yuv420p" -c:v "$encoder" -crf 28 -movflags +faststart \
        "$OUTDIR/t01-loop-v0.mp4"
    echo "capture-demo: wrote $OUTDIR/t01-loop-v0.mp4 ($encoder)"
fi

cat "$SCRATCH/walkthrough.txt" 2>/dev/null || true
echo "capture-demo: done"
