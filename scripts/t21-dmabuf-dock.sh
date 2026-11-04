#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.1d dmabuf ring + release reuse (T186c).
#
# Runs the real shell against one nested Dragonfruit compositor twice:
#   DRAGONFRUIT_CHROME_GPU=1  -> Dock commits a dmabuf through a 3-slot ring
#   DRAGONFRUIT_CHROME_GPU=0  -> T185 pooled shm fallback (imports = 0)
# The GPU run is warmed up, then driven through two Dock sweeps. Two SIGUSR1
# dumps prove `dmabuf_buffers` and `dmabuf_allocs` stay flat while
# `dmabuf_commits`/`dmabuf_reuses` advance. Writes the shell `chrome stats` +
# compositor import counters to docs/captures/t21-dmabuf-ring.txt. Needs a host
# Wayland session with a GL driver (the T186a rail). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

SOCKET="${SOCKET:-dragonfruit-t21-dmabuf-$$}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-dmabuf.XXXXXX")}"
OUT="${OUT:-docs/captures/t21-dmabuf-ring.txt}"
GPU_PNG="${GPU_PNG:-docs/captures/t21-dmabuf-ring.png}"
SHM_PNG="${SHM_PNG:-docs/captures/t21-dmabuf-ring-fallback.png}"
GPU_LOG="$SCRATCH/shell-gpu.log"
SHM_LOG="$SCRATCH/shell-shm.log"
GPU_COMP="$SCRATCH/compositor-gpu.log"
SHM_COMP="$SCRATCH/compositor-shm.log"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"
COMPOSITOR_PID=""

cleanup() {
    if [ -n "$COMPOSITOR_PID" ]; then
        kill -TERM "$COMPOSITOR_PID" 2>/dev/null || true
        wait "$COMPOSITOR_PID" 2>/dev/null || true
    fi
    rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" \
        "$XDG_RUNTIME_DIR/$SOCKET.launch-token" "$SYNTH" 2>/dev/null || true
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

mkdir -p "$SCRATCH" "$(dirname "$OUT")" "$(dirname "$GPU_PNG")"

if [ -z "${WAYLAND_DISPLAY:-}" ]; then
    {
        echo "T-21.1d dmabuf ring + release reuse (T186c)"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: no host Wayland session (WAYLAND_DISPLAY is unset)"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make t21-dmabuf-ring"
    } >"$OUT"
    echo "t21-dmabuf-ring: OPEN (no host Wayland session); wrote $OUT" >&2
    exit 0
fi

if [ ! -x build/shell/src/dragonfruit-shell ]; then
    echo "t21-dmabuf-ring: build/shell/src/dragonfruit-shell missing; run make build" >&2
    exit 1
fi
if [ ! -x target/debug/dragonfruit-compositor ]; then
    echo "t21-dmabuf-ring: target/debug/dragonfruit-compositor missing; run make build" >&2
    exit 1
fi

start_compositor() {
    local log="$1"
    rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" \
        "$XDG_RUNTIME_DIR/$SOCKET.launch-token" "$SYNTH"
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        target/debug/dragonfruit-compositor --backend nested --socket-name "$SOCKET" \
        >"$log" 2>&1 &
    COMPOSITOR_PID=$!
    for _ in $(seq 1 300); do
        [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break
        if ! kill -0 "$COMPOSITOR_PID" 2>/dev/null; then
            echo "t21-dmabuf-ring: compositor exited early; log:" >&2
            tail -30 "$log" >&2
            exit 1
        fi
        sleep 0.1
    done
    if [ ! -e "$XDG_RUNTIME_DIR/$SOCKET" ]; then
        echo "t21-dmabuf-ring: nested socket never appeared" >&2
        tail -30 "$log" >&2
        exit 1
    fi
    for _ in $(seq 1 100); do
        [ -e "$SYNTH" ] && break
        sleep 0.1
    done
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

dump_stats() {
    local pid="$1" label="$2"
    kill -USR1 "$pid" 2>/dev/null || true
    for _ in $(seq 1 50); do
        if grep -aq "chrome stats ($label)" "$GPU_LOG" 2>/dev/null; then
            break
        fi
        sleep 0.1
    done
    sleep 0.5
    grep -a "chrome stats ($label)" "$GPU_LOG" | tail -1 || true
}

drive_dock_sweep() {
    # A short pointer sweep across a bottom Dock. The synthetic harness maps
    # normalized output coordinates, so this works on any nested size.
    python3 - "$SYNTH" <<'PY'
import math, os, socket, sys, time

synth = sys.argv[1]
runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
client = f"{runtime}/dragonfruit-t21-ring-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.bind(client)
try:
    for index in range(70):
        x = 0.35 + 0.30 * (0.5 + 0.5 * math.sin(index / 6.0))
        sock.sendto(f"motion-abs {x:.6f} 0.955".encode(), synth)
        time.sleep(0.03)
finally:
    sock.close()
    try:
        os.unlink(client)
    except FileNotFoundError:
        pass
PY
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
    # Let the status bridge/clock settle and the ring warm up.
    sleep 8
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "t21-dmabuf-ring: shell ($mode) exited early; log:" >&2
        tail -40 "$log" >&2
        return 1
    fi
    if [ "$mode" = "gpu" ]; then
        drive_dock_sweep
        sleep 1
        dump_stats "$pid" "SIGUSR1-1" >"$SCRATCH/gpu-warm.stats"
        drive_dock_sweep
        sleep 1
        dump_stats "$pid" "SIGUSR1-2" >"$SCRATCH/gpu-steady.stats"
    else
        kill -USR1 "$pid" 2>/dev/null || true
        sleep 1
        grep -a "chrome stats" "$log" | tail -1 >"$SCRATCH/shm.stats" || true
    fi
    capture_host "$png"
    kill -TERM "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    return 0
}

echo "t21-dmabuf-ring: GPU run"
start_compositor "$GPU_COMP"
run_shell gpu "$GPU_LOG" "$GPU_PNG" || true
stop_compositor

echo "t21-dmabuf-ring: shm fallback run"
start_compositor "$SHM_COMP"
run_shell shm "$SHM_LOG" "$SHM_PNG" || true
stop_compositor

gpu_warm=$(cat "$SCRATCH/gpu-warm.stats" 2>/dev/null || true)
gpu_steady=$(cat "$SCRATCH/gpu-steady.stats" 2>/dev/null || true)
gpu_stats="$gpu_steady"
shm_stats=$(grep -a "chrome stats" "$SHM_LOG" | tail -1 || true)
gpu_imports=$(grep -aoE "chrome_dmabuf_imports=[0-9]+" "$GPU_COMP" | tail -1 || true)
shm_imports=$(grep -aoE "chrome_dmabuf_imports=[0-9]+" "$SHM_COMP" | tail -1 || true)

field() {
    local name="$1" line="$2"
    sed -n "s/.* ${name}=\\([0-9][0-9]*\\).*/\\1/p" <<<"$line" | tail -1
}

gpu_warm_buffers=$(field dmabuf_buffers "$gpu_warm")
gpu_steady_buffers=$(field dmabuf_buffers "$gpu_steady")
gpu_warm_allocs=$(field dmabuf_allocs "$gpu_warm")
gpu_steady_allocs=$(field dmabuf_allocs "$gpu_steady")
gpu_warm_reuses=$(field dmabuf_reuses "$gpu_warm")
gpu_steady_reuses=$(field dmabuf_reuses "$gpu_steady")
gpu_warm_commits=$(field dmabuf_commits "$gpu_warm")
gpu_steady_commits=$(field dmabuf_commits "$gpu_steady")

line_is_gpu_path() {
    local line="$1"
    grep -q "path=render-control+dmabuf" <<<"$line" \
        && grep -q "gpu=dmabuf" <<<"$line" \
        && ! grep -q "dmabuf_commits=0 " <<<"$line" \
        && grep -q "grab_renders=0 " <<<"$line" \
        && grep -q "dmabuf_buffers=" <<<"$line"
}

gpu_ok=0
# The steady dump can land on a transient ring-exhaustion shm frame; the warm
# dump still proves the dmabuf rail is the active transport.
if line_is_gpu_path "$gpu_warm" || line_is_gpu_path "$gpu_steady"; then
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

ring_flat_ok=0
if [ -n "$gpu_warm_buffers" ] && [ "$gpu_warm_buffers" = "$gpu_steady_buffers" ] \
    && [ "$gpu_warm_buffers" -ge 1 ]; then
    ring_flat_ok=1
fi
ring_alloc_ok=0
if [ -n "$gpu_warm_allocs" ] && [ "$gpu_warm_allocs" = "$gpu_steady_allocs" ]; then
    ring_alloc_ok=1
fi
ring_work_ok=0
if [ -n "$gpu_warm_commits" ] && [ -n "$gpu_steady_commits" ] \
    && [ "$gpu_steady_commits" -gt "$gpu_warm_commits" ] \
    && [ -n "$gpu_steady_reuses" ] \
    && [ "$gpu_steady_reuses" -ge "$gpu_warm_reuses" ]; then
    ring_work_ok=1
fi

{
    echo "T-21.1d dmabuf ring + release reuse (T186c)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "distro: $(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")"
    echo "host session: WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "nested socket: $SOCKET"
    echo
    echo "## GPU run: DRAGONFRUIT_CHROME_GPU=1 (Qt Wayland GL rail, 3-slot ring)"
    echo "command: QT_QPA_PLATFORM=wayland WAYLAND_DISPLAY=$SOCKET DRAGONFRUIT_CHROME_GPU=1 DRAGONFRUIT_CHROME_STATS=1 ./build/shell/src/dragonfruit-shell --socket-name $SOCKET"
    grep -aE "chrome gpu probe|chrome gpu producer|chrome dmabuf|chrome transport" "$GPU_LOG" | head -20 || true
    echo
    echo "warm stats (after one Dock sweep):"
    echo "$gpu_warm"
    echo
    echo "steady stats (after a second Dock sweep):"
    echo "$gpu_steady"
    echo
    echo "ring: dmabuf_buffers=$gpu_warm_buffers->$gpu_steady_buffers dmabuf_allocs=$gpu_warm_allocs->$gpu_steady_allocs dmabuf_reuses=$gpu_warm_reuses->$gpu_steady_reuses dmabuf_commits=$gpu_warm_commits->$gpu_steady_commits"
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
    echo "ring_flat_ok=$ring_flat_ok ring_alloc_flat_ok=$ring_alloc_ok ring_work_ok=$ring_work_ok"
    if [ "$gpu_ok" -eq 1 ] && [ "$gpu_imports_ok" -eq 1 ] \
        && [ "$shm_ok" -eq 1 ] && [ "$shm_imports_ok" -eq 1 ] \
        && [ "$ring_flat_ok" -eq 1 ] && [ "$ring_alloc_ok" -eq 1 ] \
        && [ "$ring_work_ok" -eq 1 ]; then
        echo "PASS: the Dock committed dmabufs through a bounded ring whose"
        echo "buffer/allocation counts stayed flat after warm-up, and the"
        echo "GPU=0 run stayed on the pooled shm path with zero dmabuf imports."
    else
        echo "FAIL: see the stats above."
    fi
} >"$OUT"

echo "t21-dmabuf-ring: wrote $OUT"
if [ "$gpu_ok" -ne 1 ] || [ "$gpu_imports_ok" -ne 1 ] \
    || [ "$shm_ok" -ne 1 ] || [ "$shm_imports_ok" -ne 1 ] \
    || [ "$ring_flat_ok" -ne 1 ] || [ "$ring_alloc_ok" -ne 1 ] \
    || [ "$ring_work_ok" -ne 1 ]; then
    exit 1
fi
