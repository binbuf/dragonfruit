#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7a Dock plate geometry + spacing capture.
#
# Produces six crops in docs/captures/, one per dock position and colour
# scheme, each cropped to show the floating plate, its padding, and the gap
# to the screen edge:
#
#   * t14-dock-spacing-bottom-light.png
#   * t14-dock-spacing-bottom-dark.png
#   * t14-dock-spacing-left-light.png
#   * t14-dock-spacing-left-dark.png
#   * t14-dock-spacing-right-light.png
#   * t14-dock-spacing-right-dark.png
#
# It runs the nested demo on the current session bus alongside a scratch-home
# settingsd, flips `appearance.colorScheme` and `dock.position` with `gdbus`,
# and screenshots the active Dragonfruit window with Spectacle.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-dock.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-dock}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-spacing: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-spacing: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-spacing: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-spacing: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-spacing: $DBUS_DEST is already owned — stop the running settingsd first" >&2
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
}

stop_settingsd() {
    if [ -n "${SETTINGS_PID:-}" ]; then
        kill -KILL "$SETTINGS_PID" 2>/dev/null || true
        wait "$SETTINGS_PID" 2>/dev/null || true
        SETTINGS_PID=""
    fi
}

start_settingsd() {
    XDG_CONFIG_HOME="$XDG_DIR" "$SETTINGS_BIN" >>"$SCRATCH/settingsd.log" 2>&1 &
    SETTINGS_PID=$!
    for _ in $(seq 1 200); do
        if gdbus call --session --dest org.freedesktop.DBus \
                --object-path /org/freedesktop/DBus \
                --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
            return
        fi
        sleep 0.05
    done
    echo "capture-dock-spacing: settingsd never took $DBUS_DEST; log:" >&2
    tail -20 "$SCRATCH/settingsd.log" >&2
    exit 1
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
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_dock_capture >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

# capture <position> <scheme>: active-window still, shadow trimmed, cropped
# to the Dock edge strip (the plate, its end/cross padding, and the gap).
capture() {
    local position="$1" scheme="$2"
    local dest="$OUTDIR/t14-dock-spacing-$position-$scheme.png"
    local raw="$SCRATCH/t14-dock-spacing-$position-$scheme-raw.png"
    raise_dragonfruit
    spectacle -b -n -a -o "$raw"
    python3 - "$raw" "$dest" "$position" <<'PY'
import sys
from PIL import Image

# The nested backend runs at a fixed 1920x1200 output; the active-window grab
# also carries the host's window frame, so trim the alpha then crop to the
# nested output rectangle anchored bottom-centre, exactly as the T-08 capture
# does. That puts the screen edges back on the image edges.
NESTED_W, NESTED_H = 1920, 1200

raw, dest, position = sys.argv[1], sys.argv[2], sys.argv[3]
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
# The dock surface thickness (plate + magnify band + edge gap) is 151 px at
# the default icon size; add context so the whole floating treatment reads.
strip = 190
if position == "bottom":
    crop = (0, max(0, h - strip), w, h)
elif position == "left":
    crop = (0, 0, min(w, strip), h)
else:
    crop = (max(0, w - strip), 0, w, h)
im.crop(crop).save(dest)
print(f"capture-dock-spacing: saved {dest} ({im.crop(crop).size[0]}x{im.crop(crop).size[1]})")
PY
}

start_settingsd

echo "capture-dock-spacing: launching the nested demo ($SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-dock-spacing: synthetic socket never appeared; demo log:" >&2
    tail -30 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

set_key "dock.size" "<0.5>"

echo "capture-dock-spacing: bottom (dark, light)"
set_key "dock.position" "<'bottom'>"
set_key "appearance.colorScheme" "<'dark'>"
sleep 2
capture bottom dark
set_key "appearance.colorScheme" "<'light'>"
sleep 2
capture bottom light

echo "capture-dock-spacing: left (light, dark)"
set_key "dock.position" "<'left'>"
sleep 2
capture left light
set_key "appearance.colorScheme" "<'dark'>"
sleep 2
capture left dark

echo "capture-dock-spacing: right (dark, light)"
set_key "dock.position" "<'right'>"
sleep 2
capture right dark
set_key "appearance.colorScheme" "<'light'>"
sleep 2
capture right light

echo "capture-dock-spacing: done"