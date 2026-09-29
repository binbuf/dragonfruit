#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-19.2 Applications drawer capture.
#
# Runs the nested demo on the current session bus alongside a scratch-home
# settingsd. The shell's `DF_APPS_DRAWER_FIXTURE` seam seeds a synthetic corpus
# (so the grid is populated without a full host app corpus) *on top of* the
# staged first-party `.desktop` entries, and opens the drawer once the chrome is
# up. Stills:
#
#   * docs/captures/t19-apps-drawer-light.png
#   * docs/captures/t19-apps-drawer-dark.png
#   * docs/captures/t19-apps-drawer-launch.png  (a real click on the Files tile
#     launched Files; the demo log is grepped for the launch)
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-10}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t19-drawer.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t19-drawer}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"
# The Files tile in the seeded corpus (fixture apps sort before the real ones:
# Arcade, Browser, Calculator, Calendar, Editor, Files, ...). Clicking its center
# launches Files through the production path.
FILES_TILE_X="${FILES_TILE_X:-0.48125}"
FILES_TILE_Y="${FILES_TILE_Y:-0.2208}"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t19-apps-drawer: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-t19-apps-drawer: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle gdbus python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t19-apps-drawer: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t19-apps-drawer: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-t19-apps-drawer: $DBUS_DEST already owned — stop the running settingsd" >&2
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
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_drawer_capture >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

# Send a synthetic absolute pointer click (device-normalized 0..1).
synth_click() {
    local x="$1" y="$2"
    python3 - "$SYNTH" "$x" "$y" <<'PY'
import os
import socket
import sys
import time

path, x, y = sys.argv[1], sys.argv[2], sys.argv[3]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/dragonfruit-t19-drawer-click-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)
sock.sendto(f"motion-abs {x} {y}".encode(), path)
time.sleep(0.4)
sock.sendto(b"button 272 down", path)
time.sleep(0.15)
sock.sendto(b"button 272 up", path)
time.sleep(1.0)
PY
}

# crop <raw> <dest> <crop-mode>: alpha-crop to the nested window, then either
# keep the full frame (full) or a 2x centered drawer band (band).
crop() {
    local raw="$1" dest="$2" mode="$3"
    python3 - "$raw" "$dest" "$mode" <<'PY'
import sys
from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
raw, dest, mode = sys.argv[1:4]
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
if mode == "band":
    im = im.crop((0, 0, NESTED_W, 720))
im.save(dest)
print(f"capture-t19-apps-drawer: saved {dest}")
PY
}

echo "capture-t19-apps-drawer: starting the scratch settingsd"
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
set_key "dock.position" "<'bottom'>"
set_key "appearance.colorScheme" "<'light'>"

echo "capture-t19-apps-drawer: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
DF_APPS_DRAWER_FIXTURE=1 \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-t19-apps-drawer: synthetic socket never appeared; demo log:" >&2
    tail -40 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

raise_dragonfruit
RAW_LIGHT="$SCRATCH/light-raw.png"
spectacle -b -n -a -o "$RAW_LIGHT"
crop "$RAW_LIGHT" "$OUTDIR/t19-apps-drawer-light.png" full

set_key "appearance.colorScheme" "<'dark'>"
sleep 2
raise_dragonfruit
RAW_DARK="$SCRATCH/dark-raw.png"
spectacle -b -n -a -o "$RAW_DARK"
crop "$RAW_DARK" "$OUTDIR/t19-apps-drawer-dark.png" full

# Activate the Files tile through the real pointer path and confirm the launch
# in the demo log. The shell's capture seam logs the tile rect; parse it and
# click its center (device-normalized to the configured output).
set_key "appearance.colorScheme" "<'light'>"
sleep 2
raise_dragonfruit
TILE_XY="$(python3 - "$SCRATCH/demo.log" <<'PY'
import re
import sys

text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
out_w = out_h = 0
m = re.search(r"apps-drawer configured (\d+)x(\d+)", text)
if m:
    out_w, out_h = int(m.group(1)), int(m.group(2))
m = re.search(r"apps-drawer tile \"?org\.dragonfruit\.Files\.desktop\"? x ([\d.]+) y ([\d.]+) "
              r"w ([\d.]+) h ([\d.]+)", text)
if not m or out_w == 0 or out_h == 0:
    sys.exit(0)
x, y, w, h = (float(v) for v in m.groups())
print(f"{(x + w / 2) / out_w:.5f} {(y + h / 2) / out_h:.5f}")
PY
)"
if [ -n "$TILE_XY" ]; then
    # shellcheck disable=SC2086
    synth_click $TILE_XY
else
    echo "capture-t19-apps-drawer: tile rect not logged; falling back to a fixed point" >&2
    synth_click "$FILES_TILE_X" "$FILES_TILE_Y"
fi
sleep 4
raise_dragonfruit
RAW_LAUNCH="$SCRATCH/launch-raw.png"
spectacle -b -n -a -o "$RAW_LAUNCH"
crop "$RAW_LAUNCH" "$OUTDIR/t19-apps-drawer-launch.png" full

if grep -q "Dock launched.*Files" "$SCRATCH/demo.log"; then
    echo "capture-t19-apps-drawer: launch confirmed: $(grep "Dock launched.*Files" "$SCRATCH/demo.log" | tail -1)"
else
    echo "capture-t19-apps-drawer: WARNING — no Files launch in the demo log" >&2
    grep -i "launch\|Files" "$SCRATCH/demo.log" | tail -10 >&2 || true
fi

echo "capture-t19-apps-drawer: done"