#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T203 Control Center material capture.
#
# Runs the real nested compositor on the replacement rail and captures the
# compositor's own render for the collapsed and expanded Control Center in
# Light/Dark, Increase Contrast, Reduce Transparency and an absent-adapter
# case. Every case is captured over a neutral mid-grey and a fine grid so the
# outer/module/list attenuation and the moving expanded region are visible.
#
# Outputs: docs/captures/t203-*.png, matching query dumps, and
# docs/captures/t203-control-center-capture.json. Not part of `make e2e`;
# needs a host Wayland session (or kwin_wayland --virtual).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-5}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t203.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
KWIN_SOCKET="df-t203-kwin-$$"
KWIN_PID=""
HOST_DISPLAY="${WAYLAND_DISPLAY:-}"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-}"
FORCE_KWIN="${FORCE_KWIN:-0}"
MANIFEST="$OUTDIR/t203-control-center-capture.json"
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
    echo "capture-t203: XDG_RUNTIME_DIR is unset — writing OPEN" >&2
    cat >"$OUTDIR/t203-control-center.open.txt" <<EOF
T203 Control Center materials — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make control-center-materials-capture
EOF
    exit 0
fi

if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t203: starting kwin_wayland --virtual (socket $KWIN_SOCKET)"
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
            echo "capture-t203: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t203: no host Wayland session and no kwin_wayland — writing OPEN" >&2
        cat >"$OUTDIR/t203-control-center.open.txt" <<EOF
T203 Control Center materials — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: no usable host Wayland session and kwin_wayland is not installed.
rerun: WAYLAND_DISPLAY=<host-socket> make control-center-materials-capture
EOF
        exit 0
    fi
fi

NEUTRAL="$SCRATCH/neutral.png"
GRID="$SCRATCH/grid.png"
python3 scripts/t203-control-center.py wallpaper --kind neutral --out "$NEUTRAL" \
    --width 1920 --height 1200 >/dev/null
python3 scripts/t203-control-center.py wallpaper --kind grid --out "$GRID" \
    --width 1920 --height 1200 >/dev/null

run_case() {
    local label="$1" fixture="$2" scheme="$3" reduce="$4" contrast="$5" \
          absent="$6" wallpaper="$7"
    local out="$OUTDIR/t203-${label}.png"
    local query="$OUTDIR/t203-${label}.txt"
    local log="$SCRATCH/${label}.log"
    local socket="df-t203-${label}-$$"
    local source="$NEUTRAL"
    [ "$wallpaper" = "grid" ] && source="$GRID"
    echo "capture-t203: $label ($fixture, $scheme, $wallpaper)"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        DRAGONFRUIT_NESTED_SIZE="1920x1200" \
        DRAGONFRUIT_GLASS_ROLLOUT=replacement \
        FULL_SESSION=1 SCALE=1 SOCKET="$socket" \
        WALLPAPER_SOURCE="$source" WALLPAPER_FIT=fill \
        COLOR_SCHEME="$scheme" TRANSPARENCY=0.5255 \
        REDUCE_TRANSPARENCY="$reduce" INCREASE_CONTRAST="$contrast" \
        CONTROL_CENTER_FIXTURE="$fixture" \
        CONTROL_CENTER_ABSENT="$absent" \
        FORCE_TIER=full \
        QUERY_OUT="$query" SETTLE="$SETTLE" OUT="$out" \
        SCRATCH="$SCRATCH/$label" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh control-center >"$log" 2>&1
    rm -rf "$HOST_RUNTIME/$socket.session" "$HOST_RUNTIME/$socket" \
           "$HOST_RUNTIME/$socket.lock" "$HOST_RUNTIME/$socket.launch-token" \
           "$HOST_RUNTIME/$socket.desktop-launch-token" 2>/dev/null || true
    if [ ! -s "$out" ]; then
        echo "capture-t203: $label produced no capture" >&2
        tail -40 "$log" >&2 || true
        return 1
    fi
    echo "capture-t203: $label -> $out"
}

run_case collapsed-light-neutral hold light 0 0 "" neutral
run_case collapsed-light-grid hold light 0 0 "" grid
run_case collapsed-dark-neutral hold dark 0 0 "" neutral
run_case collapsed-dark-grid hold dark 0 0 "" grid
run_case expanded-light-neutral expanded light 0 0 "" neutral
run_case expanded-light-grid expanded light 0 0 "" grid
run_case expanded-dark-neutral expanded dark 0 0 "" neutral
run_case expanded-dark-grid expanded dark 0 0 "" grid
run_case expanded-dark-contrast-neutral expanded dark 0 1 "" neutral
run_case expanded-dark-contrast-grid expanded dark 0 1 "" grid
run_case expanded-dark-reduce-neutral expanded dark 1 0 "" neutral
run_case expanded-dark-reduce-grid expanded dark 1 0 "" grid
run_case expanded-absent-neutral expanded light 0 0 "bluetooth,storage,input,battery" neutral
run_case expanded-absent-grid expanded light 0 0 "bluetooth,storage,input,battery" grid

python3 - "$OUTDIR" "$MANIFEST" <<'PY'
import json
import sys
from pathlib import Path

outdir = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
cases = [
    ("collapsed-light-neutral", "collapsed", "light", False, False, "", "neutral"),
    ("collapsed-light-grid", "collapsed", "light", False, False, "", "grid"),
    ("collapsed-dark-neutral", "collapsed", "dark", False, False, "", "neutral"),
    ("collapsed-dark-grid", "collapsed", "dark", False, False, "", "grid"),
    ("expanded-light-neutral", "expanded", "light", False, False, "", "neutral"),
    ("expanded-light-grid", "expanded", "light", False, False, "", "grid"),
    ("expanded-dark-neutral", "expanded", "dark", False, False, "", "neutral"),
    ("expanded-dark-grid", "expanded", "dark", False, False, "", "grid"),
    ("expanded-dark-contrast-neutral", "expanded", "dark", False, True, "", "neutral"),
    ("expanded-dark-contrast-grid", "expanded", "dark", False, True, "", "grid"),
    ("expanded-dark-reduce-neutral", "expanded", "dark", True, False, "", "neutral"),
    ("expanded-dark-reduce-grid", "expanded", "dark", True, False, "", "grid"),
    ("expanded-absent-neutral", "expanded", "light", False, False,
     "bluetooth,storage,input,battery", "neutral"),
    ("expanded-absent-grid", "expanded", "light", False, False,
     "bluetooth,storage,input,battery", "grid"),
]
rows = []
for label, state, scheme, reduce, contrast, absent, wallpaper in cases:
    image = outdir / f"t203-{label}.png"
    query = outdir / f"t203-{label}.txt"
    rows.append({
        "label": label,
        "state": state,
        "scheme": scheme,
        "reduce_transparency": reduce,
        "increase_contrast": contrast,
        "absent": absent,
        "wallpaper": wallpaper,
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
    "note": "Compositor-own render; the material score compares the sampled "
            "outer/module luma to the resolved presets over the neutral "
            "backdrop and checks grid attenuation plus expanded-region "
            "movement.",
}
manifest_path.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
PY

python3 scripts/t203-control-center.py --score "$MANIFEST" --write

echo "capture-t203: done"
