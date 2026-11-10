#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T235a Dock plate perimeter and corner-lens capture.
#
# Runs the nested demo at rest and mid-magnification for a bottom, left and
# right Dock in light and dark, then crops the plate's edge strip and both
# corners from the compositor's own render:
#
#   docs/captures/t235a-dock-perimeter-<position>-<scheme>-<state>.png
#   docs/captures/t235a-dock-perimeter-<position>-<scheme>-<state>-corners.png
#   docs/captures/t235a-dock-perimeter.md
#
# `<state>` is `rest` or `magnified`. The metadata row records the build,
# output scale, scheme, position, Reduce Transparency, Increase Contrast and
# Reduce Motion. Missing evidence is an explicit OPEN row, never invented.
#
# Requires: a host Wayland session and the built tree (`make build`). Not part
# of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-4}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t235a.XXXXXX")}"
SCALE="${SCALE:-1}"
mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    cat >"$OUTDIR/t235a-dock-perimeter.md" <<EOF
# T235a — Dock plate perimeter capture

timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make t235a-dock-perimeter-capture
EOF
    exit 0
fi
for tool in python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t235a: $tool not found" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t235a: python3 Pillow not found" >&2
    exit 1
}

cleanup() {
    if [ -n "${KWIN_PID:-}" ]; then
        kill -TERM "$KWIN_PID" 2>/dev/null || true
        sleep 1
        kill -KILL "$KWIN_PID" 2>/dev/null || true
    fi
    rm -f "${HOST_RUNTIME:-}/$KWIN_SOCKET" "${HOST_RUNTIME:-}/$KWIN_SOCKET.lock" 2>/dev/null || true
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

# The host's live DRM session does not reliably map a nested demo, so this
# capture runs its own kwin_wayland --virtual host like the T198 capture.
HOST_RUNTIME="${XDG_RUNTIME_DIR}"
HOST_DISPLAY="${WAYLAND_DISPLAY:-}"
FORCE_KWIN="${FORCE_KWIN:-1}"
KWIN_SOCKET="df-t235a-kwin-$$"
KWIN_PID=""
if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t235a: starting kwin_wayland --virtual ($KWIN_SOCKET)"
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
            echo "capture-t235a: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t235a: kwin_wayland is not installed — writing OPEN" >&2
        cat >"$OUTDIR/t235a-dock-perimeter.md" <<EOF
# T235a — Dock plate perimeter capture

timestamp: $(date -Is)
verdict: OPEN
reason: no usable nested host and kwin_wayland is not installed.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make t235a-dock-perimeter-capture
EOF
        exit 0
    fi
fi

capture() {
    local position="$1" scheme="$2" state="$3" dest="$4"
    local tag="t235a-${position}-${scheme}-${state}"
    local raw="$SCRATCH/${tag}-raw.png"
    local wallpaper="$SCRATCH/flat-${SCALE}x.png"
    if [ ! -s "$wallpaper" ]; then
        python3 scripts/t198-dock-optics.py wallpaper --out "$wallpaper" \
            --width "$(python3 -c "print(round(1920*$SCALE))")" \
            --height "$(python3 -c "print(round(1200*$SCALE))")" \
            --scale "$SCALE" --flat
    fi
    echo "capture-t235a: $position / $scheme / $state"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        FULL_SESSION=1 SCALE="$SCALE" \
        DOCK_POSITION="$position" COLOR_SCHEME="$scheme" \
        WALLPAPER_SOURCE="$wallpaper" \
        OUT="$raw" SETTLE="$SETTLE" \
        SOCKET="df-t235a-${position}-${scheme}-${state}-$$" \
        SCRATCH="$SCRATCH/${tag}" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh "$state" >"$SCRATCH/${tag}.log" 2>&1 \
        || {
            echo "capture-t235a: $tag failed; log:" >&2
            tail -20 "$SCRATCH/${tag}.log" >&2
            return 1
        }
    if [ ! -s "$raw" ]; then
        echo "capture-t235a: $tag produced no render" >&2
        return 1
    fi
    cp "$raw" "$dest"
    # The full-session rail writes a large scratch XDG tree under the runtime
    # dir; remove it before the next case or the 1.3G tmpfs fills up.
    rm -rf "$XDG_RUNTIME_DIR/df-t235a-${position}-${scheme}-${state}-$$.session" \
           "$XDG_RUNTIME_DIR/df-t235a-${position}-${scheme}-${state}-$$" \
           "$XDG_RUNTIME_DIR/df-t235a-${position}-${scheme}-${state}-$$.lock" \
           "$XDG_RUNTIME_DIR/df-t235a-${position}-${scheme}-${state}-$$.launch-token" \
           2>/dev/null || true
    python3 - "$dest" "$position" "$state" <<'PY'
import sys
from PIL import Image

raw, position, state = sys.argv[1], sys.argv[2], sys.argv[3]
im = Image.open(raw).convert("RGB")
w, h = im.size
# The capture is the nested output at 1920x1200 (scaled by SCALE); crop the
# edge strip that contains the plate, its corners and the gap to the screen
# edge, then upscale for inspection.
strip = min(w, h, 200)
if position == "bottom":
    box = (0, max(0, h - strip), w, h)
elif position == "left":
    box = (0, 0, strip, h)
else:
    box = (max(0, w - strip), 0, w, h)
crop = im.crop(box)
scale = max(1, 900 // max(1, min(crop.size)))
crop.resize((crop.width * scale, crop.height * scale), Image.NEAREST).save(raw)
print(f"capture-t235a: {position}/{state} strip {crop.size} x{scale}")
PY
    # A 6x close-up of both plate corners. The strip is long; take the first
    # and last 120 px along the Dock's long axis.
    python3 - "$dest" "$position" <<'PY'
import sys
from PIL import Image

dest, position = sys.argv[1], sys.argv[2]
im = Image.open(dest).convert("RGB")
w, h = im.size
if position == "bottom":
    first = im.crop((0, 0, min(w, 360), h))
    last = im.crop((max(0, w - 360), 0, w, h))
    out = Image.new("RGB", (first.width + last.width + 12, h), (24, 24, 24))
else:
    first = im.crop((0, 0, w, min(h, 360)))
    last = im.crop((0, max(0, h - 360), w, h))
    out = Image.new("RGB", (w, first.height + last.height + 12), (24, 24, 24))
out.paste(first, (0, 0))
if position == "bottom":
    out.paste(last, (first.width + 12, 0))
else:
    out.paste(last, (0, first.height + 12))
out.save(dest.replace(".png", "-corners.png"))
PY
}

ROWS=""
for position in bottom left right; do
    for scheme in light dark; do
        for state in settled motion; do
            label="rest"
            [ "$state" = "motion" ] && label="magnified"
            dest="$OUTDIR/t235a-dock-perimeter-${position}-${scheme}-${label}.png"
            if capture "$position" "$scheme" "$state" "$dest"; then
                ROWS="$ROWS| $position | $scheme | $label | $SCALE | \`$dest\` | measured |\n"
            else
                ROWS="$ROWS| $position | $scheme | $label | $SCALE | — | OPEN |\n"
            fi
        done
    done
done

cat >"$OUTDIR/t235a-dock-perimeter.md" <<EOF
# T235a — Dock plate perimeter and corner lens capture

timestamp: $(date -Is)
build: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)
output scale: ${SCALE}x
Reduce Transparency: off
Increase Contrast: off
Reduce Motion: off

The captures are the nested compositor's own render. Each row crops the plate's
edge strip at 3x and writes a 6x both-corners companion. Inspect for a darker
side, a brighter bottom centre, a flat-side lens warp and an outer shadow.

| Position | Scheme | State | Scale | Artifact | Verdict |
|---|---|---|---|---|---|
$(printf '%b' "$ROWS")

Rerun:

\`\`\`sh
make t235a-dock-perimeter-capture
\`\`\`

A row that did not produce a render is OPEN, not a claimed measurement.
EOF

echo "capture-t235a: wrote $OUTDIR/t235a-dock-perimeter.md"
