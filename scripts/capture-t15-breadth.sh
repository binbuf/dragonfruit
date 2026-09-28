#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-15.16 breadth capture.
#
# Produces:
#   * t15-breadth.png  the nested demo desktop with **no status/settings
#                      fixtures and no bridge host** (the honest absent state)
#
# The task ships no pane of its own, so this still is the live check that the
# whole desktop — menu bar, Dock, wallpaper, and the demo client window — still
# composites with every host daemon absent, the same state the headless
# absence matrix asserts. The per-subsystem pane and tile stills are linked
# from docs/captures/t15-breadth.md.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-12}"
SOCKET="${SOCKET:-dragonfruit-t15-breadth}"
SYNTH="${XDG_RUNTIME_DIR:?}/$SOCKET.synth"
LOG=/tmp/opencode/t15-breadth-demo.log
mkdir -p "$OUTDIR" /tmp/opencode
rm -f "$SYNTH" "$LOG"

# No DF_STATUS_FIXTURE / DF_SETTINGS_FIXTURE: every provider is absent, so the
# desktop is exactly the matrix's "no daemon" state.
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
[ -e "$SYNTH" ] || { echo "capture-t15-breadth: no synthetic socket"; tail -30 "$LOG"; exit 1; }
sleep "$SETTLE"

# Nudge the pointer so the nested compositor repaints the live state.
python3 - "$SYNTH" <<'PY'
import os, socket, sys, time
path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/opencode-t15breadth-nudge-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)
for x in (0.30, 0.70, 0.50):
    sock.sendto(f"motion-abs {x} 0.6".encode(), path)
    time.sleep(0.25)
time.sleep(1.5)
PY

RAW="/tmp/opencode/t15-breadth-raw.png"
spectacle -b -n -a -o "$RAW" >/dev/null 2>&1 || spectacle -b -n -f -o "$RAW" >/dev/null 2>&1 || true
python3 - "$RAW" "$OUTDIR/t15-breadth.png" <<'PY'
import sys
from PIL import Image
raw, dest = sys.argv[1], sys.argv[2]
im = Image.open(raw).convert("RGB")
im.save(dest)
print("saved", dest, im.size)
PY

echo "capture-t15-breadth: done"
tail -5 "$LOG" || true