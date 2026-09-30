#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-20.4 live visual check: the material at each degrade tier.
#
# Runs the nested demo with the Applications drawer open over the live scene
# (`DF_APPS_DRAWER_FIXTURE=1`) and captures the same card with the tier forced
# to Full, Reduced, and Minimal through the compositor's synthetic-input
# harness. The three stills show the tier stepping down (Full = edge lens +
# specular rim + adaptive tint; Reduced = specular + tint, no lens; Minimal =
# the deterministic flat feather fallback):
#
#   * docs/captures/t20-material-tiers-{full,reduced,minimal}.png
#
# Not part of `make e2e`: CI has no host session and no screenshot tool.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-16}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t20-tiers.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t20-tiers}"

mkdir -p "$OUTDIR" "$SCRATCH"

[ -n "${XDG_RUNTIME_DIR:-}" ] || {
    echo "capture-t20-material-tiers: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
}
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t20-material-tiers: $tool not found" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t20-material-tiers: python3 Pillow not found" >&2
    exit 1
}

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"

cleanup() {
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

rm -f "$SYNTH"
LOG="$SCRATCH/demo.log"
echo "capture-t20-material-tiers: starting the nested demo (socket $SOCKET)"
env DF_APPS_DRAWER_FIXTURE=1 DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid ./target/debug/dragonfruit dev --demo --socket-name "$SOCKET" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-t20-material-tiers: synthetic socket never appeared; demo log:" >&2
    tail -40 "$LOG" >&2
    exit 1
fi
sleep "$SETTLE"

python3 scripts/t20-liquid-glass-driver.py --synth "$SYNTH" --outdir "$OUTDIR" \
    --steps "cmd:set degrade-tier full; sleep:1.5; shot:t20-material-tiers-full"
python3 scripts/t20-liquid-glass-driver.py --synth "$SYNTH" --outdir "$OUTDIR" \
    --steps "cmd:set degrade-tier reduced; sleep:1.5; shot:t20-material-tiers-reduced"
python3 scripts/t20-liquid-glass-driver.py --synth "$SYNTH" --outdir "$OUTDIR" \
    --steps "cmd:set degrade-tier minimal; sleep:1.5; shot:t20-material-tiers-minimal"

echo "capture-t20-material-tiers: done"
echo "  ${OUTDIR}/t20-material-tiers-{full,reduced,minimal}.png"