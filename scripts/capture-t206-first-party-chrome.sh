#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T206 first-party Settings/Files chrome capture.
#
# Runs the real nested compositor with the Qt app as the demo client and a
# private-bus settingsd, so each app's app-local Theme/Glass bindings follow
# the live Light/Dark, Reduce Transparency and Increase Contrast state. The
# window sits over a neutral or saturated full-screen backdrop; the sidebar and
# toolbar samples show the measured near-opaque transmission, while the detail
# sheet and path bar stay opaque.
#
# Also captures the unsupported/non-composited opaque fallback via
# DF_APP_CHROME_SOLID=1 and an unfocused state (the material is focus-invariant;
# only the titlebar/controls state changes).
#
# Outputs: docs/captures/t206-*.png, matching query dumps, and
# docs/captures/t206-first-party-chrome.json/.md. Not part of `make e2e`;
# needs a host Wayland session (or kwin_wayland --virtual).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-5}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t206.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
KWIN_SOCKET="df-t206-kwin-$$"
KWIN_PID=""
HOST_DISPLAY="${WAYLAND_DISPLAY:-}"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-}"
FORCE_KWIN="${FORCE_KWIN:-0}"
MANIFEST="$OUTDIR/t206-first-party-chrome-capture.json"
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
    echo "capture-t206: XDG_RUNTIME_DIR is unset — writing OPEN" >&2
    cat >"$OUTDIR/t206-first-party-chrome.open.txt" <<EOF
T206 first-party chrome — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make first-party-chrome-capture
EOF
    exit 0
fi

if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t206: starting kwin_wayland --virtual (socket $KWIN_SOCKET)"
        XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY= \
            kwin_wayland --virtual --fullscreen 1 --socket "$KWIN_SOCKET" \
            --width 1920 --height 1200 --no-lockscreen \
            --no-global-shortcuts --no-kactivities \
            >"$SCRATCH/kwin.log" 2>&1 &
        KWIN_PID=$!
        for _ in $(seq 1 100); do
            [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ] && break
            sleep 0.1
        done
        if ! [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ]; then
            echo "capture-t206: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t206: no host Wayland session and no kwin_wayland — writing OPEN" >&2
        cat >"$OUTDIR/t206-first-party-chrome.open.txt" <<EOF
T206 first-party chrome — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: no usable host Wayland session and kwin_wayland is not installed.
rerun: WAYLAND_DISPLAY=<host-socket> make first-party-chrome-capture
EOF
        exit 0
    fi
fi

NEUTRAL="$SCRATCH/neutral.png"
SATURATED="$SCRATCH/saturated.png"
python3 scripts/t206-first-party-chrome.py wallpaper --kind neutral --out "$NEUTRAL" \
    --width 1920 --height 1200 >/dev/null
python3 scripts/t206-first-party-chrome.py wallpaper --kind saturated --out "$SATURATED" \
    --width 1920 --height 1200 >/dev/null

run_case() {
    local label="$1" app="$2" scheme="$3" wallpaper="$4" variant="$5" \
          focus="$6"
    if [ -n "${ONLY:-}" ] && [[ ",$ONLY," != *",$label,"* ]]; then
        return 0
    fi
    local out="$OUTDIR/t206-${label}.png"
    local query="$OUTDIR/t206-${label}.txt"
    local log="$SCRATCH/${label}.log"
    local socket="df-t206-${label}-$$"
    local source="$NEUTRAL"
    [ "$wallpaper" = "saturated" ] && source="$SATURATED"
    local reduce=0 contrast=0 solid=0 unfocus=0
    case "$variant" in
        reduce) reduce=1 ;;
        contrast) contrast=1 ;;
        solid) solid=1 ;;
    esac
    [ "$focus" = "unfocused" ] && unfocus=1
    # The bridge checks whether the variable is set, so only export it for the
    # explicit fallback rows (an empty "0" would also force the plate).
    if [ "$solid" = "1" ]; then
        export DF_APP_CHROME_SOLID=1
    else
        unset DF_APP_CHROME_SOLID
    fi
    local extra=()
    if [ "$app" = "settings" ]; then
        extra=(SETTINGS_START_PANE="${SETTINGS_START_PANE:-appearance}")
    else
        extra=(FILES_START_VIEW="${FILES_START_VIEW:-list}")
        [ -n "${FILES_START_URI:-}" ] && extra+=(FILES_START_URI="$FILES_START_URI")
    fi
    echo "capture-t206: $label ($app, $scheme, $wallpaper, $variant, $focus)"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        DRAGONFRUIT_NESTED_SIZE="1920x1200" \
        DRAGONFRUIT_GLASS_ROLLOUT=replacement \
        FULL_SESSION=0 APP_SESSION=1 SCALE=1 SOCKET="$socket" \
        WALLPAPER_SOURCE="$source" WALLPAPER_FIT=fill \
        COLOR_SCHEME="$scheme" TRANSPARENCY=0.5255 \
        REDUCE_TRANSPARENCY="$reduce" INCREASE_CONTRAST="$contrast" \
        UNFOCUS="$unfocus" \
        "${extra[@]}" \
        FORCE_TIER=full FORCE_MATERIAL_PATH=gpu \
        QUERY_OUT="$query" SETTLE="$SETTLE" OUT="$out" \
        SCRATCH="$SCRATCH/$label" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh "${app}-window" >"$log" 2>&1
    rm -rf "$HOST_RUNTIME/$socket.session" "$HOST_RUNTIME/$socket" \
           "$HOST_RUNTIME/$socket.lock" "$HOST_RUNTIME/$socket.launch-token" \
           "$HOST_RUNTIME/$socket.desktop-launch-token" 2>/dev/null || true
    if [ ! -s "$out" ]; then
        echo "capture-t206: $label produced no capture" >&2
        tail -40 "$log" >&2 || true
        return 1
    fi
    echo "capture-t206: $label -> $out"
}

run_app() {
    local app="$1"
    run_case "${app}-light-neutral" "$app" light neutral sampled focused
    run_case "${app}-light-saturated" "$app" light saturated sampled focused
    run_case "${app}-dark-neutral" "$app" dark neutral sampled focused
    run_case "${app}-dark-saturated" "$app" dark saturated sampled focused
    run_case "${app}-light-unfocused" "$app" light neutral sampled unfocused
    run_case "${app}-dark-unfocused" "$app" dark neutral sampled unfocused
    run_case "${app}-light-reduce" "$app" light neutral reduce focused
    run_case "${app}-dark-reduce" "$app" dark neutral reduce focused
    run_case "${app}-light-solid" "$app" light saturated solid focused
    run_case "${app}-dark-solid" "$app" dark neutral solid focused
    run_case "${app}-light-contrast" "$app" light neutral contrast focused
}

run_app settings
run_app files

python3 - "$OUTDIR" "$MANIFEST" <<'PY'
import json
import sys
from pathlib import Path

outdir = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
variants = ["sampled", "sampled", "sampled", "sampled", "sampled", "sampled",
            "reduce", "reduce", "solid", "solid", "contrast"]
schemes = ["light", "light", "dark", "dark", "light", "dark",
           "light", "dark", "light", "dark", "light"]
wallpapers = ["neutral", "saturated", "neutral", "saturated", "neutral",
              "neutral", "neutral", "neutral", "saturated", "neutral",
              "neutral"]
focus = ["focused", "focused", "focused", "focused", "unfocused", "unfocused",
         "focused", "focused", "focused", "focused", "focused"]
suffixes = ["light-neutral", "light-saturated", "dark-neutral",
            "dark-saturated", "light-unfocused", "dark-unfocused",
            "light-reduce", "dark-reduce", "light-solid", "dark-solid",
            "light-contrast"]
rows = []
for app in ("settings", "files"):
    for suffix, scheme, wallpaper, variant, focused in zip(
            suffixes, schemes, wallpapers, variants, focus):
        label = f"{app}-{suffix}"
        image = outdir / f"t206-{label}.png"
        query = outdir / f"t206-{label}.txt"
        rows.append({
            "label": label,
            "app": app,
            "scheme": scheme,
            "variant": variant,
            "wallpaper": wallpaper,
            "focus": focused,
            "reduce_transparency": variant == "reduce",
            "increase_contrast": variant == "contrast",
            "solid_fallback": variant == "solid",
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
    "note": "Compositor-own render of the live Qt Settings/Files window over "
            "neutral and saturated backdrops. Sidebar and toolbar are sampled "
            "separately; the detail sheet and path bar are opaque. The "
            "unfocused rows verify the material is focus-invariant, and the "
            "solid rows force the non-composited opaque plate.",
}
manifest_path.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
PY

python3 scripts/t206-first-party-chrome.py score "$MANIFEST" --write

echo "capture-t206: done"
