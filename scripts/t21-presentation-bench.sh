#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.4b presentation benchmark (nested scenario set, T192b).
#
# Runs the nested demo with the synthetic-input harness, the opt-in chrome
# counters, GPU timer queries, and the per-frame trace, then drives the
# benchmark scenario set:
#
#   idle 60 s, Dock sweep 10 s, menu-bar open/close, Dock popover open/close,
#   app launch + window appear, Mission Control with 10+ windows, fullscreen
#
# plus a refresh matrix (60/120/240 Hz overrides; the main run is 144 Hz) and
# a material-tier matrix (Full/Reduced/Minimal). Every scenario is bracketed by
# SIGUSR1 dumps on both the shell and the compositor, so the report carries
# per-scenario shell `commit_us` / `frame_callback_us` percentiles, compositor
# frame-time percentiles, GPU timer percentiles, and the transport mix
# (`path=dmabuf|shm`, `dmabuf_fallbacks`, `sync_timeouts`, `copy_bytes`).
#
# This is **nested reference evidence**, not native presentation proof. Native
# rows (DRM panel modes, 1080p/4K, mixed outputs, missed-deadline counts) are
# T192c and are written OPEN here.
#
# Requires a host Wayland session and the built tree (`make build`). Not part
# of `make e2e`. Overrides: IDLE_S (default 60), SWEEP_S (default 10),
# SETTLE (default 3), MATRIX_REFRESHES (default "60 120 240"), OUT, RAW_DIR,
# OPEN_OUT, KEEP_SCRATCH, SKIP_NATIVE.
set -euo pipefail

cd "$(dirname "$0")/.."

. "$(dirname "$0")/t21-capture-lib.sh"

OUT="${OUT:-docs/captures/t21-presentation-bench.md}"
RAW_DIR="${RAW_DIR:-docs/captures/t21-presentation-bench-raw}"
OPEN_OUT="${OPEN_OUT:-docs/captures/t21-presentation-bench.open.txt}"
IDLE_S="${IDLE_S:-60}"
SWEEP_S="${SWEEP_S:-10}"
SETTLE="${SETTLE:-3}"
MATRIX_REFRESHES="${MATRIX_REFRESHES:-60 120 240}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
SKIP_NATIVE="${SKIP_NATIVE:-0}"
SKIP_TIERS="${SKIP_TIERS:-0}"
# The default GPU rail includes the overview surface. On this tree the
# overview's GPU producer crashes the shell during Mission Control teardown
# (SIGSEGV in GpuChromeFrameProducer::destroySlotResources, with null
# Shadow.qml material warnings). T192b records that as a follow-up and keeps
# the benchmark's measured surfaces on the GPU rail; the overview row is shm.
# Set CHROME_GPU_SURFACES=all to reproduce the full-rail crash.
CHROME_GPU_SURFACES="${CHROME_GPU_SURFACES:-dock,dock-popup,menubar,menubar-popup}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-bench.XXXXXX")}"
HOST_INFO="$SCRATCH/host-info.txt"
PREFLIGHT="$SCRATCH/preflight.txt"
NATIVE_INFO="$SCRATCH/native.txt"
SOCKET=""
SYNTH=""
SHOTS=""
SESSION_XDG=""
COMP_PID=""
SHELL_PID=""
SERVICE_PIDS=""
BUS_PID=""
SESSION_ACTIVE=0

mkdir -p "$SCRATCH" "$RAW_DIR" "$(dirname "$OUT")" "$(dirname "$OPEN_OUT")"
: >"$NATIVE_INFO"

write_open() {
    local out="$1" reason="$2"
    {
        echo "T-21.4b presentation benchmark — native rows (T192b)"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: $reason"
        echo "hardware:"
        echo "  host: $(uname -srm)"
        echo "  GPU: $(lspci -nnk 2>/dev/null | grep -A3 -E 'VGA compatible|3D controller|Display controller' | tr '\n' ' ' || echo 'unknown')"
        echo "  DRM nodes: $(ls /dev/dri 2>/dev/null | tr '\n' ' ' || echo 'none')"
        echo "  connected outputs: $(for status in /sys/class/drm/card*-*/status; do [ -r "$status" ] || continue; if [ "$(cat "$status" 2>/dev/null)" = "connected" ]; then basename "$(dirname "$status")"; fi; done | tr '\n' ' ' || echo 'none')"
        echo "  modes: $(for modes in /sys/class/drm/card*-*/modes; do [ -r "$modes" ] || continue; head -1 "$modes"; done | sort -u | tr '\n' ' ' || echo 'none')"
        echo "rerun: native rows are T192c; on a free DRM seat run the same scenario set"
        echo "  with the DRM backend and attach its 'presentation stats' lines."
        echo "  The nested harness is: WAYLAND_DISPLAY=<host-socket> make t21-presentation-bench"
    } >"$out"
    echo "t21-presentation-bench: OPEN ($reason); wrote $out" >&2
}

if [ -z "${XDG_RUNTIME_DIR:-}" ] || [ -z "${WAYLAND_DISPLAY:-}" ]; then
    write_open "$OPEN_OUT" "no host Wayland session (XDG_RUNTIME_DIR or WAYLAND_DISPLAY is unset)"
    exit 0
fi

command -v python3 >/dev/null 2>&1 || {
    echo "t21-presentation-bench: python3 not found" >&2
    exit 1
}
command -v dbus-daemon >/dev/null 2>&1 || {
    echo "t21-presentation-bench: dbus-daemon not found; a private session bus is required for a deterministic scale" >&2
    exit 1
}
[ -x target/debug/dragonfruit-compositor ] || {
    echo "t21-presentation-bench: target/debug/dragonfruit-compositor missing; run make build" >&2
    exit 1
}
[ -x build/shell/src/dragonfruit-shell ] || {
    echo "t21-presentation-bench: build/shell/src/dragonfruit-shell missing; run make build" >&2
    exit 1
}

cleanup() {
    stop_session >/dev/null 2>&1 || true
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

stop_session() {
    [ "$SESSION_ACTIVE" = "1" ] || return 0
    if [ -n "$SHELL_PID" ]; then
        kill -TERM "$SHELL_PID" 2>/dev/null || true
        wait "$SHELL_PID" 2>/dev/null || true
        SHELL_PID=""
    fi
    for pid in $SERVICE_PIDS; do
        kill -TERM "$pid" 2>/dev/null || true
    done
    SERVICE_PIDS=""
    if [ -n "$COMP_PID" ]; then
        kill -TERM "$COMP_PID" 2>/dev/null || true
        for _ in $(seq 1 150); do
            [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] || break
            sleep 0.1
        done
        wait "$COMP_PID" 2>/dev/null || true
        COMP_PID=""
    fi
    if [ -n "$BUS_PID" ]; then
        kill -TERM "$BUS_PID" 2>/dev/null || true
        wait "$BUS_PID" 2>/dev/null || true
        BUS_PID=""
    fi
    rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" \
          "$XDG_RUNTIME_DIR/$SOCKET.launch-token" "$SYNTH" 2>/dev/null || true
    SESSION_ACTIVE=0
}

start_session() {
    local name="$1" refresh="$2"
    local log_dir="$RAW_DIR/$name"
    local scratch_dir="$SCRATCH/$name"
    local bus_info bus_addr
    SOCKET="dragonfruit-t21-bench-${name}-$$"
    SYNTH="$XDG_RUNTIME_DIR/$SOCKET.synth"
    SHOTS="$scratch_dir/shots"
    SESSION_XDG="$scratch_dir/xdg"
    mkdir -p "$log_dir" "$SHOTS" "$SESSION_XDG/config" \
        "$SESSION_XDG/data" "$SESSION_XDG/state" "$SESSION_XDG/cache" \
        "$SESSION_XDG/share/applications"
    cp apps/settings/*.desktop "$SESSION_XDG/share/applications/" 2>/dev/null || true
    cp apps/files/*.desktop "$SESSION_XDG/share/applications/" 2>/dev/null || true

    bus_info="$(dbus-daemon --session --fork --print-address=1 --print-pid=1)"
    bus_addr="$(printf '%s\n' "$bus_info" | sed -n '1p')"
    BUS_PID="$(printf '%s\n' "$bus_info" | sed -n '2p')"
    if [ -z "$bus_addr" ] || [ -z "$BUS_PID" ]; then
        echo "t21-presentation-bench: private session bus did not start" >&2
        exit 1
    fi

    SESSION_ENV=(
        "DBUS_SESSION_BUS_ADDRESS=$bus_addr"
        "XDG_CONFIG_HOME=$SESSION_XDG/config"
        "XDG_DATA_HOME=$SESSION_XDG/data"
        "XDG_STATE_HOME=$SESSION_XDG/state"
        "XDG_CACHE_HOME=$SESSION_XDG/cache"
        "XDG_DATA_DIRS=$SESSION_XDG/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
        "XDG_CURRENT_DESKTOP=dragonfruit"
        "PATH=$PWD/build/apps/settings:$PWD/build/apps/files:$PATH"
        "QML_IMPORT_PATH=$PWD/build/qml"
    )

    echo "t21-presentation-bench: session $name at ${refresh} Hz (socket $SOCKET)"
    rm -f "$SYNTH" "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" \
          "$XDG_RUNTIME_DIR/$SOCKET.launch-token"
    DRAGONFRUIT_REFRESH_HZ="$refresh" \
    DRAGONFRUIT_GPU_TIMING=1 \
    DRAGONFRUIT_FRAME_TRACE=1 \
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        target/debug/dragonfruit-compositor --backend nested --socket-name "$SOCKET" \
        >"$log_dir/compositor.txt" 2>&1 &
    COMP_PID=$!
    SESSION_ACTIVE=1
    if ! t21_wait_for_file "$XDG_RUNTIME_DIR/$SOCKET" 300 0.1; then
        echo "t21-presentation-bench: compositor socket never appeared; log:" >&2
        tail -30 "$log_dir/compositor.txt" >&2
        exit 1
    fi
    if ! t21_wait_for_file "$SYNTH" 100 0.1; then
        echo "t21-presentation-bench: synthetic-input socket never appeared; log:" >&2
        tail -30 "$log_dir/compositor.txt" >&2
        exit 1
    fi

    # Core dev services: the shell's Dock identity and menu model use them.
    # They are best-effort; the shell degrades when one is missing.
    for service in dragonfruit-app-index dragonfruit-menu-broker; do
        if [ -x "target/debug/$service" ]; then
            env "${SESSION_ENV[@]}" WAYLAND_DISPLAY="$SOCKET" \
                "target/debug/$service" >"$log_dir/$service.txt" 2>&1 &
            SERVICE_PIDS="$SERVICE_PIDS $!"
        fi
    done
    # Wait for the two service names before starting the shell; the shell's
    # own client still tolerates their absence.
    if command -v gdbus >/dev/null 2>&1; then
        for name in org.dragonfruit.AppIndex1 org.dragonfruit.MenuBroker1; do
            for _ in $(seq 1 100); do
                if gdbus call --session --dest org.freedesktop.DBus \
                    --object-path /org/freedesktop/DBus \
                    --method org.freedesktop.DBus.NameHasOwner "$name" \
                    2>/dev/null | grep -q true; then
                    break
                fi
                sleep 0.05
            done
        done
    else
        sleep 0.5
    fi

    env "${SESSION_ENV[@]}" \
        WAYLAND_DISPLAY="$SOCKET" \
        DRAGONFRUIT_CHROME_STATS=1 \
        DRAGONFRUIT_CHROME_GPU=1 \
        DRAGONFRUIT_CHROME_GPU_SURFACES="$CHROME_GPU_SURFACES" \
        QT_QPA_PLATFORM=wayland \
        DF_DOCK_CHOOSER_FIXTURE=1 \
        DF_SCREENSHOT_DIR="$SHOTS" \
        build/shell/src/dragonfruit-shell --socket-name "$SOCKET" \
        >"$log_dir/shell.txt" 2>&1 &
    SHELL_PID=$!

    local i
    for i in $(seq 1 1800); do
        grep -aq "menu bar configured" "$log_dir/shell.txt" 2>/dev/null && break
        if ! kill -0 "$SHELL_PID" 2>/dev/null; then
            echo "t21-presentation-bench: shell exited early; log:" >&2
            tail -40 "$log_dir/shell.txt" >&2
            exit 1
        fi
        sleep 0.1
    done
    if ! grep -aq "menu bar configured" "$log_dir/shell.txt" 2>/dev/null; then
        echo "t21-presentation-bench: shell never configured its menu bar; log:" >&2
        tail -40 "$log_dir/shell.txt" >&2
        exit 1
    fi
    sleep "$SETTLE"
}

run_driver() {
    local name="$1" mode="$2"
    shift 2
    local log_dir="$RAW_DIR/$name"
    echo "t21-presentation-bench: driving $name ($mode)"
    python3 scripts/t21-presentation-bench-driver.py \
        --synth "$SYNTH" \
        --socket "$SOCKET" \
        --shell-pid "$SHELL_PID" \
        --compositor-pid "$COMP_PID" \
        --screenshot-dir "$SHOTS" \
        --log "$log_dir/compositor.txt" \
        --shell-log "$log_dir/shell.txt" \
        --outdir docs/captures \
        --mode "$mode" \
        "$@" 2>&1 | tee "$log_dir/driver.txt"
}

# ---------------------------------------------------------------------------
# Host and preflight.
# ---------------------------------------------------------------------------
{
    echo "== make t21-doctor =="
    make t21-doctor 2>&1
} >"$PREFLIGHT"

{
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "distro: $(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")"
    echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>} XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-<unset>} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "DRM nodes: $(ls /dev/dri 2>/dev/null | tr '\n' ' ' || echo 'none')"
    echo "connected outputs: $(for status in /sys/class/drm/card*-*/status; do [ -r "$status" ] || continue; if [ "$(cat "$status" 2>/dev/null)" = "connected" ]; then basename "$(dirname "$status")"; fi; done | tr '\n' ' ' || echo 'none')"
    echo "modes: $(for modes in /sys/class/drm/card*-*/modes; do [ -r "$modes" ] || continue; head -1 "$modes"; done | sort -u | tr '\n' ' ' || echo 'none')"
    echo
    echo "## Host Wayland output"
    if command -v wayland-info >/dev/null 2>&1; then
        timeout 10 wayland-info 2>/dev/null \
            | grep -A 12 "interface: 'wl_output'" || true
    else
        echo "wayland-info not installed"
    fi
    echo
    echo "## GPU"
    if command -v lspci >/dev/null 2>&1; then
        lspci -nnk 2>/dev/null \
            | grep -A3 -E "VGA compatible|3D controller|Display controller" || true
    else
        echo "lspci not installed"
    fi
    echo
    echo "## Nested backend"
    echo "The nested output follows the host window. The report records the"
    echo "actual physical mode and scale from the run's stats; it is not a"
    echo "native DRM mode."
} >"$HOST_INFO"

# ---------------------------------------------------------------------------
# Main scenario set: 144 Hz, Full material.
# ---------------------------------------------------------------------------
start_session "main-144-full" 144
run_driver "main-144-full" full --idle-s "$IDLE_S" --sweep-s "$SWEEP_S" --tier full
stop_session

# ---------------------------------------------------------------------------
# Refresh matrix: 60/120/240 Hz overrides (144 Hz is the main run above).
# ---------------------------------------------------------------------------
for hz in $MATRIX_REFRESHES; do
    start_session "refresh-$hz" "$hz"
    run_driver "refresh-$hz" sweep --tier full --sweep-s "$SWEEP_S"
    stop_session
done

# ---------------------------------------------------------------------------
# Material tiers: one session, a Dock sweep at each tier.
# ---------------------------------------------------------------------------
if [ "$SKIP_TIERS" != "1" ]; then
    start_session "tiers-144" 144
    run_driver "tiers-144" tiers --sweep-s "$SWEEP_S"
    stop_session
fi

# ---------------------------------------------------------------------------
# Native diagnostics (rows stay OPEN; T192c owns the native run).
# ---------------------------------------------------------------------------
if [ "$SKIP_NATIVE" != "1" ] && [ "${XDG_CURRENT_DESKTOP:-}" = "dragonfruit" ]; then
    echo "t21-presentation-bench: recording the native session snapshot (native rows OPEN)"
    NATIVE_DIR="$(DF_CAPTURE_SETTLE=2 bash scripts/native-capture.sh once 2>/dev/null | tail -1 || true)"
    {
        echo "native capture: ${NATIVE_DIR:-unavailable}"
        if [ -n "$NATIVE_DIR" ] && [ -d "$NATIVE_DIR" ]; then
            echo
            echo "## Native session diagnostics (installed release compositor)"
            grep -aE "presentation stats|frame timing|material stats|degrade stats|render stats|chrome stats|gpu stats|DRM bring-up|Multi-GPU" \
                "$NATIVE_DIR"/key-startup.txt 2>/dev/null | tail -20 || true
        fi
    } >"$NATIVE_INFO"
fi

write_open "$OPEN_OUT" "the host is already a native Dragonfruit session; restarting its DRM compositor to run the benchmark would end the session this task is running in. Native panel-mode, 1080p/4K, mixed-output, and missed-deadline rows are T192c."

cat >"$SCRATCH/run-notes.txt" <<EOF
## GPU rail surfaces

The measured chrome surfaces use the GPU dmabuf rail:
\`DRAGONFRUIT_CHROME_GPU_SURFACES=$CHROME_GPU_SURFACES\`.
The overview surface is excluded because the full GPU rail crashes the shell
during Mission Control in this tree (finding below); the overview row is
therefore the pooled-shm fallback with reason \`not-selected\`, and its copy
bytes are counted in the shm column.

## Finding: full GPU rail + Mission Control teardown crash

With the default full GPU rail (\`CHROME_GPU_SURFACES=all\`), the shell
segfaults during the Mission Control scenario. The core backtrace is:

\`\`\`text
#0 QQuickRenderTarget::operator=(QQuickRenderTarget const&)
#1 QQuickWindow::setRenderTarget(QQuickRenderTarget const&)
#2 GpuChromeFrameProducer::destroySlotResources(int)
#3 GpuChromeFrameProducer::releaseGpu()
#4 GpuChromeFrameProducer::~GpuChromeFrameProducer()
#5 ShellController::~ShellController()
#6 main
\`\`\`

The shell log immediately before the crash is full of
\`qrc:/Dragonfruit/components/Shadow.qml:27/28: TypeError: Cannot read
property 'offsetY'/'layers' of null\`, so the overview surface's material
state is null before the GPU producer tears down. This is a real
shell/GPU-overview regression found by the benchmark, not a benchmark-script
bug. It is out of T192b's scope (T192b does not fix regressions); file it as a
T193 follow-up. Reproduce with:

\`\`\`bash
CHROME_GPU_SURFACES=all make t21-presentation-bench
\`\`\`

The workaround above keeps every measured scenario on the GPU rail except the
overview surface, so the rest of the benchmark is unblocked and the fallback is
visible rather than hidden.

## Live visual check

The nested Dock-sweep and Mission Control stills are captured after the
measured scenarios through the trusted compositor screenshot path:

- \`docs/captures/t21-presentation-bench-dock.png\`
- \`docs/captures/t21-presentation-bench-mission-control.png\`

They are reviewed in the Hand-off; the motion verdict is recorded there (no
black frame, no material drop, no visible stutter).
EOF

# ---------------------------------------------------------------------------
# Report.
# ---------------------------------------------------------------------------
python3 scripts/t21-presentation-bench-report.py \
    --raw-root "$RAW_DIR" \
    --main-name "main-144-full" \
    --matrix-refreshes "$MATRIX_REFRESHES" \
    --tiers-name "tiers-144" \
    --host-info "$HOST_INFO" \
    --preflight "$PREFLIGHT" \
    --native "$NATIVE_INFO" \
    --open "$OPEN_OUT" \
    --run-notes "$SCRATCH/run-notes.txt" \
    --out "$OUT" \
    --command "make t21-presentation-bench" \
    --idle-s "$IDLE_S" \
    --sweep-s "$SWEEP_S"

echo "t21-presentation-bench: report written to $OUT"
grep -E "^### |^## |shm rate|Verdict:" "$OUT" | head -80 || true
