#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-15.11b Users & Groups pane + Control Center tile capture.
#
# Produces:
#   * t15-11b-users-pane.png           the Settings Users & Groups pane
#   * t15-11b-users-control-center.png the Control Center with the Users tile
#
# The nested demo runs with `DF_SETTINGS_START_PANE=users-groups`,
# `DF_SETTINGS_FIXTURE`, and `DF_ACCOUNTS_FIXTURE` so the Settings pane shows a
# deterministic two-user / two-group workstation with no AccountsService and no
# distro group provider; the shell's status fixture (`DF_STATUS_FIXTURE`) keeps
# the rest of the Control Center populated.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-12}"
SOCKET="${SOCKET:-dragonfruit-t15-users-pane}"
SYNTH="${XDG_RUNTIME_DIR:?}/$SOCKET.synth"
LOG=/tmp/opencode/t15-users-pane-demo.log
mkdir -p "$OUTDIR" /tmp/opencode
rm -f "$SYNTH" "$LOG"

DF_SETTINGS_START_PANE=users-groups DF_SETTINGS_FIXTURE=1 DF_ACCOUNTS_FIXTURE=1 \
    DF_STATUS_FIXTURE=1 DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
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
[ -e "$SYNTH" ] || { echo "capture-t15-users: no synthetic socket"; tail -30 "$LOG"; exit 1; }
sleep "$SETTLE"

# Nudge the pointer so the nested compositor repaints the live state, then
# capture the active Settings window (the pane this task added).
python3 - "$SYNTH" <<'PY'
import socket, sys, time
path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/opencode-t15users-pane-nudge-{__import__('os').getpid()}.sock"
try:
    __import__('os').unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)
for x in (0.30, 0.70, 0.50):
    sock.sendto(f"motion-abs {x} 0.6".encode(), path)
    time.sleep(0.25)
time.sleep(1.5)
PY

capture_active() {
    local raw="$1" dest="$2"
    spectacle -b -n -a -o "$raw" >/dev/null 2>&1 || true
    python3 - "$raw" "$dest" <<'PY'
import sys
from PIL import Image
raw, dest = sys.argv[1], sys.argv[2]
im = Image.open(raw)
if im.mode == "RGBA":
    alpha = im.getchannel("A")
    box = alpha.getbbox()
    if box:
        im = im.crop(box)
im.convert("RGB").save(dest)
print("saved", dest, im.size)
PY
}

PANE_RAW="/tmp/opencode/t15-users-pane-raw.png"
capture_active "$PANE_RAW" "$OUTDIR/t15-11b-users-pane.png"

# Open the Control Center through the real shortcut (Control-Option-C) and
# crop the panel from the nested output's top-right corner.
python3 - "$SYNTH" <<'PY'
import socket, sys, time
path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/opencode-t15users-pane-cc-{__import__('os').getpid()}.sock"
try:
    __import__('os').unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)
def send(command):
    sock.sendto(command.encode(), path)
KEY_LEFTCTRL, KEY_LEFTALT, KEY_C = 29, 56, 46
for code in (KEY_LEFTCTRL, KEY_LEFTALT):
    send(f"key {code} down")
send(f"key {KEY_C} down")
time.sleep(0.15)
send(f"key {KEY_C} up")
for code in (KEY_LEFTALT, KEY_LEFTCTRL):
    send(f"key {code} up")
time.sleep(2.0)
PY

FULL_RAW="/tmp/opencode/t15-users-pane-full-raw.png"
spectacle -b -n -f -o "$FULL_RAW" >/dev/null 2>&1 || true
python3 - "$FULL_RAW" "$OUTDIR/t15-11b-users-control-center.png" <<'PY'
import sys
from PIL import Image
raw, dest = sys.argv[1], sys.argv[2]
im = Image.open(raw).convert("RGB")
w, h = im.size
NESTED_W, NESTED_H, BAR, GAP = 1920, 1200, 28, 8
x = (w - NESTED_W) // 2
y = (h - NESTED_H) // 2
PANEL_W, PANEL_H = 360, 1160
panel = im.crop((x + NESTED_W - PANEL_W, y + BAR + GAP,
                 x + NESTED_W, y + BAR + GAP + PANEL_H))
panel.save(dest)
print("saved", dest, panel.size)
PY

echo "capture-t15-users: done"
tail -5 "$LOG" || true