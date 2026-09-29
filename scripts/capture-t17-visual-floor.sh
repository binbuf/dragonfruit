#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-17.3 visual-floor and reduced-motion sign-off capture.
#
# Runs the nested demo (`make demo`) with the compositor's synthetic-input
# harness and drives scripts/t17-visual-floor-driver.py in a *single* session
# so the live chrome cannot drift between variants. For each of dark, light,
# and dark+reduced-motion it captures the whole desktop plus
# the menu-bar band, the compositor SSD titlebar, and the Dock band, and
# records the live `query material` / `query degrade` tones into
# docs/captures/t17-visual-floor.txt.
#
# It then composes the review sheets: `t17-visual-floor-gallery.png` pairs each
# captured desktop with its `window_*` golden and its titlebar with the `ssd_*`
# golden (including the `_reduced` twins), and `t17-visual-floor-menubar.png`/
# `-dock.png` stack the chrome bands across the three variants. The stills and
# the sheets are the evidence for the premium gate's visual-floor sign-off;
# the reduced-motion *behaviour* is pinned by
# compositor/tests/reduced_motion_sweep.rs and the gallery `--strict` gate.
#
# Requires: a host Wayland session, `spectacle`, python3 with Pillow, ffmpeg
# (optional clip), and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."
export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

OUTDIR="${OUTDIR:-docs/captures}"
GOLDEN_DIR="design-system/gallery/snapshots"
SETTLE="${SETTLE:-12}"
SOCKET="${SOCKET:-dragonfruit-t17-visual-floor}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t17-floor.XXXXXX")}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"

mkdir -p "$OUTDIR" "$SCRATCH"

for tool in spectacle python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t17-visual-floor: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t17-visual-floor: python3 Pillow not found" >&2
    exit 1
}

DEMO_PGID=""
cleanup() {
    if [ -n "$DEMO_PGID" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

rm -f "$SYNTH"

echo "capture-t17-visual-floor: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-t17-visual-floor: synthetic socket never appeared; demo log:" >&2
    tail -30 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

echo "capture-t17-visual-floor: driving the dark/light/reduced floor sweep"
python3 scripts/t17-visual-floor-driver.py --synth "$SYNTH" --outdir "$OUTDIR" \
    --scratch "$SCRATCH" --prefix t17-visual-floor

if [ -f "$SCRATCH/visual-floor.txt" ]; then
    cp "$SCRATCH/visual-floor.txt" "$OUTDIR/t17-visual-floor.txt"
    echo "capture-t17-visual-floor: wrote $OUTDIR/t17-visual-floor.txt"
fi

# The review sheet: one row per variant, captured surface beside its golden.
# Pillow only; no font needed.
python3 - "$OUTDIR" "$GOLDEN_DIR" <<'PY'
import os
import sys

from PIL import Image

outdir, golden_dir = sys.argv[1], sys.argv[2]

# (capture suffix, window golden, ssd golden)
ROWS = [
    ("dark", "window_dark.png", "ssd_dark.png"),
    ("light", "window_light.png", "ssd_light.png"),
    ("reduced", "window_dark_reduced.png", "ssd_dark_reduced.png"),
]
MAX_H = 360


def load(path):
    return Image.open(path).convert("RGB") if os.path.exists(path) else None


def fit(im, max_h=MAX_H):
    """Downscale to at most `max_h` tall; never upscale a small crop."""
    if im is None or im.height <= max_h:
        return im
    width = max(1, round(im.width * max_h / im.height))
    return im.resize((width, max_h), Image.LANCZOS)


def row(entries):
    return [fit(im) for _, im in entries if im is not None]


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
    print(f"capture-t17-visual-floor: wrote {dest} ({sheet.width}x{sheet.height})")


# Variant rows: captured desktop beside the design-system window/SSD goldens.
stack(
    [
        row([
            ("desktop", load(os.path.join(outdir, f"t17-visual-floor-{suffix}.png"))),
            ("window", load(os.path.join(golden_dir, window))),
            ("titlebar", load(os.path.join(outdir, f"t17-visual-floor-{suffix}-titlebar.png"))),
            ("ssd", load(os.path.join(golden_dir, ssd))),
        ])
        for suffix, window, ssd in ROWS
    ],
    "t17-visual-floor-gallery.png",
)
# The menu-bar and Dock chrome bands, dark/light/reduced, for the review of the
# translucent chrome material itself.
stack(
    [row([(s, load(os.path.join(outdir, f"t17-visual-floor-{s}-menubar.png"))) for s, *_ in ROWS])],
    "t17-visual-floor-menubar.png",
)
stack(
    [row([(s, load(os.path.join(outdir, f"t17-visual-floor-{s}-dock.png"))) for s, *_ in ROWS])],
    "t17-visual-floor-dock.png",
)
PY

echo "capture-t17-visual-floor: done (scratch $SCRATCH)"
[ "${KEEP_SCRATCH:-0}" = "1" ] && echo "capture-t17-visual-floor: kept scratch $SCRATCH" || true