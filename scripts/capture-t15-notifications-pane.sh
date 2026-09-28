#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-15.7b Notifications and Focus panes + Control Center tile capture.
#
# Produces:
#   * t15-7b-notifications-pane.png     the Settings Notifications pane
#   * t15-7b-focus-pane.png             the Settings Focus pane
#   * t15-7b-control-center.png         the Control Center (Focus tile)
#
# The nested demo runs with `DF_SETTINGS_FIXTURE` so the pane shows the schema
# defaults, `DF_NOTIFICATIONS_FIXTURE` so the Focus/adapter view is
# deterministic with no notification service on the host bus, and
# `DF_STATUS_FIXTURE` so the Control Center has its usual tiles. The pane is
# selected with `DF_SETTINGS_START_PANE`.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-12}"
mkdir -p "$OUTDIR" /tmp/opencode

start_demo() {
    local pane="$1" socket="$2" log="$3"
    local synth="${XDG_RUNTIME_DIR:?}/$socket.synth"
    rm -f "$synth" "$log"
    DF_SETTINGS_START_PANE="$pane" DF_SETTINGS_FIXTURE=1 \
        DF_NOTIFICATIONS_FIXTURE=1 DF_STATUS_FIXTURE=1 \
        DRAGONFRUIT_SYNTHETIC_INPUT="$synth" \
        setsid make demo DEMO_ARGS="--socket-name $socket" >"$log" 2>&1 &
    echo "$!"
}

wait_synth() {
    local socket="$1" log="$2"
    local synth="${XDG_RUNTIME_DIR:?}/$socket.synth"
    for _ in $(seq 1 600); do [ -e "$synth" ] && break; sleep 0.1; done
    [ -e "$synth" ] || { echo "capture-t15-notifications: no synthetic socket"; tail -30 "$log"; exit 1; }
    sleep "$SETTLE"
}

stop_demo() {
    local pgid="$1" socket="$2"
    kill -TERM -"$pgid" 2>/dev/null || true
    sleep 1
    kill -KILL -"$pgid" 2>/dev/null || true
    rm -f "${XDG_RUNTIME_DIR:?}/$socket.synth"
}

nudge() {
    local synth="$1"
    python3 - "$synth" <<'PY'
import os, socket, sys, time
path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/opencode-t15notif-nudge-{os.getpid()}.sock"
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
}

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

# ── The Notifications pane ──────────────────────────────────────────────
SOCKET="dragonfruit-t15-notif"
LOG=/tmp/opencode/t15-notif-demo.log
PGID=$(start_demo notifications "$SOCKET" "$LOG")
trap 'stop_demo "$PGID" "$SOCKET"' EXIT
wait_synth "$SOCKET" "$LOG"
nudge "${XDG_RUNTIME_DIR:?}/$SOCKET.synth"
capture_active "/tmp/opencode/t15-notif-pane-raw.png" \
    "$OUTDIR/t15-7b-notifications-pane.png"
stop_demo "$PGID" "$SOCKET"
trap - EXIT

# ── The Focus pane + Control Center ─────────────────────────────────────
SOCKET="dragonfruit-t15-focus"
LOG=/tmp/opencode/t15-focus-demo.log
PGID=$(start_demo focus "$SOCKET" "$LOG")
trap 'stop_demo "$PGID" "$SOCKET"' EXIT
wait_synth "$SOCKET" "$LOG"
nudge "${XDG_RUNTIME_DIR:?}/$SOCKET.synth"
capture_active "/tmp/opencode/t15-focus-pane-raw.png" \
    "$OUTDIR/t15-7b-focus-pane.png"

# Open the Control Center through the real shortcut (Control-Option-C).
python3 - "${XDG_RUNTIME_DIR:?}/$SOCKET.synth" <<'PY'
import os, socket, sys, time
path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/opencode-t15focus-cc-{os.getpid()}.sock"
try:
    os.unlink(client)
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

FULL_RAW="/tmp/opencode/t15-notif-full-raw.png"
spectacle -b -n -f -o "$FULL_RAW" >/dev/null 2>&1 || true
python3 - "$FULL_RAW" "$OUTDIR/t15-7b-control-center.png" <<'PY'
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

stop_demo "$PGID" "$SOCKET"
trap - EXIT
echo "capture-t15-notifications: done"