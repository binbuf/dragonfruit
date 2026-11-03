#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.1a chrome transport baseline (nested).
#
# Runs the nested demo with the synthetic-input harness and the opt-in
# `DRAGONFRUIT_CHROME_STATS=1` counters, drives the six baseline scenarios
# (idle, slow Dock sweep, menu-bar dropdown, Dock popover, one app launch,
# Mission Control with 10+ windows), and brackets every scenario with a
# SIGUSR1 snapshot on the shell and the compositor. The committed report is
# docs/captures/t21-chrome-baseline.txt.
#
# The run is nested: it measures the `offscreen QPA -> grabWindow() -> QImage ->
# wl_shm` transport honestly, but its presentation interval is not a native
# vblank. The native half is attempted through `make native-capture`; when the
# running native compositor cannot be restarted without ending the session it
# is recorded OPEN, never silently omitted.
#
# Requires a host Wayland session and the built tree (`make build`). Not part
# of `make e2e`. Overrides: SETTLE (default 10), OUT (report path),
# KEEP_SCRATCH, SKIP_NATIVE.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="${OUT:-docs/captures/t21-chrome-baseline.txt}"
SETTLE="${SETTLE:-10}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
SKIP_NATIVE="${SKIP_NATIVE:-0}"
SOCKET="${SOCKET:-dragonfruit-t21-baseline-$$}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-baseline.XXXXXX")}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"
SHOTS="$SCRATCH/shots"
LOG="$SCRATCH/demo.log"
SHELL_LOG="$SCRATCH/shell.log"
DRIVER_LOG="$SCRATCH/driver.log"
HOST_INFO="$SCRATCH/host-info.txt"
NATIVE_INFO="$SCRATCH/native.txt"

mkdir -p "$SCRATCH" "$SHOTS"
rm -f "$SYNTH"
: >"$NATIVE_INFO"

cleanup() {
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    rm -f "$SYNTH"
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

echo "t21-chrome-baseline: starting the nested demo ($SOCKET, private bus)"
QSG_INFO=1 \
DRAGONFRUIT_CHROME_STATS=1 \
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
DF_DOCK_CHOOSER_FIXTURE=1 \
DF_SCREENSHOT_DIR="$SHOTS" \
DF_SHELL_LOG="$SHELL_LOG" \
setsid make demo DEMO_ARGS="--socket-name $SOCKET --private-bus" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!

for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "t21-chrome-baseline: synthetic-input socket never appeared; demo log:" >&2
    tail -40 "$LOG" >&2
    exit 1
fi
# Wait for the shell's scene-graph commit path, not just the early policy
# lines. The synthetic socket appears before the shell has loaded QML and
# created its chrome surfaces; sending SIGUSR1 too early only queues the
# self-pipe byte and the first snapshot sees an empty shell.
for _ in $(seq 1 900); do
    grep -q "Dock scene-graph commit path active" "$LOG" && break
    sleep 0.1
done
sleep "$SETTLE"

SHELL_PID="$(pgrep -f "dragonfruit-shell --socket-name $SOCKET" | head -1 || true)"
COMPOSITOR_PID="$(pgrep -f "dragonfruit-compositor --backend nested --socket-name $SOCKET" | head -1 || true)"
if [ -z "$SHELL_PID" ] || [ -z "$COMPOSITOR_PID" ]; then
    echo "t21-chrome-baseline: shell/compositor pid not found" >&2
    pgrep -af "dragonfruit.*$SOCKET" >&2 || true
    exit 1
fi

{
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "distro: $(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")"
    echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>} XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-<unset>} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "nested socket: $SOCKET (private session bus, scratch XDG dirs)"
    echo "shell pid: $SHELL_PID; compositor pid: $COMPOSITOR_PID"
    echo
    echo "## QSG_INFO (shell scene-graph backend)"
    {
        grep -aE "dragonfruit-shell: .*(scenegraph|QSG|RHI|OpenGL|Graphics|render loop|Loading backend)" "$LOG" || true
        grep -aE "dragonfruit-shell: .*(scenegraph|QSG|RHI|OpenGL|Graphics|render loop|Loading backend)" "$SHELL_LOG" || true
    } | sed 's/^dragonfruit-shell: //' | sort -u || true
    echo "QSG verdict: $(grep -aE 'Loading backend' "$SHELL_LOG" 2>/dev/null | head -1 | sed 's/^dragonfruit-shell: //' || echo 'not found')"
    echo
    echo "## GPU / driver"
    if command -v lspci >/dev/null 2>&1; then
        lspci -nnk 2>/dev/null | grep -A3 -E "VGA compatible|3D controller|Display controller" || true
    else
        echo "lspci not installed"
    fi
    echo "DRM nodes: $(ls /dev/dri 2>/dev/null | tr '\n' ' ')"
    echo
    echo "## Shell process maps (GL/EGL/DRI)"
    grep -aE "libGL|libGLES|libEGL|/dri/|swrast|vulkan|libgallium" "/proc/$SHELL_PID/maps" 2>/dev/null \
        | awk '{print $6}' | sort -u || true
    if grep -q "Loading backend software" "$SHELL_LOG" 2>/dev/null; then
        echo "verdict: QSG software backend; no DRI driver mapped (loader libraries alone do not make a GL backend)"
    fi
    echo
    echo "## Exact command"
    echo "QSG_INFO=1 DRAGONFRUIT_CHROME_STATS=1 DRAGONFRUIT_SYNTHETIC_INPUT=$SYNTH DF_DOCK_CHOOSER_FIXTURE=1 DF_SCREENSHOT_DIR=$SHOTS DF_SHELL_LOG=$SHELL_LOG make demo DEMO_ARGS=\"--socket-name $SOCKET --private-bus\""
} >"$HOST_INFO"

echo "t21-chrome-baseline: driving the scenarios"
python3 scripts/t21-chrome-baseline-driver.py \
    --synth "$SYNTH" \
    --socket "$SOCKET" \
    --shell-pid "$SHELL_PID" \
    --compositor-pid "$COMPOSITOR_PID" \
    --screenshot-dir "$SHOTS" \
    --log "$LOG" \
    --shell-log "$SHELL_LOG" \
    --outdir docs/captures \
    2>&1 | tee "$DRIVER_LOG"

# Stop the compositor directly so its `exit` dump lands in the log; the demo
# process group is reaped by the trap.
kill -TERM "$COMPOSITOR_PID" 2>/dev/null || true
for _ in $(seq 1 150); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] || break; sleep 0.1; done
sleep 1

# Native half: record the running native session's diagnostics. The installed
# release compositor predates T-21.1a's chrome counters and was not started
# with the synthetic harness, so the scenario-driven native half is OPEN.
if [ "$SKIP_NATIVE" != "1" ] && [ "${XDG_CURRENT_DESKTOP:-}" = "dragonfruit" ]; then
    echo "t21-chrome-baseline: recording the native session snapshot (OPEN for chrome scenarios)"
    NATIVE_DIR="$(DF_CAPTURE_SETTLE=2 bash scripts/native-capture.sh once 2>/dev/null | tail -1 || true)"
    {
        echo "native capture: ${NATIVE_DIR:-unavailable}"
        if [ -n "$NATIVE_DIR" ] && [ -d "$NATIVE_DIR" ]; then
            echo
            echo "## Native session diagnostics (installed release compositor)"
            grep -aE "presentation stats|frame timing|material stats|degrade stats|render stats|chrome stats|DRM bring-up|Multi-GPU" \
                "$NATIVE_DIR"/key-startup.txt 2>/dev/null | tail -5 || true
            echo
            echo "## Why the native scenario half is OPEN"
            echo "The host is already a native Dragonfruit session. Its running compositor is"
            echo "the installed release build: it has no T-21.1a chrome counters and was not"
            echo "started with DRAGONFRUIT_SYNTHETIC_INPUT. Restarting it to run the same six"
            echo "scenarios would end the session this task is executing in. The native"
            echo "presentation line above is real; the native chrome scenario comparison is"
            echo "left OPEN for the T-21 hardware rail."
        else
            echo "native-capture produced no snapshot directory; native half OPEN."
        fi
    } >"$NATIVE_INFO"
fi

VISUAL=""
for name in dock menu mission-control; do
    path="docs/captures/t21-chrome-baseline-$name.png"
    [ -f "$path" ] && VISUAL="${VISUAL}${VISUAL:+,}$path"
done

python3 scripts/t21-chrome-baseline-report.py \
    --log "$LOG" \
    --shell-log "$SHELL_LOG" \
    --driver-log "$DRIVER_LOG" \
    --out "$OUT" \
    --host-info "$HOST_INFO" \
    --native "$NATIVE_INFO" \
    --visual "$VISUAL" \
    --command "make t21-chrome-baseline"

echo "t21-chrome-baseline: report written to $OUT"
grep -E "^### |shell delta:|compositor delta:|Verdict:" "$OUT" | head -60 || true
