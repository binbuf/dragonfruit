#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7t Dock keyboard reordering capture.
#
# Drives the real input path in the nested session: Ctrl+F3 focuses the Dock
# (the compositor's FocusDock shortcut), then Ctrl+Shift+Right moves the focused
# pinned entry one slot. A still of the Dock band is saved so the focus ring on
# the moved entry is visible next to its reflowed neighbours:
#
#   * docs/captures/t14-dock-keyboard-reorder.png
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-10}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-reorder.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-reorder}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-keyboard-reorder: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-keyboard-reorder: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-keyboard-reorder: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-keyboard-reorder: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-keyboard-reorder: $DBUS_DEST already owned — stop the running settingsd" >&2
    exit 1
fi

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
XDG_DIR="$SCRATCH/xdg"
rm -f "$SYNTH"

cleanup() {
    stop_demo
    stop_settingsd
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

stop_demo() {
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
        DEMO_PGID=""
    fi
    for _ in $(seq 1 100); do [ ! -e "$SYNTH" ] && break; sleep 0.05; done
}

stop_settingsd() {
    if [ -n "${SETTINGS_PID:-}" ]; then
        kill -KILL "$SETTINGS_PID" 2>/dev/null || true
        wait "$SETTINGS_PID" 2>/dev/null || true
        SETTINGS_PID=""
    fi
}

set_key() {
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Set" "$1" "$2" >/dev/null
}

# Focus the Dock with Ctrl+F3, then reorder the focused pinned entry one slot
# with Ctrl+Shift+Right, through the compositor's synthetic-input harness.
# Linux evdev keycodes: Ctrl 29, Shift 42, F3 61, Right 106.
drive_reorder() {
    python3 - "$SYNTH" <<'PY'
import os
import socket
import sys
import time

path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = "/tmp/opencode-kbd-reorder-%d.sock" % os.getpid()
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)

def send(cmd):
    sock.sendto(cmd.encode(), path)

def tap(*codes, hold=0.12):
    for c in codes:
        send(f"key {c} down")
    time.sleep(hold)
    for c in reversed(codes):
        send(f"key {c} up")

# Ctrl+F3 focuses the Dock (compositor FocusDock).
send("key 29 down")
time.sleep(0.05)
tap(61)
send("key 29 up")
time.sleep(1.0)
# Ctrl+Shift+Right moves the focused pinned entry one slot.
send("key 29 down")
time.sleep(0.05)
send("key 42 down")
time.sleep(0.05)
tap(106)
send("key 42 up")
send("key 29 up")
time.sleep(0.6)
PY
}

raise_dragonfruit() {
    local script="$SCRATCH/raise-dragonfruit.js"
    cat >"$script" <<'JS'
function raiseDragonfruit() {
    var wins = workspace.windowList();
    for (var i = 0; i < wins.length; ++i) {
        var w = wins[i];
        var cap = (w.caption || "").toString();
        var cls = (w.resourceClass || "").toString().toLowerCase();
        if (cap.indexOf("Dragonfruit") >= 0 || cls.indexOf("dragonfruit") >= 0) {
            w.minimized = false;
            if (workspace.activateWindow) { workspace.activateWindow(w); }
            else { workspace.activeWindow = w; }
        }
    }
}
raiseDragonfruit();
JS
    # df-allow-desktop-name: the host compositor's scripting D-Bus, used only
    # to raise the nested window for the active-window capture.
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_reorder_capture >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

echo "capture-dock-keyboard-reorder: starting the scratch settingsd"
XDG_CONFIG_HOME="$XDG_DIR" "$SETTINGS_BIN" >>"$SCRATCH/settingsd.log" 2>&1 &
SETTINGS_PID=$!
for _ in $(seq 1 200); do
    if gdbus call --session --dest org.freedesktop.DBus \
            --object-path /org/freedesktop/DBus \
            --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
        break
    fi
    sleep 0.05
done

rm -f "$SYNTH"
env QT_QPA_PLATFORM=wayland QML_IMPORT_PATH="$PWD/build/qml" \
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo "DEMO_ARGS=--socket-name $SOCKET" \
    >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-dock-keyboard-reorder: synthetic socket never appeared; demo log:" >&2
    tail -40 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

set_key "dock.position" "<'bottom'>"
set_key "dock.size" "<0.5>"
set_key "appearance.colorScheme" "<'light'>"
sleep 2

echo "capture-dock-keyboard-reorder: focusing the Dock and reordering"
drive_reorder
sleep 1

raise_dragonfruit
RAW="$SCRATCH/reorder-raw.png"
spectacle -b -n -a -o "$RAW"
DEST="$OUTDIR/t14-dock-keyboard-reorder.png"
python3 - "$RAW" "$DEST" <<'PY'
import sys
from PIL import Image

NESTED_W, NESTED_H = 1920, 1200

raw, dest = sys.argv[1], sys.argv[2]
im = Image.open(raw)
if im.mode == "RGBA":
    alpha = im.getchannel("A")
    w, h = alpha.size
    px = alpha.load()
    minx, miny, maxx, maxy = w, h, -1, -1
    for y in range(h):
        for x in range(w):
            if px[x, y] == 255:
                minx = min(minx, x); maxx = max(maxx, x)
                miny = min(miny, y); maxy = max(maxy, y)
    if maxx > minx:
        im = im.crop((minx, miny, maxx + 1, maxy + 1))
im = im.convert("RGB")
w, h = im.size
if w >= NESTED_W and h >= NESTED_H:
    x0 = (w - NESTED_W) // 2
    y0 = h - NESTED_H
    im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
    w, h = im.size
# The Dock band at the bottom, full width so the moved tile and its reflowed
# neighbours read in context with the focus ring.
band = im.crop((0, max(0, h - 260), w, h))
band = band.resize((band.width * 2, band.height * 2), Image.LANCZOS)
band.save(dest)
print(f"capture-dock-keyboard-reorder: saved {dest} ({band.width}x{band.height})")
PY

echo "capture-dock-keyboard-reorder: done"