#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-12.5b: the T-12 session track capture.
#
# Produces the T-12 artifacts in docs/captures/:
#
#   * t12-session.png          the nested session (our desktop rendering)
#   * t12-session-lock.png     the first-party lock UI after Cmd+Ctrl+Q
#   * t12-session.mp4          the stills in sequence
#   * t12-session.txt          the synthetic-harness transcript: `query lock`
#                              and `query session` before and while locked
#
# The desktop is launched through the normal `make demo` nested session with
# the synthetic-input harness bound; the driver locks with the same shortcut a
# user presses and reads the compositor's fail-secure lock report. The real
# login/logout ends of the demo are DRM-gated (T-12.6), so this capture proves
# the session + lock surface, not the greeter round trip.
#
# Requires: a host Wayland session, `spectacle`, `ffmpeg`, python3 with
# Pillow, and the built tree (`make build`). Not part of `make e2e`.

set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
SOCKET="${SOCKET:-dragonfruit-t87-session}"
SYNTH="${XDG_RUNTIME_DIR:?}/$SOCKET.synth"
LOG="${LOG:-/tmp/opencode/t12-demo.log}"
rm -f "$SYNTH" "$LOG"

mkdir -p "$OUTDIR"

DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" >"$LOG" 2>&1 &
PGID=$!
cleanup() {
    kill -TERM -"$PGID" 2>/dev/null || true
    sleep 1
    kill -KILL -"$PGID" 2>/dev/null || true
    rm -f "$SYNTH"
}
trap cleanup EXIT

for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
[ -e "$SYNTH" ] || { echo "capture-session: no synthetic socket"; tail -40 "$LOG"; exit 1; }
sleep 12

T12_SYNTH="$SYNTH" T12_OUT="$root/$OUTDIR" T12_LOG="$LOG" \
    python3 "$root/scripts/capture-session-driver.py"

# The supervision half of the track: the headless kill matrix, committed as a
# transcript beside the stills (the same test `make e2e`/CI runs).
{
    echo "T-12 kill matrix (T-12.5b) — dragonfruit-session kill_matrix"
    echo
    cargo test -p dragonfruit-session --test kill_matrix -- --nocapture 2>&1
} >"$OUTDIR/t12-session-kill-matrix.txt" || true

# The stills in sequence, the same pattern the other slice captures use.
if command -v ffmpeg >/dev/null 2>&1; then
    ffmpeg -y -loglevel error \
        -loop 1 -t 2 -i "$OUTDIR/t12-session.png" \
        -loop 1 -t 2 -i "$OUTDIR/t12-session-lock.png" \
        -filter_complex "[0][1]concat=n=2:v=1:a=0" -pix_fmt yuv420p \
        "$OUTDIR/t12-session.mp4" || true
fi

echo "capture-session: wrote $OUTDIR/t12-session.png, t12-session-lock.png"