#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-07.6b live menu-bar track capture.
#
# Produces the T-07 slice artifacts:
#
#   * docs/captures/t07-shell-idle-trace.txt — the menu-bar idle trace (the
#     mapped menubar chrome sits idle; zero frames, zero client wakeups). Runs
#     headless via `make menubar-idle-trace`, no display needed.
#   * docs/captures/t07-live-menubar.png — the nested desktop with all three
#     live status items (Wi-Fi, volume, battery), served by the bridge host
#     fixture (`DF_STATUS_FIXTURE=1`; CI/this host has no battery of its own).
#   * docs/captures/t07-live-menubar-absent.png — the honest absent-daemon
#     still: no fixture and no bridge host, so every live item is hidden.
#
# The stills capture the *active* compositor window (via Spectacle `-a`) so a
# host window covering the nested session cannot crop the bar out; a KWin
# script raises and activates the Dragonfruit window first (best effort — on a
# non-KWin host the active-window capture still works if the window is front).
#
# Requires a host Wayland session, `spectacle`, and Pillow for the stills; the
# trace half runs anywhere. Not part of `make e2e`: CI has no host session or
# screenshot tool.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-menubar-capture.XXXXXX")}"

mkdir -p "$OUTDIR"

stop_demo() {
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
        DEMO_PGID=""
    fi
}

cleanup() {
    stop_demo
    rm -f "${SYNTH:-}"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

# --- 1. the idle trace (headless) -----------------------------------------
echo "capture-live-menubar: recording the menu-bar idle trace"
make menubar-idle-trace 2>&1 | tee "$OUTDIR/t07-shell-idle-trace.txt"

# --- 2. the live stills (host session) ------------------------------------
if ! command -v spectacle >/dev/null 2>&1; then
    echo "capture-live-menubar: spectacle not found — skipping the stills" >&2
    exit 0
fi
if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-live-menubar: XDG_RUNTIME_DIR unset — skipping the stills" >&2
    exit 0
fi

SOCKET_BASE="${SOCKET:-dragonfruit-t07-live-menubar}"

# launch_demo <socket> [fixture]
launch_demo() {
    local socket="$1" fixture="${2:-}"
    local synth="${XDG_RUNTIME_DIR}/$socket.synth"
    rm -f "$synth"
    local log="$SCRATCH/$socket.log"
    echo "capture-live-menubar: starting the nested demo (socket $socket, fixture '${fixture:-none}')"
    if [ -n "$fixture" ]; then
        DF_STATUS_FIXTURE=1 DRAGONFRUIT_SYNTHETIC_INPUT="$synth" \
            setsid make demo DEMO_ARGS="--socket-name $socket" >"$log" 2>&1 &
    else
        DRAGONFRUIT_SYNTHETIC_INPUT="$synth" \
            setsid make demo DEMO_ARGS="--socket-name $socket" >"$log" 2>&1 &
    fi
    DEMO_PGID=$!
    for _ in $(seq 1 300); do [ -e "$XDG_RUNTIME_DIR/$socket" ] && break; sleep 0.1; done
    for _ in $(seq 1 300); do [ -e "$synth" ] && break; sleep 0.1; done
    if [ ! -e "$synth" ]; then
        echo "capture-live-menubar: synthetic socket never appeared; demo log:" >&2
        tail -30 "$log" >&2
        exit 1
    fi
    SYNTH="$synth"
    sleep "$SETTLE"
}

# raise_dragonfruit: best-effort bring the nested window to the front and give
# it focus so Spectacle's active-window capture targets it, not the host app.
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

# capture_window <dest>: active-window screenshot, shadow trimmed.
capture_window() {
    local dest="$1"
    local raw="$SCRATCH/$(basename "$dest" .png)-raw.png"
    raise_dragonfruit
    spectacle -b -n -a -o "$raw"
    python3 - "$raw" "$dest" <<'PY'
import sys
from PIL import Image

# The nested compositor output is a fixed 1920x1200; the host window capture
# includes the decoration (titlebar at the top) and a drop shadow, so trim to
# the bottom-centred output rect.
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
                if x < minx:
                    minx = x
                if x > maxx:
                    maxx = x
                if y < miny:
                    miny = y
                if y > maxy:
                    maxy = y
    if maxx > minx:
        im = im.crop((minx, miny, maxx + 1, maxy + 1))
if im.width >= NESTED_W and im.height >= NESTED_H:
    x0 = (im.width - NESTED_W) // 2
    y0 = im.height - NESTED_H
    im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
im.convert("RGB").save(dest)
print(f"capture-live-menubar: saved {dest} ({im.width}x{im.height})")
PY
}

launch_demo "${SOCKET_BASE}-fixture" fixture
capture_window "$OUTDIR/t07-live-menubar.png"
stop_demo

launch_demo "${SOCKET_BASE}-absent"
capture_window "$OUTDIR/t07-live-menubar-absent.png"
stop_demo

echo "capture-live-menubar: done"