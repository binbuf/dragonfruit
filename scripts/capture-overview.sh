#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-05.6 Mission Control live capture.
#
# Runs the nested T-01/T-05 demo twice — once normally, once with
# `accessibility.reduceMotion` on — drives the Mission Control walkthrough
# (open on the live surfaces, Desktop Reveal, a per-Space wallpaper slide)
# over the synthetic-input harness with scripts/capture-overview-driver.py,
# screenshots each state with Spectacle, and assembles the ordered states into
# a short clip per mode.
#
# Each run also samples the T-05.6 gesture frame-budget instrument
# (`query gesture`) after every gesture; the raw samples plus the compositor's
# exit `gesture budget`/`degrade stats` lines are written to
# docs/captures/t05-gesture-budget-nested.txt so the full-gesture 60 Hz verdict
# (or its honest shortfall) is recorded next to the stills.
#
# The stills land in docs/captures/ as t05-mission-control-live[-reduced]*.
# Note: like the T-01..T-04 captures, this slice assembles the clip from
# settled stills; the live gesture cadence is what the trace file records.
#
# Requires: a host Wayland session, `spectacle`, `ffmpeg`, python3 with
# Pillow, and the Qt/CMake tree built (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-overview.XXXXXX")}"
SETTLE="${SETTLE:-9}"
mkdir -p "$SCRATCH"

for tool in spectacle ffmpeg python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-overview: $tool not found — the walkthrough capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-overview: python3 Pillow not found" >&2
    exit 1
}

# run_mode <prefix> <driver-flags...>
run_mode() {
    local prefix="$1"; shift
    local socket="dragonfruit-t05-capture-${prefix##*-}-$$"
    local synth="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$socket.synth"
    local log="$SCRATCH/$prefix.log"
    rm -f "$synth"

    echo "capture-overview: starting the nested demo for $prefix (socket $socket)"
    DRAGONFRUIT_SYNTHETIC_INPUT="$synth" setsid make demo DEMO_ARGS="--socket-name $socket" \
        >"$log" 2>&1 &
    local pgid=$!

    for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$socket" ] && break; sleep 0.1; done
    for _ in $(seq 1 300); do [ -e "$synth" ] && break; sleep 0.1; done
    if [ ! -e "$synth" ]; then
        echo "capture-overview: synthetic-input socket never appeared; demo log:" >&2
        tail -30 "$log" >&2
        kill -TERM -"$pgid" 2>/dev/null || true
        exit 1
    fi
    sleep "$SETTLE"

    echo "capture-overview: driving the $prefix walkthrough"
    python3 scripts/capture-overview-driver.py \
        --synth "$synth" --outdir "$OUTDIR" --scratch "$SCRATCH" \
        --prefix "$prefix" "$@"

    # Shut down first so the compositor prints its exit stats; then record the
    # honest whole-session gesture budget and the degrade tier reached under
    # pressure.
    kill -TERM -"$pgid" 2>/dev/null || true
    for _ in $(seq 1 50); do
        grep -q "clean exit" "$log" 2>/dev/null && break
        sleep 0.2
    done
    {
        echo "# $prefix session exit:"
        grep -E "gesture budget \(exit\)|degrade stats \(exit\)" "$log" || true
        echo ""
    } >"$SCRATCH/$prefix-exit.txt"
    kill -KILL -"$pgid" 2>/dev/null || true
    rm -f "$synth"
}

# assemble_clip <prefix> <out.mp4>
assemble_clip() {
    local prefix="$1" out="$2"
    local list="$SCRATCH/$prefix-concat.txt"
    : >"$list"
    local stills=(
        "$OUTDIR/$prefix.png"
        "$OUTDIR/$prefix-reveal.png"
        "$OUTDIR/$prefix-slide.png"
        "$OUTDIR/$prefix-slide-settled.png"
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
        echo "capture-overview: wrote $out ($encoder)"
    fi
}

run_mode "t05-mission-control-live"
run_mode "t05-mission-control-live-reduced" --reduced

assemble_clip "t05-mission-control-live" "$OUTDIR/t05-mission-control-live.mp4"
assemble_clip "t05-mission-control-live-reduced" "$OUTDIR/t05-mission-control-live-reduced.mp4"

# Merge both modes' per-gesture samples and both sessions' exit stats into the
# one trace file the acceptance names.
TRACE="$OUTDIR/t05-gesture-budget-nested.txt"
{
    echo "# T-05.6 gesture frame-budget trace (nested backend)."
    echo "# Each line is one \`query gesture\` sample taken after the named gesture."
    echo "# budget_us=16000 is one 60 Hz frame; held=0 records a shortfall rather"
    echo "# than hiding it. tier is the live T-04.4a material degrade tier."
    echo ""
    cat "$SCRATCH/gesture-t05-mission-control-live.txt"
    cat "$SCRATCH/gesture-t05-mission-control-live-reduced.txt"
    cat "$SCRATCH/t05-mission-control-live-exit.txt" 2>/dev/null || true
    cat "$SCRATCH/t05-mission-control-live-reduced-exit.txt" 2>/dev/null || true
} >"$TRACE"
echo "capture-overview: wrote $TRACE"
echo "capture-overview: done (scratch $SCRATCH)"