#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.3 stable panel/scene IDs and damage evidence (T190c).
#
# Runs the stable-ID/damage unit test, the capture preflight, and one nested
# capture of a stationary Dock with no pointer motion. The compositor's
# `material stats` snapshots are included so the allocation counters can be
# compared with the settled frame.
#
# Outputs:
#   docs/captures/t21-stable-dock-1x.png
#   docs/captures/t21-stable-damage.txt
#
# Requires a host Wayland session and the built tree. Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
LOG="$OUTDIR/t21-stable-damage.txt"
mkdir -p "$OUTDIR"

{
    echo "T-21.3 stable scene/panel IDs and damage — T190c"
    echo "timestamp: $(date -Is)"
    echo
    echo "== stable-ID/damage unit test =="
    cargo test -p dragonfruit-compositor --bin dragonfruit-compositor \
        window::material_cache::tests::a_stationary_panel_with_an_unchanged_scene_produces_no_damage \
        2>&1
    echo
    echo "== make t21-doctor =="
    make t21-doctor 2>&1
    echo
} >"$LOG"

{
    echo "== settled Dock capture, 1x =="
    echo "command: FULL_SESSION=0 SCALE=1 FORCE_TIER=full DUMP_COMPOSITOR_STATS=1 OUT=$OUTDIR/t21-stable-dock-1x.png bash scripts/t21-live-capture.sh settled"
} | tee -a "$LOG"

FULL_SESSION=0 SCALE=1 FORCE_TIER=full DUMP_COMPOSITOR_STATS=1 \
    OUT="$OUTDIR/t21-stable-dock-1x.png" \
    bash scripts/t21-live-capture.sh settled 2>&1 | tee -a "$LOG"

{
    echo
    echo "== artifacts =="
    ls -l "$OUTDIR/t21-stable-dock-1x.png"
    echo
    echo "The still is a settled Dock: no synthetic pointer motion runs before"
    echo "the capture. The unit test above pins the damage contract with a real"
    echo "OutputDamageTracker: a settled panel over an unchanged scene returns"
    echo "no damage, a wallpaper commit damages the full output, and a Dock move"
    echo "damages only the old/new panel rects."
} | tee -a "$LOG"

echo "t21-stable-damage: wrote $LOG"
