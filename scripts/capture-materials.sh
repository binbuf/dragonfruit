#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-04.4b materials sign-off capture.
#
# Runs the T-01/T-02 nested demo several times to capture the T-04 material
# pass in every required variant — dark, light, dark+reduced-motion, and a
# pre-material (degrade tier `minimal`, blur off) baseline — driving the
# lifecycle walkthrough over the synthetic-input harness with
# scripts/capture-demo-driver.py (now with `--scheme`/`--tier`/`--reduced`)
# and screenshotting each settled state with Spectacle. It then assembles a
# short clip per variant and two review sheets:
#
#   t04-materials-gallery-side-by-side.png  compositor SSD titlebar vs the
#                                           design-system gallery golden
#                                           (ssd_light/ssd_dark)
#   t04-materials-before-after.png          blur off vs full material
#
# The stills land in docs/captures/ as t04-materials-*.
#
# Requires: a host Wayland session, `spectacle`, `ffmpeg`, python3 with
# Pillow, and the Qt/CMake tree built (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
SETTLE="${SETTLE:-12}"

for tool in spectacle ffmpeg python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-materials: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-materials: python3 Pillow not found" >&2
    exit 1
}

# assemble_clip <prefix> <out.mp4>
assemble_clip() {
    local prefix="$1" out="$2"
    local scratch
    scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-clip.XXXXXX")"
    local list="$scratch/concat.txt"
    : >"$list"
    local stills=(
        "$OUTDIR/$prefix.png"
        "$OUTDIR/$prefix-wayland.png"
        "$OUTDIR/$prefix-menu.png"
        "$OUTDIR/$prefix-zoomed.png"
        "$OUTDIR/$prefix-minimized.png"
        "$OUTDIR/$prefix-restored.png"
        "$OUTDIR/$prefix-closed.png"
    )
    local last="" still
    for still in "${stills[@]}"; do
        [ -f "$still" ] || continue
        printf "file '%s'\nduration 1.4\n" "$(realpath "$still")" >>"$list"
        last="$still"
    done
    if [ -n "$last" ]; then
        printf "file '%s'\n" "$(realpath "$last")" >>"$list"
        local encoder="mpeg4" candidate
        for candidate in libx264 libopenh264; do
            if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q " $candidate "; then
                encoder="$candidate"
                break
            fi
        done
        ffmpeg -y -loglevel error -f concat -safe 0 -i "$list" \
            -vf "scale=1280:-2,format=yuv420p" -c:v "$encoder" -crf 28 -movflags +faststart \
            "$out"
        echo "capture-materials: wrote $out ($encoder)"
    fi
    rm -rf "$scratch"
}

# run_mode <suffix> <driver-flags...>
run_mode() {
    local suffix="$1"; shift
    local socket="dragonfruit-t04-capture${suffix:+$suffix}-$$"
    local synth="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$socket.synth"
    local scratch
    scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-capture.XXXXXX")"
    local log="$scratch/demo.log"
    rm -f "$synth"

    echo "capture-materials: starting the nested demo ($socket)"
    DRAGONFRUIT_SYNTHETIC_INPUT="$synth" setsid make demo DEMO_ARGS="--socket-name $socket" \
        >"$log" 2>&1 &
    local pgid=$!

    cleanup() {
        kill -TERM -"$pgid" 2>/dev/null || true
        sleep 1
        kill -KILL -"$pgid" 2>/dev/null || true
        rm -f "$synth"
        [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$scratch"
    }
    trap cleanup RETURN

    local _
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$socket" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$synth" ] && break; sleep 0.1; done
    if [ ! -e "$synth" ]; then
        echo "capture-materials: synthetic-input socket never appeared; demo log:" >&2
        tail -30 "$log" >&2
        return 1
    fi
    sleep "$SETTLE"

    echo "capture-materials: driving the lifecycle walkthrough ($suffix)"
    python3 scripts/capture-demo-driver.py --synth "$synth" --outdir "$OUTDIR" \
        --scratch "$scratch" "$@"

    cleanup
    trap - RETURN
    rm -rf "$scratch"
}

run_mode "" --prefix t04-materials-dark --scheme dark --materials-only
run_mode "-light" --prefix t04-materials-light --scheme light --materials-only
run_mode "-reduced" --prefix t04-materials-reduced --scheme dark --reduced --materials-only
run_mode "-before" --prefix t04-materials-before --scheme dark --tier minimal --materials-only

assemble_clip t04-materials-dark "$OUTDIR/t04-materials.mp4"
assemble_clip t04-materials-light "$OUTDIR/t04-materials-light.mp4"
assemble_clip t04-materials-reduced "$OUTDIR/t04-materials-reduced.mp4"

# Review sheets: the compositor titlebar against the gallery golden, and the
# pre-material baseline against the full pass. Pillow only; no font needed.
python3 - "$OUTDIR" "$PWD/design-system/gallery/snapshots" <<'PY'
import os
import sys

from PIL import Image

outdir, golden_dir = sys.argv[1], sys.argv[2]


def load(name, root=None):
    path = os.path.join(root or outdir, name)
    return Image.open(path).convert("RGB") if os.path.exists(path) else None


def row(entries):
    """One horizontal row from (path-root, name) pairs, skipping missing."""
    present = [im for im in (load(n, r) for r, n in entries) if im is not None]
    return present


def stack(rows, name):
    rows = [r for r in rows if r]
    if not rows:
        return
    margin = 8
    width = max(sum(i.width for i in r) + margin * (len(r) + 1) for r in rows)
    height = sum(max(i.height for i in r) for r in rows) + margin * (len(rows) + 1)
    sheet = Image.new("RGB", (width, height), (0, 0, 0))
    y = margin
    for r in rows:
        row_h = max(i.height for i in r)
        x = margin
        for im in r:
            sheet.paste(im, (x, y + (row_h - im.height) // 2))
            x += im.width + margin
        y += row_h + margin
    dest = os.path.join(outdir, name)
    sheet.save(dest)
    print(f"capture-materials: wrote {dest} ({sheet.width}x{sheet.height})")


# Compositor SSD titlebar vs the design-system gallery golden, per scheme
# (top row dark, bottom row light).
stack(
    [
        row([(outdir, "t04-materials-dark-titlebar.png"), (golden_dir, "ssd_dark.png")]),
        row([(outdir, "t04-materials-light-titlebar.png"), (golden_dir, "ssd_light.png")]),
    ],
    "t04-materials-gallery-side-by-side.png",
)
# Before (degrade minimal: blur off) / after (full material): the titlebar
# crop for a tight comparison and the whole-window view.
stack(
    [
        row([(outdir, "t04-materials-before-titlebar.png"), (outdir, "t04-materials-dark-titlebar.png")]),
    ],
    "t04-materials-before-after-titlebar.png",
)
stack(
    [
        row([(outdir, "t04-materials-before.png"), (outdir, "t04-materials-dark.png")]),
        row([(outdir, "t04-materials-before-dock.png"), (outdir, "t04-materials-dark-dock.png")]),
    ],
    "t04-materials-before-after.png",
)
PY

echo "capture-materials: done"