#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T353 chrome presentation correlation run (nested).
#
# Opens the real Dock chooser and the real Menu Bar system dropdown in
# separate nested sessions, with `DRAGONFRUIT_FRAME_TRACE=1` and
# `DRAGONFRUIT_CHROME_TRACE=1`. The T238 driver joins each shell interaction
# to the compositor's accepted `chrome-commit` generation and the `present`
# line that actually carried it. A frame callback is recorded as a pacing
# event; it is never the native endpoint.
#
# The run is nested only. Native DRM scanout remains the T363 row:
#
#   make t238-native-acceptance
#
# Requires a host Wayland session and `make build`.
# Overrides: CYCLES, INTERVAL_MS, SETTLE, OUT_DIR, KEEP_SCRATCH.
set -euo pipefail

cd "$(dirname "$0")/.."

# The nested compositor links the local toolchain's libseat/libinput.
TOOLCHAIN="${DF_TOOLCHAIN:-$HOME/.local/df-toolchain/usr}"
if [ -d "$TOOLCHAIN/lib64" ]; then
    export LD_LIBRARY_PATH="$TOOLCHAIN/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

. "$(dirname "$0")/t21-capture-lib.sh"

OUT_DIR="${OUT_DIR:-docs/captures/stability2/t353-presentation}"
CYCLES="${CYCLES:-40}"
INTERVAL_MS="${INTERVAL_MS:-300}"
SETTLE="${SETTLE:-1.0}"

t21_capture_init "t353-chrome-presentation"
t21_require_host "$OUT_DIR/open.txt" "T353 chrome presentation correlation"
t21_require_exec "$T21_ROOT/build/shell/src/dragonfruit-shell" "run make build"
t21_require_exec "$T21_ROOT/target/debug/dragonfruit-compositor" "run make build"
command -v python3 >/dev/null 2>&1 || {
    echo "t353-chrome-presentation: python3 not found" >&2
    exit 1
}

mkdir -p "$OUT_DIR"
COMMIT="$(git -C "$T21_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"

export DRAGONFRUIT_FRAME_TRACE=1
export DRAGONFRUIT_REFRESH_HZ=144

run_workload() {
    local name="$1" workload="$2" surfaces="$3"
    local dir="$OUT_DIR/$name"
    mkdir -p "$dir"
    echo "t353-chrome-presentation: $name (workload=$workload, cycles=$CYCLES)"
    t21_start_compositor nested "$dir/compositor.txt"
    if [ "$workload" = "dock" ]; then
        t21_start_shell "$dir/shell.txt" \
            QT_QPA_PLATFORM=wayland \
            QML_IMPORT_PATH="$T21_ROOT/build/qml" \
            DRAGONFRUIT_CHROME_STATS=1 \
            DRAGONFRUIT_CHROME_TRACE=1 \
            DRAGONFRUIT_CHROME_GPU=1 \
            DRAGONFRUIT_CHROME_GPU_SURFACES="$surfaces" \
            DRAGONFRUIT_POPUP_TARGETS=independent \
            DF_DOCK_POPOVER_AB="$CYCLES" \
            DF_DOCK_POPOVER_AB_INTERVAL_MS="$INTERVAL_MS"
    else
        t21_start_shell "$dir/shell.txt" \
            QT_QPA_PLATFORM=wayland \
            QML_IMPORT_PATH="$T21_ROOT/build/qml" \
            DRAGONFRUIT_CHROME_STATS=1 \
            DRAGONFRUIT_CHROME_TRACE=1 \
            DRAGONFRUIT_CHROME_GPU=1 \
            DRAGONFRUIT_CHROME_GPU_SURFACES="$surfaces" \
            DRAGONFRUIT_POPUP_TARGETS=independent \
            DF_MENUBAR_MENU_AB="$CYCLES" \
            DF_MENUBAR_MENU_AB_INTERVAL_MS="$INTERVAL_MS"
    fi
    python3 scripts/t238-dock-popover-ab-driver.py run \
        --socket "$T21_SOCKET" \
        --shell-pid "$T21_SHELL_PID" \
        --compositor-pid "$T21_COMPOSITOR_PID" \
        --shell-log "$dir/shell.txt" \
        --compositor-log "$dir/compositor.txt" \
        --out "$dir/report.json" \
        --label "$name" \
        --mode independent \
        --workload "$workload" \
        --cycles "$CYCLES" \
        --commit "$COMMIT" \
        --settle "$SETTLE"
    t21_stop_shell
    t21_stop_compositor
}

REPORTS=()
run_workload dock dock "dock,dock-popup"
REPORTS+=("$OUT_DIR/dock/report.json")
run_workload menubar menubar "menubar,menubar-popup"
REPORTS+=("$OUT_DIR/menubar/report.json")

python3 scripts/t238-dock-popover-ab-driver.py summary "${REPORTS[@]}" \
    | tee "$OUT_DIR/report.md"

{
    echo
    echo "## Native-only step"
    echo
    echo "verdict: OPEN"
    echo "owner: T363 (native chrome acceptance matrix)"
    echo "command: make t238-native-acceptance"
    echo "reason: this run is nested (host swap completion, not a DRM vblank)."
    echo "The nested endpoint is the host swap; the native row needs a free DRM seat."
} >>"$OUT_DIR/report.md"

echo "t353-chrome-presentation: wrote $OUT_DIR/report.md" >&2
