#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T198 Dock optics capture and report.
#
# Runs the real nested compositor at 1x/1.5x/2x over a deterministic
# multi-frequency checker wallpaper and captures the compositor's own render.
# Each case records the live `query scale`/`query material` snapshot and scores
# the flat band, checker MTF, rim and flat-edge geometry. A final Dock sweep
# records the material-cache counters while the pointer moves.
#
# The host session in CI is often a DRM session with no mapped nested window,
# so the script starts its own `kwin_wayland --virtual` host when available.
# Without either a usable host or kwin_wayland it writes an explicit OPEN
# report and leaves the committed model/tests as the hardware-free evidence.
#
# Outputs:
#   docs/captures/t198-dock-optics-{light,dark,reduce-transparency,increase-contrast}-1x.png
#   docs/captures/t198-dock-optics-light-1_5x.png
#   docs/captures/t198-dock-optics-light-2x.png
#   docs/captures/t198-dock-optics-*.json
#   docs/captures/t198-dock-optics-sweep.txt
#   docs/captures/t198-dock-optics.md
#
# Requires: a built tree, python3 with Pillow, and kwin_wayland or a host
# Wayland session. Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-6}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t198.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
KWIN_SOCKET="df-t198-kwin-$$"
KWIN_PID=""
HOST_DISPLAY="${WAYLAND_DISPLAY:-}"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-}"
FORCE_KWIN="${FORCE_KWIN:-0}"
mkdir -p "$OUTDIR" "$SCRATCH"

cleanup() {
    if [ -n "$KWIN_PID" ]; then
        kill -TERM "$KWIN_PID" 2>/dev/null || true
        sleep 1
        kill -KILL "$KWIN_PID" 2>/dev/null || true
    fi
    rm -f "${XDG_RUNTIME_DIR:-/tmp}/$KWIN_SOCKET" \
          "${XDG_RUNTIME_DIR:-/tmp}/$KWIN_SOCKET.lock" 2>/dev/null || true
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

if [ -z "$HOST_RUNTIME" ]; then
    echo "capture-t198: XDG_RUNTIME_DIR is unset — writing OPEN" >&2
    cat >"$OUTDIR/t198-dock-optics.open.txt" <<EOF
T198 Dock optics — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make dock-optics-capture
EOF
    exit 0
fi

if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t198: starting kwin_wayland --virtual (socket $KWIN_SOCKET)"
        XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY= \
            kwin_wayland --virtual --fullscreen 1 --socket "$KWIN_SOCKET" \
            --width 3840 --height 2446 --no-lockscreen \
            --no-global-shortcuts --no-kactivities \
            >"$SCRATCH/kwin.log" 2>&1 &
        KWIN_PID=$!
        for _ in $(seq 1 100); do
            [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ] && break
            sleep 0.1
        done
        if ! [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ]; then
            echo "capture-t198: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t198: no host Wayland session and no kwin_wayland — writing OPEN" >&2
        cat >"$OUTDIR/t198-dock-optics.open.txt" <<EOF
T198 Dock optics — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: no usable host Wayland session and kwin_wayland is not installed.
rerun: WAYLAND_DISPLAY=<host-socket> make dock-optics-capture
EOF
        exit 0
    fi
fi

checker_for() {
    local scale="$1" width="$2" height="$3" background="${4:-multi}"
    local path="$SCRATCH/${background}-${scale}.png"
    if [ ! -s "$path" ]; then
        python3 scripts/t198-dock-optics.py wallpaper --out "$path" \
            --width "$((width))" --height "$((height))" --scale "$scale" \
            --background "$background"
    fi
    printf '%s\n' "$path"
}

run_case() {
    local label="$1" scale="$2" scheme="$3" reduce="$4" contrast="$5"
    local background="${6:-multi}"
    local width height wallpaper socket case_scratch out capture query score
    width="$(python3 -c "print(round(1920*$scale))")"
    height="$(python3 -c "print(round(1200*$scale))")"
    wallpaper="$(checker_for "$scale" "$width" "$height" "$background")"
    socket="df-t198-${label}-$$"
    case_scratch="$SCRATCH/$label"
    out="$OUTDIR/t198-dock-optics-${label}.png"
    query="$SCRATCH/$label-query.txt"
    score="$OUTDIR/t198-dock-optics-${label}.json"
    mkdir -p "$case_scratch"
    echo "capture-t198: $label (scale $scale, scheme $scheme)"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        DRAGONFRUIT_NESTED_SIZE="${width}x${height}" \
        DRAGONFRUIT_GLASS_ROLLOUT=replacement \
        FULL_SESSION=1 SCALE="$scale" SOCKET="$socket" \
        WALLPAPER_SOURCE="$wallpaper" COLOR_SCHEME="$scheme" \
        REDUCE_TRANSPARENCY="$reduce" INCREASE_CONTRAST="$contrast" \
        QUERY_OUT="$query" SETTLE="$SETTLE" OUT="$out" \
        SCRATCH="$case_scratch" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh settled >"$SCRATCH/$label.log" 2>&1
    capture="$case_scratch/capture.png"
    if [ ! -s "$capture" ]; then
        capture="$out"
    fi
    if [ ! -s "$capture" ]; then
        echo "capture-t198: $label produced no capture" >&2
        return 1
    fi
    if [ "$capture" != "$out" ]; then
        cp "$capture" "$out"
    fi
    # The full-session rail keeps its scratch XDG tree under the runtime dir.
    # It is large and not needed once the capture is copied out.
    rm -rf "$HOST_RUNTIME/$socket.session" "$HOST_RUNTIME/$socket" \
           "$HOST_RUNTIME/$socket.lock" "$HOST_RUNTIME/$socket.launch-token" \
           "$HOST_RUNTIME/$socket.desktop-launch-token" 2>/dev/null || true
    python3 scripts/t198-dock-optics.py score --capture "$out" --query "$query" \
        --out "$score" --background "$background" \
        >"$SCRATCH/$label-score.txt" 2>&1 || true
    echo "capture-t198: $label -> $out"
}

run_case light-1x 1 light 0 0
run_case dark-1x 1 dark 0 0
run_case reduce-transparency-1x 1 light 1 0
run_case increase-contrast-1x 1 light 0 1
run_case light-1_5x 1.5 light 0 0
run_case light-2x 2 light 0 0

# T235a: the review background matrix. It is opt-in because it is a large
# sweep; the default run above stays the fast T198 evidence set. Each case is
# named `t235a-<background>-<scale>x-<condition>` and the scorer records the
# background it measured.
if [ "${T235A_MATRIX:-0}" = "1" ]; then
    for background in checker_4 checker_8 checker_16 checker_32 checker_64 \
            lines_grid stripes_h stripes_v; do
        for scale in 1 1.5 2; do
            run_case "t235a-${background}-${scale}x-light" "$scale" light 0 0 "$background"
            run_case "t235a-${background}-${scale}x-dark" "$scale" dark 0 0 "$background"
        done
    done
    for scale in 1 1.5 2; do
        run_case "t235a-checker_16-${scale}x-reduce-transparency" "$scale" light 1 0 checker_16
        run_case "t235a-checker_16-${scale}x-increase-contrast" "$scale" light 0 1 checker_16
    done
fi

# Cache behaviour during a Dock sweep. The motion state keeps the pointer
# moving across the capture instant; the two material snapshots one second
# apart show the warm-up flat counters.
SWEEP_OUT="$OUTDIR/t198-dock-optics-sweep.txt"
SWEEP_STATS="$SCRATCH/sweep-stats.txt"
SWEEP_QUERY="$SCRATCH/sweep-query.txt"
env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
    DRAGONFRUIT_NESTED_SIZE="1920x1200" DRAGONFRUIT_GLASS_ROLLOUT=replacement \
    FULL_SESSION=1 SCALE=1 SOCKET="df-t198-sweep-$$" \
    WALLPAPER_SOURCE="$(checker_for 1 1920 1200)" COLOR_SCHEME=light \
    QUERY_OUT="$SWEEP_QUERY" SETTLE="$SETTLE" \
    OUT="$OUTDIR/t198-dock-optics-sweep.png" \
    SCRATCH="$SCRATCH/sweep" KEEP_SCRATCH=1 DUMP_COMPOSITOR_STATS=1 \
    STATS_OUT="$SWEEP_STATS" ASSERT_MATERIAL_FLAT=1 \
    bash scripts/t21-live-capture.sh motion >"$SCRATCH/sweep.log" 2>&1 || true

{
    echo "T198 Dock optics — cache behaviour during a Dock sweep"
    echo "timestamp: $(date -Is)"
    echo "backend: nested (kwin_wayland --virtual)" 
    echo "scale: 1  material: full  rollout: replacement"
    echo
    echo "== live query =="
    cat "$SWEEP_QUERY" 2>/dev/null || true
    echo
    echo "== material snapshots (warm-up + sweep) =="
    cat "$SWEEP_STATS" 2>/dev/null || true
    echo
    echo "Expected: scene/panel allocations and region pixels are flat after"
    echo "warm-up; blur_skips rises as the pointer sweeps while the cached"
    echo "Dock texture is reused. A change in blur_miss_* names the invalidator."
} >"$SWEEP_OUT"

python3 scripts/t198-dock-optics.py report --outdir "$OUTDIR"

echo "capture-t198: done"
