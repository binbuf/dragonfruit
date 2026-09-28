#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-17.1b nested navigation capture.
#
# Launches the nested Dragonfruit demo (`make demo`) with the compositor's
# synthetic-input harness and drives the workspace-switch, Mission Control, and
# app-switch navigation through scripts/t17-navigation-driver.py, by pointer
# and keyboard, on the live session. Each step is screenshotted with Spectacle
# and the stills land in docs/captures/t17-navigation*.
#
# Every path is asserted through the compositor's read-only introspection
# (`query spaces`, `query grid`, `query wallpaper`, `query switcher`): a path
# that does not reach the documented state fails the run.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-14}"
SOCKET="${SOCKET:-dragonfruit-t17-navigation}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t17-nav.XXXXXX")}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t17-nav: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t17-nav: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t17-nav: python3 Pillow not found" >&2
    exit 1
}

DEMO_PGID=""
cleanup() {
    [ -n "$DEMO_PGID" ] && kill -TERM -"$DEMO_PGID" 2>/dev/null || true
    sleep 1
    [ -n "$DEMO_PGID" ] && kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

rm -f "$SYNTH"

echo "capture-t17-nav: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-t17-nav: synthetic socket never appeared; demo log:" >&2
    tail -30 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

echo "capture-t17-nav: driving the navigation loop"
python3 scripts/t17-navigation-driver.py \
    --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH" \
    --prefix t17-navigation

if [ -f "$SCRATCH/transcript.txt" ]; then
    cp "$SCRATCH/transcript.txt" "$OUTDIR/t17-navigation.txt"
    echo "capture-t17-nav: wrote $OUTDIR/t17-navigation.txt"
fi

# A short clip from the captured states (the loop ordered as it ran).
if command -v ffmpeg >/dev/null 2>&1; then
    STILLS=(
        "$OUTDIR/t17-navigation.png"
        "$OUTDIR/t17-navigation-workspace-keyboard.png"
        "$OUTDIR/t17-navigation-workspace-gesture.png"
        "$OUTDIR/t17-navigation-mission-control-keyboard.png"
        "$OUTDIR/t17-navigation-mission-control-pointer.png"
        "$OUTDIR/t17-navigation-workspace-pointer.png"
        "$OUTDIR/t17-navigation-app-switch-keyboard.png"
        "$OUTDIR/t17-navigation-app-switch-pointer.png"
        "$OUTDIR/t17-navigation-app-switch-pointer-committed.png"
    )
    LIST="$SCRATCH/concat.txt"
    : >"$LIST"
    for still in "${STILLS[@]}"; do
        [ -f "$still" ] || continue
        printf "file '%s'\nduration 1.4\n" "$(realpath "$still")" >>"$LIST"
    done
    if [ -s "$LIST" ]; then
        last=$(grep "^file " "$LIST" | tail -1 | sed "s/^file '//; s/'$//")
        printf "file '%s'\n" "$last" >>"$LIST"
        encoder="mpeg4"
        for candidate in libx264 libopenh264; do
            if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q " $candidate "; then
                encoder="$candidate"
                break
            fi
        done
        ffmpeg -y -loglevel error -f concat -safe 0 -i "$LIST" \
            -vf "scale=1280:-2,format=yuv420p" -c:v "$encoder" -crf 28 \
            -movflags +faststart "$OUTDIR/t17-navigation.mp4"
        echo "capture-t17-nav: wrote $OUTDIR/t17-navigation.mp4 ($encoder)"
    fi
fi

cat "$SCRATCH/walkthrough.txt" 2>/dev/null || true
echo "capture-t17-nav: done (scratch $SCRATCH)"
[ "${KEEP_SCRATCH:-0}" = "1" ] && echo "capture-t17-nav: kept scratch $SCRATCH" || true