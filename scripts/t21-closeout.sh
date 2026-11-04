#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.1c close-out live checks (T185a): visual sweep, shell kill/restart, and
# the lock-surface smoke test on one nested demo. Writes the stills and the
# transcript under docs/captures/.
#
# Requires a host Wayland session and the built tree (`make build`). Not part
# of `make e2e`. Overrides: SETTLE (default 8), KEEP_SCRATCH, SOCKET.
set -euo pipefail

cd "$(dirname "$0")/.."

SETTLE="${SETTLE:-8}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
SOCKET="${SOCKET:-dragonfruit-t21-closeout-$$}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-closeout.XXXXXX")}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"
SHOTS="$SCRATCH/shots"
LOG="$SCRATCH/demo.log"
SHELL_LOG="$SCRATCH/shell.log"

mkdir -p "$SCRATCH" "$SHOTS"
rm -f "$SYNTH"

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

echo "t21-closeout: starting the nested demo ($SOCKET)"
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
    echo "t21-closeout: synthetic-input socket never appeared; demo log:" >&2
    tail -40 "$LOG" >&2
    exit 1
fi
ready=0
for _ in $(seq 1 2400); do
    if [ -f "$SHELL_LOG" ] && grep -q "Dock scene-graph commit path active" "$SHELL_LOG"; then
        ready=1
        break
    fi
    sleep 0.1
done
if [ "$ready" != "1" ]; then
    echo "t21-closeout: the shell never reached its chrome commit path; demo log:" >&2
    tail -60 "$LOG" >&2
    [ -f "$SHELL_LOG" ] && tail -40 "$SHELL_LOG" >&2
    exit 1
fi
sleep "$SETTLE"

SHELL_PID="$(pgrep -f "dragonfruit-shell --socket-name $SOCKET" | head -1 || true)"
COMPOSITOR_PID="$(pgrep -f "dragonfruit-compositor --backend nested --socket-name $SOCKET" | head -1 || true)"
if [ -z "$SHELL_PID" ] || [ -z "$COMPOSITOR_PID" ]; then
    echo "t21-closeout: shell/compositor pid not found" >&2
    pgrep -af "dragonfruit.*$SOCKET" >&2 || true
    exit 1
fi

python3 scripts/t21-closeout-driver.py \
    --synth "$SYNTH" \
    --socket "$SOCKET" \
    --shell-pid "$SHELL_PID" \
    --compositor-pid "$COMPOSITOR_PID" \
    --screenshot-dir "$SHOTS" \
    --log "$LOG" \
    --shell-log "$SHELL_LOG" \
    --outdir docs/captures

echo "t21-closeout: transcript written to docs/captures/t21-chrome-closeout.txt"
