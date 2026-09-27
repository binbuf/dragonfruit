#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7s Dock minimize-to-icon reaction capture.
#
# The reaction is a ~320 ms one-hop bounce, too fast to catch reliably on a real
# minimize. The shell's `DF_DOCK_MINIMIZE_REACTION_FIXTURE=<phase>` capture seam
# pins a constant mid-bounce phase (0.5 = the peak) on the first running entry
# so a still is deterministic. Captured on a bottom Dock (light) and a right
# Dock (dark) so both the axis and the direction read:
#
#   * docs/captures/t14-dock-minimize-reaction-bottom-light.png
#   * docs/captures/t14-dock-minimize-reaction-right-dark.png
#   * docs/captures/t14-dock-minimize-reaction.png (bottom over right)
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-10}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-minimize.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-minimize}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
SECOND_SETTINGS="$PWD/build/apps/settings/dragonfruit-settings"
FILES_APP="$PWD/build/apps/files/dragonfruit-files"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-minimize-reaction: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
for bin in "$SETTINGS_BIN" "$SECOND_SETTINGS" "$FILES_APP"; do
    if [ ! -x "$bin" ]; then
        echo "capture-dock-minimize-reaction: $bin not built — run make build" >&2
        exit 1
    fi
done
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-minimize-reaction: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-minimize-reaction: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-minimize-reaction: $DBUS_DEST already owned — stop the running settingsd" >&2
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

# run_mode <key> <position> <light|dark>: one nested run with the reaction seam
# pinned at the peak (0.5).
run_mode() {
    local key="$1" position="$2" scheme="$3"
    local raw="$SCRATCH/$key-raw.png"
    local crop="$SCRATCH/$key-crop.png"
    local demo_args="--socket-name $SOCKET --launch $SECOND_SETTINGS --launch $FILES_APP"
    set_key "dock.position" "<'$position'>"
    set_key "dock.size" "<0.5>"
    set_key "appearance.colorScheme" "<'$scheme'>"
    # The key itself is on so the reaction is honest even without the seam.
    set_key "dock.minimizeReaction" "<true>"
    rm -f "$SYNTH"
    env QT_QPA_PLATFORM=wayland QML_IMPORT_PATH="$PWD/build/qml" \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        DF_DOCK_MINIMIZE_REACTION_FIXTURE=0.5 \
        setsid make demo DEMO_ARGS="$demo_args" \
        >"$SCRATCH/demo-$key.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-minimize-reaction: synthetic socket never appeared for $key; demo log:" >&2
        tail -40 "$SCRATCH/demo-$key.log" >&2
        exit 1
    fi
    sleep "$SETTLE"
    spectacle -b -n -a -o "$raw"
    python3 - "$raw" "$crop" "$position" <<'PY'
import sys
from PIL import Image

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
if position == "bottom":
    # The Dock band at the bottom, full width so the bouncing tile reads in
    # context with its neighbours.
    band = im.crop((0, max(0, h - 260), w, h))
else:
    # A vertical Dock: the right edge, full height so the hop toward the
    # interior is visible.
    band = im.crop((max(0, w - 260), 0, w, h))
band = band.resize((band.width * 2, band.height * 2), Image.LANCZOS)
band.save(dest)
print(f"capture-dock-minimize-reaction: cropped {dest} ({band.width}x{band.height})")
PY
    stop_demo
}

echo "capture-dock-minimize-reaction: starting the scratch settingsd"
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

run_mode bottom bottom light
run_mode right right dark

cp "$SCRATCH/bottom-crop.png" "$OUTDIR/t14-dock-minimize-reaction-bottom-light.png"
cp "$SCRATCH/right-crop.png" "$OUTDIR/t14-dock-minimize-reaction-right-dark.png"

DEST="$OUTDIR/t14-dock-minimize-reaction.png"
python3 - "$SCRATCH/bottom-crop.png" "$SCRATCH/right-crop.png" "$DEST" <<'PY'
import sys
from PIL import Image, ImageDraw

bottom, right, dest = sys.argv[1:4]
labels = ["bottom Dock (light): the acting tile at the peak of its one-hop bounce",
          "right Dock (dark): the hop points away from the screen edge"]
panels = [Image.open(bottom), Image.open(right)]
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
print(f"capture-dock-minimize-reaction: saved {dest} ({out.width}x{out.height})")
PY

echo "capture-dock-minimize-reaction: done"