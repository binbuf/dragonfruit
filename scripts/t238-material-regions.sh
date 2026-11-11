#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 follow-up: three-way material scene/composite comparison.
#
# Runs the same revision and the same Dock-popover workload three times in the
# nested 144 Hz session:
#
#   union  the current implementation: one scene texture over the padded union
#          of every chrome panel (a top menu bar + bottom Dock spans nearly the
#          whole output)
#   off    material rendering disabled (diagnostic only): no scene texture, no
#          Kawase chain, no glass/veil compose, no feather backdrop
#   split  independently bounded top/bottom material regions
#
# The driver records per-frame CPU stage totals and joins them with the GPU
# timer queries by frame id, so the report separates CPU work, GPU execution,
# and the CPU-minus-GPU gap instead of comparing unrelated percentiles.
#
# PROBE=1 adds `DRAGONFRUIT_GPU_FINISH_PROBE=1`: a diagnostic glFinish after
# each scene/composite pass. That serialises the frame, so use it only to
# answer "how much GPU work was still outstanding when the CPU submitted?"
# and do not compare its absolute latency with the non-probe arms.
#
# Requires a host Wayland session and the built tree (`make build`).
# Overrides: ARMS, PROBE, CYCLES, INTERVAL_MS, SETTLE, OUT_DIR, KEEP_SCRATCH.
set -euo pipefail

cd "$(dirname "$0")/.."

. "$(dirname "$0")/t21-capture-lib.sh"

OUT_DIR="${OUT_DIR:-docs/captures/t238-material-regions}"
ARMS="${ARMS:-union off split full}"
PROBE="${PROBE:-0}"
CYCLES="${CYCLES:-100}"
INTERVAL_MS="${INTERVAL_MS:-300}"
SETTLE="${SETTLE:-1.0}"
CHROME_GPU_SURFACES="${CHROME_GPU_SURFACES:-dock,dock-popup}"
# The compositor is measured on its release profile: a debug build adds
# several milliseconds of CPU per draw call and hides the real GPU/host wait.
# The shell remains the existing debug build; its own trace is reported
# separately.
PROFILE="${PROFILE:-release}"
T21_COMPOSITOR_BIN="${COMPOSITOR_BIN:-$T21_ROOT/target/$PROFILE/dragonfruit-compositor}"
export T21_COMPOSITOR_BIN

t21_capture_init "t238-material-regions"
t21_require_host "$OUT_DIR/open.txt" "T238 material scene/composite comparison"
t21_require_exec "$T21_SHELL_BIN" "run make build"
t21_require_exec "$T21_COMPOSITOR_BIN" "run make build"
command -v python3 >/dev/null 2>&1 || {
    echo "t238-material-regions: python3 not found" >&2
    exit 1
}
command -v xmessage >/dev/null 2>&1 || {
    echo "t238-material-regions: xmessage not found; a grouped X11 entry is required" >&2
    exit 1
}

mkdir -p "$OUT_DIR"
COMMIT="$(git -C "$T21_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"

export DRAGONFRUIT_GPU_TIMING=1
export DRAGONFRUIT_FRAME_TRACE=1
export DRAGONFRUIT_REFRESH_HZ=144
if [ "$PROBE" = "1" ]; then
    export DRAGONFRUIT_GPU_FINISH_PROBE=1
else
    unset DRAGONFRUIT_GPU_FINISH_PROBE
fi

arm_env() {
    local arm="$1"
    unset DRAGONFRUIT_MATERIAL_OFF
    unset DRAGONFRUIT_MATERIAL_FULL_OUTPUT
    unset DRAGONFRUIT_SCENE_CACHE_SKIP
    case "$arm" in
        union)
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=union
            ;;
        off)
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=union
            export DRAGONFRUIT_MATERIAL_OFF=1
            ;;
        split)
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS=split
            ;;
        full)
            # Diagnostic rollback shape: one full-output scene texture used as
            # the output base, so the final composite draws one scene quad
            # instead of the live element list.
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
        union-finish|split-finish)
            export DRAGONFRUIT_MATERIAL_REGION_GROUPS="${arm%-finish}"
            export DRAGONFRUIT_GPU_FINISH_PROBE=1
            ;;
        *)
            echo "t238-material-regions: unknown arm '$arm'" >&2
            exit 2
            ;;
    esac
}

run_arm() {
    local arm="$1"
    local dir="$OUT_DIR/$arm"
    mkdir -p "$dir"
    arm_env "$arm"
    echo "t238-material-regions: $arm (cycles=$CYCLES, probe=$PROBE)"
    t21_start_compositor nested "$dir/compositor.txt"
    t21_start_shell "$dir/shell.txt" \
        QT_QPA_PLATFORM=wayland \
        QML_IMPORT_PATH="$T21_ROOT/build/qml" \
        DRAGONFRUIT_CHROME_STATS=1 \
        DRAGONFRUIT_CHROME_TRACE=1 \
        DRAGONFRUIT_CHROME_GPU=1 \
        DRAGONFRUIT_CHROME_GPU_SURFACES="$CHROME_GPU_SURFACES" \
        DRAGONFRUIT_DOCK_STABLE_POPOVER=0 \
        DF_DOCK_POPOVER_AB="$CYCLES" \
        DF_DOCK_POPOVER_AB_INTERVAL_MS="$INTERVAL_MS"
    python3 scripts/t238-dock-popover-ab-driver.py run \
        --socket "$T21_SOCKET" \
        --shell-pid "$T21_SHELL_PID" \
        --compositor-pid "$T21_COMPOSITOR_PID" \
        --shell-log "$dir/shell.txt" \
        --compositor-log "$dir/compositor.txt" \
        --out "$dir/report.json" \
        --label "$arm" \
        --arm "$arm" \
        --stable 0 \
        --cycles "$CYCLES" \
        --commit "$COMMIT" \
        --settle "$SETTLE"
    t21_stop_shell
    t21_stop_compositor
}

reports=()
for arm in $ARMS; do
    run_arm "$arm"
    reports+=("$OUT_DIR/$arm/report.json")
done

python3 scripts/t238-dock-popover-ab-driver.py summary "${reports[@]}" \
    | tee "$OUT_DIR/report.md"

echo "t238-material-regions: wrote $OUT_DIR/report.md" >&2
