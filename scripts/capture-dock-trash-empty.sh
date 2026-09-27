#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7r Dock Trash empty progress and result capture.
#
# A real empty finishes too fast to catch the busy state reliably, so the
# shell's `DF_DOCK_TRASH_EMPTY_FIXTURE` capture seam drives the progress/result
# popover into `busy` and `success` deterministically, over a populated trash.
# Each state is captured in light and dark:
#
#   * t14-dock-trash-empty-busy-light.png / -dark.png
#   * t14-dock-trash-empty-success-light.png / -dark.png
#   * t14-dock-trash-empty.png (busy over success composite)
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-10}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-trash-empty.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-trash-empty}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-trash-empty: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-trash-empty: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-trash-empty: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-trash-empty: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-trash-empty: $DBUS_DEST already owned — stop the running settingsd" >&2
    exit 1
fi

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
XDG_DIR="$SCRATCH/xdg"
DATA="$SCRATCH/data"
rm -f "$SYNTH"

# A populated home trash so the emptying state is honest.
mkdir -p "$DATA/Trash/files" "$DATA/Trash/info"
for n in report.pdf notes.txt photo.png; do
    printf 'deleted %s\n' "$n" >"$DATA/Trash/files/$n"
    cat >"$DATA/Trash/info/$n.trashinfo" <<EOF
[Trash Info]
Path=/home/user/Documents/$n
DeletionDate=2026-09-25T11:00:00
EOF
done

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
    echo "capture-dock-trash-empty: settingsd never took $DBUS_DEST; log:" >&2
    tail -20 "$SCRATCH/settingsd.log" >&2
    exit 1
}

# run_mode <state> <scheme>: one nested run with the trash-empty seam on.
run_mode() {
    local state="$1" scheme="$2"
    local raw="$SCRATCH/$state-$scheme-raw.png"
    local crop="$SCRATCH/$state-$scheme-crop.png"
    set_key "dock.position" "<'bottom'>"
    set_key "dock.size" "<0.5>"
    set_key "appearance.colorScheme" "<'$scheme'>"
    rm -f "$SYNTH"
    env QT_QPA_PLATFORM=wayland QML_IMPORT_PATH="$PWD/build/qml" \
        XDG_DATA_HOME="$DATA" \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        DF_DOCK_TRASH_EMPTY_FIXTURE="$state" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo-$state-$scheme.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-trash-empty: synthetic socket never appeared for $state/$scheme; demo log:" >&2
        tail -40 "$SCRATCH/demo-$state-$scheme.log" >&2
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
# The Dock and the popover above the Trash: the bottom band, full width so the
# Trash entry and the anchored popover both read.
band = im.crop((0, max(0, h - 420), w, h))
band = band.resize((band.width * 2, band.height * 2), Image.LANCZOS)
band.save(dest)
print(f"capture-dock-trash-empty: cropped {dest} ({band.width}x{band.height})")
PY
    stop_demo
}

start_settingsd
run_mode busy light
run_mode busy dark
run_mode success light
run_mode success dark

cp "$SCRATCH/busy-light-crop.png" "$OUTDIR/t14-dock-trash-empty-busy-light.png"
cp "$SCRATCH/busy-dark-crop.png" "$OUTDIR/t14-dock-trash-empty-busy-dark.png"
cp "$SCRATCH/success-light-crop.png" "$OUTDIR/t14-dock-trash-empty-success-light.png"
cp "$SCRATCH/success-dark-crop.png" "$OUTDIR/t14-dock-trash-empty-success-dark.png"

DEST="$OUTDIR/t14-dock-trash-empty.png"
python3 - "$SCRATCH/busy-light-crop.png" "$SCRATCH/success-light-crop.png" "$DEST" <<'PY'
import sys
from PIL import Image, ImageDraw

busy, success, dest = sys.argv[1:4]
labels = ["emptying: delayed busy indicator", "succeeded: check + removed count"]
panels = [Image.open(busy), Image.open(success)]
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
print(f"capture-dock-trash-empty: saved {dest} ({out.width}x{out.height})")
PY

echo "capture-dock-trash-empty: done"