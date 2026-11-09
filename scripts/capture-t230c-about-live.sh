#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Capture the real nested system-menu -> About -> More Info journey.
# Reuses T-21's compositor-owned screenshot runner, not host screencopy.
set -euo pipefail
cd "$(dirname "$0")/.."
outdir="${OUTDIR:-docs/captures}"
mkdir -p "$outdir"
for scheme in ${SCHEMES:-light dark}; do
    for surface in ${SURFACES:-panel page}; do
        echo "capture-t230c-about-live: $surface $scheme"
        MODE="$surface" COLOR_SCHEME="$scheme" FULL_SESSION=1 \
            CAPTURE_DELAY=18000 SETTLE=0 \
            PRE_CAPTURE_CMD='python3 scripts/t230c-about-live-driver.py --synth "$SYNTH" --mode "$MODE"' \
            OUT="$outdir/t230c-about-live-$surface-$scheme.png" \
            bash scripts/t21-live-capture.sh settled
    done
done
echo "capture-t230c-about-live: captured requested surfaces"
