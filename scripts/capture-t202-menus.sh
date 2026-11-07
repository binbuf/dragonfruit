#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T202 menu-family live capture.
#
# Runs the real nested compositor on the replacement rail and captures the
# compositor's own render for:
#   * the light/dark attached menu-bar dropdown,
#   * the light attached menu at both transparency slider endpoints,
#   * representative Wi-Fi and Bluetooth status menus,
#   * the Dock context menu and the desktop (Files) context menu.
#
# The menu seams (`DF_MENUBAR_MENU_FIXTURE`, `DF_STATUS_MENU_FIXTURE`,
# `DF_DOCK_MENU_FIXTURE`) open the surfaces without depending on right-anchored
# pixel positions. The desktop menu is opened with a synthetic right-click.
#
# Outputs: docs/captures/t202-*.png plus t202-capture.json. Not part of
# `make e2e`; needs a host Wayland session (or kwin_wayland --virtual).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-6}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t202.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
KWIN_SOCKET="df-t202-kwin-$$"
KWIN_PID=""
HOST_DISPLAY="${WAYLAND_DISPLAY:-}"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-}"
FORCE_KWIN="${FORCE_KWIN:-0}"
mkdir -p "$OUTDIR" "$SCRATCH"

cleanup() {
    if [ -n "$KWIN_PID" ]; then
        kill -TERM "$KWIN_PID" 2>/dev/null || true
        sleep 1
        kill -KILL "$KWIN_PID" 2>/dev/null || true
    fi
    rm -f "${XDG_RUNTIME_DIR:-/tmp}/$KWIN_SOCKET" \
          "${XDG_RUNTIME_DIR:-/tmp}/$KWIN_SOCKET.lock" 2>/dev/null || true
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

if [ -z "$HOST_RUNTIME" ]; then
    echo "capture-t202: XDG_RUNTIME_DIR is unset — writing OPEN" >&2
    cat >"$OUTDIR/t202-menus.open.txt" <<EOF
T202 menu families — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make menu-families-capture
EOF
    exit 0
fi

if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t202: starting kwin_wayland --virtual (socket $KWIN_SOCKET)"
        XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY= \
            kwin_wayland --virtual --fullscreen 1 --socket "$KWIN_SOCKET" \
            --width 3840 --height 2446 --no-lockscreen \
            --no-global-shortcuts --no-kactivities \
            >"$SCRATCH/kwin.log" 2>&1 &
        KWIN_PID=$!
        for _ in $(seq 1 100); do
            [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ] && break
            sleep 0.1
        done
        if ! [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ]; then
            echo "capture-t202: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t202: no host Wayland session and no kwin_wayland — writing OPEN" >&2
        cat >"$OUTDIR/t202-menus.open.txt" <<EOF
T202 menu families — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: no usable host Wayland session and kwin_wayland is not installed.
rerun: WAYLAND_DISPLAY=<host-socket> make menu-families-capture
EOF
        exit 0
    fi
fi

WALLPAPER="$SCRATCH/checker.png"
python3 scripts/t198-dock-optics.py wallpaper --out "$WALLPAPER" \
    --width 1920 --height 1200 --scale 1 >/dev/null

run_case() {
    local label="$1" state="$2" scheme="$3" transparency="$4" status="$5"
    local out="$OUTDIR/t202-${label}.png"
    local query="$OUTDIR/t202-${label}.txt"
    local log="$SCRATCH/${label}.log"
    local socket="df-t202-${label}-$$"
    echo "capture-t202: $label ($state, $scheme)"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        DRAGONFRUIT_NESTED_SIZE="1920x1200" \
        DRAGONFRUIT_GLASS_ROLLOUT=replacement \
        FULL_SESSION=1 SCALE=1 SOCKET="$socket" \
        WALLPAPER_SOURCE="$WALLPAPER" COLOR_SCHEME="$scheme" \
        TRANSPARENCY="$transparency" STATUS_MENU="$status" \
        QUERY_OUT="$query" SETTLE="$SETTLE" OUT="$out" \
        SCRATCH="$SCRATCH/$label" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh "$state" >"$log" 2>&1
    # The full-session rail keeps a private XDG tree; remove it so repeated
    # cases do not exhaust the runtime tmpfs.
    rm -rf "$HOST_RUNTIME/$socket.session" "$HOST_RUNTIME/$socket" \
           "$HOST_RUNTIME/$socket.lock" "$HOST_RUNTIME/$socket.launch-token" \
           "$HOST_RUNTIME/$socket.desktop-launch-token" 2>/dev/null || true
    if [ ! -s "$out" ]; then
        echo "capture-t202: $label produced no capture" >&2
        tail -30 "$log" >&2 || true
        return 1
    fi
    echo "capture-t202: $label -> $out"
}

run_case attached-light menu light 0.5255 wifi
run_case attached-dark menu dark 0.5255 wifi
run_case attached-light-low menu light 0.0 wifi
run_case attached-light-high menu light 1.0 wifi
run_case status-wifi status light 0.5255 wifi
run_case status-bluetooth status light 0.5255 bluetooth
run_case context-dock dock-context light 0.5255 wifi
run_case context-desktop desktop-context light 0.5255 wifi
run_case context-files files-context light 0.5255 wifi

python3 - "$OUTDIR" <<'PY'
import json
import sys
from pathlib import Path

outdir = Path(sys.argv[1])
cases = [
    ("attached-light", "menu", "light", 0.5255, "menuAttached"),
    ("attached-dark", "menu", "dark", 0.5255, "menuAttached"),
    ("attached-light-low", "menu", "light", 0.0, "menuAttached"),
    ("attached-light-high", "menu", "light", 1.0, "menuAttached"),
    ("status-wifi", "status", "light", 0.5255, "statusMenu"),
    ("status-bluetooth", "status", "light", 0.5255, "statusMenu"),
    ("context-dock", "dock-context", "light", 0.5255, "menuDetached"),
    ("context-desktop", "desktop-context", "light", 0.5255, "menuDetached"),
    ("context-files", "files-context", "light", 0.5255, "menuDetached"),
]
rows = []
for label, state, scheme, transparency, preset in cases:
    image = outdir / f"t202-{label}.png"
    rows.append({
        "label": label,
        "state": state,
        "scheme": scheme,
        "transparency": transparency,
        "preset": preset,
        "artifact": (str(image.relative_to(outdir.parent.parent))
                     if image.exists() else None),
        "present": image.exists(),
    })
report = {
    "backend": "nested",
    "rollout": "replacement",
    "cases": rows,
    "note": "Compositor-own render through the screenshot fixture; the "
            "optical fit is checked by scripts/t202-menu-families.py because "
            "the nested desktop has no controlled backdrop.",
}
(outdir / "t202-capture.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
PY

echo "capture-t202: done"
