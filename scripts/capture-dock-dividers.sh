#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7v Dock region dividers capture.
#
# Produces two composites in docs/captures/, one per colour scheme, each a
# vertical stack of three Dock states cropped to the Dock edge strip:
#
#   * three regions  — pinned | temporary/recent | stacks + Trash (two rules)
#   * empty tail     — pinned | stacks + Trash (one rule)
#   * single region  — stacks + Trash only (no rule)
#
# The rules use the over-sized `divider.gap` and the near-plate-height
# `divider.heightRatio`; only the app | right-region rule carries the resize
# handle.
#
# It runs the nested demo on the current session bus alongside a scratch-home
# settingsd, changes `dock.pinned` and `appearance.colorScheme` with `gdbus`,
# and screenshots the active Dragonfruit window with Spectacle.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-dock-dividers.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-dock-dividers}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-dividers: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-dividers: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-dividers: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-dividers: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-dividers: $DBUS_DEST is already owned — stop the running settingsd first" >&2
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
    echo "capture-dock-dividers: settingsd never took $DBUS_DEST; log:" >&2
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
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_dock_dividers_capture >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

# capture <state> <scheme>: active-window still, cropped to the bottom Dock
# edge strip; writes the per-state raw crop under $SCRATCH.
capture() {
    local state="$1" scheme="$2"
    local raw="$SCRATCH/raw-$state-$scheme.png"
    local dest="$SCRATCH/crop-$state-$scheme.png"
    raise_dragonfruit
    spectacle -b -n -a -o "$raw"
    python3 - "$raw" "$dest" <<'PY'
import sys
from PIL import Image

# The nested backend runs at a fixed 1920x1200 output; trim the host window
# frame's alpha then crop to the nested output rectangle anchored bottom-centre.
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
# The dock surface thickness is 159 px at the default icon size; take a 190 px
# strip so the floating plate and its dividers read.
strip = 190
im.crop((0, max(0, h - strip), w, h)).save(dest)
PY
}

start_settingsd

echo "capture-dock-dividers: launching the nested demo ($SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-dock-dividers: synthetic socket never appeared; demo log:" >&2
    tail -30 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

set_key "dock.position" "<'bottom'>"
set_key "dock.size" "<0.5>"

# State A: pinned + the running demo client + Downloads + Trash (three regions).
echo "capture-dock-dividers: three regions (dark, light)"
set_key "appearance.colorScheme" "<'dark'>"
sleep 2
capture three dark
set_key "appearance.colorScheme" "<'light'>"
sleep 2
capture three light

# State B: close the unpinned running client -> pinned | stacks + Trash.
echo "capture-dock-dividers: empty tail (dark, light)"
pkill -x xmessage 2>/dev/null || true
sleep 3
set_key "appearance.colorScheme" "<'dark'>"
sleep 2
capture tail dark
set_key "appearance.colorScheme" "<'light'>"
sleep 2
capture tail light

# State C: no pinned apps and no running app windows either -> the fixed
# stacks/Trash region alone, with no rule at all.
echo "capture-dock-dividers: single region (dark, light)"
set_key "dock.pinned" "<@as []>"
pkill -f 'build/apps/settings/dragonfruit-settings' 2>/dev/null || true
sleep 3
set_key "appearance.colorScheme" "<'dark'>"
sleep 2
capture single dark
set_key "appearance.colorScheme" "<'light'>"
sleep 2
capture single light

python3 - "$SCRATCH" "$OUTDIR" <<'PY'
import sys
from PIL import Image, ImageDraw

scratch, outdir = sys.argv[1], sys.argv[2]
states = [("three", "three regions"), ("tail", "empty tail"), ("single", "single region")]
for scheme in ("light", "dark"):
    ims = []
    for state, _ in states:
        im = Image.open(f"{scratch}/crop-{state}-{scheme}.png").convert("RGB")
        ims.append(im)
    pad, label_h = 24, 22
    w = max(im.width for im in ims)
    total_h = sum(im.height + label_h for im in ims) + pad * (len(ims) + 1)
    canvas = Image.new("RGB", (w + 2 * pad, total_h), (32, 32, 32) if scheme == "dark" else (238, 238, 238))
    draw = ImageDraw.Draw(canvas)
    fg = (235, 235, 235) if scheme == "dark" else (30, 30, 30)
    y = pad
    for (state, label), im in zip(states, ims):
        draw.text((pad, y + 4), label, fill=fg)
        y += label_h
        canvas.paste(im, (pad, y))
        y += im.height + pad
    dest = f"{outdir}/t14-dock-dividers-{scheme}.png"
    canvas.save(dest)
    print(f"capture-dock-dividers: saved {dest} ({canvas.width}x{canvas.height})")
PY

echo "capture-dock-dividers: done"