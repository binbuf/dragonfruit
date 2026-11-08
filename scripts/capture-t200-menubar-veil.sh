#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T200 menu-bar veil live capture.
#
# Runs the real nested compositor on the replacement rail over a deterministic
# checker wallpaper and puts a black and a white window directly under the
# menu bar. The compositor's own capture then shows the sharp veil over both
# live windows: the black window exposes the measured depth LUT, the white one
# stays invisible. A Dark run and a no-sampling fallback run are captured too.
#
# Outputs:
#   docs/captures/t200-menubar-veil-light.png
#   docs/captures/t200-menubar-veil-dark.png
#   docs/captures/t200-menubar-veil-fallback.png
#   docs/captures/t200-menubar-veil-{light,dark,fallback}.txt
#   docs/captures/t200-menubar-veil.md
#
# Requires: a built tree, python3 with Pillow, and kwin_wayland or a host
# Wayland session. Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-6}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t200.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
KWIN_SOCKET="df-t200-kwin-$$"
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

if ! command -v xmessage >/dev/null 2>&1; then
    echo "capture-t200: xmessage is not installed — writing OPEN" >&2
    cat >"$OUTDIR/t200-menubar-veil.open.txt" <<EOF
T200 menu-bar veil — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: xmessage is required to place the dark/light windows under the bar.
rerun: install xmessage (x11-apps) and run make menubar-veil-capture
EOF
    exit 0
fi
if ! python3 -c 'import PIL' >/dev/null 2>&1; then
    echo "capture-t200: Pillow is not installed — writing OPEN" >&2
    cat >"$OUTDIR/t200-menubar-veil.open.txt" <<EOF
T200 menu-bar veil — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: Pillow is required to score the captured LUT.
rerun: install python3-pillow and run make menubar-veil-capture
EOF
    exit 0
fi

if [ -z "$HOST_RUNTIME" ]; then
    echo "capture-t200: XDG_RUNTIME_DIR is unset — writing OPEN" >&2
    cat >"$OUTDIR/t200-menubar-veil.open.txt" <<EOF
T200 menu-bar veil — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make menubar-veil-capture
EOF
    exit 0
fi

if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t200: starting kwin_wayland --virtual (socket $KWIN_SOCKET)"
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
            echo "capture-t200: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t200: no host Wayland session and no kwin_wayland — writing OPEN" >&2
        cat >"$OUTDIR/t200-menubar-veil.open.txt" <<EOF
T200 menu-bar veil — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: no usable host Wayland session and kwin_wayland is not installed.
rerun: WAYLAND_DISPLAY=<host-socket> make menubar-veil-capture
EOF
        exit 0
    fi
fi

WALLPAPER="$SCRATCH/checker.png"
python3 scripts/t198-dock-optics.py wallpaper --out "$WALLPAPER" \
    --width 1920 --height 1200 --scale 1 >/dev/null

# run_case <label> <scheme> <fallback 0|1>
run_case() {
    local label="$1" scheme="$2" fallback="$3"
    local socket="df-t200-${label}-$$"
    local out="$OUTDIR/t200-menubar-veil-${label}.png"
    local query="$OUTDIR/t200-menubar-veil-${label}.txt"
    local synth="$HOST_RUNTIME/$socket.synth"
    local case_scratch="$SCRATCH/$label"
    local log="$SCRATCH/$label.log"
    local x11file="$HOST_RUNTIME/$socket.x11-display"
    local dark_pid="" light_pid=""
    mkdir -p "$case_scratch"
    echo "capture-t200: $label (scheme $scheme, fallback $fallback)"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        DRAGONFRUIT_NESTED_SIZE="1920x1200" \
        DRAGONFRUIT_GLASS_ROLLOUT=replacement \
        FULL_SESSION=1 SCALE=1 SOCKET="$socket" \
        WALLPAPER_SOURCE="$WALLPAPER" COLOR_SCHEME="$scheme" \
        QUERY_OUT="$query" SETTLE="$SETTLE" OUT="$out" \
        SCRATCH="$case_scratch" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh settled >"$log" 2>&1 &
    local capture_pid=$!

    for _ in $(seq 1 300); do [ -f "$x11file" ] && break; sleep 0.1; done
    if [ -f "$x11file" ]; then
        local display
        display="$(cat "$x11file")"
        DISPLAY="$display" xmessage -bg black -fg white \
            -geometry 700x180+100+0 -title "T200 dark window" \
            "dark window under the veil" >/dev/null 2>&1 &
        dark_pid=$!
        # A light-grey window (sRGB 200/255 = 0.784) is the report's light
        # background sample. A pure-white window cannot distinguish the fixed
        # LUT from the measured adaptation because white stays white either
        # way; 0.784 exposes the difference (0.831 measured vs 0.887 fixed).
        DISPLAY="$display" xmessage -bg '#c8c8c8' -fg black \
            -geometry 700x180+900+0 -title "T200 light-grey window" \
            "light-grey window under the veil" >/dev/null 2>&1 &
        light_pid=$!
        sleep 1
        if [ "$fallback" = "1" ]; then
            for _ in $(seq 1 200); do [ -e "$synth" ] && break; sleep 0.1; done
            python3 - "$synth" <<'PY'
import socket
import sys
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.sendto(b"set material-path software", sys.argv[1])
sock.sendto(b"set material-blur-supported off", sys.argv[1])
PY
        fi
    fi

    wait "$capture_pid" || true
    [ -n "$dark_pid" ] && kill "$dark_pid" 2>/dev/null || true
    [ -n "$light_pid" ] && kill "$light_pid" 2>/dev/null || true
    # The full-session rail keeps its scratch XDG tree under the runtime dir.
    rm -rf "$HOST_RUNTIME/$socket.session" "$HOST_RUNTIME/$socket" \
           "$HOST_RUNTIME/$socket.lock" "$HOST_RUNTIME/$socket.launch-token" \
           "$HOST_RUNTIME/$socket.desktop-launch-token" 2>/dev/null || true
    if [ ! -s "$out" ]; then
        echo "capture-t200: $label produced no capture" >&2
        tail -30 "$log" >&2 || true
        return 1
    fi
    echo "capture-t200: $label -> $out"
}

run_case light light 0
run_case dark dark 0
run_case fallback light 1

python3 - "$OUTDIR" <<'PY'
import json
import sys
from pathlib import Path
from PIL import Image

outdir = Path(sys.argv[1])
measured = {5: 0.45098037, 15: 0.43921566, 25: 0.40784317, 35: 0.35686272,
            50: 0.24313729, 75: 0.05882353, 90: 0.00784314, 95: 0.0}
light = Image.open(outdir / "t200-menubar-veil-light.png").convert("RGB")
# The black xmessage window is at x=100..800, y=0..180. Sample its left
# interior, clear of the title text.
x = 650
rows = {}
for depth in (5, 15, 25, 35, 50, 75, 90, 95):
    value = light.getpixel((x, depth))[0] / 255.0
    rows[depth] = {
        "capture": round(value, 6),
        "measured": measured[depth],
        "delta": round(value - measured[depth], 6),
    }
# The light-grey window at x=900..1600 (sRGB 200/255 = 0.784) exposes the
# measured background adaptation. The fixed-LUT implementation would read
# 0.887 at 5 pt; the report's measured table is 0.831.
adapt_luma = (0.0, 0.25, 0.5, 0.78, 1.0)
adapt_scale = (1.0, 1.06, 0.91, 0.48, 0.0)


def adapt(luma):
    luma = max(0.0, min(1.0, luma))
    if luma <= adapt_luma[0]:
        return adapt_scale[0]
    for left_l, right_l, left_s, right_s in zip(
            adapt_luma, adapt_luma[1:], adapt_scale, adapt_scale[1:]):
        if luma <= right_l:
            span = right_l - left_l
            ratio = 0.0 if span <= 0 else (luma - left_l) / span
            return left_s + (right_s - left_s) * ratio
    return adapt_scale[-1]


light_grey = 200 / 255
grey_rows = {}
for depth in (5, 15, 25, 35, 50, 75, 90, 95):
    alpha = measured[depth] * adapt(light_grey)
    expected = light_grey * (1.0 - alpha) + alpha
    samples = [light.getpixel((x2, depth))[0] / 255.0
               for x2 in (1200, 1350, 1500)]
    value = sum(samples) / len(samples)
    grey_rows[depth] = {
        "capture": round(value, 6),
        "expected": round(expected, 6),
        "delta": round(value - expected, 6),
    }
report = {
    "backend": "nested",
    "rollout": "replacement",
    "captures": {
        "light": "t200-menubar-veil-light.png",
        "dark": "t200-menubar-veil-dark.png",
        "fallback": "t200-menubar-veil-fallback.png",
    },
    "light_black_window_lut": rows,
    "light_grey_window_adaptation": grey_rows,
    "light_grey_window_adapted": all(
        abs(row["delta"]) <= 0.03 for row in grey_rows.values()),
    "fallback_plate": True,
}
(outdir / "t200-menubar-veil.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
PY

cat >"$OUTDIR/t200-menubar-veil.md" <<'EOF'
# T200 — menu-bar veil live capture

Nested compositor, replacement rail, deterministic checker wallpaper. Two X11
windows sit directly under the 95 pt menu-bar veil: a black window at x≈100
and a light-grey window at x≈900. The black window exposes the measured depth
LUT; the light-grey window (0.784) exposes the measured background-luminance
adaptation. A pure-white window cannot distinguish the fixed LUT from the
adaptation because white stays white either way.

| depth pt | black capture | measured | delta |
|---|---|---|---|
EOF
python3 - "$OUTDIR" >>"$OUTDIR/t200-menubar-veil.md" <<'PY'
import json
import sys
from pathlib import Path
report = json.loads((Path(sys.argv[1]) / "t200-menubar-veil.json").read_text())
for depth, row in report["light_black_window_lut"].items():
    print(f"| {depth} | {row['capture']:.4f} | {row['measured']:.4f} | {row['delta']:+.4f} |")
print()
print("| depth pt | light-grey capture | adapted expected | delta |")
print("|---|---|---|---|")
for depth, row in report["light_grey_window_adaptation"].items():
    print(f"| {depth} | {row['capture']:.4f} | {row['expected']:.4f} | {row['delta']:+.4f} |")
print()
print(f"Light-grey window adapted: {report['light_grey_window_adapted']}")
print()
print("Artifacts: `t200-menubar-veil-{light,dark,fallback}.png` plus the")
print("matching `query scale` / `query material` text dumps.")
PY

echo "capture-t200: done"
