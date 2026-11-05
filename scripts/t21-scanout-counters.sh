#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.3 direct-scanout preservation and material counters (T190d).
#
# Records three things in one transcript:
#   1. the pure 4K full-output vs Dock-only region arithmetic (the same
#      `MaterialRegion` helper the backends call),
#   2. a settled nested Dock frame with two material snapshots, asserting
#      `material_allocations` / `material_region_px` are flat after warm-up,
#   3. a settled nested fullscreen client: the menu bar/Dock are hidden, the
#      sampled pass stops (`material_frames` flat, `direct_scanout_candidates`
#      advances), and the still shows the unobstructed client.
#
# The nested backend cannot advance the real `direct_scanouts` counter: only
# the DRM primary plane can. The hardware half is recorded OPEN here and is
# swept by T191a on a free seat; `scripts/t21-direct-scanout-native.sh`
# asserts `direct_scanouts>0` there. Never fake the counter.
#
# Outputs:
#   docs/captures/t21-scanout-counters.txt
#   docs/captures/t21-scanout-settled-1x.png
#   docs/captures/t21-scanout-fullscreen-1x.png
#
# Requires a host Wayland session and the built tree. Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
LOG="$OUTDIR/t21-scanout-counters.txt"
mkdir -p "$OUTDIR"

{
    echo "T-21.3 direct-scanout preservation and material counters — T190d"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "session: WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo
    echo "== make t21-doctor =="
    make t21-doctor 2>&1
    echo
    echo "== direct-scanout policy unit tests =="
    cargo test -p dragonfruit-compositor --bin dragonfruit-compositor \
        window::scanout -- --nocapture 2>&1
    echo
    echo "== 4K full-output vs region arithmetic (MaterialRegion) =="
    cargo test -p dragonfruit-compositor --bin dragonfruit-compositor \
        window::material_region::tests::a_4k_output_reports_the_full_output_and_dock_only_region_areas \
        -- --nocapture 2>&1
    echo
} >"$LOG"

run_capture() {
    local label="$1" state="$2" out="$3" stats="$4"
    {
        echo "== $label =="
        echo "command: FULL_SESSION=0 SCALE=1 FORCE_TIER=full DUMP_COMPOSITOR_STATS=1 ASSERT_MATERIAL_FLAT=1 STATS_OUT=$stats OUT=$out bash scripts/t21-live-capture.sh $state"
    } | tee -a "$LOG"
    FULL_SESSION=0 SCALE=1 FORCE_TIER=full DUMP_COMPOSITOR_STATS=1 \
        ASSERT_MATERIAL_FLAT=1 STATS_OUT="$stats" OUT="$out" \
        bash scripts/t21-live-capture.sh "$state" 2>&1 | tee -a "$LOG"
    echo | tee -a "$LOG"
}

run_capture "settled Dock, 1x: warm-up allocation/region flat check" \
    settled "$OUTDIR/t21-scanout-settled-1x.png" \
    "$OUTDIR/t21-scanout-settled-1x-stats.txt"
run_capture "fullscreen client, 1x: skip the sampled pass" \
    fullscreen "$OUTDIR/t21-scanout-fullscreen-1x.png" \
    "$OUTDIR/t21-scanout-fullscreen-1x-stats.txt"

{
    echo "== direct-scanout policy assertion =="
    python3 - "$OUTDIR/t21-scanout-fullscreen-1x-stats.txt" <<'PY'
import re
import sys

text = open(sys.argv[1]).read()
groups = [group for group in text.split("snapshot") if group.strip()]
if len(groups) < 2:
    raise SystemExit("t21-scanout: expected two fullscreen snapshots")


def field(group, key):
    match = re.search(rf"{key}=(\d+)", group)
    if not match:
        raise SystemExit(f"t21-scanout: missing {key} in snapshot {group!r}")
    return int(match.group(1))


before, after = groups[-2], groups[-1]
candidates = field(after, "direct_scanout_candidates")
material_before = field(before, "material_frames")
material_after = field(after, "material_frames")
alloc_before = field(before, "material_allocations")
alloc_after = field(after, "material_allocations")
region_before = field(before, "material_region_px")
region_after = field(after, "material_region_px")
direct = field(after, "direct_scanouts")
if candidates <= 0:
    raise SystemExit("t21-scanout: direct_scanout_candidates did not advance")
if material_after != material_before:
    raise SystemExit(
        f"t21-scanout: material_frames advanced in fullscreen: {material_before} -> {material_after}"
    )
if alloc_after != alloc_before or region_after != region_before:
    raise SystemExit(
        "t21-scanout: allocations/region changed in fullscreen: "
        f"{alloc_before}/{region_before} -> {alloc_after}/{region_after}"
    )
print(
    "t21-scanout: PASS fullscreen policy: "
    f"direct_scanout_candidates={candidates} material_frames={material_after} "
    f"(flat) material_allocations={alloc_after} (flat) "
    f"material_region_px={region_after} (flat)"
)
print(
    "t21-scanout: nested direct_scanouts="
    f"{direct} (nested has no primary plane; the real counter is DRM-only)"
)
PY
    echo
    echo "== native direct-scanout (DRM primary plane) =="
    if [ -x target/debug/dragonfruit-compositor ]; then
        if python3 scripts/drm-seat-probe.py >/dev/null 2>&1; then
            echo "A free DRM seat is available: run"
            echo "  make t21-direct-scanout-native"
        else
            echo "OPEN: no free logind seat on this host (the live session holds DRM master)."
            echo "The nested trace above proves the policy and the skipped pass; the real"
            echo "direct_scanouts>0 trace is swept by T191a on a free seat."
            echo "Reproduce on a free seat:"
            echo "  make t21-direct-scanout-native"
        fi
    else
        echo "OPEN: target/debug/dragonfruit-compositor is not built"
    fi
    echo
    echo "== artifacts =="
    ls -l "$OUTDIR"/t21-scanout-*.png "$OUTDIR"/t21-scanout-*-stats.txt
    echo
    echo "The fullscreen still is the compositor's own render with the menu bar"
    echo "and Dock hidden; the stats snapshots show the sampled pass stopped."
    echo "The 4K arithmetic is the helper's output, not a native screenshot."
} | tee -a "$LOG"

echo "t21-scanout-counters: wrote $LOG"
