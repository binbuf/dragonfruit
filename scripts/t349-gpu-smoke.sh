#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T349 GPU chrome ring/alpha smoke.
#
# Runs the shipped GpuChromeFrameProducer through the real QQuickRenderControl
# path in one nested compositor:
#   * three distinct painted ring slots,
#   * reuse of the two older slots with an unchanged scene and a 1x1 logical
#     damage report,
#   * a full-frame comparison and a sample outside the damage box,
#   * premultiplied alpha (transparent corner + 50% card) at 1x, 1.5x, 2x.
#
# Writes docs/captures/stability2/t349-gpu-smoke.txt plus one PNG per scale.
# When there is no host Wayland/GL session the artifact is an explicit OPEN
# row; the deterministic `make verify-chrome-stability` gate still runs.
set -euo pipefail

cd "$(dirname "$0")/.."

# The dev toolchain's lib64 carries the DRM/nested runtime (libseat, ...).
TOOLCHAIN="${DF_TOOLCHAIN:-$HOME/.local/df-toolchain/usr}"
if [ -d "$TOOLCHAIN/lib64" ]; then
    export LD_LIBRARY_PATH="$TOOLCHAIN/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

. "$(dirname "$0")/t21-capture-lib.sh"

OUT_DIR="${OUT_DIR:-docs/captures/stability2}"
OUT="${OUT_DIR}/t349-gpu-smoke.txt"
REFRESH_HZ="${DRAGONFRUIT_REFRESH_HZ:-144}"
mkdir -p "$OUT_DIR"

t21_capture_init "t349-gpu-smoke"

write_open() {
    local reason="$1"
    {
        echo "T349 GPU chrome ring/alpha smoke"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: ${reason}"
        echo "backend: nested (not run)"
        echo "requested_cadence_hz: ${REFRESH_HZ}"
        echo "surface_family: chrome primary view (three ring slots)"
        echo "sample_count: 0"
        echo "smoke_command: WAYLAND_DISPLAY=<host-socket> DRAGONFRUIT_REFRESH_HZ=${REFRESH_HZ} make t349-gpu-smoke"
        echo "direct_command: QT_QPA_PLATFORM=wayland WAYLAND_DISPLAY=<nested-socket> DRAGONFRUIT_CHROME_GPU=1 build/shell/tests/t21-gpu-smoke --ring-check --dpr <1|1.5|2> --out docs/captures/stability2/t349-gpu-ring-<scale>x.png"
    } >"$OUT"
    echo "t349-gpu-smoke: OPEN (${reason}); wrote $OUT" >&2
}

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    write_open "XDG_RUNTIME_DIR is not set"
    exit 0
fi
if [ -z "${WAYLAND_DISPLAY:-}" ]; then
    write_open "no host Wayland session (WAYLAND_DISPLAY is unset)"
    exit 0
fi
t21_require_exec "$T21_ROOT/build/shell/tests/t21-gpu-smoke" "run make build"
t21_require_exec "$T21_ROOT/target/debug/dragonfruit-compositor" "run make build"

export DRAGONFRUIT_REFRESH_HZ="$REFRESH_HZ"
t21_start_compositor nested "$T21_SCRATCH/compositor.txt"

RING_STATUS=0
for dpr in 1 1.5 2; do
    log="$T21_SCRATCH/ring-${dpr}.log"
    set +e
    QT_QPA_PLATFORM=wayland \
    WAYLAND_DISPLAY="$T21_SOCKET" \
    DRAGONFRUIT_CHROME_GPU=1 \
        "$T21_ROOT/build/shell/tests/t21-gpu-smoke" --ring-check \
        --dpr "$dpr" --out "$OUT_DIR/t349-gpu-ring-${dpr}x.png" \
        >"$log" 2>&1
    status=$?
    set -e
    [ "$status" -eq 0 ] || RING_STATUS=$status
done

FALLBACK_LOG="$T21_SCRATCH/fallback.log"
set +e
QT_QPA_PLATFORM=offscreen \
WAYLAND_DISPLAY="$T21_SOCKET" \
DRAGONFRUIT_CHROME_GPU=1 \
    "$T21_ROOT/build/shell/tests/t21-gpu-smoke" --expect-fallback --probe-only \
    --dpr 2 --out "$T21_SCRATCH/fallback.png" >"$FALLBACK_LOG" 2>&1
FALLBACK_STATUS=$?
set -e

t21_stop_compositor

{
    echo "T349 GPU chrome ring/alpha smoke"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "distro: $(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")"
    echo "backend: nested Wayland/GL"
    echo "requested_cadence_hz: ${REFRESH_HZ}"
    echo "surface_family: chrome primary view (three ring slots)"
    echo "sample_count: 3 painted slots + 2 partial-damage reuses per scale"
    echo "smoke_command: WAYLAND_DISPLAY=<host-socket> DRAGONFRUIT_REFRESH_HZ=${REFRESH_HZ} make t349-gpu-smoke"
    for dpr in 1 1.5 2; do
        echo "ring_command: QT_QPA_PLATFORM=wayland WAYLAND_DISPLAY=<nested-socket> DRAGONFRUIT_CHROME_GPU=1 build/shell/tests/t21-gpu-smoke --ring-check --dpr ${dpr} --out ${OUT_DIR}/t349-gpu-ring-${dpr}x.png"
    done
    echo
    for dpr in 1 1.5 2; do
        echo "## scale ${dpr}x"
        cat "$T21_SCRATCH/ring-${dpr}.log"
        echo
    done
    echo "## offscreen no-GL fallback"
    cat "$FALLBACK_LOG"
    echo
    echo "## Verdict"
    if [ "$RING_STATUS" -eq 0 ] && [ "$FALLBACK_STATUS" -eq 0 ]; then
        echo "PASS: three slots painted, two older slots reused with partial damage,"
        echo "full-frame and outside-damage pixels match, and premultiplied alpha"
        echo "survives at 1x/1.5x/2x."
    else
        echo "FAIL: ring=$RING_STATUS fallback=$FALLBACK_STATUS"
    fi
} >"$OUT"

echo "t349-gpu-smoke: wrote $OUT"
if [ "$RING_STATUS" -ne 0 ] || [ "$FALLBACK_STATUS" -ne 0 ]; then
    exit 1
fi
