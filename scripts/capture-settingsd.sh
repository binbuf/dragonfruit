#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-08.3 settingsd restart/resync track capture.
#
# Produces the T-08 slice artifacts in docs/captures/:
#
#   * t08-settingsd.txt          the D-Bus flip transcript + the persisted file
#   * t08-settingsd-before-dark.png   settingsd seeded dark, dock.size 0.5
#   * t08-settingsd-after-light.png   colorScheme + dock.size flipped live
#   * t08-settingsd-down.png          after `kill -9` settingsd (nothing lost)
#   * t08-settingsd-restart.png       settingsd restarted; the shell re-synced
#                                      to a value changed on disk while it was
#                                      down (dark + a smaller Dock)
#   * t08-settingsd.png               the representative still (the restart)
#   * t08-settingsd.mp4               the four stills in order
#
# The `kill -9` half needs no real daemon kill in CI (that guarantee is the
# `cargo test -p dragonfruit-settingsd --test restart` suite); this script is
# the human-verifiable recording. It runs the nested demo on the current
# session bus alongside a scratch-home settingsd, drives the flip with
# `gdbus`, and screenshots the active Dragonfruit window with Spectacle.
#
# Requires: a host Wayland session, `spectacle`, `ffmpeg`, python3 with
# Pillow, and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t08-capture.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t08-settingsd}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
TRANSCRIPT="$OUTDIR/t08-settingsd.txt"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-settingsd: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-settingsd: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle ffmpeg python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-settingsd: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-settingsd: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-settingsd: $DBUS_DEST is already owned — stop the running settingsd first" >&2
    exit 1
fi

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
XDG_DIR="$SCRATCH/xdg"
SETTINGS_FILE="$XDG_DIR/dragonfruit/settings.json"
rm -f "$SYNTH"
: >"$TRANSCRIPT"
echo "# T-08.3 settingsd flip + restart capture" >>"$TRANSCRIPT"
echo "# session bus: $DBUS_DEST at $DBUS_PATH (scratch XDG_CONFIG_HOME=$XDG_DIR)" >>"$TRANSCRIPT"

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

# stop_settingsd kills with SIGKILL — exactly the "kill -9" the demo promises.
# It is a no-op when settingsd is already down.
stop_settingsd() {
    if [ -n "${SETTINGS_PID:-}" ]; then
        kill -KILL "$SETTINGS_PID" 2>/dev/null || true
        wait "$SETTINGS_PID" 2>/dev/null || true
        SETTINGS_PID=""
    fi
    for _ in $(seq 1 200); do
        if ! gdbus call --session --dest org.freedesktop.DBus \
                --object-path /org/freedesktop/DBus \
                --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
            return
        fi
        sleep 0.05
    done
    echo "capture-settingsd: $DBUS_DEST was not released after kill -9" >&2
    exit 1
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
    echo "capture-settingsd: settingsd never took $DBUS_DEST; log:" >&2
    tail -20 "$SCRATCH/settingsd.log" >&2
    exit 1
}

set_key() {
    local key="$1" value="$2"
    echo "\$ $DBUS_IFACE.Set $key $value" >>"$TRANSCRIPT"
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Set" "$key" "$value" >>"$TRANSCRIPT"
}

get_key() {
    local key="$1"
    echo "\$ $DBUS_IFACE.Get $key" >>"$TRANSCRIPT"
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Get" "$key" >>"$TRANSCRIPT"
}

raise_dragonfruit() {
    command -v gdbus >/dev/null 2>&1 || return 0
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

# capture <dest>: active-window still, shadow trimmed and cropped to the
# nested 1920x1200 output rectangle.
capture() {
    local dest="$1"
    local raw="$SCRATCH/$(basename "$dest" .png)-raw.png"
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
if im.width >= NESTED_W and im.height >= NESTED_H:
    x0 = (im.width - NESTED_W) // 2
    y0 = im.height - NESTED_H
    im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
im.convert("RGB").save(dest)
print(f"capture-settingsd: saved {dest} ({im.width}x{im.height})")
PY
}

# --- 1. settingsd seeded dark before the shell starts. ---------------------
start_settingsd
set_key "appearance.colorScheme" "<'dark'>"
set_key "dock.size" "<0.5>"

# --- 2. the nested demo (the shell binds Dock + Theme to settingsd). -------
echo "capture-settingsd: launching the nested demo ($SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 300); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-settingsd: synthetic socket never appeared; demo log:" >&2
    tail -30 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"
capture "$OUTDIR/t08-settingsd-before-dark.png"

# --- 3. the live D-Bus flip: theme + Dock react in one beat. ---------------
echo "capture-settingsd: flipping appearance.colorScheme + dock.size"
set_key "appearance.colorScheme" "<'light'>"
set_key "dock.size" "<0.75>"
sleep 2
capture "$OUTDIR/t08-settingsd-after-light.png"

# --- 4. kill -9: the desktop keeps the last-known state. -------------------
echo "capture-settingsd: kill -9 settingsd (nothing visually lost)"
echo "# kill -9 settingsd; the shell keeps its last-known values" >>"$TRANSCRIPT"
stop_settingsd
sleep 1
capture "$OUTDIR/t08-settingsd-down.png"

# --- 5. a change made while down, then restart: the shell re-syncs. --------
# The daemon is the durable owner; a value changed on disk while it was down
# is loaded on restart and the shell's `GetAll` resync applies it. This is the
# visible form of "the while-down change is not silently dropped".
echo "capture-settingsd: editing the durable file while settingsd is down"
python3 - "$SETTINGS_FILE" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path) as handle:
    document = json.load(handle)
document["keys"]["appearance.colorScheme"] = "dark"
document["keys"]["dock.size"] = 0.35
with open(path, "w") as handle:
    json.dump(document, handle, indent=2)
    handle.write("\n")
print("capture-settingsd: wrote dark + dock.size 0.35 while down")
PY
echo "# while down: appearance.colorScheme=dark, dock.size=0.35 written to disk" >>"$TRANSCRIPT"
echo "capture-settingsd: restarting settingsd (shell re-syncs via GetAll)"
start_settingsd
sleep 2
capture "$OUTDIR/t08-settingsd-restart.png"
get_key "appearance.colorScheme"
get_key "dock.size"

cp "$OUTDIR/t08-settingsd-restart.png" "$OUTDIR/t08-settingsd.png"

# --- 6. the persisted file and a short clip. -------------------------------
{
    echo
    echo "# persisted $SETTINGS_FILE after the restart"
    cat "$SETTINGS_FILE"
} >>"$TRANSCRIPT"

clip="$SCRATCH/concat.txt"
: >"$clip"
for still in before-dark after-light down restart; do
    path="$OUTDIR/t08-settingsd-$still.png"
    [ -f "$path" ] || continue
    printf "file '%s'\nduration 1.6\n" "$(realpath "$path")" >>"$clip"
done
printf "file '%s'\n" "$(realpath "$OUTDIR/t08-settingsd-restart.png")" >>"$clip"
encoder="mpeg4"
for candidate in libx264 libopenh264; do
    if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q " $candidate "; then
        encoder="$candidate"
        break
    fi
done
ffmpeg -y -loglevel error -f concat -safe 0 -i "$clip" \
    -vf "scale=1280:-2,format=yuv420p" -c:v "$encoder" -crf 28 -movflags +faststart \
    "$OUTDIR/t08-settingsd.mp4"
echo "capture-settingsd: wrote $OUTDIR/t08-settingsd.mp4 ($encoder)"

echo "capture-settingsd: done"