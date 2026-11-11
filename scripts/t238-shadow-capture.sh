#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 decoration optimisation: deterministic shadow visual A/B.
#
# Starts one nested compositor WITHOUT the shell (so the shell's screenshot
# handler cannot move the capture into Pictures/), maps a fixed set of Xwayland
# windows, then captures the same frame twice: once with the span shadow and
# once with the one-quad shader shadow. Because it is one compositor session,
# window placement, focus, scheme, and scale are identical between the two
# images; only the shadow path changes.
#
# Output: $OUT_DIR/shadow-off.png and $OUT_DIR/shadow-on.png.
#
# Overrides: WINDOWS, DURATION (settle seconds), OUT_DIR, BACKEND, PROFILE.
set -euo pipefail

cd "$(dirname "$0")/.."

. "$(dirname "$0")/t21-capture-lib.sh"

OUT_DIR="${OUT_DIR:-/tmp/t238-shadow-capture}"
WINDOWS="${WINDOWS:-3}"
SETTLE="${SETTLE:-1.5}"
WATCHDOG_TIMEOUT="${WATCHDOG_TIMEOUT:-20}"
TIER="${TIER:-full}"
BACKEND="${BACKEND:-nested}"
PROFILE="${PROFILE:-release}"
T21_COMPOSITOR_BIN="${COMPOSITOR_BIN:-$T21_ROOT/target/$PROFILE/dragonfruit-compositor}"
export T21_COMPOSITOR_BIN

t21_capture_init "t238-shadow-capture"
t21_require_exec "$T21_COMPOSITOR_BIN" "run make build"
command -v xmessage >/dev/null 2>&1 || {
    echo "t238-shadow-capture: xmessage not found" >&2
    exit 1
}

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/baseline.png "$OUT_DIR"/shadow.png "$OUT_DIR"/titlebar.png "$OUT_DIR"/both.png

export SDL_VIDEODRIVER=x11
export DRAGONFRUIT_REFRESH_HZ=144

t21_start_compositor "$BACKEND" "$OUT_DIR/compositor.txt"
python3 scripts/benchmark-watchdog.py \
    --pid "$T21_COMPOSITOR_PID" \
    --synth "$T21_SYNTH" \
    --log "$OUT_DIR/compositor.txt" \
    --out "$OUT_DIR/hang.txt" \
    --timeout "$WATCHDOG_TIMEOUT" &
WATCHDOG_PID=$!

display_file="$XDG_RUNTIME_DIR/$T21_SOCKET.x11-display"
for _ in $(seq 1 100); do
    [ -s "$display_file" ] && break
    sleep 0.1
done
if [ ! -s "$display_file" ]; then
    echo "t238-shadow-capture: no Xwayland DISPLAY hand-off" >&2
    t21_stop_compositor
    exit 1
fi
display="$(cat "$display_file")"

children=()
logical_w=1920
logical_h=1200
columns=$(( WINDOWS < 6 ? WINDOWS : 6 ))
[ "$columns" -lt 1 ] && columns=1
rows=$(( (WINDOWS + columns - 1) / columns ))
cell_w=$(( logical_w / columns ))
cell_h=$(( logical_h / rows ))
for index in $(seq 0 $((WINDOWS - 1))); do
    x=$(( (index % columns) * cell_w + 8 ))
    y=$(( (index / columns) * cell_h + 40 ))
    w=$(( cell_w - 24 ))
    h=$(( cell_h - 60 ))
    DISPLAY="$display" xmessage -geometry "${w}x${h}+${x}+${y}" \
        -title "T238 shadow window $index" "T238 shadow window $index" \
        >/dev/null 2>&1 &
    children+=($!)
done

cleanup() {
    for pid in "${children[@]:-}"; do
        kill "$pid" 2>/dev/null || true
    done
    [ -n "${WATCHDOG_PID:-}" ] && kill "$WATCHDOG_PID" 2>/dev/null || true
    t21_stop_compositor
    if [ -s "$OUT_DIR/hang.txt" ]; then
        echo "t238-shadow-capture: compositor hang; see $OUT_DIR/hang.txt" >&2
    fi
}
trap cleanup EXIT

sleep "$SETTLE"
# Pin the full tier so the span arm is the real 24-layer shadow, not the
# auto-degraded 12-layer one.
t21_send_synth "set degrade-tier $TIER"
sleep 0.4

capture() {
    local name="$1" shadow="$2" titlebar="$3"
    t21_send_synth "set shadow-gpu $shadow"
    t21_send_synth "set titlebar-gpu $titlebar"
    sleep 0.5
    t21_send_synth "capture $OUT_DIR/$name.png"
    for _ in $(seq 1 60); do
        [ -s "$OUT_DIR/$name.png" ] && break
        sleep 0.1
    done
}

# One compositor session, one window set, one focus state: only the two
# decoration paths change between the four captures.
capture baseline off off
capture shadow on off
capture titlebar off on
capture both on on

for name in baseline shadow titlebar both; do
    if [ ! -s "$OUT_DIR/$name.png" ]; then
        echo "t238-shadow-capture: capture $name failed; compositor log:" >&2
        tail -40 "$OUT_DIR/compositor.txt" >&2
        exit 1
    fi
done

python3 - "$OUT_DIR" <<'PY'
import os
import sys
from PIL import Image, ImageChops

out = sys.argv[1]
baseline = Image.open(os.path.join(out, "baseline.png")).convert("RGB")
total = baseline.size[0] * baseline.size[1]
print(f"decoration capture A/B: size={baseline.size}")
for name in ("shadow", "titlebar", "both"):
    image = Image.open(os.path.join(out, f"{name}.png")).convert("RGB")
    if image.size != baseline.size:
        print(f"{name}: size mismatch {image.size}")
        continue
    diff = ImageChops.difference(baseline, image)
    data = diff.tobytes()
    changed = 0
    max_diff = 0
    over_two = 0
    for i in range(0, len(data), 3):
        value = max(data[i], data[i + 1], data[i + 2])
        if value:
            changed += 1
        if value >= 3:
            over_two += 1
        if value > max_diff:
            max_diff = value
    print(
        f"{name}: changed={changed} ({100.0 * changed / total:.3f}%) "
        f">=3/255={over_two} ({100.0 * over_two / total:.3f}%) "
        f"max={max_diff}"
    )
PY

echo "t238-shadow-capture: wrote baseline/shadow/titlebar/both in $OUT_DIR" >&2
