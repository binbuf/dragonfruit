#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-16.7 localization capture.
#
# Produces the locale-switch A/B stills in docs/captures/:
#
#   * t16-i18n-en.png   the Settings window with the shell's default strings
#   * t16-i18n-es.png   the same surface rendered in Spanish
#   * t16-i18n.png      the two stills stacked (English above, Spanish below)
#
# The locale is chosen at process start, so each still is a fresh nested demo
# launched with `DRAGONFRUIT_LOCALE` set to `en_US` (no catalog — the source
# strings) or `es_ES` (the checked-in catalog). The Settings app opens on the
# General pane, whose header card and sidebar carry the translated strings.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, and the
# built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-14}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t16-i18n.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t16-i18n}"
PANE="${PANE:-general}"

mkdir -p "$OUTDIR"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t16-i18n: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t16-i18n: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t16-i18n: python3 Pillow not found" >&2
    exit 1
}

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
rm -f "$SYNTH"

cleanup() {
    stop_demo
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

start_demo() {
    local locale="$1"
    rm -f "$SYNTH"
    DRAGONFRUIT_LOCALE="$locale" \
        DF_SETTINGS_START_PANE="$PANE" \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo-$locale.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-t16-i18n: synthetic socket never appeared; demo log:" >&2
        tail -30 "$SCRATCH/demo-$locale.log" >&2
        exit 1
    fi
    sleep "$SETTLE"
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
    # df-allow-desktop-name: the KWin host compositor's scripting D-Bus, used
    # only to raise the nested window for the active-window capture.
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_capture_raise >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

nudge() {
    python3 - "$SYNTH" <<'PY'
import os
import socket
import sys
import time

path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/dragonfruit-t16-i18n-nudge-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)
for x in (0.20, 0.80, 0.50):
    sock.sendto(f"motion-abs {x} 0.97".encode(), path)
    time.sleep(0.25)
time.sleep(1.0)
PY
}

zoom_settings() {
    python3 - "$SYNTH" <<'PY'
import os
import socket
import sys
import time

path = sys.argv[1]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/dragonfruit-t16-i18n-zoom-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)

def send(command):
    sock.sendto(command.encode(), path)

send("motion-abs 0.469 0.2267")
time.sleep(0.3)
for _ in range(2):
    send("button 272 down")
    send("button 272 up")
    time.sleep(0.08)
time.sleep(1.2)
PY
}

capture() {
    local dest="$1"
    local raw="$SCRATCH/$(basename "$dest" .png)-raw.png"
    raise_dragonfruit
    nudge
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
print(f"capture-t16-i18n: saved {dest} ({im.width}x{im.height})")
PY
}

# --- English baseline. ------------------------------------------------------
echo "capture-t16-i18n: English (en_US)"
start_demo en_US
zoom_settings
capture "$OUTDIR/t16-i18n-en.png"
stop_demo

# --- Spanish. ---------------------------------------------------------------
echo "capture-t16-i18n: Spanish (es_ES)"
start_demo es_ES
zoom_settings
capture "$OUTDIR/t16-i18n-es.png"
stop_demo

# --- Stack the two for the one-glance artifact. -----------------------------
python3 - "$OUTDIR/t16-i18n-en.png" "$OUTDIR/t16-i18n-es.png" "$OUTDIR/t16-i18n.png" <<'PY'
import sys
from PIL import Image

en, es, dest = sys.argv[1], sys.argv[2], sys.argv[3]
top = Image.open(en).convert("RGB")
bottom = Image.open(es).convert("RGB")
width = max(top.width, bottom.width)
image = Image.new("RGB", (width, top.height + bottom.height), (0, 0, 0))
image.paste(top, (0, 0))
image.paste(bottom, (0, top.height))
image.save(dest)
print(f"capture-t16-i18n: saved {dest} ({image.width}x{image.height})")
PY

echo "capture-t16-i18n: done"