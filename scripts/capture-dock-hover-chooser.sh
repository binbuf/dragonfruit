#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7p Dock hover-open chooser capture.
#
# Runs the nested demo twice (light over dark) with `dock.chooserOnHover` on and
# two grouped, multi-window Dock entries (two Settings and two Files windows),
# then uses the shell's `DF_DOCK_HOVER_FIXTURE` seam (the production hover-open
# path minus a real pointer dwell) to show:
#
#   * light — the dwell-open chooser anchored to the first grouped entry;
#   * dark  — the same popover retargeted to the second grouped entry, with the
#             magnification pointer parked between them.
#
# Stills:
#
#   * docs/captures/t14-dock-hover-chooser-light.png
#   * docs/captures/t14-dock-hover-chooser-dark.png
#   * docs/captures/t14-dock-hover-chooser.png (light over dark)
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-11}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-hover.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-hover}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
SETTINGS_APP="$PWD/build/apps/settings/dragonfruit-settings"
FILES_APP="$PWD/build/apps/files/dragonfruit-files"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-hover-chooser: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
for bin in "$SETTINGS_BIN" "$SETTINGS_APP" "$FILES_APP"; do
    if [ ! -x "$bin" ]; then
        echo "capture-dock-hover-chooser: $bin not built — run make build" >&2
        exit 1
    fi
done
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-hover-chooser: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-hover-chooser: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-hover-chooser: $DBUS_DEST already owned — stop the running settingsd" >&2
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

# run_mode <light|dark> <open|retarget>: one nested run. Two Settings windows
# and two Files windows make two grouped entries; the fixture seam opens (and
# for `retarget`, retargets) the hover chooser.
run_mode() {
    local scheme="$1" mode="$2"
    local raw="$SCRATCH/$mode-$scheme-raw.png"
    local crop="$SCRATCH/$mode-$scheme-crop.png"
    set_key "appearance.colorScheme" "<'$scheme'>"
    rm -f "$SYNTH"
    # The extra Settings/Files launches group their Dock entries so the hover
    # chooser is reachable and retargetable. `QML_IMPORT_PATH`/`QT_QPA_PLATFORM`
    # are exported so the launched apps map on the nested Wayland socket.
    env QT_QPA_PLATFORM=wayland QML_IMPORT_PATH="$PWD/build/qml" \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" DF_DOCK_HOVER_FIXTURE="$mode" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET \
            --launch $SETTINGS_APP --launch $SETTINGS_APP \
            --launch $FILES_APP --launch $FILES_APP" \
        >"$SCRATCH/demo-$mode-$scheme.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-hover-chooser: synthetic socket never appeared for $mode/$scheme; demo log:" >&2
        tail -40 "$SCRATCH/demo-$mode-$scheme.log" >&2
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
# The chooser floats just above the Dock at the bottom centre; a wider band
# keeps both grouped entries and the pointer between them in frame.
crop = im.crop((int(w * 0.24), max(0, h - 470), int(w * 0.76), h))
crop = crop.resize((crop.width * 2, crop.height * 2), Image.LANCZOS)
crop.save(dest)
print(f"capture-dock-hover-chooser: cropped {dest} ({crop.width}x{crop.height})")
PY
    stop_demo
}

echo "capture-dock-hover-chooser: starting the scratch settingsd"
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
# The opt-in key under test.
set_key "dock.chooserOnHover" "<true>"

run_mode light open
run_mode dark retarget

cp "$SCRATCH/open-light-crop.png" "$OUTDIR/t14-dock-hover-chooser-light.png"
cp "$SCRATCH/retarget-dark-crop.png" "$OUTDIR/t14-dock-hover-chooser-dark.png"

DEST="$OUTDIR/t14-dock-hover-chooser.png"
python3 - "$SCRATCH/open-light-crop.png" "$SCRATCH/retarget-dark-crop.png" "$DEST" <<'PY'
import sys
from PIL import Image, ImageDraw

light, dark, dest = sys.argv[1:4]
labels = ["light: hover dwell opens the chooser on the first grouped entry",
          "dark: pointer between entries retargets the same popover"]
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
print(f"capture-dock-hover-chooser: saved {dest} ({out.width}x{out.height})")
PY

echo "capture-dock-hover-chooser: done"