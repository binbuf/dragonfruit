#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-17.6 premium-gate sign-off capture.
#
# Runs the assembled desktop (`make demo`) in a live nested session and captures
# the whole nested output to docs/captures/t17-premium-gate.png, then re-runs
# the fast headless halves of the premium-gate checklist and writes
# docs/captures/t17-premium-gate.txt. The reviewed sign-off report is
# docs/captures/t17-premium-gate.md; this script is its reproducible evidence.
#
# The full checklist lives in docs/design/tracks/17-premium-gate.md. The
# per-behaviour captures it indexes are the T-17.1a/1b/1c, T-17.3, T-17.5a,
# and T-17.5b units (`make t17-window-loop-capture`, `make
# t17-navigation-capture`, `make t17-flatpak-browser-capture`, `make
# t17-visual-floor-capture`, `make t17-robustness-matrix`, `make
# t17-leak-lock-soak`).
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
#
# Overrides: OUTDIR, SCRATCH, KEEP_SCRATCH, SETTLE, SKIP_LIVE=1.
set -uo pipefail

cd "$(dirname "$0")/.."
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-12}"
SKIP_LIVE="${SKIP_LIVE:-0}"
SOCKET="${SOCKET:-dragonfruit-t17-premium-gate}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t17-gate.XXXXXX")}"
TRANSCRIPT="$OUTDIR/t17-premium-gate.txt"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"
fail=0

mkdir -p "$OUTDIR" "$SCRATCH"

section() { printf '\n================================================================\n## %s\n================================================================\n\n' "$*" >>"$SCRATCH/gate.txt"; }
say() { printf '%s\n' "$*" >>"$SCRATCH/gate.txt"; }

run_check() {
    local label="$1"
    shift
    {
        printf '$ %s\n' "$*"
    } >>"$SCRATCH/gate.txt"
    if "$@" >>"$SCRATCH/gate.txt" 2>&1; then
        printf '\n[PASS] %s\n' "$label" >>"$SCRATCH/gate.txt"
    else
        printf '\n[FAIL] %s\n' "$label" >>"$SCRATCH/gate.txt"
        fail=1
    fi
}

DEMO_PGID=""
cleanup() {
    if [ -n "$DEMO_PGID" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

{
    echo "T-17.6 premium-gate sign-off verification"
    echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "checklist: docs/design/tracks/17-premium-gate.md"
} >"$SCRATCH/gate.txt"

if [ "$SKIP_LIVE" != "1" ]; then
    for tool in spectacle python3; do
        command -v "$tool" >/dev/null 2>&1 || {
            echo "capture-t17-premium-gate: $tool not found — the capture needs a host session" >&2
            exit 1
        }
    done
    python3 -c 'import PIL' 2>/dev/null || {
        echo "capture-t17-premium-gate: python3 Pillow not found" >&2
        exit 1
    }

    rm -f "$SYNTH"
    echo "capture-t17-premium-gate: starting the nested demo (socket $SOCKET)"
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" >"$SCRATCH/demo.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-t17-premium-gate: synthetic socket never appeared; demo log:" >&2
        tail -30 "$SCRATCH/demo.log" >&2
        exit 1
    fi
    sleep "$SETTLE"

    echo "capture-t17-premium-gate: capturing the assembled desktop"
    section "1. Live assembled desktop"
    if python3 scripts/t17-premium-gate-driver.py --synth "$SYNTH" \
        --outdir "$OUTDIR" --scratch "$SCRATCH" --prefix t17-premium-gate; then
        if [ -f "$SCRATCH/premium-gate.txt" ]; then
            cat "$SCRATCH/premium-gate.txt" >>"$SCRATCH/gate.txt"
        fi
        printf '\n[PASS] nested desktop captured to %s/t17-premium-gate.png\n' "$OUTDIR" \
            >>"$SCRATCH/gate.txt"
    else
        printf '\n[FAIL] the live nested capture failed\n' >>"$SCRATCH/gate.txt"
        fail=1
    fi

    # Tear the demo down before the headless rows so the two do not contend.
    kill -TERM -"$DEMO_PGID" 2>/dev/null || true
    sleep 2
    kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    DEMO_PGID=""
else
    say "1. Live assembled desktop"
    say "[SKIP] SKIP_LIVE=1: the live capture was skipped; see the committed"
    say "       t17-premium-gate.png and the report section 'Live visual check'."
fi

section "2. Headless premium-gate floor"
run_check "gallery visual regression (84 snapshots)" \
    ./scripts/check-gallery-snapshots.py --strict
run_check "reduced-motion structural sweep" \
    cargo test -p dragonfruit-compositor --test reduced_motion_sweep
run_check "absent services never block session start" \
    cargo test -p dragonfruit-session --test absent_services

cp "$SCRATCH/gate.txt" "$TRANSCRIPT"

if [ "$fail" -ne 0 ]; then
    echo "capture-t17-premium-gate: at least one row FAILED (see $TRANSCRIPT)" >&2
    exit 1
fi
echo "capture-t17-premium-gate: wrote $TRANSCRIPT (all rows passed)"