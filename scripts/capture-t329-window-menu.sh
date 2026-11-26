#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T329 window-menu live capture.
#
# Runs the real nested compositor and shell, opens an SSD window's titlebar
# menu through synthetic input, and captures the compositor's own render for
# light/dark at 1x/2x, both the top-level menu and the Move to Space submenu.
#
# The menu is compositor-owned; the trusted shell renders the readable rows,
# Tabler icons, highlight, and submenu card into its `windowmenu` overlay.
#
# Outputs: docs/captures/t329-window-menu-<scheme>-<scale>-<state>.png plus
# t329-window-menu.json. Not part of `make e2e`; needs a host Wayland session
# (or kwin_wayland --virtual).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-4}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t329.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
KWIN_SOCKET="df-t329-kwin-$$"
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
    echo "capture-t329: XDG_RUNTIME_DIR is unset — writing OPEN" >&2
    cat >"$OUTDIR/t329-window-menu.open.txt" <<EOF
T329 window menu — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make t329-window-menu-capture
EOF
    exit 0
fi

if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t329: starting kwin_wayland --virtual (socket $KWIN_SOCKET)"
        XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY= \
            kwin_wayland --virtual --fullscreen 1 --socket "$KWIN_SOCKET" \
            --width 1280 --height 800 --no-lockscreen \
            --no-global-shortcuts --no-kactivities \
            >"$SCRATCH/kwin.log" 2>&1 &
        KWIN_PID=$!
        for _ in $(seq 1 100); do
            [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ] && break
            sleep 0.1
        done
        if ! [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ]; then
            echo "capture-t329: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t329: no host Wayland session and no kwin_wayland — writing OPEN" >&2
        cat >"$OUTDIR/t329-window-menu.open.txt" <<EOF
T329 window menu — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: no usable host Wayland session and kwin_wayland is not installed.
rerun: WAYLAND_DISPLAY=<host-socket> make t329-window-menu-capture
EOF
        exit 0
    fi
fi

run_case() {
    local label="$1" state="$2" scheme="$3" scale="$4"
    local out="$OUTDIR/t329-window-menu-${scheme}-${scale}x-${label}.png"
    local query="$OUTDIR/t329-window-menu-${scheme}-${scale}x-${label}.txt"
    local log="$SCRATCH/${scheme}-${scale}x-${label}.log"
    local socket="df-t329-${scheme}-${scale}x-${label}-$$"
    echo "capture-t329: $label ($scheme, ${scale}x)"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        SCALE="$scale" COLOR_SCHEME="$scheme" \
        FULL_SESSION=0 SETTLE="$SETTLE" SOCKET="$socket" \
        QUERY_OUT="$query" OUT="$out" \
        SCRATCH="$SCRATCH/$label" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh "$state" >"$log" 2>&1
    rm -rf "$HOST_RUNTIME/$socket.session" "$HOST_RUNTIME/$socket" \
           "$HOST_RUNTIME/$socket.lock" "$HOST_RUNTIME/$socket.launch-token" \
           "$HOST_RUNTIME/$socket.desktop-launch-token" 2>/dev/null || true
    if [ ! -s "$out" ]; then
        echo "capture-t329: $label produced no capture" >&2
        tail -30 "$log" >&2 || true
        return 1
    fi
    echo "capture-t329: $label -> $out"
}

for scheme in light dark; do
    for scale in 1 2; do
        run_case menu window-menu "$scheme" "$scale"
        run_case submenu window-menu-submenu "$scheme" "$scale"
    done
done

python3 - "$OUTDIR" <<'PY'
import json
import sys
from pathlib import Path

outdir = Path(sys.argv[1])
rows = []
for scheme in ("light", "dark"):
    for scale in ("1", "2"):
        for label, state in (("menu", "window-menu"),
                             ("submenu", "window-menu-submenu")):
            image = outdir / f"t329-window-menu-{scheme}-{scale}x-{label}.png"
            rows.append({
                "label": label,
                "state": state,
                "scheme": scheme,
                "scale": int(scale),
                "artifact": (str(image.relative_to(outdir.parent.parent))
                             if image.exists() else None),
                "present": image.exists(),
            })
report = {
    "backend": "nested",
    "task": "T329",
    "cases": rows,
    "note": "The compositor owns the menu model/input; the trusted shell "
            "renders the labels, Tabler icons, highlight, and submenu card.",
}
(outdir / "t329-window-menu.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
PY

echo "capture-t329: done"
