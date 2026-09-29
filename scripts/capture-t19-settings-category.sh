#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-19.1b System Settings category-tile track capture.
#
# Produces the T-19.1b artifacts in docs/captures/:
#
#   * t19-settings-category-light.png  General pane, light scheme
#   * t19-settings-category-dark.png   General pane, dark scheme
#
# The System Settings category tiles (gradient container + near-white Phosphor
# glyph) are visible in the sidebar rows and, for a header-card pane like
# General, in the detail-pane hero. The capture starts a scratch-home
# settingsd, launches the nested demo with `DF_SETTINGS_START_PANE=general`,
# zooms the Settings window over the synthetic-input harness, and screenshots
# the active Dragonfruit window with Spectacle.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, `gdbus`,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-14}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t19-capture.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t19}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t19-settings-category: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-t19-settings-category: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t19-settings-category: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t19-settings-category: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-t19-settings-category: $DBUS_DEST is already owned — stop the running settingsd first" >&2
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
    # The demo's per-pane stills share one socket name; make sure the old
    # compositor is fully gone before the next iteration binds it.
    for _ in $(seq 1 100); do [ ! -e "$SYNTH" ] && break; sleep 0.05; done
}

stop_settingsd() {
    if [ -n "${SETTINGS_PID:-}" ]; then
        kill -TERM "$SETTINGS_PID" 2>/dev/null || true
        wait "$SETTINGS_PID" 2>/dev/null || true
        SETTINGS_PID=""
    fi
}

start_settingsd() {
    XDG_CONFIG_HOME="$XDG_DIR" "$SETTINGS_BIN" >"$SCRATCH/settingsd.log" 2>&1 &
    SETTINGS_PID=$!
    for _ in $(seq 1 200); do
        if gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
                --method "$DBUS_IFACE.Get" appearance.colorScheme >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    echo "capture-t19-settings-category: settingsd never took $DBUS_DEST; log:" >&2
    tail -20 "$SCRATCH/settingsd.log" >&2
    exit 1
}

set_key() {
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Set" "$1" "$2" >/dev/null 2>&1 || true
}

start_demo() {
    local pane="$1"
    rm -f "$SYNTH"
    DF_SETTINGS_START_PANE="$pane" \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo-$pane.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-t19-settings-category: synthetic socket never appeared; demo log:" >&2
        tail -30 "$SCRATCH/demo-$pane.log" >&2
        exit 1
    fi
    sleep "$SETTLE"
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
    # df-allow-desktop-name: the KWin host compositor's scripting D-Bus, used
    # only to raise the nested window for the active-window capture.
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_capture_raise >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

# nudge: synthetic pointer motion into the nested session, so the nested
# compositor repaints the live state before the screenshot (T-05.6 harness).
nudge() {
    python3 - "$SYNTH" <<'PY'
import os
import socket
import sys
import time

path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/dragonfruit-t19-nudge-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)
for x in (0.20, 0.80, 0.50):
    sock.sendto(f"motion-abs {x} 0.97".encode(), path)
    time.sleep(0.25)
time.sleep(1.0)
PY
}

# zoom_settings: double-click the Settings titlebar over synthetic input so the
# 900x620 window fills the nested output and no pane row is clipped.
zoom_settings() {
    python3 - "$SYNTH" <<'PY'
import os
import socket
import sys
import time

path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/dragonfruit-t19-zoom-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)

def send(command):
    sock.sendto(command.encode(), path)

# The Settings window opens centered; its titlebar is near the top of the
# work area at this horizontal fraction.
send("motion-abs 0.469 0.2267")
time.sleep(0.3)
for _ in range(2):
    send("button 272 down")
    send("button 272 up")
    time.sleep(0.08)
time.sleep(1.2)
PY
}

# capture <dest>: active-window still, shadow trimmed and cropped to the
# nested 1920x1200 output rectangle (the whole desktop, like T-08's stills).
capture() {
    local dest="$1"
    local raw="$SCRATCH/$(basename "$dest" .png)-raw.png"
    raise_dragonfruit
    nudge
    spectacle -b -n -a -o "$raw"
    python3 - "$raw" "$dest" <<'PY'
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
if im.width >= NESTED_W and im.height >= NESTED_H:
    x0 = (im.width - NESTED_W) // 2
    y0 = im.height - NESTED_H
    im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
im.convert("RGB").save(dest)
print(f"capture-t19-settings-category: saved {dest} ({im.width}x{im.height})")
PY
}

# --- 1. scratch settingsd + a known light start. ---------------------------
start_settingsd
set_key "appearance.colorScheme" "<'light'>"
set_key "appearance.accent" "<''>"
set_key "accessibility.reduceMotion" "<false>"

# --- 2. General pane: header hero + sidebar tiles, light and dark. ---------
echo "capture-t19-settings-category: General pane (light / dark)"
start_demo general
zoom_settings
capture "$OUTDIR/t19-settings-category-light.png"

set_key "appearance.colorScheme" "<'dark'>"
sleep 2
capture "$OUTDIR/t19-settings-category-dark.png"
stop_demo

echo "capture-t19-settings-category: done"
