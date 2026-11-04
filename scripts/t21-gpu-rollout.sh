#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.1d GPU chrome rollout (T186d).
#
# Runs the real shell against one nested Dragonfruit compositor per scenario
# and records the transport that each chrome surface actually used:
#
#   dock      resting Dock after a slow pointer sweep
#   menubar   the system dropdown open
#   popover   the Dock window chooser open
#   lock      the first-party lock surface
#   fallback  DRAGONFRUIT_CHROME_GPU=0 (T185 pooled shm) with the menu open
#
# Each run takes a trusted compositor still (no screencopy client) and one
# SIGUSR1 `chrome stats` dump with the per-surface `path=`/`copy_bytes=` lines.
# Writes docs/captures/t21-gpu-rollout.txt plus the five PNGs. Needs a host
# Wayland session with a GL driver. Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

SOCKET="${SOCKET:-dragonfruit-t21-rollout-$$}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-rollout.XXXXXX")}"
OUT="${OUT:-docs/captures/t21-gpu-rollout.txt}"
SHOTS="${SHOTS:-docs/captures}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"
COMPOSITOR_PID=""
SHELL_PID=""
X11_PIDS=""

mkdir -p "$SCRATCH" "$SHOTS"

cleanup() {
    if [ -n "$SHELL_PID" ]; then
        kill -TERM "$SHELL_PID" 2>/dev/null || true
        wait "$SHELL_PID" 2>/dev/null || true
    fi
    for pid in $X11_PIDS; do
        kill -TERM "$pid" 2>/dev/null || true
    done
    if [ -n "$COMPOSITOR_PID" ]; then
        kill -TERM "$COMPOSITOR_PID" 2>/dev/null || true
        wait "$COMPOSITOR_PID" 2>/dev/null || true
    fi
    rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" \
        "$XDG_RUNTIME_DIR/$SOCKET.launch-token" "$SYNTH" 2>/dev/null || true
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

if [ -z "${WAYLAND_DISPLAY:-}" ]; then
    {
        echo "T-21.1d GPU chrome rollout (T186d)"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: no host Wayland session (WAYLAND_DISPLAY is unset)"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make t21-gpu-rollout"
    } >"$OUT"
    echo "t21-gpu-rollout: OPEN (no host Wayland session); wrote $OUT" >&2
    exit 0
fi

if [ ! -x build/shell/src/dragonfruit-shell ]; then
    echo "t21-gpu-rollout: build/shell/src/dragonfruit-shell missing; run make build" >&2
    exit 1
fi
if [ ! -x target/debug/dragonfruit-compositor ]; then
    echo "t21-gpu-rollout: target/debug/dragonfruit-compositor missing; run make build" >&2
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
            echo "t21-gpu-rollout: compositor exited early; log:" >&2
            tail -30 "$log" >&2
            exit 1
        fi
        sleep 0.1
    done
    if [ ! -e "$XDG_RUNTIME_DIR/$SOCKET" ]; then
        echo "t21-gpu-rollout: nested socket never appeared" >&2
        tail -30 "$log" >&2
        exit 1
    fi
    for _ in $(seq 1 100); do
        [ -e "$SYNTH" ] && break
        sleep 0.1
    done
    sleep 0.5
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

drive_dock_sweep() {
    python3 - "$SYNTH" <<'PY'
import math, os, socket, sys, time

synth = sys.argv[1]
runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
client = f"{runtime}/dragonfruit-t21-rollout-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.bind(client)
try:
    for index in range(80):
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

spawn_grouped_x11() {
    local display_file="$XDG_RUNTIME_DIR/$SOCKET.x11-display"
    for _ in $(seq 1 100); do [ -f "$display_file" ] && break; sleep 0.1; done
    [ -f "$display_file" ] || return 0
    local display
    display="$(cat "$display_file")"
    DISPLAY="$display" xmessage -geometry 300x150+80+120 \
        -title "T21d popover A" "T21d popover A" >/dev/null 2>&1 &
    X11_PIDS="$X11_PIDS $!"
    DISPLAY="$display" xmessage -geometry 300x150+410+120 \
        -title "T21d popover B" "T21d popover B" >/dev/null 2>&1 &
    X11_PIDS="$X11_PIDS $!"
    sleep 2
}

dump_stats() {
    local pid="$1" log="$2" label="$3"
    kill -USR1 "$pid" 2>/dev/null || true
    for _ in $(seq 1 80); do
        grep -aq "chrome stats ($label)" "$log" 2>/dev/null && break
        sleep 0.1
    done
    sleep 0.4
    grep -a "chrome stats ($label)" "$log" | tail -1 || true
    grep -a "chrome surface ($label)" "$log" || true
}

run_scenario() {
    local name="$1" mode="$2" fixture="$3" png="$4" delay="$5" action="$6"
    local stats_delay="$7"
    local comp_log="$SCRATCH/$name-compositor.log"
    local shell_log="$SCRATCH/$name-shell.log"
    local stats="$SCRATCH/$name.stats"
    local env_args=(DRAGONFRUIT_CHROME_STATS=1)
    if [ "$mode" = "gpu" ]; then
        env_args+=(DRAGONFRUIT_CHROME_GPU=1 QT_QPA_PLATFORM=wayland)
        case "$name" in
            dock) env_args+=(DRAGONFRUIT_CHROME_GPU_SURFACES=dock) ;;
            menubar) env_args+=(DRAGONFRUIT_CHROME_GPU_SURFACES=menubar,menubar-popup) ;;
            popover) env_args+=(DRAGONFRUIT_CHROME_GPU_SURFACES=dock,dock-popup) ;;
            lock) env_args+=(DRAGONFRUIT_CHROME_GPU_SURFACES=lock) ;;
        esac
    else
        env_args+=(DRAGONFRUIT_CHROME_GPU=0 QT_QPA_PLATFORM=offscreen)
    fi
    if [ -n "$fixture" ]; then
        env_args+=("$fixture")
    fi
    env_args+=("DF_SCREENSHOT_CAPTURE_FIXTURE=$delay:$png")

    echo "t21-gpu-rollout: $name ($mode)"
    rm -f "$png"
    start_compositor "$comp_log"
    env "${env_args[@]}" WAYLAND_DISPLAY="$SOCKET" \
        ./build/shell/src/dragonfruit-shell --socket-name "$SOCKET" \
        >"$shell_log" 2>&1 &
    SHELL_PID=$!
    for _ in $(seq 1 240); do
        grep -aq "menu bar configured" "$shell_log" 2>/dev/null && break
        if ! kill -0 "$SHELL_PID" 2>/dev/null; then
            echo "t21-gpu-rollout: shell ($name) exited early; log:" >&2
            tail -40 "$shell_log" >&2
            exit 1
        fi
        sleep 0.1
    done

    case "$action" in
        dock)
            sleep 1
            drive_dock_sweep
            sleep 0.5
            ;;
        popover)
            spawn_grouped_x11
            ;;
        *)
            ;;
    esac

    # Take the stats before the compositor's capture fixture fires. A full
    # output readback can stall the GPU for a moment; it must not turn a
    # successfully committed dmabuf frame into a recorded sync timeout.
    sleep "$stats_delay"
    dump_stats "$SHELL_PID" "$shell_log" "SIGUSR1-1" | tee "$stats"

    # The compositor's capture fixture writes the still after `delay` ms.
    # The portal save path may win over the requested temporary path, so also
    # accept the newest file in ~/Pictures/Screenshots.
    local before_shot=""
    before_shot="$(find "$HOME/Pictures/Screenshots" -maxdepth 1 -name 'Screenshot_*.png' \
        -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1 | cut -d' ' -f2- || true)"
    for _ in $(seq 1 $((delay / 100 + 120))); do
        [ -s "$png" ] && break
        sleep 0.1
    done
    if [ ! -s "$png" ]; then
        local saved=""
        saved="$(find "$HOME/Pictures/Screenshots" -maxdepth 1 -name 'Screenshot_*.png' \
            -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1 | cut -d' ' -f2- || true)"
        if [ -n "$saved" ] && [ "$saved" != "$before_shot" ] && [ -s "$saved" ]; then
            cp "$saved" "$png"
        fi
    fi
    if [ ! -s "$png" ]; then
        echo "t21-gpu-rollout: no capture appeared for $name" >&2
        tail -30 "$shell_log" >&2
    fi
    kill -TERM "$SHELL_PID" 2>/dev/null || true
    wait "$SHELL_PID" 2>/dev/null || true
    SHELL_PID=""
    for pid in $X11_PIDS; do kill -TERM "$pid" 2>/dev/null || true; done
    X11_PIDS=""
    stop_compositor
}

run_scenario dock gpu "" "$SHOTS/t21-gpu-rollout-dock.png" 9000 dock 6
run_scenario menubar gpu "DF_MENUBAR_MENU_FIXTURE=0" \
    "$SHOTS/t21-gpu-rollout-menubar.png" 7000 none 4
run_scenario popover gpu "DF_DOCK_CHOOSER_FIXTURE=1" \
    "$SHOTS/t21-gpu-rollout-popover.png" 11000 popover 8
run_scenario lock gpu "DF_LOCK_FIXTURE=1000" \
    "$SHOTS/t21-gpu-rollout-lock.png" 7000 none 4
run_scenario fallback shm "DF_MENUBAR_MENU_FIXTURE=0" \
    "$SHOTS/t21-gpu-rollout-fallback.png" 7000 none 4

field() {
    local name="$1" line="$2"
    sed -n "s/.* ${name}=\\([0-9][0-9]*\\).*/\\1/p" <<<"$line" | tail -1
}

surface_line() {
    local file="$1" surface="$2"
    grep -a "surface=$surface " "$file" | tail -1 || true
}

surface_ok() {
    local file="$1" surface="$2" want="$3"
    local line
    line="$(surface_line "$file" "$surface")"
    [ -n "$line" ] || return 1
    grep -q "path=$want " <<<"$line" || return 1
    local commits
    commits="$(field dmabuf_commits "$line")"
    [ -n "$commits" ] || return 1
    if [ "$want" = "dmabuf" ]; then
        [ "$commits" -gt 0 ] || return 1
        local copies
        copies="$(field copy_bytes "$line")"
        [ "$copies" = "0" ] || return 1
    fi
    return 0
}

gpu_ok=0
dock_ok=0
menubar_ok=0
popover_ok=0
lock_ok=0
fallback_ok=0

gpu_stats=$(cat "$SCRATCH/dock.stats" "$SCRATCH/menubar.stats" \
    "$SCRATCH/popover.stats" "$SCRATCH/lock.stats" 2>/dev/null || true)
if grep -q "path=render-control+dmabuf" <<<"$gpu_stats" \
    && ! grep -q "gpu=off" <<<"$gpu_stats" \
    && grep -q "grab_renders=0 " <<<"$gpu_stats"; then
    gpu_ok=1
fi
surface_ok "$SCRATCH/dock.stats" dock dmabuf && dock_ok=1
surface_ok "$SCRATCH/menubar.stats" menubar dmabuf && menubar_ok=1
surface_ok "$SCRATCH/menubar.stats" menubar-popup dmabuf && menubar_ok=$((menubar_ok + 1))
surface_ok "$SCRATCH/popover.stats" dock dmabuf && popover_ok=1
surface_ok "$SCRATCH/popover.stats" dock-popup dmabuf && popover_ok=$((popover_ok + 1))
surface_ok "$SCRATCH/lock.stats" lock dmabuf && lock_ok=1

fallback_stats=$(cat "$SCRATCH/fallback.stats" 2>/dev/null || true)
if grep -q "path=render-control+shm" <<<"$fallback_stats" \
    && grep -q "gpu=off" <<<"$fallback_stats" \
    && grep -q "dmabuf_commits=0 " <<<"$fallback_stats" \
    && surface_ok "$SCRATCH/fallback.stats" menubar shm; then
    fallback_ok=1
fi

{
    echo "T-21.1d GPU chrome rollout (T186d)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "distro: $(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")"
    echo "host session: WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "nested socket: $SOCKET"
    echo
    echo "## GPU run: DRAGONFRUIT_CHROME_GPU=1 (QQuickRenderControl + dmabuf)"
    echo "Each scenario is a fresh shell against a fresh nested compositor;"
    echo "the still is the compositor's own render, not a screencopy client."
    for name in dock menubar popover lock; do
        echo
        echo "### $name"
        echo "command: QT_QPA_PLATFORM=wayland WAYLAND_DISPLAY=$SOCKET DRAGONFRUIT_CHROME_GPU=1 DRAGONFRUIT_CHROME_STATS=1 ./build/shell/src/dragonfruit-shell --socket-name $SOCKET"
        echo "still: docs/captures/t21-gpu-rollout-$name.png"
        echo "stats:"
        cat "$SCRATCH/$name.stats" 2>/dev/null || true
    done
    echo
    echo "## Fallback run: DRAGONFRUIT_CHROME_GPU=0 (offscreen QPA, T185 shm)"
    echo "command: QT_QPA_PLATFORM=offscreen WAYLAND_DISPLAY=$SOCKET DRAGONFRUIT_CHROME_GPU=0 DRAGONFRUIT_CHROME_STATS=1 ./build/shell/src/dragonfruit-shell --socket-name $SOCKET"
    echo "still: docs/captures/t21-gpu-rollout-fallback.png"
    echo "stats:"
    cat "$SCRATCH/fallback.stats" 2>/dev/null || true
    echo
    echo "## Verdict"
    echo "gpu_transport_ok=$gpu_ok dock_dmabuf_ok=$dock_ok menubar_and_popup_dmabuf_ok=$menubar_ok"
    echo "dock_popover_dmabuf_ok=$popover_ok lock_dmabuf_ok=$lock_ok fallback_shm_ok=$fallback_ok"
    if [ "$gpu_ok" -eq 1 ] && [ "$dock_ok" -eq 1 ] && [ "$menubar_ok" -eq 2 ] \
        && [ "$popover_ok" -eq 2 ] && [ "$lock_ok" -eq 1 ] \
        && [ "$fallback_ok" -eq 1 ]; then
        echo "PASS: the Dock, menu bar, both popovers, and lock surface all"
        echo "committed dmabuf frames with zero copy bytes, and GPU=0 stayed"
        echo "on the pooled shm path."
    else
        echo "FAIL: see the per-surface stats above."
    fi
} >"$OUT"

echo "t21-gpu-rollout: wrote $OUT"
if [ "$gpu_ok" -ne 1 ] || [ "$dock_ok" -ne 1 ] || [ "$menubar_ok" -ne 2 ] \
    || [ "$popover_ok" -ne 2 ] || [ "$lock_ok" -ne 1 ] \
    || [ "$fallback_ok" -ne 1 ]; then
    exit 1
fi
