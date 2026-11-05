#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.3 region-sized scene render evidence.
#
# Runs the nested demo at 1x/1.5x/2x and captures the Dock sweep with the
# compositor's own screenshot path. The 1.5x run also opens the menu-bar menu
# so the menu panel is represented, and a 1x full-output diagnostic run proves
# the rollback switch still renders. Each capture dumps the compositor's
# `material stats` line while the frame's chrome is mapped.
#
# Outputs:
#   docs/captures/t21-region-motion-{1x,1_5x,2x}.png
#   docs/captures/t21-region-menu-1_5x.png
#   docs/captures/t21-region-motion-1x-full.png
#   docs/captures/t21-region-render.txt
#
# Requires a host Wayland session and the built tree. Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
LOG="$OUTDIR/t21-region-render.txt"
mkdir -p "$OUTDIR"

{
    echo "T-21.3 region-sized scene render — nested 1x/1.5x/2x"
    echo "timestamp: $(date -Is)"
    echo
    echo "== make t21-doctor =="
    make t21-doctor 2>&1
    echo
} >"$LOG"

run_capture() {
    local label="$1" scale="$2" state="$3" out="$4" full="${5:-0}"
    # Scale 1 does not need settingsd; the thin demo is lighter and gives the
    # 1920x1200 nested window when the host allows it. Scales above 1 need the
    # full session so settingsd can carry display.scale.
    local full_session=1
    if [ "$scale" = "1" ]; then
        full_session=0
    fi
    local -a env_args=(
        FULL_SESSION="$full_session"
        SCALE="$scale"
        OUT="$out"
        DUMP_COMPOSITOR_STATS=1
        FORCE_TIER=full
    )
    if [ "$full" = "1" ]; then
        env_args+=(DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1)
    fi
    local full_env=""
    if [ "$full" = "1" ]; then
        full_env="DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1 "
    fi
    {
        echo "== $label (scale $scale, $state, full_output=$full) =="
        echo "command: FULL_SESSION=$full_session SCALE=$scale FORCE_TIER=full OUT=$out ${full_env}bash scripts/t21-live-capture.sh $state"
    } | tee -a "$LOG"
    env "${env_args[@]}" bash scripts/t21-live-capture.sh "$state" 2>&1 | tee -a "$LOG"
    echo | tee -a "$LOG"
}

run_capture "Dock sweep, 1x" 1 motion "$OUTDIR/t21-region-motion-1x.png"
run_capture "Dock sweep, 1.5x" 1.5 motion "$OUTDIR/t21-region-motion-1_5x.png"
run_capture "Dock sweep, 2x" 2 motion "$OUTDIR/t21-region-motion-2x.png"
run_capture "Menu bar, 1.5x" 1.5 menu "$OUTDIR/t21-region-menu-1_5x.png"
run_capture "Dock sweep, 1x full-output diagnostic" 1 motion \
    "$OUTDIR/t21-region-motion-1x-full.png" 1

{
    echo "== artifacts =="
    ls -l "$OUTDIR"/t21-region-*.png
    echo
    echo "The nested window size follows the host, so the stats line above is"
    echo "the authority for the physical output and the material region. Each"
    echo "run dumps two material snapshots one second apart: the allocation"
    echo "counts are the warm-up check. The region is the padded panel union;"
    echo "with both the top menu bar and the bottom Dock blurred that union can"
    echo "be the full output, while a Dock-only frame is smaller (the"
    echo "material_region unit tests pin the single-panel case)."
} | tee -a "$LOG"

echo "t21-region-render: wrote $LOG"
