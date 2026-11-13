#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 Dock popover stable-size A/B (diagnostic).
#
# Runs the same shell revision twice in the nested 144 Hz session:
#
#   baseline  DRAGONFRUIT_DOCK_STABLE_POPOVER=0  (content-sized scene: the
#             resting strip <-> the current overlay, reused within one
#             interaction session)
#   stable    DRAGONFRUIT_DOCK_STABLE_POPOVER=1  (full popover budget resident
#             for the whole session)
#
# Each configuration opens and closes the real window-chooser popover at least
# `CYCLES` times (default 100). The shell emits one `chrome trace id=...` per
# opening plus a `chrome stage stats` dump; the driver parses both and writes
# a JSON file, then prints a side-by-side markdown table.
#
# This is not a production switch. The permanent choice must also be compared
# against a dedicated content-sized popup producer (memory, sustained
# animation, and load resilience), and the Mission Control/app-launch stalls
# are tracked separately.
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
    local name="$1" stable="$2"
    local dir="$OUT_DIR/$name"
    mkdir -p "$dir"
    echo "t238-dock-popover-ab: $name (stable=$stable, cycles=$CYCLES)"
    t21_start_compositor nested "$dir/compositor.txt"
    t21_start_shell "$dir/shell.txt" \
        QT_QPA_PLATFORM=wayland \
        QML_IMPORT_PATH="$T21_ROOT/build/qml" \
        DRAGONFRUIT_CHROME_STATS=1 \
        DRAGONFRUIT_CHROME_TRACE=1 \
        DRAGONFRUIT_CHROME_GPU=1 \
        DRAGONFRUIT_CHROME_GPU_SURFACES="$CHROME_GPU_SURFACES" \
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
        --stable "$stable" \
        --cycles "$CYCLES" \
        --commit "$COMMIT" \
        --settle "$SETTLE"
    t21_stop_shell
    t21_stop_compositor
}

run_config baseline 0
run_config stable 1

python3 scripts/t238-dock-popover-ab-driver.py compare \
    "$OUT_DIR/baseline/report.json" \
    "$OUT_DIR/stable/report.json" | tee "$OUT_DIR/report.md"

echo "t238-dock-popover-ab: wrote $OUT_DIR/report.md" >&2
