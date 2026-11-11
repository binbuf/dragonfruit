#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 decoration validation matrix: deterministic pixel A/B for the shadow
# and titlebar shaders.
#
# For each output scale (1.0, 1.5, 2.0), theme (dark, light), and hover state,
# one compositor session captures the same frame four times: span decorations,
# shader shadow, shader titlebar, and both shaders. Only the decoration path
# changes between captures, so the diff isolates the shader.
#
# Runs without the shell so the shell's screenshot handler cannot move the
# capture files. Outputs PNGs plus `matrix.txt` (diff statistics per state).
#
# Overrides: SCALES, THEMES, WINDOWS, SIZE, OUT_DIR, BACKEND, PROFILE.
set -euo pipefail

cd "$(dirname "$0")/.."

. "$(dirname "$0")/t21-capture-lib.sh"

OUT_DIR="${OUT_DIR:-/tmp/t238-decoration-matrix}"
SCALES="${SCALES:-1.0 1.5 2.0}"
THEMES="${THEMES:-dark light}"
WINDOWS="${WINDOWS:-3}"
SIZE="${SIZE:-1280x800}"
WATCHDOG_TIMEOUT="${WATCHDOG_TIMEOUT:-20}"
BACKEND="${BACKEND:-nested}"
PROFILE="${PROFILE:-release}"
T21_COMPOSITOR_BIN="${COMPOSITOR_BIN:-$T21_ROOT/target/$PROFILE/dragonfruit-compositor}"
export T21_COMPOSITOR_BIN

t21_capture_init "t238-decoration-matrix"
t21_require_exec "$T21_COMPOSITOR_BIN" "run make build"
command -v xmessage >/dev/null 2>&1 || {
    echo "t238-decoration-matrix: xmessage not found" >&2
    exit 1
}

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/*.png "$OUT_DIR/matrix.txt"

export SDL_VIDEODRIVER=x11
export DRAGONFRUIT_REFRESH_HZ=144
export DRAGONFRUIT_NESTED_SIZE="$SIZE"

capture() {
    local name="$1" shadow="$2" titlebar="$3"
    t21_send_synth "set shadow-gpu $shadow"
    t21_send_synth "set titlebar-gpu $titlebar"
    sleep 0.4
    t21_send_synth "capture $OUT_DIR/$name.png"
    for _ in $(seq 1 80); do
        [ -s "$OUT_DIR/$name.png" ] && break
        sleep 0.1
    done
}

for scale in $SCALES; do
    echo "t238-decoration-matrix: scale=$scale"
    export DRAGONFRUIT_NESTED_SCALE="$scale"
    t21_start_compositor "$BACKEND" "$OUT_DIR/compositor-$scale.txt"
    python3 scripts/benchmark-watchdog.py \
        --pid "$T21_COMPOSITOR_PID" \
        --synth "$T21_SYNTH" \
        --log "$OUT_DIR/compositor-$scale.txt" \
        --out "$OUT_DIR/hang-$scale.txt" \
        --timeout "$WATCHDOG_TIMEOUT" &
    watchdog=$!

    display_file="$XDG_RUNTIME_DIR/$T21_SOCKET.x11-display"
    for _ in $(seq 1 100); do
        [ -s "$display_file" ] && break
        sleep 0.1
    done
    if [ ! -s "$display_file" ]; then
        echo "t238-decoration-matrix: no Xwayland DISPLAY for scale $scale" >&2
        kill "$watchdog" 2>/dev/null || true
        t21_stop_compositor
        exit 1
    fi
    display="$(cat "$display_file")"

    # Tile the windows inside the logical output; the X11 geometry is logical
    # because Xwayland runs at integer scale 1.
    logical_w=$(awk -v s="$SIZE" -v k="$scale" 'BEGIN { split(s, p, "x"); printf "%d", p[1] / k }')
    logical_h=$(awk -v s="$SIZE" -v k="$scale" 'BEGIN { split(s, p, "x"); printf "%d", p[2] / k }')
    columns=$(( WINDOWS < 4 ? WINDOWS : 4 ))
    rows=$(( (WINDOWS + columns - 1) / columns ))
    cell_w=$(( logical_w / columns ))
    cell_h=$(( logical_h / rows ))
    children=()
    for index in $(seq 0 $((WINDOWS - 1))); do
        x=$(( (index % columns) * cell_w + 6 ))
        y=$(( (index / columns) * cell_h + 72 ))
        w=$(( cell_w - 16 ))
        h=$(( cell_h - 96 ))
        DISPLAY="$display" xmessage -geometry "${w}x${h}+${x}+${y}" \
            -title "T238 decoration $index" "T238 decoration $index" \
            >/dev/null 2>&1 &
        children+=($!)
    done
    sleep 1.2
    t21_send_synth "set degrade-tier full"
    sleep 0.3

    # First window's titlebar centre, read from the compositor so the hover
    # lands on the real SSD rect at every scale.
    read -r hover_x hover_y <<<"$(python3 scripts/t238-hover-point.py "$T21_SYNTH")"
    echo "t238-decoration-matrix: scale=$scale hover=$hover_x,$hover_y"

    for theme in $THEMES; do
        t21_send_synth "set color-scheme $theme"
        sleep 0.2
        for hover in off on; do
            if [ "$hover" = "on" ]; then
                t21_send_synth "motion-abs $hover_x $hover_y"
            else
                t21_send_synth "motion-abs 0.001 0.001"
            fi
            sleep 0.2
            prefix="s${scale}-${theme}-${hover}"
            capture "$prefix-baseline" off off
            capture "$prefix-shadow" on off
            capture "$prefix-titlebar" off on
            capture "$prefix-both" on on
        done
    done

    for pid in "${children[@]:-}"; do
        kill "$pid" 2>/dev/null || true
    done
    kill "$watchdog" 2>/dev/null || true
    wait "$watchdog" 2>/dev/null || true
    t21_stop_compositor
    if [ -s "$OUT_DIR/hang-$scale.txt" ]; then
        echo "t238-decoration-matrix: compositor hang at scale $scale" >&2
        exit 1
    fi
done

python3 - "$OUT_DIR" <<'PY'
import glob
import os
import sys
from PIL import Image, ImageChops

out = sys.argv[1]
rows = []
for baseline_path in sorted(glob.glob(os.path.join(out, "*-baseline.png"))):
    prefix = baseline_path[: -len("-baseline.png")]
    baseline = Image.open(baseline_path).convert("RGB")
    total = baseline.size[0] * baseline.size[1]
    for mode in ("shadow", "titlebar", "both"):
        path = f"{prefix}-{mode}.png"
        if not os.path.exists(path):
            rows.append((os.path.basename(prefix), mode, "missing", "", "", ""))
            continue
        image = Image.open(path).convert("RGB")
        if image.size != baseline.size:
            rows.append((os.path.basename(prefix), mode, "size", "", "", ""))
            continue
        diff = ImageChops.difference(baseline, image)
        data = diff.tobytes()
        changed = over_three = 0
        maximum = 0
        for i in range(0, len(data), 3):
            value = max(data[i], data[i + 1], data[i + 2])
            if value:
                changed += 1
            if value >= 3:
                over_three += 1
            if value > maximum:
                maximum = value
        rows.append(
            (
                os.path.basename(prefix),
                mode,
                f"{100.0 * changed / total:.3f}%",
                f"{100.0 * over_three / total:.3f}%",
                str(maximum),
                f"{baseline.size[0]}x{baseline.size[1]}",
            )
        )

with open(os.path.join(out, "matrix.txt"), "w", encoding="utf-8") as handle:
    handle.write(
        "| state | mode | changed | >=3/255 | max | size |\n"
        "|---|---|---:|---:|---:|---|\n"
    )
    for row in rows:
        handle.write("| " + " | ".join(row) + " |\n")
print(open(os.path.join(out, "matrix.txt"), encoding="utf-8").read())
PY

echo "t238-decoration-matrix: wrote $OUT_DIR/matrix.txt" >&2
