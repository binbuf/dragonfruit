#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7q Dock overflow cell and "More Windows" popover capture.
#
# A full-size nested output fits dozens of running groups, so the overflow cell
# cannot be produced by launching a realistic number of apps. The shell's
# `DF_DOCK_OVERFLOW_FIXTURE=1` capture seam (T-14.7q) injects synthetic running
# groups and caps the effective Dock axis, producing a *genuine* overflow: the
# pure planner folds the groups that do not fit into the terminal cell, which
# the seam then opens. Runs light and dark:
#
#   * light — the Dock with the overflow cell and its open "More Windows" list;
#   * dark  — the same in the other color scheme.
#
# Stills:
#
#   * docs/captures/t14-dock-overflow-light.png
#   * docs/captures/t14-dock-overflow-dark.png
#   * docs/captures/t14-dock-overflow.png (light over dark)
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-10}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-overflow.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-overflow}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
SECOND_SETTINGS="$PWD/build/apps/settings/dragonfruit-settings"
FILES_APP="$PWD/build/apps/files/dragonfruit-files"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-overflow: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
for bin in "$SETTINGS_BIN" "$SECOND_SETTINGS" "$FILES_APP"; do
    if [ ! -x "$bin" ]; then
        echo "capture-dock-overflow: $bin not built — run make build" >&2
        exit 1
    fi
done
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-overflow: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-overflow: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-overflow: $DBUS_DEST already owned — stop the running settingsd" >&2
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

# run_mode <key> <light|dark>: one nested run with the overflow seam on.
run_mode() {
    local key="$1" scheme="$2"
    local raw="$SCRATCH/$key-raw.png"
    local crop="$SCRATCH/$key-crop.png"
    local demo_args="--socket-name $SOCKET --launch $SECOND_SETTINGS --launch $FILES_APP"
    set_key "dock.position" "<'bottom'>"
    set_key "dock.size" "<0.5>"
    set_key "appearance.colorScheme" "<'$scheme'>"
    rm -f "$SYNTH"
    env QT_QPA_PLATFORM=wayland QML_IMPORT_PATH="$PWD/build/qml" \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        DF_DOCK_OVERFLOW_FIXTURE=1 \
        setsid make demo DEMO_ARGS="$demo_args" \
        >"$SCRATCH/demo-$key.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-overflow: synthetic socket never appeared for $key; demo log:" >&2
        tail -40 "$SCRATCH/demo-$key.log" >&2
        exit 1
    fi
    sleep "$SETTLE"
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
# The Dock and the open list above it: the whole bottom third, full width so
# the cell, the divider, and the pinned prefix are all visible.
band = im.crop((0, max(0, h - 380), w, h))
band = band.resize((band.width * 2, band.height * 2), Image.LANCZOS)
band.save(dest)
print(f"capture-dock-overflow: cropped {dest} ({band.width}x{band.height})")
PY
    stop_demo
}

echo "capture-dock-overflow: starting the scratch settingsd"
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

run_mode light light
run_mode dark dark

cp "$SCRATCH/light-crop.png" "$OUTDIR/t14-dock-overflow-light.png"
cp "$SCRATCH/dark-crop.png" "$OUTDIR/t14-dock-overflow-dark.png"

DEST="$OUTDIR/t14-dock-overflow.png"
python3 - "$SCRATCH/light-crop.png" "$SCRATCH/dark-crop.png" "$DEST" <<'PY'
import sys
from PIL import Image, ImageDraw

light, dark, dest = sys.argv[1:4]
labels = ["light: overflow cell (last app item) + open More Windows list",
          "dark: overflow cell (last app item) + open More Windows list"]
panels = [Image.open(light), Image.open(dark)]
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
print(f"capture-dock-overflow: saved {dest} ({out.width}x{out.height})")
PY

echo "capture-dock-overflow: done"