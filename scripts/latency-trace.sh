#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-03.1b input-to-photon latency capture (nested).
#
# Runs the nested demo (shell + Wayland app + X11 app) with the synthetic-input
# harness, injects real input through the compositor's normal input router, and
# reads the latency instrument back over `query latency`. The report — raw
# sorted samples plus the min/median/p95/max and an honest pass/fail against one
# 60 Hz frame — lands in docs/captures/t03-latency-nested.txt.
#
# Requires a host Wayland session and the toolchain tree built (`make build`);
# not part of `make e2e`. The headless half of the instrument is asserted by
# `compositor/tests/latency_trace.rs` in the normal test suite.
#
# Overrides: LATENCY_SAMPLES (default 50), SETTLE (default 9), OUT (report
# path), REFRESH_HZ (default 60).
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="${OUT:-docs/captures/t03-latency-nested.txt}"
SAMPLES="${LATENCY_SAMPLES:-50}"
SETTLE="${SETTLE:-9}"
REFRESH_HZ="${REFRESH_HZ:-60}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"

[ -n "${XDG_RUNTIME_DIR:-}" ] || {
    echo "latency-trace: XDG_RUNTIME_DIR is not set" >&2
    exit 1
}
[ -n "${WAYLAND_DISPLAY:-}" ] || {
    echo "latency-trace: no WAYLAND_DISPLAY — the nested latency capture needs a host session" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "latency-trace: python3 not found" >&2
    exit 1
}

socket="dragonfruit-t03-latency-$$"
synth="$XDG_RUNTIME_DIR/$socket.synth"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-latency.XXXXXX")"
log="$scratch/demo.log"
rm -f "$synth"

echo "latency-trace: starting the nested demo ($socket)"
DRAGONFRUIT_SYNTHETIC_INPUT="$synth" setsid make demo DEMO_ARGS="--socket-name $socket" \
    >"$log" 2>&1 &
pgid=$!

cleanup() {
    kill -TERM -"$pgid" 2>/dev/null || true
    sleep 1
    kill -KILL -"$pgid" 2>/dev/null || true
    rm -f "$synth"
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$scratch"
}
trap cleanup EXIT

for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$socket" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$synth" ] && break; sleep 0.1; done
if [ ! -e "$synth" ]; then
    echo "latency-trace: synthetic-input socket never appeared; demo log:" >&2
    tail -30 "$log" >&2
    exit 1
fi
sleep "$SETTLE"

echo "latency-trace: injecting input and reading the instrument"
python3 scripts/latency-probe.py --synth "$synth" --samples "$SAMPLES" \
    --refresh-hz "$REFRESH_HZ" --out "$OUT"
echo "latency-trace: report written to $OUT"
