#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.1d GPU chrome capability probe and render-control spike (T186a).
#
# Starts one nested Dragonfruit compositor, runs the GL smoke client against
# that socket (QQuickRenderControl -> reusable FBO, DPR 2), and then runs the
# same producer on the offscreen QPA to prove the no-GL fallback records a
# reason instead of producing a black frame. Writes:
#   docs/captures/t21-gpu-smoke.png  - one rendered QML scene at 320x180 (DPR 2)
#   docs/captures/t21-gpu-probe.txt  - probe verdict, formats, pixels, fallback
#
# This is not part of `make e2e`: it needs a host Wayland session with a GL
# driver. When there is no host Wayland session the artifact is written as an
# explicit OPEN marker, never silently skipped.
set -euo pipefail

cd "$(dirname "$0")/.."

SOCKET="${SOCKET:-dragonfruit-t21-gpu-$$}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-gpu.XXXXXX")}"
OUT="${OUT:-docs/captures/t21-gpu-probe.txt}"
PNG="${PNG:-docs/captures/t21-gpu-smoke.png}"
SMOKE_LOG="$SCRATCH/smoke.log"
FALLBACK_LOG="$SCRATCH/fallback.log"
COMPOSITOR_LOG="$SCRATCH/compositor.log"
COMPOSITOR_PID=""

cleanup() {
    if [ -n "$COMPOSITOR_PID" ]; then
        kill -TERM "$COMPOSITOR_PID" 2>/dev/null || true
        wait "$COMPOSITOR_PID" 2>/dev/null || true
    fi
    rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" \
        "$XDG_RUNTIME_DIR/$SOCKET.launch-token" 2>/dev/null || true
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

mkdir -p "$SCRATCH" "$(dirname "$OUT")" "$(dirname "$PNG")"

if [ -z "${WAYLAND_DISPLAY:-}" ]; then
    {
        echo "T-21.1d GPU chrome probe (T186a)"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: no host Wayland session (WAYLAND_DISPLAY is unset)"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make t21-gpu-probe"
    } >"$OUT"
    echo "t21-gpu-probe: OPEN (no host Wayland session); wrote $OUT" >&2
    exit 0
fi

if [ ! -x build/shell/tests/t21-gpu-smoke ]; then
    echo "t21-gpu-probe: build/shell/tests/t21-gpu-smoke missing; run make build" >&2
    exit 1
fi

echo "t21-gpu-probe: starting nested compositor ($SOCKET)"
target/debug/dragonfruit-compositor --backend nested --socket-name "$SOCKET" \
    >"$COMPOSITOR_LOG" 2>&1 &
COMPOSITOR_PID=$!

for _ in $(seq 1 300); do
    [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break
    if ! kill -0 "$COMPOSITOR_PID" 2>/dev/null; then
        echo "t21-gpu-probe: compositor exited early; log:" >&2
        tail -30 "$COMPOSITOR_LOG" >&2
        exit 1
    fi
    sleep 0.1
done
if [ ! -e "$XDG_RUNTIME_DIR/$SOCKET" ]; then
    echo "t21-gpu-probe: nested socket never appeared" >&2
    tail -30 "$COMPOSITOR_LOG" >&2
    exit 1
fi
# Give the compositor one dispatch beat to finish advertising globals.
sleep 1

echo "t21-gpu-probe: running the GL render-control smoke (DPR 2)"
set +e
QT_QPA_PLATFORM=wayland \
WAYLAND_DISPLAY="$SOCKET" \
DRAGONFRUIT_CHROME_GPU=1 \
    ./build/shell/tests/t21-gpu-smoke --out "$PNG" --dpr 2 \
    >"$SMOKE_LOG" 2>&1
SMOKE_STATUS=$?
set -e

echo "t21-gpu-probe: running the offscreen no-GL fallback"
set +e
QT_QPA_PLATFORM=offscreen \
WAYLAND_DISPLAY="$SOCKET" \
DRAGONFRUIT_CHROME_GPU=1 \
    ./build/shell/tests/t21-gpu-smoke --expect-fallback --probe-only \
    --out "$SCRATCH/fallback.png" --dpr 2 \
    >"$FALLBACK_LOG" 2>&1
FALLBACK_STATUS=$?
set -e

# Stop the compositor directly so its exit stats land in the log.
kill -TERM "$COMPOSITOR_PID" 2>/dev/null || true
for _ in $(seq 1 150); do
    [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] || break
    sleep 0.1
done
wait "$COMPOSITOR_PID" 2>/dev/null || true
COMPOSITOR_PID=""

{
    echo "T-21.1d GPU chrome capability probe (T186a)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "distro: $(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")"
    echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "nested socket: $SOCKET"
    echo "smoke command: QT_QPA_PLATFORM=wayland WAYLAND_DISPLAY=$SOCKET DRAGONFRUIT_CHROME_GPU=1 build/shell/tests/t21-gpu-smoke --out $PNG --dpr 2"
    echo
    echo "## Capability probe and render-control spike (nested GL)"
    cat "$SMOKE_LOG"
    echo
    echo "## Clean no-GL fallback (offscreen QPA, same compositor)"
    cat "$FALLBACK_LOG"
    echo
    echo "## Compositor dmabuf advertisement / exit stats"
    grep -aE "dmabuf|chrome stats|frame timing|render stats" "$COMPOSITOR_LOG" | tail -20 \
        || echo "(no matching compositor lines)"
    echo
    echo "## Verdict"
    if [ "$SMOKE_STATUS" -eq 0 ] && [ "$FALLBACK_STATUS" -eq 0 ]; then
        echo "PASS: nested GL render-control frame verified at physical resolution;"
        echo "the offscreen rail recorded a clean fallback reason."
    else
        echo "FAIL: smoke=$SMOKE_STATUS fallback=$FALLBACK_STATUS"
    fi
} >"$OUT"

echo "t21-gpu-probe: wrote $OUT and $PNG"
if [ "$SMOKE_STATUS" -ne 0 ] || [ "$FALLBACK_STATUS" -ne 0 ]; then
    exit 1
fi
