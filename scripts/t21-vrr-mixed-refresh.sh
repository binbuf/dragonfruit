#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.2b VRR, chrome-callback, and mixed-refresh trace (T188c).
#
# Part 1 runs one nested compositor + shell at 144 Hz. It records the
# single-output presentation clock, the `vrr=unsupported` report (the nested
# rail has no hardware VRR interval), and the chrome frame-callback counter
# before/after a Dock sweep. The nested rail is a single-output correctness
# trace, not mixed-refresh proof.
#
# Part 2 runs the headless backend with the default 60 Hz output plus a
# synthetic 144 Hz output. Three scoped dummy animations isolate the evidence:
# only the 144 Hz output animates (the 60 Hz output must stay flat), only the
# 60 Hz output animates (the 144 Hz output must stay flat), and both animate
# (each keeps its own cadence). A final idle window proves both counters go
# flat once the animation settles. This is synthetic mixed-refresh evidence,
# not native.
#
# Part 3 records the native mixed-refresh half. On a host already running a
# native session the running compositor cannot be restarted without ending the
# session, so the artifact is an explicit OPEN with the exact rerun command.
#
# Requires a host Wayland session and the built tree. Not part of `make e2e`.
# Overrides: OUT (report path), NATIVE_OUT (OPEN artifact path),
# SETTLE (nested settle, default 2 s), DURATION_MS (default 800),
# KEEP_SCRATCH.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="${OUT:-docs/captures/t21-vrr-mixed-refresh.txt}"
NATIVE_OUT="${NATIVE_OUT:-docs/captures/t21-vrr-mixed-refresh-native.txt}"
SETTLE="${SETTLE:-2}"
DURATION_MS="${DURATION_MS:-800}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-vrr.XXXXXX")}"
SOCKET=""
SYNTH=""
OUTPUT_SOCK=""
COMPOSITOR_PID=""
SHELL_PID=""

mkdir -p "$SCRATCH" "$(dirname "$OUT")" "$(dirname "$NATIVE_OUT")"

cleanup() {
    if [ -n "$SHELL_PID" ]; then
        kill -TERM "$SHELL_PID" 2>/dev/null || true
        wait "$SHELL_PID" 2>/dev/null || true
    fi
    if [ -n "$COMPOSITOR_PID" ]; then
        kill -TERM "$COMPOSITOR_PID" 2>/dev/null || true
        wait "$COMPOSITOR_PID" 2>/dev/null || true
    fi
    if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
        rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" \
              "$SYNTH" "$OUTPUT_SOCK" 2>/dev/null || true
    fi
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

write_open() {
    {
        echo "T-21.2b VRR and mixed-refresh trace (T188c)"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: $1"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make t21-vrr-mixed-refresh"
    } >"$OUT"
    echo "t21-vrr-mixed-refresh: OPEN ($1); wrote $OUT" >&2
    exit 0
}

[ -n "${XDG_RUNTIME_DIR:-}" ] || write_open "XDG_RUNTIME_DIR is not set"
[ -n "${WAYLAND_DISPLAY:-}" ] || write_open "no host Wayland session (WAYLAND_DISPLAY is unset)"
[ -x target/debug/dragonfruit-compositor ] || {
    echo "t21-vrr-mixed-refresh: target/debug/dragonfruit-compositor missing; run make build" >&2
    exit 1
}
[ -x build/shell/src/dragonfruit-shell ] || {
    echo "t21-vrr-mixed-refresh: build/shell/src/dragonfruit-shell missing; run make build" >&2
    exit 1
}

send_datagram() {
    python3 - "$1" "$2" <<'PY'
import socket
import sys

sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.sendto(sys.argv[2].encode() + b"\n", sys.argv[1])
PY
}

wait_for_path() {
    local path="$1" tries="${2:-300}"
    for _ in $(seq 1 "$tries"); do
        [ -e "$path" ] && return 0
        sleep 0.1
    done
    return 1
}

start_compositor() {
    local backend="$1" log="$2"
    shift 2
    SOCKET="dragonfruit-t21-vrr-${backend}-$$"
    SYNTH="$XDG_RUNTIME_DIR/$SOCKET.synth"
    OUTPUT_SOCK="$XDG_RUNTIME_DIR/$SOCKET.out"
    rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock" "$SYNTH" "$OUTPUT_SOCK"
    env "$@" \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        DRAGONFRUIT_SYNTHETIC_OUTPUT="$OUTPUT_SOCK" \
        target/debug/dragonfruit-compositor --backend "$backend" --socket-name "$SOCKET" \
        >"$log" 2>&1 &
    COMPOSITOR_PID=$!
    if ! wait_for_path "$XDG_RUNTIME_DIR/$SOCKET"; then
        echo "t21-vrr-mixed-refresh: $backend socket never appeared; log:" >&2
        tail -40 "$log" >&2
        exit 1
    fi
    if ! wait_for_path "$SYNTH" 100; then
        echo "t21-vrr-mixed-refresh: $backend synthetic-input socket never appeared; log:" >&2
        tail -40 "$log" >&2
        exit 1
    fi
}

stop_compositor() {
    [ -n "$COMPOSITOR_PID" ] || return 0
    kill -TERM "$COMPOSITOR_PID" 2>/dev/null || true
    for _ in $(seq 1 150); do
        kill -0 "$COMPOSITOR_PID" 2>/dev/null || break
        sleep 0.1
    done
    wait "$COMPOSITOR_PID" 2>/dev/null || true
    COMPOSITOR_PID=""
}

start_shell() {
    local log="$1"
    env DRAGONFRUIT_CHROME_STATS=1 WAYLAND_DISPLAY="$SOCKET" \
        build/shell/src/dragonfruit-shell --socket-name "$SOCKET" >"$log" 2>&1 &
    SHELL_PID=$!
    for _ in $(seq 1 300); do
        grep -aq "menu bar configured" "$log" 2>/dev/null && return 0
        if ! kill -0 "$SHELL_PID" 2>/dev/null; then
            echo "t21-vrr-mixed-refresh: shell exited early; log:" >&2
            tail -40 "$log" >&2
            exit 1
        fi
        sleep 0.1
    done
    echo "t21-vrr-mixed-refresh: shell never configured its menu bar; log:" >&2
    tail -40 "$log" >&2
    exit 1
}

stop_shell() {
    [ -n "$SHELL_PID" ] || return 0
    kill -TERM "$SHELL_PID" 2>/dev/null || true
    wait "$SHELL_PID" 2>/dev/null || true
    SHELL_PID=""
}

latest_line() {
    local pattern="$1" log="$2"
    grep -a "$pattern" "$log" | tail -1 || true
}

wait_for_stat() {
    local log="$1" label="$2"
    for _ in $(seq 1 120); do
        grep -aq "frame scheduler stats ($label)" "$log" 2>/dev/null && return 0
        sleep 0.1
    done
    return 1
}

snapshot() {
    local log="$1" label="$2"
    kill -USR1 "$COMPOSITOR_PID" 2>/dev/null || true
    wait_for_stat "$log" "$label" || true
    sleep 0.15
}

# Poll snapshots until the frame scheduler has no animation request and no
# armed timer. The nested host can throttle submit() to ~1 Hz, so a fixed
# sleep is not enough there.
wait_animation_idle() {
    local log="$1" prefix="$2"
    local attempt
    for attempt in $(seq 1 12); do
        snapshot "$log" "${prefix}-${attempt}"
        if [ "$(scheduler_field "$log" animation)" = "0" ] \
            && [ "$(scheduler_field "$log" timer_armed)" = "0" ]; then
            return 0
        fi
        sleep 1
    done
    return 1
}

# Read one field from the latest `frame request` line for `output`.
request_field() {
    local log="$1" output="$2" field="$3"
    grep -a "frame request .*output=$output " "$log" \
        | tail -1 \
        | sed -n "s/.*$field=\\([-0-9][0-9]*\\).*/\\1/p" \
        || true
}

# Read one field from the latest `chrome stats` line.
chrome_field() {
    local log="$1" field="$2"
    grep -a "chrome stats" "$log" \
        | tail -1 \
        | sed -n "s/.*$field=\\([0-9][0-9]*\\).*/\\1/p" \
        || true
}

# Read one field from the latest `frame scheduler stats` line.
scheduler_field() {
    local log="$1" field="$2"
    grep -a "frame scheduler stats" "$log" \
        | tail -1 \
        | sed -n "s/.*$field=\\([0-9][0-9]*\\).*/\\1/p" \
        || true
}

# ---------------------------------------------------------------------------
# Part 1: nested single-output trace (144 Hz) + chrome callbacks.
# ---------------------------------------------------------------------------
NESTED_LOG="$SCRATCH/nested.log"
SHELL_LOG="$SCRATCH/shell.log"
NESTED_SOCKET=""
start_compositor nested "$NESTED_LOG" DRAGONFRUIT_REFRESH_HZ=144
NESTED_SOCKET="$SOCKET"
start_shell "$SHELL_LOG"
sleep "$SETTLE"
wait_animation_idle "$NESTED_LOG" SIGUSR1-1 || true

nested_idle_presentations="$(request_field "$NESTED_LOG" NESTED-1 presentations)"
if [ -z "$nested_idle_presentations" ]; then
    # Fall back to the only `frame request` line if the backend names its
    # output differently.
    nested_idle_presentations="$(latest_line 'frame request' "$NESTED_LOG" \
        | sed -n 's/.*presentations=\([0-9][0-9]*\).*/\1/p')"
fi
nested_idle_presentations="${nested_idle_presentations:-0}"
chrome_before="$(chrome_field "$NESTED_LOG" chrome_frame_callbacks)"
nested_idle_line="$(latest_line 'frame request' "$NESTED_LOG")"
nested_idle_scheduler="$(latest_line 'frame scheduler stats' "$NESTED_LOG")"

# A Dock sweep makes the shell animate and request `wl_surface.frame`; the
# compositor answers those callbacks on each presented frame.
python3 - "$SYNTH" <<'PY'
import math
import socket
import sys
import time

synth = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
for index in range(70):
    x = 0.38 + 0.25 * (0.5 + 0.5 * math.sin(index / 6.0))
    sock.sendto(f"motion-abs {x:.6f} 0.96\n".encode(), synth)
    time.sleep(0.02)
PY
# The Control Center chord opens an animated shell overlay. It is the most
# deterministic chrome animation in the nested rail: the menu bar/Dock sweep
# can settle between samples, while the panel's open transition commits a
# fresh chrome frame with a pending callback.
send_datagram "$SYNTH" "key 29 down"
send_datagram "$SYNTH" "key 56 down"
send_datagram "$SYNTH" "key 46 down"
sleep 0.05
send_datagram "$SYNTH" "key 46 up"
send_datagram "$SYNTH" "key 56 up"
send_datagram "$SYNTH" "key 29 up"
sleep 0.6
snapshot "$NESTED_LOG" SIGUSR1-2
nested_live_line="$(latest_line 'frame request' "$NESTED_LOG")"
nested_live_scheduler="$(latest_line 'frame scheduler stats' "$NESTED_LOG")"
chrome_after="$(chrome_field "$NESTED_LOG" chrome_frame_callbacks)"
nested_live_presentations="$(latest_line 'frame request' "$NESTED_LOG" \
    | sed -n 's/.*presentations=\([0-9][0-9]*\).*/\1/p')"

wait_animation_idle "$NESTED_LOG" SIGUSR1-3 || true
nested_settled_scheduler="$(latest_line 'frame scheduler stats' "$NESTED_LOG")"
nested_settled_line="$(latest_line 'frame request' "$NESTED_LOG")"
stop_shell
stop_compositor

chrome_before="${chrome_before:-0}"
chrome_after="${chrome_after:-0}"
chrome_delta=$(( chrome_after - chrome_before ))
nested_presentation_delta=$(( ${nested_live_presentations:-0} - nested_idle_presentations ))

# ---------------------------------------------------------------------------
# Part 2: headless synthetic mixed refresh, 60 Hz + 144 Hz.
# ---------------------------------------------------------------------------
MIXED_LOG="$SCRATCH/mixed.log"
start_compositor headless "$MIXED_LOG"
if ! wait_for_path "$OUTPUT_SOCK" 100; then
    echo "t21-vrr-mixed-refresh: synthetic-output socket never appeared; log:" >&2
    tail -40 "$MIXED_LOG" >&2
    exit 1
fi
send_datagram "$OUTPUT_SOCK" "add MIXED-144 1280 720 1280 0 1.0 144000"
sleep 1

snapshot "$MIXED_LOG" SIGUSR1-1
slow_before="$(request_field "$MIXED_LOG" HEADLESS-1 presentations)"
fast_before="$(request_field "$MIXED_LOG" MIXED-144 presentations)"
idle_line_before="$(grep -a 'frame request' "$MIXED_LOG" | tail -2 || true)"

# Scenario A: only the 144 Hz output animates. The 60 Hz output must stay flat.
send_datagram "$SYNTH" "animate-dummy-on MIXED-144 $DURATION_MS"
sleep 0.35
snapshot "$MIXED_LOG" SIGUSR1-2
slow_after_fast_live="$(request_field "$MIXED_LOG" HEADLESS-1 presentations)"
fast_after_fast_live="$(request_field "$MIXED_LOG" MIXED-144 presentations)"
fast_only_lines="$(grep -a 'frame request' "$MIXED_LOG" | tail -2 || true)"
fast_only_scheduler="$(latest_line 'frame scheduler stats' "$MIXED_LOG")"
fast_interval="$(request_field "$MIXED_LOG" MIXED-144 last_interval_us)"
wait_animation_idle "$MIXED_LOG" SIGUSR1-A-settle || true
slow_after_fast="$(request_field "$MIXED_LOG" HEADLESS-1 presentations)"
fast_after_fast="$(request_field "$MIXED_LOG" MIXED-144 presentations)"

# Scenario B: only the 60 Hz output animates. The 144 Hz output must stay flat.
send_datagram "$SYNTH" "animate-dummy-on HEADLESS-1 $DURATION_MS"
sleep 0.35
snapshot "$MIXED_LOG" SIGUSR1-3
slow_after_slow_live="$(request_field "$MIXED_LOG" HEADLESS-1 presentations)"
fast_after_slow_live="$(request_field "$MIXED_LOG" MIXED-144 presentations)"
slow_only_lines="$(grep -a 'frame request' "$MIXED_LOG" | tail -2 || true)"
slow_only_scheduler="$(latest_line 'frame scheduler stats' "$MIXED_LOG")"
slow_interval="$(request_field "$MIXED_LOG" HEADLESS-1 last_interval_us)"
wait_animation_idle "$MIXED_LOG" SIGUSR1-B-settle || true
slow_after_slow="$(request_field "$MIXED_LOG" HEADLESS-1 presentations)"
fast_after_slow="$(request_field "$MIXED_LOG" MIXED-144 presentations)"

# Scenario C: both outputs animate on the same clock; each keeps its cadence.
send_datagram "$SYNTH" "animate-dummy $DURATION_MS"
sleep 0.35
snapshot "$MIXED_LOG" SIGUSR1-4
slow_mixed_live="$(request_field "$MIXED_LOG" HEADLESS-1 presentations)"
fast_mixed_live="$(request_field "$MIXED_LOG" MIXED-144 presentations)"
mixed_live_lines="$(grep -a 'frame request' "$MIXED_LOG" | tail -2 || true)"
mixed_scheduler_live="$(latest_line 'frame scheduler stats' "$MIXED_LOG")"
wait_animation_idle "$MIXED_LOG" SIGUSR1-C-settle || true
slow_after_mixed="$(request_field "$MIXED_LOG" HEADLESS-1 presentations)"
fast_after_mixed="$(request_field "$MIXED_LOG" MIXED-144 presentations)"

# Final idle window: both counters must be flat after the animation settles.
sleep 1
snapshot "$MIXED_LOG" SIGUSR1-6
slow_idle="$(request_field "$MIXED_LOG" HEADLESS-1 presentations)"
fast_idle="$(request_field "$MIXED_LOG" MIXED-144 presentations)"
mixed_settled_lines="$(grep -a 'frame request' "$MIXED_LOG" | tail -2 || true)"
mixed_scheduler_settled="$(latest_line 'frame scheduler stats' "$MIXED_LOG")"
stop_compositor

slow_before="${slow_before:-0}"
fast_before="${fast_before:-0}"
slow_after_fast_live="${slow_after_fast_live:-0}"
fast_after_fast_live="${fast_after_fast_live:-0}"
slow_after_slow_live="${slow_after_slow_live:-0}"
fast_after_slow_live="${fast_after_slow_live:-0}"
slow_after_fast="${slow_after_fast:-0}"
fast_after_fast="${fast_after_fast:-0}"
slow_after_slow="${slow_after_slow:-0}"
fast_after_slow="${fast_after_slow:-0}"
slow_mixed_live="${slow_mixed_live:-0}"
fast_mixed_live="${fast_mixed_live:-0}"
slow_after_mixed="${slow_after_mixed:-0}"
fast_after_mixed="${fast_after_mixed:-0}"
slow_idle="${slow_idle:-0}"
fast_idle="${fast_idle:-0}"

fast_only_slow_delta=$(( slow_after_fast_live - slow_before ))
fast_only_fast_delta=$(( fast_after_fast_live - fast_before ))
slow_only_slow_delta=$(( slow_after_slow_live - slow_after_fast ))
slow_only_fast_delta=$(( fast_after_slow_live - fast_after_fast ))
mixed_slow_delta=$(( slow_mixed_live - slow_after_slow ))
mixed_fast_delta=$(( fast_mixed_live - fast_after_slow ))
idle_slow_delta=$(( slow_idle - slow_after_mixed ))
idle_fast_delta=$(( fast_idle - fast_after_mixed ))

# ---------------------------------------------------------------------------
# Part 3: native mixed-refresh evidence or an explicit OPEN marker.
# ---------------------------------------------------------------------------
CONNECTED="$(for status in /sys/class/drm/card*-*/status; do
    [ -r "$status" ] || continue
    [ "$(cat "$status" 2>/dev/null)" = "connected" ] || continue
    basename "$(dirname "$status")"
done | tr '\n' ' ' | sed 's/ $//')"
NATIVE_REASON=""
if [ "${DRAGONFRUIT_T21_NATIVE:-0}" = "1" ]; then
    NATIVE_REASON="the native two-output run is not automated in this task; run the DRM rail with a 60 Hz + 144 Hz seat and attach its presentation stats"
elif [ "${XDG_CURRENT_DESKTOP:-}" = "dragonfruit" ]; then
    NATIVE_REASON="the host is already a native Dragonfruit session; restarting its compositor to run the 60 Hz + 144 Hz trace would end the session this task is executing in"
else
    NATIVE_REASON="no native DRM seat with a 60 Hz + 144 Hz pair was available to this session"
fi
NATIVE_DRM="$(ls /dev/dri 2>/dev/null | tr '\n' ' ' || true)"
{
    echo "T-21.2b native mixed-refresh trace (T188c)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>} XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-<unset>}"
    echo "DRM nodes: ${NATIVE_DRM:-<none>}"
    echo "connected outputs: ${CONNECTED:-<none>}"
    echo "verdict: OPEN"
    echo "reason: $NATIVE_REASON"
    echo "rerun: on a free DRM seat with a 60 Hz + 144 Hz pair, run"
    echo "  DRAGONFRUIT_T21_NATIVE=1 make t21-vrr-mixed-refresh"
    echo "  (or attach the DRM 'presentation stats' lines from a native two-output session)"
} >"$NATIVE_OUT"

# ---------------------------------------------------------------------------
# Report.
# ---------------------------------------------------------------------------
{
    echo "T-21.2b VRR, chrome callbacks, and mixed-refresh trace (T188c)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>} XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-<unset>} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "DRM nodes: ${NATIVE_DRM:-<none>}; connected outputs: ${CONNECTED:-<none>}"
    echo "command: DRAGONFRUIT_REFRESH_HZ=144 target/debug/dragonfruit-compositor --backend nested; target/debug/dragonfruit-compositor --backend headless"
    echo
    echo "## 1. Nested single-output (144 Hz): VRR report and chrome callbacks"
    echo "nested socket: $NESTED_SOCKET (compositor + shell)"
    echo "idle sample:"
    echo "  $nested_idle_line"
    echo "  $nested_idle_scheduler"
    echo "live sample (Dock sweep):"
    echo "  $nested_live_line"
    echo "  $nested_live_scheduler"
    echo "settled sample:"
    echo "  $nested_settled_line"
    echo "  $nested_settled_scheduler"
    echo "chrome_frame_callbacks: before=$chrome_before after=$chrome_after delta=$chrome_delta"
    echo "presentations delta during the sweep: $nested_presentation_delta"
    echo "note: the nested host throttles submit() (last_interval_us is ~1 s);"
    echo "      the derived 144 Hz period and VRR report are the single-output evidence."
    echo
    echo "## 2. Headless synthetic mixed refresh: 60 Hz + 144 Hz"
    echo "outputs: HEADLESS-1 (60 Hz) and MIXED-144 (144 Hz)"
    echo "before:"
    echo "$idle_line_before" | sed 's/^/  /'
    echo "A. only 144 Hz animates (${DURATION_MS} ms):"
    echo "$fast_only_lines" | sed 's/^/  /'
    echo "  $fast_only_scheduler"
    echo "  presentations delta: 60 Hz=$fast_only_slow_delta (idle, must be 0), 144 Hz=$fast_only_fast_delta"
    echo "  last_interval_us: 144 Hz=$fast_interval"
    echo "B. only 60 Hz animates (${DURATION_MS} ms):"
    echo "$slow_only_lines" | sed 's/^/  /'
    echo "  $slow_only_scheduler"
    echo "  presentations delta: 60 Hz=$slow_only_slow_delta, 144 Hz=$slow_only_fast_delta (idle, must be 0)"
    echo "  last_interval_us: 60 Hz=$slow_interval"
    echo "C. both animate on the same clock (${DURATION_MS} ms):"
    echo "$mixed_live_lines" | sed 's/^/  /'
    echo "  $mixed_scheduler_live"
    echo "  presentations delta: 60 Hz=$mixed_slow_delta, 144 Hz=$mixed_fast_delta"
    echo "settled:"
    echo "$mixed_settled_lines" | sed 's/^/  /'
    echo "  $mixed_scheduler_settled"
    echo "presentations delta after settling: 60 Hz=$idle_slow_delta 144 Hz=$idle_fast_delta"
    echo
    echo "## 3. Native mixed refresh"
    echo "verdict: OPEN"
    echo "artifact: $NATIVE_OUT"
    echo "reason: $NATIVE_REASON"
    echo "rerun: on a free DRM seat with a 60 Hz + 144 Hz pair, run"
    echo "  DRAGONFRUIT_T21_NATIVE=1 make t21-vrr-mixed-refresh"
    echo "  (or attach the DRM 'presentation stats' lines from a native two-output session)"
    echo
    echo "## Verdict"
    ok=1
    if [ "$chrome_delta" -gt 0 ]; then
        echo "chrome callbacks: PASS (delta=$chrome_delta on presented nested frames)"
    else
        echo "chrome callbacks: FAIL (no chrome frame callbacks observed)"
        ok=0
    fi
    if [ -n "$nested_idle_line" ] && grep -aq "vrr=unsupported" <<<"$nested_idle_line"; then
        echo "nested VRR report: PASS (vrr=unsupported, fixed 144 Hz period)"
    else
        echo "nested VRR report: FAIL (expected vrr=unsupported)"
        ok=0
    fi
    if grep -aq "animation=0" <<<"$nested_settled_scheduler" \
        && grep -aq "timer_armed=0" <<<"$nested_settled_scheduler"; then
        echo "nested settle: PASS (animation=0 timer_armed=0)"
    else
        echo "nested settle: FAIL ($nested_settled_scheduler)"
        ok=0
    fi
    if [ "$fast_only_slow_delta" -eq 0 ] \
        && [ "$fast_only_fast_delta" -ge 10 ] \
        && [ "${fast_interval:-0}" -ge 5000 ] && [ "${fast_interval:-0}" -le 12000 ]; then
        echo "144 Hz-only animation: PASS (60 Hz stayed idle, 144 Hz cadence ~${fast_interval} us)"
    else
        echo "144 Hz-only animation: FAIL (60 Hz delta=$fast_only_slow_delta, 144 Hz delta=$fast_only_fast_delta, interval=${fast_interval:-?})"
        ok=0
    fi
    if [ "$slow_only_fast_delta" -eq 0 ] \
        && [ "$slow_only_slow_delta" -ge 5 ] \
        && [ "${slow_interval:-0}" -ge 12000 ] && [ "${slow_interval:-0}" -le 30000 ]; then
        echo "60 Hz-only animation: PASS (144 Hz stayed idle, 60 Hz cadence ~${slow_interval} us)"
    else
        echo "60 Hz-only animation: FAIL (60 Hz delta=$slow_only_slow_delta, 144 Hz delta=$slow_only_fast_delta, interval=${slow_interval:-?})"
        ok=0
    fi
    if [ "$mixed_slow_delta" -gt 0 ] && [ "$mixed_fast_delta" -gt "$mixed_slow_delta" ]; then
        echo "mixed cadence: PASS (60 Hz=$mixed_slow_delta, 144 Hz=$mixed_fast_delta in the same window)"
    else
        echo "mixed cadence: FAIL (60 Hz=$mixed_slow_delta, 144 Hz=$mixed_fast_delta)"
        ok=0
    fi
    if [ "$idle_slow_delta" -eq 0 ] && [ "$idle_fast_delta" -eq 0 ]; then
        echo "settled outputs: PASS (both presentation counters flat)"
    else
        echo "settled outputs: FAIL (60 Hz=$idle_slow_delta, 144 Hz=$idle_fast_delta)"
        ok=0
    fi
    if grep -aq "vrr=unsupported" <<<"$mixed_live_lines"; then
        echo "synthetic VRR report: PASS (vrr=unsupported on both outputs)"
    else
        echo "synthetic VRR report: FAIL (expected vrr=unsupported)"
        ok=0
    fi
    if [ "$ok" -eq 1 ]; then
        echo "PASS: chrome callbacks follow presented frames; mixed refresh keeps per-output cadence; VRR is not claimed without backend support."
    fi
} >"$OUT"

echo "t21-vrr-mixed-refresh: wrote $OUT and $NATIVE_OUT"
if ! grep -aq "PASS: chrome callbacks follow presented frames" "$OUT"; then
    exit 1
fi
