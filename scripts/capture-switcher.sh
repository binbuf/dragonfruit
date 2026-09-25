#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-06.2a/T-06.2b app-switcher live capture.
#
# Runs the nested demo twice — once normally, once with
# `accessibility.reduceMotion` on — opens the compositor-owned Cmd-Tab switcher
# over the synthetic-input harness with scripts/capture-switcher-driver.py,
# cycles the app selection and the within-app window cursor (Cmd+`), and
# screenshots the overlay with Spectacle. The compositor renders the live
# preview surfaces through the T-04 scene transform; the shell draws the
# centered app cards.
#
# The stills land in docs/captures/ as t06-app-switcher[-reduced]*.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# Qt/CMake tree built (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-switcher.XXXXXX")}"
SETTLE="${SETTLE:-9}"
mkdir -p "$SCRATCH"

for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-switcher: $tool not found — the walkthrough capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-switcher: python3 Pillow not found" >&2
    exit 1
}

# run_mode <prefix> <driver-flags...>
run_mode() {
    local prefix="$1"; shift
    local socket="dragonfruit-t06-capture-${prefix##*-}-$$"
    local synth="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$socket.synth"
    local log="$SCRATCH/$prefix.log"
    rm -f "$synth"

    echo "capture-switcher: starting the nested demo for $prefix (socket $socket)"
    DRAGONFRUIT_SYNTHETIC_INPUT="$synth" setsid make demo DEMO_ARGS="--socket-name $socket" \
        >"$log" 2>&1 &
    local pgid=$!

    for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$socket" ] && break; sleep 0.1; done
    for _ in $(seq 1 300); do [ -e "$synth" ] && break; sleep 0.1; done
    if [ ! -e "$synth" ]; then
        echo "capture-switcher: synthetic-input socket never appeared; demo log:" >&2
        tail -30 "$log" >&2
        kill -TERM -"$pgid" 2>/dev/null || true
        exit 1
    fi
    sleep "$SETTLE"

    echo "capture-switcher: driving the $prefix walkthrough"
    python3 scripts/capture-switcher-driver.py \
        --synth "$synth" --outdir "$OUTDIR" --scratch "$SCRATCH" \
        --prefix "$prefix" "$@"

    kill -TERM -"$pgid" 2>/dev/null || true
    for _ in $(seq 1 50); do
        grep -q "clean exit" "$log" 2>/dev/null && break
        sleep 0.2
    done
    kill -KILL -"$pgid" 2>/dev/null || true
    rm -f "$synth"
}

run_mode "t06-app-switcher"
run_mode "t06-app-switcher-reduced" --reduced

echo "capture-switcher: done (scratch $SCRATCH)"