#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-03.3 hardware input validation matrix.
#
# Real-device validation needs the compositor to own a libinput seat. On a
# workstation already running a desktop the host compositor holds the seat, so
# the compositor cannot open the event nodes; the script then records an
# explicit open matrix (the unit is marked open, not skipped — see
# docs/SLICING-REVIEW.md):
#
#   * with a free seat:  docs/captures/t03-input-matrix.txt
#   * with no free seat: docs/captures/t03-input-matrix.open.txt
#
# Always exits 0: a no-seat host must not fail the pipeline. Re-run
# `make input-validation` on a machine with a free seat / spare GPU / clean VM.
#
# Overrides: OUT_MATRIX, OUT_OPEN.

set -euo pipefail

cd "$(dirname "$0")/.."

OUT_MATRIX="${OUT_MATRIX:-docs/captures/t03-input-matrix.txt}"
OUT_OPEN="${OUT_OPEN:-docs/captures/t03-input-matrix.open.txt}"

command -v python3 >/dev/null 2>&1 || {
    echo "input-validation: python3 not found" >&2
    exit 1
}

scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-input.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

probe="$scratch/seat.json"
probe_rc=0
python3 scripts/drm-seat-probe.py >"$probe" || probe_rc=$?

# Exit 0 from the probe means a card accepted (and released) DRM master — the
# same precondition a libinput seat needs (T-03.2).
seat_owned=0
[ "$probe_rc" -eq 0 ] && seat_owned=1

seat_detail="$(python3 - "$probe" <<'PY'
import json
import sys

try:
    report = json.load(open(sys.argv[1]))
except Exception:
    report = {"cards": [], "free_card": None}

card = report.get("free_card")
if card:
    print(f"free DRM master on {card}; the compositor can own the libinput seat")
elif report.get("cards"):
    states = ", ".join(
        f"{c['path']} busy ({c.get('error') or 'master held'})"
        if c.get("opened")
        else f"{c['path']} unopenable ({c.get('error')})"
        for c in report["cards"]
    )
    print(f"no free DRM master ({states})")
else:
    print("no /dev/dri/card* node")
PY
)"

out="$OUT_OPEN"
if [ "$seat_owned" -eq 1 ]; then
    out="$OUT_MATRIX"
fi

python3 scripts/input-validation-probe.py \
    --seat-owned "$seat_owned" \
    --seat-detail "$seat_detail" >"$out"

echo "input-validation: wrote $out"