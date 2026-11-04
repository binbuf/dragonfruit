#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.2a simulated-144 Hz refresh trace (T187b).
#
# Part 1 starts one nested Dragonfruit compositor with
# `DRAGONFRUIT_REFRESH_HZ=144` and records the derived 6.944 ms tick and the
# matching degrade budget from the SIGUSR1 stats. The nested host compositor on
# a headless CI/dev display can throttle a dummy animation's presentation, so
# the nested section is the derived-interval/budget evidence, not a cadence
# measurement.
#
# Part 2 runs the same 144 Hz override on the headless backend, where the clock
# timer is not behind a host vblank, and records the animation frame delta. It
# also runs a 60 Hz headless control so the trace shows 6.944 ms vs 16.666 ms
# pacing side by side.
#
# The override is test-only and absent from a normal session. This script is
# not part of `make e2e`: the nested half needs a host Wayland session. When
# there is no host session the artifact is written as an explicit OPEN marker.
#
# Overrides: OUT (report path), SETTLE (nested settle, default 2 s),
# DURATION_MS (default 300).
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="${OUT:-docs/captures/t21-refresh-144.txt}"
SETTLE="${SETTLE:-2}"
DURATION_MS="${DURATION_MS:-300}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-refresh.XXXXXX")}"
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
        echo "T-21.2a simulated 144 Hz refresh trace (T187b)"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: $1"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make t21-refresh-144"
    } >"$OUT"
    echo "t21-refresh-144: OPEN ($1); wrote $OUT" >&2
    exit 0
}

[ -n "${XDG_RUNTIME_DIR:-}" ] || write_open "XDG_RUNTIME_DIR is not set"
[ -n "${WAYLAND_DISPLAY:-}" ] || write_open "no host Wayland session (WAYLAND_DISPLAY is unset)"
[ -x target/debug/dragonfruit-compositor ] || {
    echo "t21-refresh-144: target/debug/dragonfruit-compositor missing; run make build" >&2
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
    SOCKET="dragonfruit-t21-refresh-${backend}-$$"
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
            echo "t21-refresh-144: $backend compositor exited early; log:" >&2
            tail -30 "$log" >&2
            exit 1
        fi
        sleep 0.1
    done
    if [ ! -e "${XDG_RUNTIME_DIR}/$SOCKET" ] || [ ! -e "$SYNTH" ]; then
        echo "t21-refresh-144: $backend sockets never appeared; log:" >&2
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

stepped() {
    local log="$1"
    local label="$2"
    grep -a "animation stats ($label)" "$log" \
        | sed -n 's/.*frames_stepped=\([0-9][0-9]*\).*/\1/p' \
        | head -1
}

run_headless() {
    local hz="$1"
    local log="$SCRATCH/headless-$hz.log"
    start_session "$log" headless "$hz" 1
    snapshot
    send_command "animate-dummy $DURATION_MS"
    sleep 0.6
    snapshot
    stop_session
    local before after
    before="$(stepped "$log" SIGUSR1-1)"
    after="$(stepped "$log" SIGUSR1-2)"
    echo "$(( ${after:-0} - ${before:-0} ))"
}

# Part 1: the nested run. A nested host that does not present at 144 Hz can
# throttle the dummy animation's *rendered* frames; the derived interval and
# budget are still the policy evidence this section records.
NESTED_LOG="$SCRATCH/nested.log"
start_session "$NESTED_LOG" nested 144 "$SETTLE"
NESTED_SOCKET="$SOCKET"
snapshot
send_command "animate-dummy $DURATION_MS"
sleep 0.7
snapshot
stop_session
nested_before="$(stepped "$NESTED_LOG" SIGUSR1-1)"
nested_after="$(stepped "$NESTED_LOG" SIGUSR1-2)"
nested_delta=$(( ${nested_after:-0} - ${nested_before:-0} ))

# Part 2: headless cadence controls (no host vblank in the way).
headless_144="$(run_headless 144)"
headless_60="$(run_headless 60)"

{
    echo "T-21.2a simulated 144 Hz refresh trace (T187b)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "command: DRAGONFRUIT_REFRESH_HZ=<hz> target/debug/dragonfruit-compositor --backend <nested|headless>"
    echo "synthetic command: animate-dummy $DURATION_MS"
    echo "derived period: 1e12 / 144000 mHz = 6944 ns = 6.944 ms"
    echo "derived budget: 6944 us (degrade stats budget_us, source=refresh)"
    echo
    echo "## Nested simulated 144 Hz: derived interval and budget"
    echo "nested socket: $NESTED_SOCKET"
    grep -aE "refresh override active|refresh stats \(SIGUSR1-1\)|animation stats \(SIGUSR1-1\)|degrade stats \(SIGUSR1-1\)" "$NESTED_LOG" || true
    echo
    echo "## Headless simulated 144 Hz: clock cadence"
    grep -aE "refresh override active|refresh stats \(SIGUSR1-1\)|animation stats \(SIGUSR1-2\)|degrade stats \(SIGUSR1-1\)|frame timing \(SIGUSR1-2\)" "$SCRATCH/headless-144.log" || true
    echo "animation_frames_stepped delta=$headless_144 over ${DURATION_MS} ms (~$(( DURATION_MS / 7 )) frames at 6.944 ms)"
    echo
    echo "## Headless simulated 60 Hz: baseline cadence"
    grep -aE "refresh stats \(SIGUSR1-1\)|animation stats \(SIGUSR1-2\)|degrade stats \(SIGUSR1-1\)|frame timing \(SIGUSR1-2\)" "$SCRATCH/headless-60.log" || true
    echo "animation_frames_stepped delta=$headless_60 over ${DURATION_MS} ms (~$(( DURATION_MS / 17 )) frames at 16.666 ms)"
    echo
    echo "## Verdict"
    echo "nested derived-interval frames_stepped delta=$nested_delta (host-vblank throttled; interval/budget are the evidence)"
    local_ok=1
    if grep -aq "refresh stats (SIGUSR1-1): refresh_hz=144.000 source=override interval_us=6944 deadline_us=6944" "$NESTED_LOG" \
        && grep -aq "degrade stats (SIGUSR1-1): .*budget_us=6944 .*budget_source=refresh" "$NESTED_LOG"; then
        echo "nested interval/budget: PASS (refresh_hz=144.000 interval_us=6944 budget_us=6944)"
    else
        echo "nested interval/budget: FAIL"
        local_ok=0
    fi
    if [ "$headless_144" -ge 30 ] && [ "$headless_60" -ge 12 ] && [ "$headless_60" -le 24 ]; then
        echo "headless cadence: PASS (144 Hz delta=$headless_144 >= 30; 60 Hz delta=$headless_60 in 12..24)"
    else
        echo "headless cadence: FAIL (144 Hz delta=$headless_144, 60 Hz delta=$headless_60)"
        local_ok=0
    fi
    if [ "$local_ok" -eq 1 ]; then
        echo "PASS: the clock and degrade budget use the derived 144 Hz period."
    fi
} >"$OUT"

echo "t21-refresh-144: wrote $OUT"
if ! grep -aq "PASS: the clock and degrade budget use the derived 144 Hz period." "$OUT"; then
    exit 1
fi
