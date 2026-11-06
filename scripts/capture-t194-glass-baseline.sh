#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T194 glass baseline capture.
#
# On a host with a usable Wayland screenshot path this runs the existing
# ``capture-t20-chrome-material.sh`` scenario set (Dock, menu bar, attached and
# detached menus, Control Center, banner; light and dark) into a scratch
# directory, then derives the T194 surface crops and baseline manifest from it.
#
# When the host path is unavailable the script does not fake a capture.  It
# writes ``docs/captures/t194-glass-baseline.open.txt`` with the exact probe
# commands and reason, and still records the committed v0.1.1 baseline crops
# and the T-21 benchmark counters through ``scripts/t194-baseline.py``.
#
# Requires a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree for the live half.  The OPEN half runs anywhere.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t194-baseline.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t194-baseline}"
SETTLE="${SETTLE:-16}"
OPEN="$OUTDIR/t194-glass-baseline.open.txt"
CAPTURE_NOTE="$OUTDIR/t194-glass-baseline.capture.txt"
SPECTACLE_LOG="$OUTDIR/t194-glass-baseline-spectacle-probe.log"
NESTED_LOG="$OUTDIR/t194-glass-baseline-nested-probe.log"

mkdir -p "$OUTDIR" "$SCRATCH"

write_open() {
    local reason="$1"
    {
        echo "T194 glass baseline — live host capture"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: $reason"
        echo "command: make glass-baseline-capture"
        echo "probe:"
        echo "  XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-<unset>}"
        echo "  WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
        echo "  spectacle=$(command -v spectacle || echo '<missing>')"
        echo "  nested_socket=$SOCKET"
        echo "  spectacle_probe_log=$SPECTACLE_LOG"
        echo "  nested_probe_log=$NESTED_LOG"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make glass-baseline-capture"
        echo
        echo "The committed v0.1.1 t20-chrome-* stills, the T-21 benchmark"
        echo "counters, and the native live-content moving-panel stills are still"
        echo "recorded in docs/captures/t194-glass-baseline.{json,md}."
        echo "A fresh live host recapture remains OPEN."
    } >"$OPEN"
    echo "capture-t194-glass-baseline: OPEN ($reason); wrote $OPEN" >&2
}

cleanup() {
    rm -f "$XDG_RUNTIME_DIR/$SOCKET-probe" \
          "$XDG_RUNTIME_DIR/$SOCKET-probe.lock" \
          "$XDG_RUNTIME_DIR/$SOCKET-probe.launch-token" \
          "$XDG_RUNTIME_DIR/$SOCKET-probe.synth" 2>/dev/null || true
    if [ "${KEEP_SCRATCH:-0}" != "1" ]; then
        rm -rf "$SCRATCH"
    fi
}
trap cleanup EXIT

host_reason=""
if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    host_reason="XDG_RUNTIME_DIR is not set"
elif [ -z "${WAYLAND_DISPLAY:-}" ]; then
    host_reason="no host Wayland session (WAYLAND_DISPLAY is unset)"
elif ! command -v spectacle >/dev/null 2>&1; then
    host_reason="spectacle is not installed"
elif [ ! -x target/debug/dragonfruit-compositor ]; then
    host_reason="target/debug/dragonfruit-compositor is missing; run make build"
fi

# Probe the host screenshot path first: a nested session is useless if the
# still cannot be captured.
if [ -z "$host_reason" ]; then
    if ! timeout 20 spectacle -b -n -f -o "$SCRATCH/spectacle-probe.png" \
            >"$SPECTACLE_LOG" 2>&1 \
        || [ ! -s "$SCRATCH/spectacle-probe.png" ]; then
        host_reason="spectacle could not capture the host session (probe exited non-zero or wrote no PNG)"
    fi
fi

# Probe the nested window mapping even when the screenshot probe already
# failed, so the OPEN artifact records both independent host limitations.  The
# compositor creates its Wayland socket before the backend init hook, so a
# missing socket within 10 s means the host compositor never mapped the nested
# window (locked/headless host or a broken host screenshot path).
if [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -n "${WAYLAND_DISPLAY:-}" ] \
        && [ -x target/debug/dragonfruit-compositor ]; then
    rm -f "$XDG_RUNTIME_DIR/$SOCKET-probe" \
          "$XDG_RUNTIME_DIR/$SOCKET-probe.lock" \
          "$XDG_RUNTIME_DIR/$SOCKET-probe.launch-token"
    DRAGONFRUIT_SYNTHETIC_INPUT="$XDG_RUNTIME_DIR/$SOCKET-probe.synth" \
        setsid target/debug/dragonfruit-compositor --backend nested \
        --socket-name "$SOCKET-probe" >"$NESTED_LOG" 2>&1 &
    probe_pid=$!
    for _ in $(seq 1 100); do
        [ -e "$XDG_RUNTIME_DIR/$SOCKET-probe" ] && break
        sleep 0.1
    done
    if [ ! -e "$XDG_RUNTIME_DIR/$SOCKET-probe" ]; then
        nested_reason="the nested compositor did not create its Wayland socket within 10 s (the host did not map the nested window)"
        if [ -n "$host_reason" ]; then
            host_reason="$host_reason; $nested_reason"
        else
            host_reason="$nested_reason"
        fi
        kill -TERM "$probe_pid" 2>/dev/null || true
        sleep 1
        kill -KILL "$probe_pid" 2>/dev/null || true
        wait "$probe_pid" 2>/dev/null || true
    else
        kill -TERM "$probe_pid" 2>/dev/null || true
        wait "$probe_pid" 2>/dev/null || true
    fi
fi

if [ -n "$host_reason" ]; then
    write_open "$host_reason"
    rm -f "$CAPTURE_NOTE"
    python3 scripts/t194-baseline.py --write
    python3 scripts/t194-baseline.py --check
    echo "capture-t194-glass-baseline: baseline written from committed evidence"
    exit 0
fi

echo "capture-t194-glass-baseline: host probes passed; running the existing t20 chrome capture"
mkdir -p "$SCRATCH/t20"
if ! OUTDIR="$SCRATCH/t20" SETTLE="$SETTLE" SOCKET="$SOCKET" \
        bash scripts/capture-t20-chrome-material.sh; then
    write_open "capture-t20-chrome-material.sh failed; see $SCRATCH for logs"
    rm -f "$CAPTURE_NOTE"
    python3 scripts/t194-baseline.py --write
    python3 scripts/t194-baseline.py --check
    exit 0
fi

rm -f "$OPEN"
{
    echo "T194 glass baseline — live host capture"
    echo "timestamp: $(date -Is)"
    echo "verdict: PASS"
    echo "command: make glass-baseline-capture"
    echo "source_dir: $SCRATCH/t20"
    echo "note: the T-21 benchmark counters remain the timing reference until a fresh benchmark run."
} >"$CAPTURE_NOTE"

python3 scripts/t194-baseline.py --write --source-dir "$SCRATCH/t20"
python3 scripts/t194-baseline.py --check
echo "capture-t194-glass-baseline: done"
