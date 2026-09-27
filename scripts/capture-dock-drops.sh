#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7f Dock drag-and-drop identity and feedback capture.
#
# Runs the nested demo three times on the current session bus alongside a
# scratch-home settingsd, once per hover state, using the shell's
# `DF_DOCK_DROP_FIXTURE` seam (the production Dock presentation minus a real
# drag source):
#
#   * `app`        — an app-alias ghost with the app's real tile and name;
#   * `file-app`   — a file dragged over an app entry ("Open with …");
#   * `file-trash` — a file dragged over the Trash ("Move to Trash").
#
# The three crops are stacked into one still:
#
#   * docs/captures/t14-dock-drops.png
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-drops.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-drops}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-drops: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-drops: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-drops: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-drops: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-drops: $DBUS_DEST is already owned — stop the running settingsd first" >&2
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
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_drops_capture >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

# run_mode <mode>: one nested run with the drop fixture, cropped to a temp PNG.
run_mode() {
    local mode="$1"
    local raw="$SCRATCH/$mode-raw.png"
    local crop="$SCRATCH/$mode-crop.png"
    rm -f "$SYNTH"
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    DF_DOCK_DROP_FIXTURE="$mode" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo-$mode.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-drops: synthetic socket never appeared for $mode; demo log:" >&2
        tail -40 "$SCRATCH/demo-$mode.log" >&2
        exit 1
    fi
    sleep "$SETTLE"
    raise_dragonfruit
    spectacle -b -n -a -o "$raw"
    python3 - "$raw" "$crop" <<'PY'
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
# The Dock sits at the bottom centre; the affordance capsule floats above it.
# Crop a generous bottom band and zoom 2x so the ghost, name and capsule read.
x0 = int(w * 0.18)
x1 = int(w * 0.82)
crop = im.crop((x0, max(0, h - 300), x1, h))
crop = crop.resize((crop.width * 2, crop.height * 2), Image.LANCZOS)
crop.save(dest)
print(f"capture-dock-drops: cropped {dest} ({crop.width}x{crop.height})")
PY
    stop_demo
}

echo "capture-dock-drops: starting the scratch settingsd"
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
set_key "appearance.colorScheme" "<'dark'>"

run_mode app
run_mode file-app
run_mode file-trash

DEST="$OUTDIR/t14-dock-drops.png"
python3 - "$SCRATCH/app-crop.png" "$SCRATCH/file-app-crop.png" \
    "$SCRATCH/file-trash-crop.png" "$DEST" <<'PY'
import sys
from PIL import Image, ImageDraw

paths = sys.argv[1:4]
dest = sys.argv[4]
panels = [Image.open(p) for p in paths]
gap = 8
w = max(p.width for p in panels)
h = sum(p.height for p in panels) + gap * (len(panels) - 1)
out = Image.new("RGB", (w, h), (24, 24, 28))
y = 0
for p in panels:
    out.paste(p, ((w - p.width) // 2, y))
    y += p.height + gap
out.save(dest)
print(f"capture-dock-drops: saved {dest} ({out.width}x{out.height})")
PY

echo "capture-dock-drops: done"