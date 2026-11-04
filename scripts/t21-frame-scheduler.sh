#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.2b per-output frame request / earliest-deadline trace (T188b).
#
# Part 1 runs one nested compositor at 144 Hz and samples the frame scheduler
# before, during, and after the synthetic dummy animation. It records the
# request lifecycle (idle: no request/timer; live: one animation request and an
# armed timer; settled: no request/timer, flat counters) and the per-output
# clock deadline from the `frame request` line. The nested rail is the
# single-output correctness trace, not mixed-refresh proof.
#
# Part 2 runs the same scheduler on the headless backend at 144 Hz and 60 Hz so
# the derived animation cadence is visible without a host vblank throttling the
# nested submit. The pure 60+144 mixed test lives in
# compositor/src/frame_scheduler.rs; native mixed-refresh evidence is T188c.
#
# This script is not part of `make e2e`: the nested half needs a host Wayland
# session. When there is no host session the artifact is an explicit OPEN
# marker.
#
# Overrides: OUT (report path), SETTLE (nested settle, default 2 s),
# DURATION_MS (default 600).
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="${OUT:-docs/captures/t21-frame-scheduler.txt}"
SETTLE="${SETTLE:-2}"
DURATION_MS="${DURATION_MS:-600}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-scheduler.XXXXXX")}"
PID=""
SOCKET=""
SYNTH=""

cleanup() {
    if [ -n "$PID" ]; then
        kill -TERM "$PID" 2>/dev/null || true
        wait "$PID" 2>/dev/null || true
    fi
    rm -f "${XDG_RUNTIME_DIR:-}/$SOCKET" "${XDG_RUNTIME_DIR:-}/$SOCKET.lock" \
        "$SYNTH" 2>/dev/null || true
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

mkdir -p "$SCRATCH" "$(dirname "$OUT")"

write_open() {
    {
        echo "T-21.2b per-output frame request trace (T188b)"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: $1"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make t21-frame-scheduler"
    } >"$OUT"
    echo "t21-frame-scheduler: OPEN ($1); wrote $OUT" >&2
    exit 0
}

[ -n "${XDG_RUNTIME_DIR:-}" ] || write_open "XDG_RUNTIME_DIR is not set"
[ -n "${WAYLAND_DISPLAY:-}" ] || write_open "no host Wayland session (WAYLAND_DISPLAY is unset)"
[ -x target/debug/dragonfruit-compositor ] || {
    echo "t21-frame-scheduler: target/debug/dragonfruit-compositor missing; run make build" >&2
    exit 1
}

send_command() {
    python3 - "$SYNTH" "$1" <<'PY'
import socket
import sys

sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.sendto(sys.argv[2].encode() + b"\n", sys.argv[1])
PY
}

start_session() {
    local log="$1"
    local backend="$2"
    local hz="$3"
    local settle="$4"
    SOCKET="dragonfruit-t21-scheduler-${backend}-$$"
    SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
    rm -f "$SYNTH"
    DRAGONFRUIT_REFRESH_HZ="$hz" \
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        target/debug/dragonfruit-compositor --backend "$backend" --socket-name "$SOCKET" \
        >"$log" 2>&1 &
    PID=$!
    for _ in $(seq 1 300); do
        [ -e "${XDG_RUNTIME_DIR}/$SOCKET" ] && [ -e "$SYNTH" ] && break
        if ! kill -0 "$PID" 2>/dev/null; then
            echo "t21-frame-scheduler: $backend compositor exited early; log:" >&2
            tail -30 "$log" >&2
            exit 1
        fi
        sleep 0.1
    done
    if [ ! -e "${XDG_RUNTIME_DIR}/$SOCKET" ] || [ ! -e "$SYNTH" ]; then
        echo "t21-frame-scheduler: $backend sockets never appeared; log:" >&2
        tail -30 "$log" >&2
        exit 1
    fi
    sleep "$settle"
}

snapshot() {
    kill -USR1 "$PID"
    sleep 0.3
}

stop_session() {
    kill -TERM "$PID" 2>/dev/null || true
    for _ in $(seq 1 100); do
        kill -0 "$PID" 2>/dev/null || break
        sleep 0.1
    done
    wait "$PID" 2>/dev/null || true
    rm -f "${XDG_RUNTIME_DIR}/$SOCKET" "${XDG_RUNTIME_DIR}/$SOCKET.lock" \
        "$SYNTH" 2>/dev/null || true
    PID=""
}

# Read one field from the `frame scheduler stats (<label>)` line.
scheduler_field() {
    local log="$1"
    local label="$2"
    local name="$3"
    grep -a "frame scheduler stats ($label)" "$log" \
        | sed -n "s/.*$name=\\([0-9][0-9]*\\).*/\\1/p" \
        | head -1
}

# The most recent scheduler line/field, independent of the SIGUSR1 sequence:
# a nested host can throttle the loop so a dump arrives after the next signal.
latest_scheduler_line() {
    grep -a "frame scheduler stats" "$1" | tail -1
}

latest_scheduler_field() {
    latest_scheduler_line "$1" \
        | sed -n "s/.*$2=\\([0-9][0-9]*\\).*/\\1/p"
}

latest_scheduler_label() {
    latest_scheduler_line "$1" \
        | sed -n 's/.*frame scheduler stats (\([^)]*\)).*/\1/p'
}

# Read one field from the `animation stats (<label>)` line.
animation_field() {
    local log="$1"
    local label="$2"
    local name="$3"
    grep -a "animation stats ($label)" "$log" \
        | sed -n "s/.*$name=\\([0-9][0-9]*\\).*/\\1/p" \
        | head -1
}

run_headless() {
    local hz="$1"
    local log="$SCRATCH/headless-$hz.log"
    start_session "$log" headless "$hz" 1
    snapshot
    send_command "animate-dummy $DURATION_MS"
    sleep 0.9
    snapshot
    stop_session
    local before after
    before="$(animation_field "$log" SIGUSR1-1 frames_stepped)"
    after="$(animation_field "$log" SIGUSR1-2 frames_stepped)"
    echo "$(( ${after:-0} - ${before:-0} ))"
}

# Part 1: the nested single-output trace.
NESTED_LOG="$SCRATCH/nested.log"
start_session "$NESTED_LOG" nested 144 "$SETTLE"
NESTED_SOCKET="$SOCKET"
snapshot
idle_label="$(latest_scheduler_label "$NESTED_LOG")"
send_command "animate-dummy $DURATION_MS"

# The nested host can throttle submit() far below the mode rate, so wait for
# the request to appear and then for it to drain instead of assuming a fixed
# wall-clock duration. Read the latest scheduler line rather than a predicted
# SIGUSR1 sequence: a throttled loop can deliver a dump after the next signal.
live_label=""
for _ in $(seq 1 40); do
    sleep 0.05
    snapshot
    if [ "$(latest_scheduler_field "$NESTED_LOG" animation)" = "1" ]; then
        live_label="$(latest_scheduler_label "$NESTED_LOG")"
        break
    fi
done

settled_label=""
for _ in $(seq 1 80); do
    sleep 0.2
    snapshot
    if [ "$(latest_scheduler_field "$NESTED_LOG" animation)" = "0" ]; then
        settled_label="$(latest_scheduler_label "$NESTED_LOG")"
        break
    fi
done
stop_session

idle_animation="$(scheduler_field "$NESTED_LOG" "${idle_label:-missing}" animation)"
idle_timer="$(scheduler_field "$NESTED_LOG" "${idle_label:-missing}" timer_armed)"
live_animation="$(scheduler_field "$NESTED_LOG" "${live_label:-missing}" animation)"
live_timer="$(scheduler_field "$NESTED_LOG" "${live_label:-missing}" timer_armed)"
settled_animation="$(scheduler_field "$NESTED_LOG" "${settled_label:-missing}" animation)"
settled_timer="$(scheduler_field "$NESTED_LOG" "${settled_label:-missing}" timer_armed)"
stepped_live="$(animation_field "$NESTED_LOG" "${live_label:-missing}" frames_stepped)"
stepped_settled="$(animation_field "$NESTED_LOG" "${settled_label:-missing}" frames_stepped)"
stepped_exit="$(animation_field "$NESTED_LOG" exit frames_stepped)"

# Part 2: headless cadence controls (no host vblank in the way).
headless_144="$(run_headless 144)"
headless_60="$(run_headless 60)"

{
    echo "T-21.2b per-output frame request trace (T188b)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "command: DRAGONFRUIT_REFRESH_HZ=<hz> target/debug/dragonfruit-compositor --backend <nested|headless>"
    echo "synthetic command: animate-dummy $DURATION_MS"
    echo "derived period: 1e12 / 144000 mHz = 6944 ns = 6.944 ms"
    echo
    echo "## Nested single-output: request lifecycle (144 Hz)"
    echo "nested socket: $NESTED_SOCKET"
    echo "idle sample: $idle_label"
    grep -aE "refresh stats \($idle_label\)|frame scheduler stats \($idle_label\)|frame request \($idle_label\)|animation stats \($idle_label\)" "$NESTED_LOG" || true
    echo "live sample: ${live_label:-missing}"
    if [ -n "$live_label" ]; then
        grep -aE "frame scheduler stats \($live_label\)|frame request \($live_label\)|animation stats \($live_label\)" "$NESTED_LOG" || true
    fi
    echo "settled sample: ${settled_label:-missing}"
    if [ -n "$settled_label" ]; then
        grep -aE "frame scheduler stats \($settled_label\)|frame request \($settled_label\)|animation stats \($settled_label\)" "$NESTED_LOG" || true
    fi
    echo
    echo "idle:    animation=${idle_animation:-?} timer_armed=${idle_timer:-?}"
    echo "live:    animation=${live_animation:-?} timer_armed=${live_timer:-?}"
    echo "settled: animation=${settled_animation:-?} timer_armed=${settled_timer:-?}"
    echo "frames_stepped: live=$stepped_live settled=$stepped_settled exit=$stepped_exit"
    echo "settled clock must not step after the final frame: settled == exit."
    echo
    echo "## Headless cadence controls"
    echo "animation_frames_stepped delta over ${DURATION_MS} ms:"
    echo "  144 Hz: $headless_144 (~$(( DURATION_MS / 7 )) frames at 6.944 ms)"
    echo "   60 Hz: $headless_60 (~$(( DURATION_MS / 17 )) frames at 16.666 ms)"
    echo
    echo "## Verdict"
    ok=1
    if [ "${idle_animation:-1}" = "0" ] && [ "${idle_timer:-1}" = "0" ]; then
        echo "idle request lifecycle: PASS (animation=0 timer_armed=0)"
    else
        echo "idle request lifecycle: FAIL (animation=${idle_animation:-?} timer_armed=${idle_timer:-?})"
        ok=0
    fi
    if [ "${live_animation:-0}" = "1" ] && [ "${live_timer:-0}" = "1" ]; then
        echo "live request lifecycle: PASS (animation=1 timer_armed=1)"
    else
        echo "live request lifecycle: FAIL (animation=${live_animation:-?} timer_armed=${live_timer:-?})"
        ok=0
    fi
    if [ "${settled_animation:-1}" = "0" ] && [ "${settled_timer:-1}" = "0" ]; then
        echo "settled request lifecycle: PASS (animation=0 timer_armed=0)"
    else
        echo "settled request lifecycle: FAIL (animation=${settled_animation:-?} timer_armed=${settled_timer:-?})"
        ok=0
    fi
    if [ "${stepped_settled:-0}" = "${stepped_exit:-1}" ]; then
        echo "settled clock: PASS (no animation step after the final frame)"
    else
        echo "settled clock: FAIL (settled=$stepped_settled exit=$stepped_exit)"
        ok=0
    fi
    if [ "$headless_144" -ge 60 ] && [ "$headless_60" -ge 24 ] && [ "$headless_60" -le 48 ]; then
        echo "headless cadence: PASS (144 Hz delta=$headless_144 >= 60; 60 Hz delta=$headless_60 in 24..48)"
    else
        echo "headless cadence: FAIL (144 Hz delta=$headless_144, 60 Hz delta=$headless_60)"
        ok=0
    fi
    if [ "$ok" -eq 1 ]; then
        echo "PASS: per-output requests arm from the earliest deadline and settle with no timer."
    fi
} >"$OUT"

echo "t21-frame-scheduler: wrote $OUT"
if ! grep -aq "PASS: per-output requests arm from the earliest deadline and settle with no timer." "$OUT"; then
    exit 1
fi
