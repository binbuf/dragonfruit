#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-03.4 multi-GPU import/fallback validation.
#
# The import/fallback paths (a secondary GPU with its own render node; a
# secondary without one falling back to the primary renderer and linear-only
# import) can only be exercised with two DRM devices. With a single card the
# classification is still recorded, but the hardware validation is **open, not
# skipped** (docs/SLICING-REVIEW.md):
#
#   * more than one card: docs/captures/t03-multigpu.txt
#   * one card / no card: docs/captures/t03-multigpu.open.txt
#
# Always exits 0: a single-GPU host must not fail the pipeline. The pure
# classification is pinned by `compositor/tests/multi_gpu.rs` either way.
#
# Overrides: OUT_MULTI, OUT_OPEN.

set -euo pipefail

cd "$(dirname "$0")/.."

OUT_MULTI="${OUT_MULTI:-docs/captures/t03-multigpu.txt}"
OUT_OPEN="${OUT_OPEN:-docs/captures/t03-multigpu.open.txt}"

command -v python3 >/dev/null 2>&1 || {
    echo "multi-gpu-validation: python3 not found" >&2
    exit 1
}

scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-multigpu.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

probe="$scratch/seat.json"
probe_rc=0
python3 scripts/drm-seat-probe.py >"$probe" || probe_rc=$?

seat_owned=0
[ "$probe_rc" -eq 0 ] && seat_owned=1

seat_detail="$(python3 - "$probe" "$probe_rc" <<'PY'
import json
import sys

report = json.load(open(sys.argv[1]))
rc = int(sys.argv[2])
if report.get("free_card"):
    print(f"free DRM master on {report['free_card']}; the compositor can own the seat")
elif report.get("cards"):
    states = ", ".join(
        f"{c['path']} busy ({c.get('error') or 'master held'})"
        if c.get("opened")
        else f"{c['path']} unopenable ({c.get('error')})"
        for c in report["cards"]
    )
    print(f"no free DRM master ({states})")
elif rc == 2:
    print("no /dev/dri/card* node")
else:
    print("no free DRM master")
PY
)"

probe_out="$scratch/probe.txt"
multiprobe_rc=0
python3 scripts/multi-gpu-probe.py \
    --seat-owned "$seat_owned" \
    --seat-detail "$seat_detail" >"$probe_out" || multiprobe_rc=$?

out="$OUT_OPEN"
[ "$multiprobe_rc" -eq 0 ] && out="$OUT_MULTI"

mkdir -p "$(dirname "$out")"
cp "$probe_out" "$out"
echo "multi-gpu-validation: wrote $out"