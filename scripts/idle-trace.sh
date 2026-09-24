#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-03.1a idle/animation frame budget trace (the 60 s acceptance).
#
# Runs the headless idle trace for `DF_IDLE_TRACE_SECS` seconds (default 60)
# through `make idle-trace` and records the raw SIGUSR1 counters to
# docs/captures/t03-idle-trace.txt. The same test runs in `make e2e` at the
# short in-suite window; this script is the reproducible acceptance run whose
# numbers are quoted in PROGRESS.md.
#
# Requires no display. `DF_IDLE_TRACE_SECS=10 bash scripts/idle-trace.sh` for
# a quick check; `OUT=<path>` overrides the log destination.
set -euo pipefail

cd "$(dirname "$0")/.."

SECS="${DF_IDLE_TRACE_SECS:-60}"
OUT="${OUT:-docs/captures/t03-idle-trace.txt}"
mkdir -p "$(dirname "$OUT")"

echo "idle-trace: running the ${SECS} s headless idle trace"
make idle-trace IDLE_TRACE_SECS="$SECS" 2>&1 | tee "$OUT"
echo "idle-trace: raw counters written to $OUT"
