#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-16.6b compositor magnifier capture.
#
# Produces:
#   * t16-magnifier.png  an A/B stack: the nested demo with the magnifier off
#     (top) and on at 2x, centred (bottom)
#
# The magnifier is compositor-owned and transforms the whole scene. This
# capture proves the render path (the nested element rescale) against the live
# session; the model and the synthetic control are proven headlessly by
# `cargo test -p dragonfruit-compositor` (`magnifier::tests`,
# `window_conformance::magnifier_reports_its_view_transform_and_zoom`).
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-12}"
SOCKET="${SOCKET:-dragonfruit-t16-magnifier}"
SYNTH="${XDG_RUNTIME_DIR:?}/$SOCKET.synth"
LOG=/tmp/opencode/t16-magnifier-demo.log
mkdir -p "$OUTDIR" /tmp/opencode
rm -f "$SYNTH" "$LOG"

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
[ -e "$SYNTH" ] || { echo "capture-t16-magnifier: no synthetic socket"; tail -30 "$LOG"; exit 1; }
sleep "$SETTLE"

NORMAL=/tmp/opencode/t16-magnifier-normal.png
MAGNIFIED=/tmp/opencode/t16-magnifier-magnified.png
spectacle -b -n -a -o "$NORMAL" >/dev/null 2>&1 || spectacle -b -n -f -o "$NORMAL" >/dev/null 2>&1 || true

# Park the pointer mid-screen, then switch the magnifier on at 2x.
python3 - "$SYNTH" <<'PY'
import os, socket, sys, time
path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/opencode-t16mag-nudge-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)
def send(cmd):
    sock.sendto(cmd.encode(), path)
    time.sleep(0.4)
for x in (0.30, 0.70, 0.50):
    send(f"motion-abs {x} 0.6")
send("set magnifier-zoom 2")
send("set magnifier on")
time.sleep(1.5)
PY

spectacle -b -n -a -o "$MAGNIFIED" >/dev/null 2>&1 || spectacle -b -n -f -o "$MAGNIFIED" >/dev/null 2>&1 || true

python3 - "$NORMAL" "$MAGNIFIED" "$OUTDIR/t16-magnifier.png" <<'PY'
import sys
from PIL import Image, ImageDraw
normal, magnified, dest = sys.argv[1], sys.argv[2], sys.argv[3]
top = Image.open(normal).convert("RGB")
bottom = Image.open(magnified).convert("RGB")
width = 1057
def fit(im):
    height = round(im.height * width / im.width)
    return im.resize((width, height))
top, bottom = fit(top), fit(bottom)
canvas = Image.new("RGB", (width, top.height + bottom.height + 30), (20, 20, 20))
canvas.paste(top, (0, 0))
canvas.paste(bottom, (0, top.height + 30))
ImageDraw.Draw(canvas).text(
    (10, top.height + 7),
    "TOP: magnifier OFF        BOTTOM: magnifier ON 2x (centred)",
    fill=(255, 255, 255),
)
canvas.save(dest)
print("saved", dest, canvas.size)
PY

echo "capture-t16-magnifier: done"
tail -5 "$LOG" || true