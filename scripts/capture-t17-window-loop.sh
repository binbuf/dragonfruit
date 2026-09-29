#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-17.1a nested window-loop capture.
#
# Launches the nested Dragonfruit demo (`make demo`) with the compositor's
# synthetic-input harness, starts a third-party Qt (SSD) client against the
# private socket, and drives the 30-second loop through
# scripts/t17-window-loop-driver.py: launch/appear, focus, move (titlebar
# drag), zoom, minimize, restore-from-Dock, and close, plus the CSD-unaffected
# and X11 SSD checks. Each step is screenshotted with Spectacle and the stills
# land in docs/captures/t17-window-loop*.
#
# The Qt SSD client is `kcalc` when installed (a real third-party Qt app that
# negotiates server-side decoration); when it is absent the loop runs on the
# X11 window and the deviation is recorded. The first-party Qt apps are CSD by
# design (apps/settings/SettingsWindow.qml), which is the CSD arm of the check.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-14}"
SOCKET="${SOCKET:-dragonfruit-t17-window-loop}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t17-loop.XXXXXX")}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t17: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t17: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t17: python3 Pillow not found" >&2
    exit 1
}

DEMO_PGID=""
KCALC_PGID=""
cleanup() {
    for pgid in "$KCALC_PGID" "$DEMO_PGID"; do
        [ -n "$pgid" ] || continue
        kill -TERM -"$pgid" 2>/dev/null || true
    done
    sleep 1
    for pgid in "$KCALC_PGID" "$DEMO_PGID"; do
        [ -n "$pgid" ] || continue
        kill -KILL -"$pgid" 2>/dev/null || true
    done
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

rm -f "$SYNTH"

echo "capture-t17: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-t17: synthetic socket never appeared; demo log:" >&2
    tail -30 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

# A third-party Qt (SSD) client. kcalc is a real third-party Qt app that requests
# server-side decoration; the first-party apps are CSD by design.
if command -v kcalc >/dev/null 2>&1; then
    echo "capture-t17: launching Qt SSD client (kcalc)"
    WAYLAND_DISPLAY="$SOCKET" QT_QPA_PLATFORM=wayland \
        setsid kcalc >"$SCRATCH/kcalc.log" 2>&1 &
    KCALC_PGID=$!
    sleep 5
else
    echo "capture-t17: kcalc not found — the loop runs on the X11 window (recorded)"
fi

echo "capture-t17: driving the window loop"
python3 scripts/t17-window-loop-driver.py \
    --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH" \
    --prefix t17-window-loop

# The step-by-step machine transcript (each lifecycle primitive and the
# decoration report around it).
if [ -f "$SCRATCH/transcript.txt" ]; then
    cp "$SCRATCH/transcript.txt" "$OUTDIR/t17-window-loop.txt"
    echo "capture-t17: wrote $OUTDIR/t17-window-loop.txt"
fi

# A short clip from the captured states (the loop ordered as it ran).
if command -v ffmpeg >/dev/null 2>&1; then
    STILLS=(
        "$OUTDIR/t17-window-loop.png"
        "$OUTDIR/t17-window-loop-focused.png"
        "$OUTDIR/t17-window-loop-move.png"
        "$OUTDIR/t17-window-loop-zoom.png"
        "$OUTDIR/t17-window-loop-minimized.png"
        "$OUTDIR/t17-window-loop-restored.png"
        "$OUTDIR/t17-window-loop-closed.png"
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
            -movflags +faststart "$OUTDIR/t17-window-loop.mp4"
        echo "capture-t17: wrote $OUTDIR/t17-window-loop.mp4 ($encoder)"
    fi
fi

cat "$SCRATCH/walkthrough.txt" 2>/dev/null || true
echo "capture-t17: done (scratch $SCRATCH)"
[ "${KEEP_SCRATCH:-0}" = "1" ] && echo "capture-t17: kept scratch $SCRATCH" || true