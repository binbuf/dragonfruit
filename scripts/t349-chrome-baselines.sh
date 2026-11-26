#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T349 shared-memory and nested-GPU popup baselines.
#
# Runs the real Dock window-chooser path through the existing T238 A/B driver
# twice (shared-memory and nested GPU) at 1x, plus a 2x GPU control. The T238
# report carries the surface counters; this script records the backend, scale,
# requested cadence, surface family, sample count, and exact invocation beside
# it. Writes docs/captures/stability2/t349-baselines/.
set -euo pipefail

cd "$(dirname "$0")/.."

# The dev toolchain's lib64 carries the DRM/nested runtime (libseat, ...).
TOOLCHAIN="${DF_TOOLCHAIN:-$HOME/.local/df-toolchain/usr}"
if [ -d "$TOOLCHAIN/lib64" ]; then
    export LD_LIBRARY_PATH="$TOOLCHAIN/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

OUT_DIR="${OUT_DIR:-docs/captures/stability2/t349-baselines}"
CYCLES="${CYCLES:-12}"
INTERVAL_MS="${INTERVAL_MS:-300}"
REFRESH_HZ="${DRAGONFRUIT_REFRESH_HZ:-144}"
SURFACES="${CHROME_GPU_SURFACES:-dock,dock-popup}"
mkdir -p "$OUT_DIR"

write_open() {
    local reason="$1"
    {
        echo "T349 shared-memory and nested-GPU popup baselines"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: ${reason}"
        echo "requested_cadence_hz: ${REFRESH_HZ}"
        echo "surface_family: Dock window chooser (dock + dock-popup)"
        echo "sample_count: 0"
        echo "smoke_command: WAYLAND_DISPLAY=<host-socket> DRAGONFRUIT_REFRESH_HZ=${REFRESH_HZ} make t349-chrome-baselines"
        echo "direct_command: CYCLES=${CYCLES} CHROME_GPU=<0|1> T238_MODES=shared OUT_DIR=docs/captures/stability2/t349-baselines/<shm|gpu> scripts/t238-dock-popover-ab.sh"
    } >"$OUT_DIR/OPEN.txt"
    echo "t349-chrome-baselines: OPEN (${reason}); wrote $OUT_DIR/OPEN.txt" >&2
}

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    write_open "XDG_RUNTIME_DIR is not set"
    exit 0
fi
if [ -z "${WAYLAND_DISPLAY:-}" ]; then
    write_open "no host Wayland session (WAYLAND_DISPLAY is unset)"
    exit 0
fi
if ! command -v xmessage >/dev/null 2>&1; then
    write_open "xmessage is missing; the T238 grouped X11 fixture needs it"
    exit 0
fi

run_one() {
    local name="$1" gpu="$2" scale="$3"
    local dir="$OUT_DIR/$name"
    local log="$dir/run.txt"
    mkdir -p "$dir"
    {
        echo "T349 baseline: $name"
        echo "backend: $([ "$gpu" = 1 ] && echo dmabuf || echo shm)"
        echo "scale: ${scale}"
        echo "requested_cadence_hz: ${REFRESH_HZ}"
        echo "surface_family: Dock window chooser (dock + dock-popup)"
        echo "sample_count: ${CYCLES} requested openings"
        echo "invocation: CYCLES=${CYCLES} INTERVAL_MS=${INTERVAL_MS} CHROME_GPU=${gpu} T238_MODES=shared OUT_DIR=${dir} DRAGONFRUIT_REFRESH_HZ=${REFRESH_HZ} DRAGONFRUIT_NESTED_SCALE=${scale} scripts/t238-dock-popover-ab.sh"
        echo
    } >"$log"
    CYCLES="$CYCLES" \
    INTERVAL_MS="$INTERVAL_MS" \
    CHROME_GPU="$gpu" \
    CHROME_GPU_SURFACES="$SURFACES" \
    T238_MODES=shared \
    OUT_DIR="$dir" \
    DRAGONFRUIT_REFRESH_HZ="$REFRESH_HZ" \
    DRAGONFRUIT_NESTED_SCALE="$scale" \
        bash scripts/t238-dock-popover-ab.sh >>"$log" 2>&1
}

run_one shm-1x 0 1
run_one gpu-1x 1 1
run_one gpu-2x 1 2

{
    echo "T349 shared-memory and nested-GPU popup baselines"
    echo "timestamp: $(date -Is)"
    echo "requested_cadence_hz: ${REFRESH_HZ}"
    echo "surface_family: Dock window chooser (dock + dock-popup)"
    echo "sample_count: ${CYCLES} requested openings per run"
    echo
    echo "| run | backend | scale | report |"
    echo "| --- | --- | ---: | --- |"
    for run in shm-1x gpu-1x gpu-2x; do
        backend="$(grep -m1 '^backend:' "$OUT_DIR/$run/run.txt" | sed 's/^backend: //')"
        scale="$(grep -m1 '^scale:' "$OUT_DIR/$run/run.txt" | sed 's/^scale: //')"
        echo "| $run | $backend | $scale | [$run/report.md]($run/report.md) |"
    done
    echo
    echo "Raw invocation and the T238 comparison table are in each run directory."
} >"$OUT_DIR/report.md"

echo "t349-chrome-baselines: wrote $OUT_DIR/report.md"
