#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T238 follow-up: split versus full-output under continuously changing content.
#
# Each (arm, workload) pair gets a fresh release compositor + shell in the
# nested 144 Hz session. The driver launches the clients, injects synthetic
# input where needed, samples RSS, dumps SIGUSR1 stats, and writes one JSON
# report. The summary reports per-frame p50/p95/p99, dirty-frame percentage,
# scene/composite CPU and GPU time, scene texture memory, RSS, presentation
# cadence, scene-cache skips, and the element breakdown.
#
# Workloads:
#   static          two xmessage windows, no input
#   scroll          a Tk text client auto-scrolling at ~60 Hz plus axis input
#   drag            synthetic titlebar drag of the first floating window
#   resize          synthetic bottom-right resize of the first floating window
#   video           ffplay testsrc2 at 60 fps in an Xwayland window
#   focus           click each window's titlebar in turn
#   mission-control Ctrl+Up Overview with pointer motion
#   scheme-pulse    alternating light/dark every 100 ms. This is a proxy for an
#                   animated wallpaper: the compositor has no animated wallpaper
#                   source yet, and the scheme change exercises the same
#                   scene-epoch invalidation path.
#
# Native DRM 4K/144 is not available on this host (the live session owns DRM
# master). The script is backend-agnostic; a free seat can run it with
# BACKEND=drm after the application systemd-scope isolation lands.
#
# Arms:
#   split / full / full-skip      span decorations (the T238 baseline)
#   split-shadow / full-shadow    one-quad shader shadows
#   split-deco / full-deco        shader shadows + one-quad traffic lights
#
# Overrides: ARMS, WORKLOADS, DURATION, WARMUP, WINDOWS, PIN_TIER, TRIALS,
# SHUFFLE, PROFILE, BACKEND, OUT_DIR, CAPTURE_DIR, CAPTURE_AB.
#
# Decoration A/B: WINDOWS=2|10|30, PIN_TIER=full, TRIALS=5, SHUFFLE=1 gives a
# randomized-order multi-trial comparison at a fixed degrade tier. Without
# PIN_TIER the nested compositor may auto-degrade under the 144 Hz budget,
# which changes the shadow layer count and therefore the element count.
set -euo pipefail

cd "$(dirname "$0")/.."

. "$(dirname "$0")/t21-capture-lib.sh"

OUT_DIR="${OUT_DIR:-docs/captures/t238-material-workloads}"
CAPTURE_DIR="${CAPTURE_DIR:-}"
CAPTURE_AB="${CAPTURE_AB:-}"
ARMS="${ARMS:-split full full-skip}"
WORKLOADS="${WORKLOADS:-static scroll drag resize video mission-control scheme-pulse}"
DURATION="${DURATION:-5}"
WARMUP="${WARMUP:-1.5}"
WINDOWS="${WINDOWS:-2}"
PIN_TIER="${PIN_TIER:-}"
TRIALS="${TRIALS:-1}"
SHUFFLE="${SHUFFLE:-0}"
WATCHDOG_TIMEOUT="${WATCHDOG_TIMEOUT:-20}"
PROFILE="${PROFILE:-release}"
BACKEND="${BACKEND:-nested}"
T21_COMPOSITOR_BIN="${COMPOSITOR_BIN:-$T21_ROOT/target/$PROFILE/dragonfruit-compositor}"
export T21_COMPOSITOR_BIN

t21_capture_init "t238-material-workloads"
if [ "$BACKEND" = "drm" ]; then
    : "${XDG_RUNTIME_DIR:?the DRM workload needs a runtime directory}"
else
    t21_require_host "$OUT_DIR/open.txt" "T238 material workload comparison"
fi
t21_require_exec "$T21_SHELL_BIN" "run make build"
t21_require_exec "$T21_COMPOSITOR_BIN" "run make build"
command -v python3 >/dev/null 2>&1 || {
    echo "t238-material-workloads: python3 not found" >&2
    exit 1
}
command -v xmessage >/dev/null 2>&1 || {
    echo "t238-material-workloads: xmessage not found" >&2
    exit 1
}

mkdir -p "$OUT_DIR"
COMMIT="$(git -C "$T21_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"

export DRAGONFRUIT_GPU_TIMING=1
export DRAGONFRUIT_FRAME_TRACE=1
if [ "$BACKEND" = "drm" ]; then
    unset DRAGONFRUIT_REFRESH_HZ
else
    export DRAGONFRUIT_REFRESH_HZ=144
fi
export SDL_VIDEODRIVER=x11

arm_env() {
    local arm="$1"
    unset DRAGONFRUIT_MATERIAL_OFF
    unset DRAGONFRUIT_MATERIAL_FULL_OUTPUT
    unset DRAGONFRUIT_SCENE_CACHE_SKIP
    unset DRAGONFRUIT_SHADOW_GPU
    unset DRAGONFRUIT_TITLEBAR_GPU
    case "$arm" in
        split)
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=split
            ;;
        full)
            export DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=union
            ;;
        full-skip)
            export DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=union
            export DRAGONFRUIT_SCENE_CACHE_SKIP=1
            ;;
        split-skip)
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=split
            export DRAGONFRUIT_SCENE_CACHE_SKIP=1
            ;;
        split-shadow)
            # Split regions with the one-quad shader shadow (T238 decoration
            # optimisation). Everything else matches `split`.
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=split
            export DRAGONFRUIT_SHADOW_GPU=1
            ;;
        split-shadow-skip)
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=split
            export DRAGONFRUIT_SHADOW_GPU=1
            export DRAGONFRUIT_SCENE_CACHE_SKIP=1
            ;;
        full-shadow)
            export DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=union
            export DRAGONFRUIT_SHADOW_GPU=1
            ;;
        split-deco)
            # Split regions with both decoration paths: one-quad shadows and
            # one-quad traffic lights, span fill/glyphs.
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=split
            export DRAGONFRUIT_SHADOW_GPU=1
            export DRAGONFRUIT_TITLEBAR_GPU=1
            ;;
        full-deco)
            export DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=union
            export DRAGONFRUIT_SHADOW_GPU=1
            export DRAGONFRUIT_TITLEBAR_GPU=1
            ;;
        *)
            echo "t238-material-workloads: unknown arm '$arm'" >&2
            exit 2
            ;;
    esac
}

run_pair() {
    local arm="$1" workload="$2" trial="$3"
    local dir="$OUT_DIR/$arm/$workload"
    mkdir -p "$dir"
    local suffix=''
    if [ "$TRIALS" -gt 1 ]; then
        suffix="-t$trial"
    fi
    local report="$dir/report$suffix.json"
    local compositor_log="$dir/compositor$suffix.txt" shell_log="$dir/shell$suffix.txt" hang_log="$dir/hang$suffix.txt"
    arm_env "$arm"
    echo "t238-material-workloads: $arm / $workload / trial $trial (duration=${DURATION}s)"
    t21_start_compositor "$BACKEND" "$compositor_log"
    # A nested compositor that stops presenting its window starves its event
    # loop: the synthetic socket and SIGUSR1 are never serviced. Detect the
    # queued datagrams and capture the evidence instead of hanging.
    python3 scripts/benchmark-watchdog.py \
        --pid "$T21_COMPOSITOR_PID" \
        --synth "$T21_SYNTH" \
        --log "$compositor_log" \
        --out "$hang_log" \
        --timeout "$WATCHDOG_TIMEOUT" &
    local watchdog=$!
    t21_start_shell "$shell_log" \
        QT_QPA_PLATFORM=wayland \
        QML_IMPORT_PATH="$T21_ROOT/build/qml" \
        DRAGONFRUIT_CHROME_STATS=1 \
        DRAGONFRUIT_POPUP_TARGETS=independent
    local display_file="$XDG_RUNTIME_DIR/$T21_SOCKET.x11-display"
    for _ in $(seq 1 100); do
        [ -s "$display_file" ] && break
        sleep 0.1
    done
    if [ ! -s "$display_file" ]; then
        echo "t238-material-workloads: no Xwayland DISPLAY hand-off for $arm/$workload; see $compositor_log" >&2
        t21_stop_shell
        t21_stop_compositor
        return 1
    fi
    local display
    display="$(cat "$display_file")"
    local capture_args=()
    if [ -n "$CAPTURE_DIR" ]; then
        mkdir -p "$CAPTURE_DIR"
        capture_args=(--capture "$CAPTURE_DIR/$arm-$workload.png")
    fi
    if [ -n "$CAPTURE_AB" ]; then
        capture_args+=(--capture-ab "$CAPTURE_AB/$arm-$workload")
    fi
    python3 scripts/t238-material-workloads-driver.py run \
        --socket "$T21_SOCKET" \
        --synth "$T21_SYNTH" \
        --display "$display" \
        --shell-pid "$T21_SHELL_PID" \
        --compositor-pid "$T21_COMPOSITOR_PID" \
        --shell-log "$shell_log" \
        --compositor-log "$compositor_log" \
        --out "$report" \
        --label "$arm-$workload" \
        --arm "$arm" \
        --workload "$workload" \
        --duration "$DURATION" \
        --warmup "$WARMUP" \
        --windows "$WINDOWS" \
        --pin-tier "$PIN_TIER" \
        "${capture_args[@]}" \
        --commit "$COMMIT"
    t21_stop_shell
    kill "$watchdog" 2>/dev/null || true
    wait "$watchdog" 2>/dev/null || true
    t21_stop_compositor
    if [ -s "$hang_log" ]; then
        echo "t238-material-workloads: compositor hang in $arm/$workload; see $hang_log" >&2
        return 1
    fi
}

reports=()
# Build the (arm, workload, trial) plan up front so a shuffled order can
# spread correlated drift across configurations.
plan=()
for workload in $WORKLOADS; do
    for arm in $ARMS; do
        for trial in $(seq 1 "$TRIALS"); do
            plan+=("$arm|$workload|$trial")
        done
    done
done
if [ "$SHUFFLE" = "1" ]; then
    mapfile -t plan < <(printf '%s\n' "${plan[@]}" | shuf)
fi
for entry in "${plan[@]}"; do
    IFS='|' read -r arm workload trial <<<"$entry"
    run_pair "$arm" "$workload" "$trial"
    if [ "$TRIALS" -gt 1 ]; then
        reports+=("$OUT_DIR/$arm/$workload/report-t$trial.json")
    else
        reports+=("$OUT_DIR/$arm/$workload/report.json")
    fi
done

if [ -n "$CAPTURE_AB" ]; then
    echo "t238-material-workloads: capture A/B only; no summary" >&2
    exit 0
fi

python3 scripts/t238-material-workloads-driver.py summary "${reports[@]}" \
    | tee "$OUT_DIR/report.md"

echo "t238-material-workloads: wrote $OUT_DIR/report.md" >&2
