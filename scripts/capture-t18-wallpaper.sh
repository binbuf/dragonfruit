#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-18.2 Wallpaper-pane capture.
#
# Produces the T-18 pane artifacts in docs/captures/:
#
#   * t18-wallpaper-offline.png   cold cache + dead network: the shipped
#                                 default desktop and the "available soon" row
#   * t18-wallpaper-fetching.png  the Featured row downloading (skeletons)
#   * t18-wallpaper-filled.png    the Featured row filled, attribution visible
#
# A scratch-home settingsd runs on the host session bus for the whole capture.
# The nested demo opens the Wallpaper pane first (`DF_SETTINGS_START_PANE`),
# while the provider is absent — the Featured row reads "available soon". A
# scratch-cache `dragonfruit-wallpaperd` is then started **through a hanging
# CONNECT proxy**, so its first Wikimedia request stays in `fetching` long
# enough for the skeleton still. The provider is restarted without the proxy
# for the real fetch, and the filled still is taken once `Status=ready`.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, `gdbus`,
# and the built tree (`make build` + `cargo build -p dragonfruit-wallpaperd`).
# Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-8}"
FILL_TIMEOUT="${FILL_TIMEOUT:-300}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t18-capture.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t18}"
PROXY_PORT="${PROXY_PORT:-39777}"
WALLPAPER_BIN="$PWD/target/debug/dragonfruit-wallpaperd"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"
WP_DEST="org.dragonfruit.Wallpaper1"
WP_PATH="/org/dragonfruit/Wallpaper1"
WP_IFACE="org.dragonfruit.Wallpaper1"

mkdir -p "$OUTDIR"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t18-wallpaper: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for bin in "$WALLPAPER_BIN" "$SETTINGS_BIN"; do
    [ -x "$bin" ] || { echo "capture-t18-wallpaper: $bin not built" >&2; exit 1; }
done
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t18-wallpaper: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t18-wallpaper: python3 Pillow not found" >&2
    exit 1
}
for dest in "$DBUS_DEST" "$WP_DEST"; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
            --method org.freedesktop.DBus.NameHasOwner "$dest" 2>/dev/null | grep -q true; then
        echo "capture-t18-wallpaper: $dest is already owned — stop the running daemon first" >&2
        exit 1
    fi
done

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
XDG_DIR="$SCRATCH/xdg"
CACHE_DIR="$SCRATCH/cache"
rm -f "$SYNTH"

cleanup() {
    stop_demo
    stop_provider
    stop_proxy
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

stop_provider() {
    if [ -n "${WP_PID:-}" ]; then
        kill -TERM "$WP_PID" 2>/dev/null || true
        wait "$WP_PID" 2>/dev/null || true
        WP_PID=""
    fi
}

stop_proxy() {
    if [ -n "${PROXY_PID:-}" ]; then
        kill -TERM "$PROXY_PID" 2>/dev/null || true
        wait "$PROXY_PID" 2>/dev/null || true
        PROXY_PID=""
    fi
}

stop_settingsd() {
    if [ -n "${SETTINGS_PID:-}" ]; then
        kill -TERM "$SETTINGS_PID" 2>/dev/null || true
        wait "$SETTINGS_PID" 2>/dev/null || true
        SETTINGS_PID=""
    fi
}

start_settingsd() {
    XDG_CONFIG_HOME="$XDG_DIR" "$SETTINGS_BIN" >"$SCRATCH/settingsd.log" 2>&1 &
    SETTINGS_PID=$!
    for _ in $(seq 1 200); do
        if gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
                --method "$DBUS_IFACE.Get" appearance.colorScheme >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    echo "capture-t18-wallpaper: settingsd never took $DBUS_DEST" >&2
    exit 1
}

# start_proxy: a CONNECT proxy that accepts and then never answers, so the
# provider's 20 s global timeout keeps it in `fetching` for the skeleton still.
start_proxy() {
    python3 - "$PROXY_PORT" >"$SCRATCH/proxy.log" 2>&1 <<'PY' &
import socket
import sys
import time

port = int(sys.argv[1])
server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(("127.0.0.1", port))
server.listen(8)
conns = []
while True:
    try:
        conn, _ = server.accept()
    except OSError:
        break
    conns.append(conn)  # hold the connection open, never respond
    time.sleep(0.1)
PY
    PROXY_PID=$!
    for _ in $(seq 1 200); do
        if python3 -c "import socket,sys; s=socket.socket(); s.settimeout(0.2); sys.exit(0 if s.connect_ex(('127.0.0.1',$PROXY_PORT))==0 else 1)"; then
            return
        fi
        sleep 0.05
    done
    echo "capture-t18-wallpaper: proxy never came up on $PROXY_PORT" >&2
    exit 1
}

# start_provider <mode>: launch the provider with a fresh cache. Mode "yes"
# routes HTTP(S) through the hanging proxy; "dead" routes it through a dead
# proxy so every fetch fails immediately (the offline state); "no" is direct.
start_provider() {
    local proxied="$1"
    local env_args=()
    case "$proxied" in
        yes)
            env_args=(env HTTPS_PROXY="http://127.0.0.1:$PROXY_PORT"
                          HTTP_PROXY="http://127.0.0.1:$PROXY_PORT"
                          ALL_PROXY="http://127.0.0.1:$PROXY_PORT")
            ;;
        dead)
            env_args=(env HTTPS_PROXY="http://127.0.0.1:1"
                          HTTP_PROXY="http://127.0.0.1:1"
                          ALL_PROXY="http://127.0.0.1:1"
                          https_proxy="http://127.0.0.1:1"
                          http_proxy="http://127.0.0.1:1"
                          all_proxy="http://127.0.0.1:1")
            ;;
    esac
    XDG_CACHE_HOME="$CACHE_DIR" HOME="$SCRATCH" "${env_args[@]}" "$WALLPAPER_BIN" \
        >"$SCRATCH/wallpaperd.log" 2>&1 &
    WP_PID=$!
    for _ in $(seq 1 400); do
        if gdbus call --session --dest "$WP_DEST" --object-path "$WP_PATH" \
                --method org.freedesktop.DBus.Properties.Get "$WP_IFACE" Status \
                >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    echo "capture-t18-wallpaper: wallpaperd never took $WP_DEST" >&2
    exit 1
}

wallpaper_status() {
    gdbus call --session --dest "$WP_DEST" --object-path "$WP_PATH" \
        --method org.freedesktop.DBus.Properties.Get "$WP_IFACE" Status 2>/dev/null \
        | sed -n "s/.*'\(.*\)'.*/\1/p" || true
}

# set_settings_key <key> <gvariant-literal>: write through the scratch
# settingsd so the live Settings app reacts without a restart.
set_settings_key() {
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Set" "$1" "$2" >/dev/null 2>&1 || true
}

# selected_wallpaper: the first cached provider path, so the filled still can
# show the live attribution for a fetched picture.
first_cached_path() {
    gdbus call --session --dest "$WP_DEST" --object-path "$WP_PATH" \
        --method org.freedesktop.DBus.Properties.Get "$WP_IFACE" Items 2>/dev/null \
        | tr -d '\\' | grep -o '"localPath": *"[^"]*"' | head -1 \
        | sed 's/.*: *"//; s/"$//' || true
}

start_demo() {
    rm -f "$SYNTH"
    DF_SETTINGS_START_PANE=wallpaper \
        DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 900); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 900); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-t18-wallpaper: synthetic socket never appeared; demo log:" >&2
        tail -30 "$SCRATCH/demo.log" >&2
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
client = f"/tmp/dragonfruit-t18-nudge-{os.getpid()}.sock"
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
client = f"/tmp/dragonfruit-t18-zoom-{os.getpid()}.sock"
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
print(f"capture-t18-wallpaper: saved {dest} ({im.width}x{im.height})")
PY
}

echo "capture-t18-wallpaper: scratch settingsd"
start_settingsd

echo "capture-t18-wallpaper: nested demo with the Wallpaper pane (provider absent)"
start_demo
zoom_settings

echo "capture-t18-wallpaper: offline still (cold cache, dead network)"
start_provider dead
for _ in $(seq 1 200); do
    [ "$(wallpaper_status)" = "offline" ] && break
    sleep 0.1
done
echo "capture-t18-wallpaper: offline status: $(wallpaper_status)"
capture "$OUTDIR/t18-wallpaper-offline.png"
stop_provider

echo "capture-t18-wallpaper: starting the provider behind a hanging proxy"
start_proxy
start_provider yes
sleep 2
echo "capture-t18-wallpaper: fetching still (status: $(wallpaper_status))"
capture "$OUTDIR/t18-wallpaper-fetching.png"

echo "capture-t18-wallpaper: restarting the provider for the real fetch"
stop_provider
stop_proxy
start_provider no
echo "capture-t18-wallpaper: waiting for the catalogue to fill (timeout ${FILL_TIMEOUT}s)"
for _ in $(seq 1 "$FILL_TIMEOUT"); do
    status="$(wallpaper_status)"
    [ "$status" = "ready" ] && break
    sleep 1
done
echo "capture-t18-wallpaper: provider status before fill still: $(wallpaper_status)"
selected="$(first_cached_path)"
if [ -n "$selected" ]; then
    echo "capture-t18-wallpaper: selecting the fetched default for attribution: $selected"
    set_settings_key "wallpaper.source" "<'$selected'>"
    sleep 3
fi
capture "$OUTDIR/t18-wallpaper-filled.png"

stop_demo
echo "capture-t18-wallpaper: done"