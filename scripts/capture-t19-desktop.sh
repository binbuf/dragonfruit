#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-19.3 Files-owned desktop capture.
#
# Runs the nested demo on the current session bus with a scratch `~/Desktop`
# (an XDG `user-dirs.dirs` pointing Desktop at a fixture tree), seeds a few
# files and folders, then drives the live desktop through the compositor's
# synthetic-input harness. Stills:
#
#   * docs/captures/t19-desktop-items.png    — the desktop with icons + labels
#   * docs/captures/t19-desktop-marquee.png  — a rubber-band drag in progress
#   * docs/captures/t19-desktop-selected.png — the committed selection
#   * docs/captures/t19-desktop-open.png     — double-click opened a Files window
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-10}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t19-desktop.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t19-desktop}"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t19-desktop: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t19-desktop: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t19-desktop: python3 Pillow not found" >&2
    exit 1
}
if [ ! -x build/apps/files/dragonfruit-files ]; then
    echo "capture-t19-desktop: build/apps/files/dragonfruit-files missing — run make build" >&2
    exit 1
fi

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
XDG_DIR="$SCRATCH/xdg"
DESKTOP_DIR="$SCRATCH/Desktop"
rm -f "$SYNTH"
mkdir -p "$XDG_DIR" "$DESKTOP_DIR"
# The compositor output / nested window size, read from the demo log once the
# desktop commits its first frame; used to centre-crop the host capture.
OUT_W=1920
OUT_H=1200

seed_desktop() {
    for name in Alpha Bravo Charlie Delta Echo Foxtrot; do
        printf 'hello %s\n' "$name" >"$DESKTOP_DIR/$name.txt"
    done
    mkdir -p "$DESKTOP_DIR/Projects" "$DESKTOP_DIR/Photos"
    printf 'notes\n' >"$DESKTOP_DIR/Projects/notes.txt"
}
seed_desktop

# Point Qt's Desktop standard location at the scratch tree without touching the
# developer's real `$HOME/.config`.
mkdir -p "$XDG_DIR"
printf 'XDG_DESKTOP_DIR="%s"\n' "$DESKTOP_DIR" >"$XDG_DIR/user-dirs.dirs"

cleanup() {
    stop_demo
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

stop_demo() {
    if [ -n "${DEMO_PID:-}" ]; then
        kill -TERM -"$DEMO_PID" 2>/dev/null || kill -TERM "$DEMO_PID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PID" 2>/dev/null || kill -KILL "$DEMO_PID" 2>/dev/null || true
        DEMO_PID=""
    fi
    for _ in $(seq 1 100); do [ ! -e "$SYNTH" ] && break; sleep 0.05; done
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
    # to raise the nested window before an active-window capture.
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_t19_desktop >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

# synth <verb...>: send one command (or a drag sequence) over the datagram
# harness. Coordinates are device-normalized 0..1 of the output.
synth() {
    python3 - "$SYNTH" "$@" <<'PY'
import os
import socket
import sys
import time

path = sys.argv[1]
verb = sys.argv[2]
client = f"/tmp/dragonfruit-t19-desktop-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.bind(client)

def send(msg, delay=0.25):
    sock.sendto(msg.encode(), path)
    time.sleep(delay)

if verb == "drag":
    sx, sy, ex, ey = (float(v) for v in sys.argv[3:7])
    send(f"motion-abs {sx:.5f} {sy:.5f}")
    send("button 272 down")
    for i in range(1, 9):
        x = sx + (ex - sx) * i / 8
        y = sy + (ey - sy) * i / 8
        send(f"motion-abs {x:.5f} {y:.5f}", 0.08)
    time.sleep(0.6)  # hold the button; the caller captures the marquee
elif verb == "release":
    send("button 272 up")
elif verb == "double":
    x, y = float(sys.argv[3]), float(sys.argv[4])
    send(f"motion-abs {x:.5f} {y:.5f}")
    send("button 272 down", 0.06)
    send("button 272 up", 0.06)
    send("button 272 down", 0.06)
    send("button 272 up", 1.5)
PY
}

# crop <raw> <dest>: alpha-crop to the nested window then centre-crop to the
# configured output size, so the stills are all OUT_W x OUT_H.
crop() {
    local raw="$1" dest="$2" w="$3" h="$4"
    python3 - "$raw" "$dest" "$w" "$h" <<'PY'
import sys
from PIL import Image

raw, dest, out_w, out_h = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
im = Image.open(raw)
if im.mode == "RGBA":
    bbox = im.getchannel("A").getbbox()
    if bbox:
        im = im.crop(bbox)
im = im.convert("RGB")
w, h = im.size
if w >= out_w and h >= out_h:
    x0 = (w - out_w) // 2
    y0 = (h - out_h) // 2
    im = im.crop((x0, y0, x0 + out_w, y0 + out_h))
im.save(dest)
print(f"capture-t19-desktop: saved {dest} ({im.size[0]}x{im.size[1]})")
PY
}

echo "capture-t19-desktop: starting the nested demo (socket $SOCKET)"
XDG_CONFIG_HOME="$XDG_DIR" \
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$SCRATCH/demo.log" 2>&1 &
DEMO_PID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-t19-desktop: synthetic socket never appeared; demo log:" >&2
    tail -40 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

# The desktop process logs the output size; use it for the crop.
read -r OUT_W OUT_H < <(python3 - "$SCRATCH/demo.log" <<'PY'
import re
import sys

text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
m = re.search(r"dragonfruit-files-desktop: configured (\d+)x(\d+)", text)
print(m.group(1), m.group(2)) if m else print(1920, 1200)
PY
)
echo "capture-t19-desktop: output ${OUT_W}x${OUT_H}"

# 1. The desktop with items.
raise_dragonfruit
spectacle -b -n -a -o "$SCRATCH/items-raw.png"
crop "$SCRATCH/items-raw.png" "$OUTDIR/t19-desktop-items.png" "$OUT_W" "$OUT_H"

# 2. A rubber-band drag in progress. Start on empty background below the icon
#    row (not over a window) and sweep up-left across the first tiles.
synth drag 0.0781 0.4167 0.2604 0.0333
raise_dragonfruit
spectacle -b -n -a -o "$SCRATCH/marquee-raw.png"
crop "$SCRATCH/marquee-raw.png" "$OUTDIR/t19-desktop-marquee.png" "$OUT_W" "$OUT_H"
synth release
sleep 1

# 3. The committed selection after the band releases.
raise_dragonfruit
spectacle -b -n -a -o "$SCRATCH/selected-raw.png"
crop "$SCRATCH/selected-raw.png" "$OUTDIR/t19-desktop-selected.png" "$OUT_W" "$OUT_H"

# 4. Double-click the second folder (Projects, which holds notes.txt) to open a
#    Files window at that path.
synth double 0.09896 0.0467
sleep 4
raise_dragonfruit
spectacle -b -n -a -o "$SCRATCH/open-raw.png"
crop "$SCRATCH/open-raw.png" "$OUTDIR/t19-desktop-open.png" "$OUT_W" "$OUT_H"

if grep -qi "dragonfruit-files-desktop: surface closed" "$SCRATCH/demo.log"; then
    echo "capture-t19-desktop: WARNING — the desktop surface closed mid-capture" >&2
fi

echo "capture-t19-desktop: done"