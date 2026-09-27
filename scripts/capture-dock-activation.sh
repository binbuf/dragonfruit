#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7g Dock activation and launch correctness capture.
#
# Runs the nested demo four times against a scratch settingsd and the shell's
# `DF_DOCK_ACTIVATION_FIXTURE` seam (the production click tree without a
# synthetic pointer tap):
#
#   * before   — Settings pinned, not running (no fixture);
#   * launch   — fixture activates the stopped entry: Settings launches and
#                its window maps;
#   * activate — fixture activates the already-running Settings (the demo's
#                own Qt app), which focuses it;
#   * missing  — fixture activates an unresolved pinned entry, which raises a
#                notice instead of silently no-op'ing.
#
# Stills:
#
#   * docs/captures/t14-dock-activation-{before,after,active}.png (Dock band)
#   * docs/captures/t14-dock-activation.png (before/after/active stacked)
#   * docs/captures/t14-dock-activation-missing.png (the notice)
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-8}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-activation.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-activation}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"
SETTINGS_APP="org.dragonfruit.Settings.desktop"

mkdir -p "$OUTDIR" "$SCRATCH" "$SCRATCH/xdg"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-activation: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-activation: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-activation: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-activation: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-activation: $DBUS_DEST already owned — stop the running settingsd" >&2
    exit 1
fi

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
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

# run_demo <name> <fixture> <qt-app>
run_demo() {
    local name="$1" fixture="$2" qt_app="$3"
    rm -f "$SYNTH"
    local envs=(DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" DF_DEMO_QT_APP="$qt_app")
    if [ -n "$fixture" ]; then
        envs+=(DF_DOCK_ACTIVATION_FIXTURE="$fixture")
    fi
    env "${envs[@]}" setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo-$name.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-dock-activation: synthetic socket never appeared for $name" >&2
        tail -40 "$SCRATCH/demo-$name.log" >&2
        exit 1
    fi
    sleep "$SETTLE"
    spectacle -b -n -a -o "$SCRATCH/$name-raw.png"
    stop_demo
}

# crop_shot <raw> <name>: alpha-crop to the nested window, save the full
# nested still and a 2x Dock band.
crop_shot() {
    local raw="$1" name="$2"
    python3 - "$raw" "$SCRATCH/$name-full.png" "$OUTDIR/t14-dock-activation-$name.png" <<'PY'
import sys
from PIL import Image

raw, full_dest, band_dest = sys.argv[1:4]
NESTED_W, NESTED_H = 1920, 1200
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
im.save(full_dest)
w, h = im.size
band = im.crop((int(w * 0.30), max(0, h - 210), int(w * 0.70), h))
band = band.resize((band.width * 2, band.height * 2), Image.LANCZOS)
band.save(band_dest)
print(f"capture-dock-activation: saved {band_dest} ({band.width}x{band.height})")
PY
}

echo "capture-dock-activation: starting the scratch settingsd"
XDG_CONFIG_HOME="$SCRATCH/xdg" "$SETTINGS_BIN" >>"$SCRATCH/settingsd.log" 2>&1 &
SETTINGS_PID=$!
for _ in $(seq 1 200); do
    if gdbus call --session --dest org.freedesktop.DBus \
            --object-path /org/freedesktop/DBus \
            --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
        break
    fi
    sleep 0.05
done
set_key "dock.pinned" "<['$SETTINGS_APP']>"
set_key "dock.position" "<'bottom'>"
set_key "appearance.colorScheme" "<'dark'>"

run_demo before "" /bin/true
crop_shot "$SCRATCH/before-raw.png" before

run_demo after launch /bin/true
crop_shot "$SCRATCH/after-raw.png" after

# The activate panel uses the demo's own Settings (running); the fixture waits
# for it, then activates it.
run_demo active activate ""
crop_shot "$SCRATCH/active-raw.png" active

# Stack before/after/active from the full nested stills so the window and the
# Dock both read.
python3 - "$SCRATCH/before-full.png" "$SCRATCH/after-full.png" \
    "$SCRATCH/active-full.png" "$OUTDIR/t14-dock-activation.png" <<'PY'
import sys
from PIL import Image, ImageDraw

paths = sys.argv[1:4]
dest = sys.argv[4]
labels = ["before: Settings pinned, not running",
          "after launch: Settings window mapped, running indicator on",
          "after activate: the running Settings window is focused"]
imgs = [Image.open(p).convert("RGB") for p in paths]
target_w = 960
scaled = [im.resize((target_w, int(im.height * target_w / im.width)), Image.LANCZOS)
          for im in imgs]
label_h, gap = 26, 8
w = target_w
h = sum(im.height + label_h for im in scaled) + gap * (len(scaled) - 1)
out = Image.new("RGB", (w, h), (24, 24, 28))
draw = ImageDraw.Draw(out)
y = 0
for im, label in zip(scaled, labels):
    draw.text((8, y + 6), label, fill=(230, 230, 235))
    y += label_h
    out.paste(im, (0, y))
    y += im.height + gap
out.save(dest)
print(f"capture-dock-activation: saved {dest} ({out.width}x{out.height})")
PY

# The missing panel pins an unresolved identity, so the Dock shows the
# not-found mark; the fixture also activates it, raising the (headless, since
# `make demo` runs no notification service) notice through the real path.
set_key "dock.pinned" "<['org.example.NotInstalled.desktop']>"
run_demo missing missing /bin/true
crop_shot "$SCRATCH/missing-raw.png" missing

echo "capture-dock-activation: done"