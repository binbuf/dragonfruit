#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.1d dmabuf export + single-buffer Dock commit (T186b).
#
# Runs the real shell against one nested Dragonfruit compositor twice:
#   DRAGONFRUIT_CHROME_GPU=1  -> Dock commits a dmabuf (imports > 0)
#   DRAGONFRUIT_CHROME_GPU=0  -> T185 pooled shm fallback (imports = 0)
# and writes the shell `chrome stats` + compositor import counters to
# docs/captures/t21-dmabuf-dock.txt. Needs a host Wayland session with a GL
# driver (the T186a rail). This is not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

SOCKET="${SOCKET:-dragonfruit-t21-dmabuf-$$}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-dmabuf.XXXXXX")}"
OUT="${OUT:-docs/captures/t21-dmabuf-dock.txt}"
GPU_PNG="${GPU_PNG:-docs/captures/t21-dmabuf-dock.png}"
SHM_PNG="${SHM_PNG:-docs/captures/t21-dmabuf-dock-fallback.png}"
GPU_LOG="$SCRATCH/shell-gpu.log"
SHM_LOG="$SCRATCH/shell-shm.log"
GPU_COMP="$SCRATCH/compositor-gpu.log"
SHM_COMP="$SCRATCH/compositor-shm.log"
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

mkdir -p "$SCRATCH" "$(dirname "$OUT")" "$(dirname "$GPU_PNG")"

if [ -z "${WAYLAND_DISPLAY:-}" ]; then
    {
        echo "T-21.1d dmabuf Dock transport (T186b)"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: no host Wayland session (WAYLAND_DISPLAY is unset)"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make t21-dmabuf-dock"
    } >"$OUT"
    echo "t21-dmabuf-dock: OPEN (no host Wayland session); wrote $OUT" >&2
    exit 0
fi

if [ ! -x build/shell/src/dragonfruit-shell ]; then
    echo "t21-dmabuf-dock: build/shell/src/dragonfruit-shell missing; run make build" >&2
    exit 1
fi
if [ ! -x target/debug/dragonfruit-compositor ]; then
    echo "t21-dmabuf-dock: target/debug/dragonfruit-compositor missing; run make build" >&2
    exit 1
fi

start_compositor() {
    local log="$1"
    rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" \
        "$XDG_RUNTIME_DIR/$SOCKET.launch-token"
    target/debug/dragonfruit-compositor --backend nested --socket-name "$SOCKET" \
        >"$log" 2>&1 &
    COMPOSITOR_PID=$!
    for _ in $(seq 1 300); do
        [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break
        if ! kill -0 "$COMPOSITOR_PID" 2>/dev/null; then
            echo "t21-dmabuf-dock: compositor exited early; log:" >&2
            tail -30 "$log" >&2
            exit 1
        fi
        sleep 0.1
    done
    if [ ! -e "$XDG_RUNTIME_DIR/$SOCKET" ]; then
        echo "t21-dmabuf-dock: nested socket never appeared" >&2
        tail -30 "$log" >&2
        exit 1
    fi
    sleep 1
}

stop_compositor() {
    if [ -z "$COMPOSITOR_PID" ]; then
        return
    fi
    kill -TERM "$COMPOSITOR_PID" 2>/dev/null || true
    for _ in $(seq 1 150); do
        [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] || break
        sleep 0.1
    done
    wait "$COMPOSITOR_PID" 2>/dev/null || true
    COMPOSITOR_PID=""
}

capture_host() {
    local png="$1"
    # Best-effort visual evidence: the nested compositor is a window on the
    # host session, so a host screenshot shows the Dock. A missing capture
    # tool does not invalidate the transport evidence.
    if command -v spectacle >/dev/null 2>&1; then
        WAYLAND_DISPLAY="${HOST_WAYLAND_DISPLAY:-$WAYLAND_DISPLAY}" \
            spectacle -b -n -o "$png" >/dev/null 2>&1 || true
    fi
    if [ ! -s "$png" ] && command -v import >/dev/null 2>&1; then
        import -window root "$png" >/dev/null 2>&1 || true
    fi
}

run_shell() {
    local mode="$1" log="$2" png="$3"
    local env_args=(DRAGONFRUIT_CHROME_STATS=1)
    if [ "$mode" = "gpu" ]; then
        env_args+=(DRAGONFRUIT_CHROME_GPU=1 QT_QPA_PLATFORM=wayland)
    else
        env_args+=(DRAGONFRUIT_CHROME_GPU=0 QT_QPA_PLATFORM=offscreen)
    fi
    env "${env_args[@]}" WAYLAND_DISPLAY="$SOCKET" \
        ./build/shell/src/dragonfruit-shell --socket-name "$SOCKET" \
        >"$log" 2>&1 &
    local pid=$!
    # Let the status bridge/clock settle before the still; the Dock's first
    # dmabuf commit happens much earlier.
    sleep 8
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "t21-dmabuf-dock: shell ($mode) exited early; log:" >&2
        tail -40 "$log" >&2
        return 1
    fi
    capture_host "$png"
    kill -USR1 "$pid" 2>/dev/null || true
    sleep 2
    kill -TERM "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    return 0
}

echo "t21-dmabuf-dock: GPU run"
start_compositor "$GPU_COMP"
run_shell gpu "$GPU_LOG" "$GPU_PNG" || true
stop_compositor

echo "t21-dmabuf-dock: shm fallback run"
start_compositor "$SHM_COMP"
run_shell shm "$SHM_LOG" "$SHM_PNG" || true
stop_compositor

gpu_stats=$(grep -a "chrome stats" "$GPU_LOG" | tail -1 || true)
shm_stats=$(grep -a "chrome stats" "$SHM_LOG" | tail -1 || true)
gpu_imports=$(grep -aoE "chrome_dmabuf_imports=[0-9]+" "$GPU_COMP" | tail -1 || true)
shm_imports=$(grep -aoE "chrome_dmabuf_imports=[0-9]+" "$SHM_COMP" | tail -1 || true)
gpu_shm=$(grep -aoE "chrome_shm_imports=[0-9]+" "$GPU_COMP" | tail -1 || true)
shm_shm=$(grep -aoE "chrome_shm_imports=[0-9]+" "$SHM_COMP" | tail -1 || true)

gpu_ok=0
if grep -q "path=render-control+dmabuf" <<<"$gpu_stats" \
    && grep -q "gpu=dmabuf" <<<"$gpu_stats" \
    && ! grep -q "dmabuf_commits=0 " <<<"$gpu_stats" \
    && grep -q "grab_renders=0 " <<<"$gpu_stats"; then
    gpu_ok=1
fi
shm_ok=0
if grep -q "path=render-control+shm" <<<"$shm_stats" \
    && grep -q "gpu=off" <<<"$shm_stats" \
    && grep -q "dmabuf_commits=0 " <<<"$shm_stats"; then
    shm_ok=1
fi
gpu_imports_ok=0
if [[ "$gpu_imports" =~ chrome_dmabuf_imports=([0-9]+) ]] && [ "${BASH_REMATCH[1]}" -gt 0 ]; then
    gpu_imports_ok=1
fi
shm_imports_ok=0
if [[ "$shm_imports" =~ chrome_dmabuf_imports=([0-9]+) ]] && [ "${BASH_REMATCH[1]}" -eq 0 ]; then
    shm_imports_ok=1
fi

{
    echo "T-21.1d dmabuf export + single-buffer Dock commit (T186b)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "distro: $(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")"
    echo "host session: WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "nested socket: $SOCKET"
    echo
    echo "## GPU run: DRAGONFRUIT_CHROME_GPU=1 (Qt Wayland GL rail)"
    echo "command: QT_QPA_PLATFORM=wayland WAYLAND_DISPLAY=$SOCKET DRAGONFRUIT_CHROME_GPU=1 DRAGONFRUIT_CHROME_STATS=1 ./build/shell/src/dragonfruit-shell --socket-name $SOCKET"
    grep -aE "chrome gpu probe|chrome gpu producer|chrome dmabuf|chrome transport" "$GPU_LOG" | head -20 || true
    echo
    echo "shell stats:"
    echo "$gpu_stats"
    echo
    echo "compositor imports:"
    grep -aE "chrome stats" "$GPU_COMP" | tail -3 || true
    echo
    echo "## Fallback run: DRAGONFRUIT_CHROME_GPU=0 (offscreen QPA, T185 shm)"
    echo "command: QT_QPA_PLATFORM=offscreen WAYLAND_DISPLAY=$SOCKET DRAGONFRUIT_CHROME_GPU=0 DRAGONFRUIT_CHROME_STATS=1 ./build/shell/src/dragonfruit-shell --socket-name $SOCKET"
    grep -aE "chrome gpu probe|chrome gpu producer|chrome transport" "$SHM_LOG" | head -10 || true
    echo
    echo "shell stats:"
    echo "$shm_stats"
    echo
    echo "compositor imports:"
    grep -aE "chrome stats" "$SHM_COMP" | tail -3 || true
    echo
    echo "## Verdict"
    echo "gpu_path_ok=$gpu_ok gpu_dmabuf_imports_ok=$gpu_imports_ok ($gpu_imports)"
    echo "fallback_path_ok=$shm_ok fallback_dmabuf_imports_ok=$shm_imports_ok ($shm_imports)"
    if [ "$gpu_ok" -eq 1 ] && [ "$gpu_imports_ok" -eq 1 ] \
        && [ "$shm_ok" -eq 1 ] && [ "$shm_imports_ok" -eq 1 ]; then
        echo "PASS: the Dock committed a dmabuf through the GPU rail and the"
        echo "GPU=0 run stayed on the pooled shm path with zero dmabuf imports."
    else
        echo "FAIL: see the stats above."
    fi
} >"$OUT"

echo "t21-dmabuf-dock: wrote $OUT"
if [ "$gpu_ok" -ne 1 ] || [ "$gpu_imports_ok" -ne 1 ] \
    || [ "$shm_ok" -ne 1 ] || [ "$shm_imports_ok" -ne 1 ]; then
    exit 1
fi
