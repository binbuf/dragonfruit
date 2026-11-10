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
# `lines_grid` is the lens evidence: a flat field cannot show the corner
# refraction. The default run keeps the flat field for the rim/glow profile;
# `WALLPAPER_BACKGROUND=lines_grid` adds the lens row set.
WALLPAPER_BACKGROUND="${WALLPAPER_BACKGROUND:-flat}"
SUFFIX="${SUFFIX:-}"
if [ -z "$SUFFIX" ] && [ "$WALLPAPER_BACKGROUND" != "flat" ]; then
    SUFFIX="-lens"
fi
mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    cat >"$OUTDIR/t235a-dock-perimeter${SUFFIX}.md" <<EOF
# T235a — Dock plate perimeter capture${SUFFIX}

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
        cat >"$OUTDIR/t235a-dock-perimeter${SUFFIX}.md" <<EOF
# T235a — Dock plate perimeter capture${SUFFIX}

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
    local wallpaper="$SCRATCH/${WALLPAPER_BACKGROUND}-${SCALE}x.png"
    local query="$SCRATCH/${tag}-query.txt"
    LAST_TIER="unknown"
    if [ ! -s "$wallpaper" ]; then
        if [ "$WALLPAPER_BACKGROUND" = "flat" ]; then
            python3 scripts/t198-dock-optics.py wallpaper --out "$wallpaper" \
                --width "$(python3 -c "print(round(1920*$SCALE))")" \
                --height "$(python3 -c "print(round(1200*$SCALE))")" \
                --scale "$SCALE" --flat
        else
            python3 scripts/t198-dock-optics.py wallpaper --out "$wallpaper" \
                --width "$(python3 -c "print(round(1920*$SCALE))")" \
                --height "$(python3 -c "print(round(1200*$SCALE))")" \
                --scale "$SCALE" --background "$WALLPAPER_BACKGROUND"
        fi
    fi
    echo "capture-t235a: $position / $scheme / $state ($WALLPAPER_BACKGROUND)"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        FULL_SESSION=1 SCALE="$SCALE" \
        DOCK_POSITION="$position" COLOR_SCHEME="$scheme" \
        WALLPAPER_SOURCE="$wallpaper" \
        OUT="$raw" SETTLE="$SETTLE" QUERY_OUT="$query" \
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
    # The plate rim is the long bright run that does not span the output; a
    # wallpaper grid line does span it. The helper crops the actual plate
    # corners, not the first/last strip pixels (which are usually wallpaper).
    if python3 scripts/t235a-crop-plate.py --capture "$raw" \
            --position "$position" --scale "$SCALE" \
            --out-strip "$dest" \
            --out-corners "${dest%.png}-corners.png"; then
        :
    else
        echo "capture-t235a: $tag corner crop OPEN" >&2
    fi
    if [ -s "$SCRATCH/${tag}/shell.log" ]; then
        LAST_TIER="$(grep -ao 'material role=dock tier=[0-9]*' \
            "$SCRATCH/${tag}/shell.log" | tail -1 | sed 's/.*tier=//')"
        case "$LAST_TIER" in
            0) LAST_TIER="full" ;;
            1) LAST_TIER="reduced" ;;
            2) LAST_TIER="minimal" ;;
            *) LAST_TIER="unknown" ;;
        esac
    fi
    echo "capture-t235a: $tag tier=$LAST_TIER"
}

ROWS=""
for position in bottom left right; do
    for scheme in light dark; do
        for state in settled motion; do
            label="rest"
            [ "$state" = "motion" ] && label="magnified"
            dest="$OUTDIR/t235a-dock-perimeter${SUFFIX}-${position}-${scheme}-${label}.png"
            if capture "$position" "$scheme" "$state" "$dest"; then
                ROWS="$ROWS| $position | $scheme | $label | $WALLPAPER_BACKGROUND | $LAST_TIER | $SCALE | \`$dest\` | measured |\n"
            else
                ROWS="$ROWS| $position | $scheme | $label | $WALLPAPER_BACKGROUND | — | $SCALE | — | OPEN |\n"
            fi
        done
    done
done

cat >"$OUTDIR/t235a-dock-perimeter${SUFFIX}.md" <<EOF
# T235a — Dock plate perimeter and corner lens capture${SUFFIX}

timestamp: $(date -Is)
build: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)
output scale: ${SCALE}x
wallpaper: ${WALLPAPER_BACKGROUND}
Reduce Transparency: ${REDUCE_TRANSPARENCY:-0}
Increase Contrast: ${INCREASE_CONTRAST:-0}
Reduce Motion: ${REDUCE_MOTION:-0}

The captures are the nested compositor's own render. Each row crops the plate's
edge strip and writes a close-up of the plate's actual rounded corners (not the
wallpaper at the strip ends). The tier column is the live Dock material tier
from the shell log; the persistent Dock floor is \`reduced\`, so the corner lens
must be visible there. Inspect for a darker side, a brighter bottom centre, a
flat-side lens warp and an outer shadow.

A flat field shows the rim and inner glow but cannot show the corner lens;
run with \`WALLPAPER_BACKGROUND=lines_grid\` for the refraction evidence.

| Position | Scheme | State | Wallpaper | Tier | Scale | Artifact | Verdict |
|---|---|---|---|---|---|---|---|
$(printf '%b' "$ROWS")

Rerun:

\`\`\`sh
make t235a-dock-perimeter-capture
WALLPAPER_BACKGROUND=lines_grid make t235a-dock-perimeter-capture
\`\`\`

A row that did not produce a render is OPEN, not a claimed measurement.
EOF

echo "capture-t235a: wrote $OUTDIR/t235a-dock-perimeter${SUFFIX}.md"
