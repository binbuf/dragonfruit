#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7d Trash entry artwork capture.
#
# Produces six crops in docs/captures/, one per Trash state and colour scheme,
# each cropped to the right end of the Dock so the Trash reads next to the
# themed app tiles:
#
#   * t14-dock-trash-empty-light.png
#   * t14-dock-trash-empty-dark.png
#   * t14-dock-trash-full-light.png
#   * t14-dock-trash-full-dark.png
#   * t14-dock-trash-unavailable-light.png
#   * t14-dock-trash-unavailable-dark.png
#
# It runs the nested demo three times (once per state) on the current session
# bus alongside a scratch-home settingsd. Each run points the shell at a
# scratch `XDG_DATA_HOME` whose home trash is empty, full, or blocked read-as
# unavailable; `appearance.colorScheme` is then flipped with `gdbus` and the
# active Dragonfruit window is screenshotted with Spectacle.
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-trash.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-trash}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-trash: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-trash: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-trash: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-trash: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-trash: $DBUS_DEST is already owned — stop the running settingsd first" >&2
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
    echo "capture-dock-trash: settingsd never took $DBUS_DEST; log:" >&2
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
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_trash_capture >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

# capture <state> <scheme>: active-window still, shadow trimmed, cropped to the
# right end of the Dock where the Trash sits.
capture() {
    local state="$1" scheme="$2"
    local dest="$OUTDIR/t14-dock-trash-$state-$scheme.png"
    local raw="$SCRATCH/t14-dock-trash-$state-$scheme-raw.png"
    raise_dragonfruit
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
im = im.convert("RGB")
w, h = im.size
if w >= NESTED_W and h >= NESTED_H:
    x0 = (w - NESTED_W) // 2
    y0 = h - NESTED_H
    im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
    w, h = im.size
# The Dock sits at the bottom centre; the Trash is the rightmost entry. Crop
# the whole Dock strip so the Trash reads next to the themed tiles, and zoom 2x
# so the small glyph reads.
strip = 200
x0 = int(w * 0.28)
x1 = int(w * 0.72)
crop = im.crop((x0, max(0, h - strip), x1, h))
crop = crop.resize((crop.width * 2, crop.height * 2), Image.LANCZOS)
crop.save(dest)
print(f"capture-dock-trash: saved {dest} ({crop.width}x{crop.height})")
PY
}

# prepare_state <state>: lay out the scratch home trash for one state.
prepare_state() {
    local state="$1"
    local data="$SCRATCH/data-$state"
    rm -rf "$data"
    mkdir -p "$data"
    case "$state" in
        empty)
            mkdir -p "$data/Trash/files" "$data/Trash/info"
            ;;
        full)
            mkdir -p "$data/Trash/files" "$data/Trash/info"
            printf 'deleted report\n' >"$data/Trash/files/report.pdf"
            printf 'deleted notes\n'  >"$data/Trash/files/notes.txt"
            cat >"$data/Trash/info/report.pdf.trashinfo" <<EOF
[Trash Info]
Path=/home/user/Documents/report.pdf
DeletionDate=2026-09-25T11:00:00
EOF
            cat >"$data/Trash/info/notes.txt.trashinfo" <<EOF
[Trash Info]
Path=/home/user/Documents/notes.txt
DeletionDate=2026-09-25T11:00:00
EOF
            ;;
        unavailable)
            # A regular file where the Trash directory must be makes the store
            # uncreatable, so the monitor reads it as unavailable.
            printf 'blocked\n' >"$data/Trash"
            ;;
    esac
    echo "$data"
}

start_demo() {
    local data="$1"
    rm -f "$SYNTH"
    env XDG_DATA_HOME="$data" DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-trash: synthetic socket never appeared; demo log:" >&2
        tail -40 "$SCRATCH/demo.log" >&2
        exit 1
    fi
    sleep "$SETTLE"
}

start_settingsd

for state in empty full unavailable; do
    data="$(prepare_state "$state")"
    echo "capture-dock-trash: $state (launching the nested demo)"
    start_demo "$data"
    set_key "dock.position" "<'bottom'>"
    set_key "appearance.colorScheme" "<'dark'>"
    sleep 2
    capture "$state" dark
    set_key "appearance.colorScheme" "<'light'>"
    sleep 2
    capture "$state" light
    stop_demo
done

echo "capture-dock-trash: done"