#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T205 notification banner, stack and lower-panel material capture.
#
# Runs the real nested compositor on the replacement rail and captures the
# banner and the open Notification Center in Light/Dark, empty, banner-only,
# populated, concurrent and scrollable states, plus Reduce Transparency,
# Increase Contrast and the no-sampled-material software path. Neutral and
# grid backdrops make the three materials' attenuation visible.
#
# Outputs: docs/captures/t205-*.png, matching query dumps, and
# docs/captures/t205-notification-materials-capture.json. Not part of
# `make e2e`; needs a host Wayland session (or kwin_wayland --virtual).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-5}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t205.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
KWIN_SOCKET="df-t205-kwin-$$"
KWIN_PID=""
HOST_DISPLAY="${WAYLAND_DISPLAY:-}"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-}"
FORCE_KWIN="${FORCE_KWIN:-0}"
MANIFEST="$OUTDIR/t205-notification-materials-capture.json"
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
    echo "capture-t205: XDG_RUNTIME_DIR is unset — writing OPEN" >&2
    cat >"$OUTDIR/t205-notification-materials.open.txt" <<EOF
T205 notification materials — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make notification-materials-capture
EOF
    exit 0
fi

if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t205: starting kwin_wayland --virtual (socket $KWIN_SOCKET)"
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
            echo "capture-t205: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t205: no host Wayland session and no kwin_wayland — writing OPEN" >&2
        cat >"$OUTDIR/t205-notification-materials.open.txt" <<EOF
T205 notification materials — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: no usable host Wayland session and kwin_wayland is not installed.
rerun: WAYLAND_DISPLAY=<host-socket> make notification-materials-capture
EOF
        exit 0
    fi
fi

NEUTRAL="$SCRATCH/neutral.png"
GRID="$SCRATCH/grid.png"
python3 scripts/t205-notification-materials.py wallpaper --kind neutral --out "$NEUTRAL" \
    --width 1920 --height 1200 >/dev/null
python3 scripts/t205-notification-materials.py wallpaper --kind grid --out "$GRID" \
    --width 1920 --height 1200 >/dev/null

run_case() {
    local label="$1" state="$2" fixture="$3" scheme="$4" variant="$5" \
          wallpaper="$6" material_path="$7" reduce="$8" contrast="$9"
    local out="$OUTDIR/t205-${label}.png"
    local query="$OUTDIR/t205-${label}.txt"
    local log="$SCRATCH/${label}.log"
    local socket="df-t205-${label}-$$"
    local source="$NEUTRAL"
    [ "$wallpaper" = "grid" ] && source="$GRID"
    echo "capture-t205: $label ($state, $fixture, $scheme, $variant, $wallpaper)"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        DRAGONFRUIT_NESTED_SIZE="1920x1200" \
        DRAGONFRUIT_GLASS_ROLLOUT=replacement \
        FULL_SESSION=1 SCALE=1 SOCKET="$socket" \
        WALLPAPER_SOURCE="$source" WALLPAPER_FIT=fill \
        COLOR_SCHEME="$scheme" TRANSPARENCY=0.5255 \
        REDUCE_TRANSPARENCY="$reduce" INCREASE_CONTRAST="$contrast" \
        NOTIFY_FIXTURE="$fixture" \
        FORCE_TIER=full FORCE_MATERIAL_PATH="$material_path" \
        QUERY_OUT="$query" SETTLE="$SETTLE" OUT="$out" \
        SCRATCH="$SCRATCH/$label" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh notification-center >"$log" 2>&1
    rm -rf "$HOST_RUNTIME/$socket.session" "$HOST_RUNTIME/$socket" \
           "$HOST_RUNTIME/$socket.lock" "$HOST_RUNTIME/$socket.launch-token" \
           "$HOST_RUNTIME/$socket.desktop-launch-token" 2>/dev/null || true
    if [ ! -s "$out" ]; then
        echo "capture-t205: $label produced no capture" >&2
        tail -40 "$log" >&2 || true
        return 1
    fi
    echo "capture-t205: $label -> $out"
}

run_case empty-light-neutral empty empty light sampled neutral gpu 0 0
run_case empty-dark-neutral empty empty dark sampled neutral gpu 0 0
run_case banner-empty-light-neutral banner-empty banner-empty light sampled neutral gpu 0 0
run_case banner-empty-dark-neutral banner-empty banner-empty dark sampled neutral gpu 0 0
run_case populated-light-neutral populated populated light sampled neutral gpu 0 0
run_case populated-dark-neutral populated populated dark sampled neutral gpu 0 0
run_case concurrent-light-neutral concurrent concurrent light sampled neutral gpu 0 0
run_case concurrent-light-grid concurrent concurrent light sampled grid gpu 0 0
run_case concurrent-dark-neutral concurrent concurrent dark sampled neutral gpu 0 0
run_case concurrent-dark-grid concurrent concurrent dark sampled grid gpu 0 0
run_case scrollable-light-neutral scrollable scrollable light sampled neutral gpu 0 0
run_case scrollable-dark-neutral scrollable scrollable dark sampled neutral gpu 0 0
run_case concurrent-light-reduce concurrent concurrent light reduce-transparency neutral gpu 1 0
run_case concurrent-dark-reduce concurrent concurrent dark reduce-transparency neutral gpu 1 0
run_case concurrent-light-contrast concurrent concurrent light increase-contrast neutral gpu 0 1
run_case concurrent-dark-contrast concurrent concurrent dark increase-contrast neutral gpu 0 1
run_case concurrent-light-software concurrent concurrent light software neutral software 0 0
run_case concurrent-dark-software concurrent concurrent dark software neutral software 0 0

python3 - "$OUTDIR" "$MANIFEST" <<'PY'
import json
import sys
from pathlib import Path

outdir = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
cases = [
    ("empty-light-neutral", "empty", "empty", "light", "sampled", "neutral", "gpu", False, False),
    ("empty-dark-neutral", "empty", "empty", "dark", "sampled", "neutral", "gpu", False, False),
    ("banner-empty-light-neutral", "banner-empty", "banner-empty", "light", "sampled", "neutral", "gpu", False, False),
    ("banner-empty-dark-neutral", "banner-empty", "banner-empty", "dark", "sampled", "neutral", "gpu", False, False),
    ("populated-light-neutral", "populated", "populated", "light", "sampled", "neutral", "gpu", False, False),
    ("populated-dark-neutral", "populated", "populated", "dark", "sampled", "neutral", "gpu", False, False),
    ("concurrent-light-neutral", "concurrent", "concurrent", "light", "sampled", "neutral", "gpu", False, False),
    ("concurrent-light-grid", "concurrent", "concurrent", "light", "sampled", "grid", "gpu", False, False),
    ("concurrent-dark-neutral", "concurrent", "concurrent", "dark", "sampled", "neutral", "gpu", False, False),
    ("concurrent-dark-grid", "concurrent", "concurrent", "dark", "sampled", "grid", "gpu", False, False),
    ("scrollable-light-neutral", "scrollable", "scrollable", "light", "sampled", "neutral", "gpu", False, False),
    ("scrollable-dark-neutral", "scrollable", "scrollable", "dark", "sampled", "neutral", "gpu", False, False),
    ("concurrent-light-reduce", "concurrent", "concurrent", "light", "reduce-transparency", "neutral", "gpu", True, False),
    ("concurrent-dark-reduce", "concurrent", "concurrent", "dark", "reduce-transparency", "neutral", "gpu", True, False),
    ("concurrent-light-contrast", "concurrent", "concurrent", "light", "increase-contrast", "neutral", "gpu", False, True),
    ("concurrent-dark-contrast", "concurrent", "concurrent", "dark", "increase-contrast", "neutral", "gpu", False, True),
    ("concurrent-light-software", "concurrent", "concurrent", "light", "software", "neutral", "software", False, False),
    ("concurrent-dark-software", "concurrent", "concurrent", "dark", "software", "neutral", "software", False, False),
]
rows = []
for (label, state, fixture, scheme, variant, wallpaper, material_path,
     reduce, contrast) in cases:
    image = outdir / f"t205-{label}.png"
    query = outdir / f"t205-{label}.txt"
    rows.append({
        "label": label,
        "state": state,
        "fixture": fixture,
        "scheme": scheme,
        "variant": variant,
        "wallpaper": wallpaper,
        "material_path": material_path,
        "reduce_transparency": reduce,
        "increase_contrast": contrast,
        "capture": str(image.relative_to(outdir.parent.parent)) if image.exists() else None,
        "query": str(query.relative_to(outdir.parent.parent)) if query.exists() else None,
        "present": image.exists(),
    })
report = {
    "backend": "nested",
    "rollout": "replacement",
    "size": "1920x1200",
    "scale": 1,
    "cases": rows,
    "note": "Compositor-own render. The scorer samples the banner, stack and "
            "lower-panel regions independently against the neutral backdrop "
            "and checks grid attenuation, region disjointness and the "
            "software/accessibility fallbacks. Dark banner/lower rows are "
            "labelled underconstrained.",
}
manifest_path.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
PY

python3 scripts/t205-notification-materials.py --score "$MANIFEST" --write

echo "capture-t205: done"
