#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 Dock popover render-target comparison (diagnostic).
#
# Runs the same shell revision three times in the nested 144 Hz session:
#
#   shared       DRAGONFRUIT_POPUP_TARGETS=shared
#                (content-sized shared scene: the resting strip and the current
#                overlay share one render target, reused within one session)
#   stable       DRAGONFRUIT_POPUP_TARGETS=stable
#                (full popover budget resident for the whole session; the
#                original stable-size diagnostic)
#   independent  DRAGONFRUIT_POPUP_TARGETS=independent
#                (strip and popup have independently sized render targets; the
#                popup target is content-sized, grow-only for one session, and
#                warm for 750 ms after it ends)
#
# Each configuration opens and closes the real window-chooser popover at least
# `CYCLES` times (default 100). The shell emits one `chrome trace id=...` per
# opening plus a `chrome stage stats` dump; the driver parses both and writes
# a JSON file, then prints a comparison table.
#
# This is not a production switch. The Overview/app-launch stalls are
# tracked separately. Repeat at another output scale by setting
# `DRAGONFRUIT_NESTED_SCALE` and a distinct `OUT_DIR` (for example 1.5 and 2.0);
# the popup target is sized in logical content and scaled by the output scale.
#
# Requires a host Wayland session and the built tree (`make build`).
# Overrides: CYCLES, INTERVAL_MS, SETTLE, OUT_DIR, KEEP_SCRATCH.
set -euo pipefail

cd "$(dirname "$0")/.."

. "$(dirname "$0")/t21-capture-lib.sh"

OUT_DIR="${OUT_DIR:-docs/captures/t238-dock-popover-ab}"
CYCLES="${CYCLES:-100}"
INTERVAL_MS="${INTERVAL_MS:-300}"
SETTLE="${SETTLE:-1.0}"
CHROME_GPU_SURFACES="${CHROME_GPU_SURFACES:-dock,dock-popup}"

t21_capture_init "t238-dock-popover-ab"
t21_require_host "$OUT_DIR/open.txt" "T238 Dock popover stable-size A/B"
t21_require_exec "$T21_ROOT/build/shell/src/dragonfruit-shell" "run make build"
t21_require_exec "$T21_ROOT/target/debug/dragonfruit-compositor" "run make build"
command -v python3 >/dev/null 2>&1 || {
    echo "t238-dock-popover-ab: python3 not found" >&2
    exit 1
}
command -v xmessage >/dev/null 2>&1 || {
    echo "t238-dock-popover-ab: xmessage not found; a grouped X11 entry is required" >&2
    exit 1
}

mkdir -p "$OUT_DIR"
COMMIT="$(git -C "$T21_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"

# Keep the compositor configuration identical for both runs. GPU timer queries
# are asynchronous and never stall the pipeline; the frame trace gives the
# per-frame present timestamps for correlation.
export DRAGONFRUIT_GPU_TIMING=1
export DRAGONFRUIT_FRAME_TRACE=1
export DRAGONFRUIT_REFRESH_HZ=144

run_config() {
    local name="$1" mode="$2" stable="$3"
    local dir="$OUT_DIR/$name"
    mkdir -p "$dir"
    echo "t238-dock-popover-ab: $name (mode=$mode, cycles=$CYCLES)"
    t21_start_compositor nested "$dir/compositor.txt"
    t21_start_shell "$dir/shell.txt" \
        QT_QPA_PLATFORM=wayland \
        QML_IMPORT_PATH="$T21_ROOT/build/qml" \
        DRAGONFRUIT_CHROME_STATS=1 \
        DRAGONFRUIT_CHROME_TRACE=1 \
        DRAGONFRUIT_CHROME_GPU=1 \
        DRAGONFRUIT_CHROME_GPU_SURFACES="$CHROME_GPU_SURFACES" \
        DRAGONFRUIT_POPUP_TARGETS="$mode" \
        DRAGONFRUIT_DOCK_STABLE_POPOVER="$stable" \
        DF_DOCK_POPOVER_AB="$CYCLES" \
        DF_DOCK_POPOVER_AB_INTERVAL_MS="$INTERVAL_MS"
    python3 scripts/t238-dock-popover-ab-driver.py run \
        --socket "$T21_SOCKET" \
        --shell-pid "$T21_SHELL_PID" \
        --compositor-pid "$T21_COMPOSITOR_PID" \
        --shell-log "$dir/shell.txt" \
        --compositor-log "$dir/compositor.txt" \
        --out "$dir/report.json" \
        --label "$name" \
        --mode "$mode" \
        --stable "$stable" \
        --cycles "$CYCLES" \
        --commit "$COMMIT" \
        --settle "$SETTLE"
    t21_stop_shell
    t21_stop_compositor
}

run_config shared shared 0
run_config stable stable 1
run_config independent independent 0

python3 scripts/t238-dock-popover-ab-driver.py summary \
    "$OUT_DIR/shared/report.json" \
    "$OUT_DIR/stable/report.json" \
    "$OUT_DIR/independent/report.json" | tee "$OUT_DIR/report.md"

echo "t238-dock-popover-ab: wrote $OUT_DIR/report.md" >&2
