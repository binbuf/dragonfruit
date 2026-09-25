#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-07.5a/T-07.5b Wi-Fi, volume, and battery status-menu capture.
#
# Launches the nested demo with the compositor's synthetic-input harness,
# clicks the Wi-Fi, volume, and battery status items, and screenshots each
# popover with Spectacle into docs/captures/. `--placeholders` is gone, so the
# script sets `DF_STATUS_FIXTURE=1` to serve the bridge host's fixture views
# (a present 82% battery among them; the CI/this host has no battery). Not
# part of `make e2e`: CI has no host session and no screenshot tool.
#
# Requires: a host Wayland session, `spectacle`, and the built tree
# (`make build`).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SOCKET="${SOCKET:-dragonfruit-t07.5a-capture}"
SETTLE="${SETTLE:-8}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-status-capture.XXXXXX")}"

cleanup() {
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

command -v spectacle >/dev/null 2>&1 || {
    echo "capture-status-menus: spectacle not found — needs a host session" >&2
    exit 1
}

rm -f "$SYNTH"
LOG="$SCRATCH/demo.log"
echo "capture-status-menus: starting the nested demo (socket $SOCKET)"
DF_STATUS_FIXTURE=1 DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!

for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-status-menus: synthetic-input socket never appeared; demo log:" >&2
    tail -30 "$LOG" >&2
    exit 1
fi

python3 scripts/capture-status-menus-driver.py \
    --synth "$SYNTH" --outdir "$OUTDIR" --settle "$SETTLE"

echo "capture-status-menus: done (${OUTDIR}/t07.5a-wifi.png, ${OUTDIR}/t07.5a-volume.png, ${OUTDIR}/t07.5b-battery.png)"