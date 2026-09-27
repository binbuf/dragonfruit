#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7n Dock window chooser row-discipline capture.
#
# Runs the nested demo twice (light and dark) with several Settings windows so
# the app's Dock entry is a long group, and uses the shell's
# `DF_DOCK_CHOOSER_FIXTURE=scroll` seam to open the chooser, scroll the bounded
# viewport mid-list, and highlight a visible row.
#
# Stills:
#
#   * docs/captures/t14-dock-chooser-scroll-light.png
#   * docs/captures/t14-dock-chooser-scroll-dark.png
#   * docs/captures/t14-dock-chooser-scroll.png (light over dark)
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-12}"
# Several more Settings windows than `component.dock.chooser.maxRows` (7) so
# the chooser caps and the list scrolls well past the first screenful.
LAUNCHES="${LAUNCHES:-11}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-chooser-scroll.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-chooser-scroll}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
SECOND_APP="$PWD/build/apps/settings/dragonfruit-settings"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-chooser-scroll: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-chooser-scroll: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
if [ ! -x "$SECOND_APP" ]; then
    echo "capture-dock-chooser-scroll: $SECOND_APP not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-chooser-scroll: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-chooser-scroll: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-chooser-scroll: $DBUS_DEST already owned — stop the running settingsd" >&2
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

# run_mode <light|dark>: one nested run with a long grouped Settings entry and
# the chooser scrolled mid-list.
run_mode() {
    local scheme="$1"
    local raw="$SCRATCH/$scheme-raw.png"
    local crop="$SCRATCH/$scheme-crop.png"
    local demo_args="--socket-name $SOCKET"
    local i
    for i in $(seq 1 "$LAUNCHES"); do
        demo_args="$demo_args --launch $SECOND_APP"
    done
    set_key "appearance.colorScheme" "<'$scheme'>"
    rm -f "$SYNTH"
    env QT_QPA_PLATFORM=wayland QML_IMPORT_PATH="$PWD/build/qml" \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" DF_DOCK_CHOOSER_FIXTURE=scroll \
        setsid make demo DEMO_ARGS="$demo_args" \
        >"$SCRATCH/demo-$scheme.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-chooser-scroll: synthetic socket never appeared for $scheme; demo log:" >&2
        tail -40 "$SCRATCH/demo-$scheme.log" >&2
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
# The capped chooser floats just above the Dock at the bottom centre.
crop = im.crop((int(w * 0.28), max(0, h - 540), int(w * 0.72), h))
crop = crop.resize((crop.width * 2, crop.height * 2), Image.LANCZOS)
crop.save(dest)
print(f"capture-dock-chooser-scroll: cropped {dest} ({crop.width}x{crop.height})")
PY
    stop_demo
}

echo "capture-dock-chooser-scroll: starting the scratch settingsd"
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

run_mode light
run_mode dark

cp "$SCRATCH/light-crop.png" "$OUTDIR/t14-dock-chooser-scroll-light.png"
cp "$SCRATCH/dark-crop.png" "$OUTDIR/t14-dock-chooser-scroll-dark.png"

DEST="$OUTDIR/t14-dock-chooser-scroll.png"
python3 - "$SCRATCH/light-crop.png" "$SCRATCH/dark-crop.png" "$DEST" <<'PY'
import sys
from PIL import Image, ImageDraw

light, dark, dest = sys.argv[1:4]
labels = ["light: long list capped at 7 rows, scrolled mid-list",
          "dark: long list capped at 7 rows, scrolled mid-list"]
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
print(f"capture-dock-chooser-scroll: saved {dest} ({out.width}x{out.height})")
PY

echo "capture-dock-chooser-scroll: done"