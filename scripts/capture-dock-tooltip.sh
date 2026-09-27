#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7i Dock hover name label (Tooltip) capture.
#
# Runs the nested demo three times on the current session bus alongside a
# scratch-home settingsd and uses the shell's `DF_DOCK_TOOLTIP_FIXTURE` seam
# (the production Dock presentation minus a synthetic pointer hover) to show
# the label over a magnified icon:
#
#   * app    — an app entry's name;
#   * folder — the Downloads stack's name and item count;
#   * trash  — the Trash state ("Trash — empty").
#
# The three crops are stacked into one still:
#
#   * docs/captures/t14-dock-tooltip.png
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-tooltip.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-tooltip}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-tooltip: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-tooltip: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-tooltip: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-tooltip: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-tooltip: $DBUS_DEST is already owned — stop the running settingsd first" >&2
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
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_tooltip_capture >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

# run_mode <kind>: one nested run with the tooltip shown for that entry kind.
run_mode() {
    local kind="$1"
    local raw="$SCRATCH/$kind-raw.png"
    local crop="$SCRATCH/$kind-crop.png"
    rm -f "$SYNTH"
    env DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" DF_DOCK_TOOLTIP_FIXTURE="$kind" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo-$kind.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-tooltip: synthetic socket never appeared for $kind; demo log:" >&2
        tail -40 "$SCRATCH/demo-$kind.log" >&2
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
# The Dock sits at the bottom centre; the hover label floats just above the
# magnified icon. Crop a tall bottom band and zoom 2x so the capsule, the
# anchor, and the magnified neighbour read.
x0 = int(w * 0.28)
x1 = int(w * 0.72)
crop = im.crop((x0, max(0, h - 360), x1, h))
crop = crop.resize((crop.width * 2, crop.height * 2), Image.LANCZOS)
crop.save(dest)
print(f"capture-dock-tooltip: cropped {dest} ({crop.width}x{crop.height})")
PY
    stop_demo
}

echo "capture-dock-tooltip: starting the scratch settingsd"
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
set_key "dock.magnification" "<0.5>"
set_key "appearance.colorScheme" "<'dark'>"

run_mode app
run_mode folder
run_mode trash

DEST="$OUTDIR/t14-dock-tooltip.png"
python3 - "$SCRATCH/app-crop.png" "$SCRATCH/folder-crop.png" \
    "$SCRATCH/trash-crop.png" "$DEST" <<'PY'
import sys
from PIL import Image, ImageDraw

paths = sys.argv[1:4]
dest = sys.argv[4]
labels = ["app", "folder (Downloads)", "trash"]
panels = [Image.open(p) for p in paths]
gap = 8
pad = 22
w = max(p.width for p in panels)
h = sum(p.height for p in panels) + gap * (len(panels) - 1) + pad * len(panels)
out = Image.new("RGB", (w, h), (24, 24, 28))
draw = ImageDraw.Draw(out)
y = 0
for p, label in zip(panels, labels):
    draw.text((6, y + 4), label, fill=(220, 220, 224))
    y += pad
    out.paste(p, ((w - p.width) // 2, y))
    y += p.height + gap
out.save(dest)
print(f"capture-dock-tooltip: saved {dest} ({out.width}x{out.height})")
PY

echo "capture-dock-tooltip: done"