#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-02 lifecycle-motion capture (T-02.4b).
#
# Runs the T-01/T-02 nested demo twice — once normally, once with
# `accessibility.reduceMotion` on — drives the window lifecycle walkthrough
# (focus, zoom, minimize, restore, close) over the synthetic-input harness
# with scripts/capture-demo-driver.py, screenshots each settled state with
# Spectacle, and assembles the ordered states into a short clip per mode.
#
# The stills land in docs/captures/ as t02-lifecycle-motion[-reduced]*.
# Note: like the T-01 capture, this slice assembles the clip from settled
# stills; the live nested motion is measured by T-03.1a's frame trace.
#
# Requires: a host Wayland session, `spectacle`, `ffmpeg`, python3 with
# Pillow, and the Qt/CMake tree built (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
SETTLE="${SETTLE:-9}"

for tool in spectacle ffmpeg python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-motion: $tool not found — the walkthrough capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-motion: python3 Pillow not found" >&2
    exit 1
}

# assemble_clip <prefix> <out.mp4>
assemble_clip() {
    local prefix="$1" out="$2"
    local scratch
    scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-clip.XXXXXX")"
    local list="$scratch/concat.txt"
    : >"$list"
    local stills=(
        "$OUTDIR/$prefix.png"
        "$OUTDIR/$prefix-wayland.png"
        "$OUTDIR/$prefix-menu.png"
        "$OUTDIR/$prefix-zoomed.png"
        "$OUTDIR/$prefix-minimized.png"
        "$OUTDIR/$prefix-restored.png"
        "$OUTDIR/$prefix-closed.png"
    )
    local last="" still
    for still in "${stills[@]}"; do
        [ -f "$still" ] || continue
        printf "file '%s'\nduration 1.4\n" "$(realpath "$still")" >>"$list"
        last="$still"
    done
    if [ -n "$last" ]; then
        printf "file '%s'\n" "$(realpath "$last")" >>"$list"
        local encoder="mpeg4" candidate
        for candidate in libx264 libopenh264; do
            if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q " $candidate "; then
                encoder="$candidate"
                break
            fi
        done
        ffmpeg -y -loglevel error -f concat -safe 0 -i "$list" \
            -vf "scale=1280:-2,format=yuv420p" -c:v "$encoder" -crf 28 -movflags +faststart \
            "$out"
        echo "capture-motion: wrote $out ($encoder)"
    fi
    rm -rf "$scratch"
}

# run_mode <suffix> <driver-flags...>
run_mode() {
    local suffix="$1"; shift
    local socket="dragonfruit-t02-capture${suffix:+$suffix}-$$"
    local synth="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$socket.synth"
    local scratch
    scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-capture.XXXXXX")"
    local log="$scratch/demo.log"
    rm -f "$synth"

    echo "capture-motion: starting the nested demo ($socket)"
    DRAGONFRUIT_SYNTHETIC_INPUT="$synth" setsid make demo DEMO_ARGS="--socket-name $socket" \
        >"$log" 2>&1 &
    local pgid=$!

    cleanup() {
        kill -TERM -"$pgid" 2>/dev/null || true
        sleep 1
        kill -KILL -"$pgid" 2>/dev/null || true
        rm -f "$synth"
        [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$scratch"
    }
    trap cleanup RETURN

    local _
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$socket" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$synth" ] && break; sleep 0.1; done
    if [ ! -e "$synth" ]; then
        echo "capture-motion: synthetic-input socket never appeared; demo log:" >&2
        tail -30 "$log" >&2
        return 1
    fi
    sleep "$SETTLE"

    echo "capture-motion: driving the lifecycle walkthrough ($suffix)"
    python3 scripts/capture-demo-driver.py --synth "$synth" --outdir "$OUTDIR" \
        --scratch "$scratch" "$@"

    cleanup
    trap - RETURN
    rm -rf "$scratch"
}

run_mode "" --prefix t02-lifecycle-motion
run_mode "-reduced" --prefix t02-lifecycle-motion-reduced --reduced

assemble_clip t02-lifecycle-motion "$OUTDIR/t02-lifecycle-motion.mp4"
assemble_clip t02-lifecycle-motion-reduced "$OUTDIR/t02-lifecycle-motion-reduced.mp4"

echo "capture-motion: done"
