#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 native DRM follow-up: repeat the Dock popover opening A/B on 4K/144 Hz
# with Full, Reduced, and Minimal material.
#
# The nested A/B answers the work-vs-wait question; this script is the native
# presentation-time half. It requires a free logind seat (the live session must
# not own DRM master). With no free seat it writes an explicit OPEN artifact
# and exits 0, exactly like the other native hardware rails.
#
# Outputs:
#   docs/captures/t238-dock-popover-ab-native/{full,reduced,minimal}/report.json
#   docs/captures/t238-dock-popover-ab-native/report.md
#   docs/captures/t238-dock-popover-ab-native.open.txt (no free seat)
set -euo pipefail

cd "$(dirname "$0")/.."

. "$(dirname "$0")/t21-capture-lib.sh"

OUT_DIR="${OUT_DIR:-docs/captures/t238-dock-popover-ab-native}"
CYCLES="${CYCLES:-100}"
INTERVAL_MS="${INTERVAL_MS:-300}"
SETTLE="${SETTLE:-1.0}"
# Native presentation timing should use the release compositor, like the live
# session. The shell stays on its existing build.
PROFILE="${PROFILE:-release}"
T21_COMPOSITOR_BIN="${COMPOSITOR_BIN:-$T21_ROOT/target/$PROFILE/dragonfruit-compositor}"
export T21_COMPOSITOR_BIN

t21_capture_init "t238-dock-popover-ab-native"
mkdir -p "$OUT_DIR"
COMMIT="$(git -C "$T21_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    t21_write_open "$OUT_DIR.open.txt" \
        "T238 — native Dock popover A/B" \
        "XDG_RUNTIME_DIR is not set"
    exit 0
fi
command -v python3 >/dev/null 2>&1 || {
    echo "t238-dock-popover-ab-native: python3 not found" >&2
    exit 1
}
command -v xmessage >/dev/null 2>&1 || {
    echo "t238-dock-popover-ab-native: xmessage not found" >&2
    exit 1
}
t21_require_exec "$T21_SHELL_BIN" "run make build"
t21_require_exec "$T21_COMPOSITOR_BIN" "run make build"

PROBE_JSON="$(python3 scripts/drm-seat-probe.py 2>/dev/null || true)"
FREE_CARD="$(printf '%s' "$PROBE_JSON" \
    | python3 -c 'import json,sys
try:
    print(json.load(sys.stdin).get("free_card") or "")
except Exception:
    print("")')"
if [ -z "$FREE_CARD" ]; then
    {
        echo "T238 — native Dock popover A/B: OPEN"
        echo "======================================"
        echo
        echo "timestamp: $(date -Is)"
        echo "revision:  $COMMIT"
        echo "host:      $(uname -srm)"
        echo "reason:    no free DRM seat: the live graphical session owns DRM master"
        echo
        echo "probe:"
        printf '%s\n' "$PROBE_JSON"
        echo
        echo "rerun on a free seat / VM:"
        echo "  make t238-dock-popover-ab-native"
        echo
        echo "The nested A/B remains the work-vs-wait evidence; native vblank"
        echo "presentation timing is not claimed without this run."
    } >"$OUT_DIR.open.txt"
    echo "t238-dock-popover-ab-native: OPEN (no free DRM seat); wrote $OUT_DIR.open.txt"
    exit 0
fi

export DRAGONFRUIT_GPU_TIMING=1
export DRAGONFRUIT_FRAME_TRACE=1
export DRAGONFRUIT_REFRESH_HZ=144

run_tier() {
    local tier="$1"
    local dir="$OUT_DIR/$tier"
    mkdir -p "$dir"
    echo "t238-dock-popover-ab-native: $tier (cycles=$CYCLES)"
    t21_start_compositor drm "$dir/compositor.txt"
    # Pin the material tier before the shell maps its first chrome frame.
    t21_send_synth "set degrade-tier $tier"
    sleep 0.3
    t21_start_shell "$dir/shell.txt" \
        QT_QPA_PLATFORM=wayland \
        QML_IMPORT_PATH="$T21_ROOT/build/qml" \
        DRAGONFRUIT_CHROME_STATS=1 \
        DRAGONFRUIT_CHROME_TRACE=1 \
        DRAGONFRUIT_CHROME_GPU=1 \
        DRAGONFRUIT_CHROME_GPU_SURFACES=dock,dock-popup \
        DRAGONFRUIT_DOCK_STABLE_POPOVER=0 \
        DF_DOCK_POPOVER_AB="$CYCLES" \
        DF_DOCK_POPOVER_AB_INTERVAL_MS="$INTERVAL_MS"
    python3 scripts/t238-dock-popover-ab-driver.py run \
        --socket "$T21_SOCKET" \
        --shell-pid "$T21_SHELL_PID" \
        --compositor-pid "$T21_COMPOSITOR_PID" \
        --shell-log "$dir/shell.txt" \
        --compositor-log "$dir/compositor.txt" \
        --out "$dir/report.json" \
        --label "native-$tier" \
        --stable 0 \
        --cycles "$CYCLES" \
        --commit "$COMMIT" \
        --settle "$SETTLE"
    t21_stop_shell
    t21_stop_compositor
}

run_tier full
run_tier reduced
run_tier minimal

python3 scripts/t238-dock-popover-ab-driver.py summary \
    "$OUT_DIR/full/report.json" \
    "$OUT_DIR/reduced/report.json" \
    "$OUT_DIR/minimal/report.json" | tee "$OUT_DIR/report.md"

echo "t238-dock-popover-ab-native: wrote $OUT_DIR/report.md" >&2
