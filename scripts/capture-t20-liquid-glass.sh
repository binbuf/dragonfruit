#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-20.2 Tahoe liquid-glass live capture.
#
# Runs the nested demo on the current session bus and captures the two panels
# this unit proves the material on: a menu-bar status dropdown (a `Popup`) and
# the Applications drawer card (a `Drawer`). For each it captures light, dark,
# and the `Reduced` degrade tier, so the SDF edge lens, the specular inner rim,
# and the adaptive tint can be inspected against the scheme and the tier:
#
#   * docs/captures/t20-liquid-glass-menubar-{light,dark}.png
#   * docs/captures/t20-liquid-glass-drawer-{light,dark,reduced}.png
#
# A scratch-home settingsd drives `appearance.colorScheme` (so the compositor
# material and the shell QML agree on the scheme); the degrade tier is set
# through the compositor's synthetic-input harness (`set degrade-tier`).
# `DRAGONFRUIT_FRAME_TRACE=1` records `glass-program=compiled` in the demo log
# as the shader-compile evidence. Not part of `make e2e`: CI has no host session
# and no screenshot tool.
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-16}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t20-glass.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t20-glass}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"
# Wi-Fi status item: right-anchored, centre x = 1920 - 276, bar centre y = 14.
WIFI_X=1644
BAR_Y=14

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t20-liquid-glass: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-t20-liquid-glass: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle gdbus python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t20-liquid-glass: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t20-liquid-glass: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-t20-liquid-glass: $DBUS_DEST already owned — stop the running settingsd" >&2
    exit 1
fi

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
XDG_DIR="$SCRATCH/xdg"

cleanup() {
    stop_demo
    stop_settingsd
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

stop_demo() {
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
        DEMO_PGID=""
    fi
    for _ in $(seq 1 100); do [ ! -e "$SYNTH" ] && break; sleep 0.05; done
}

stop_settingsd() {
    if [ -n "${SETTINGS_PID:-}" ]; then
        kill -KILL "$SETTINGS_PID" 2>/dev/null || true
        wait "$SETTINGS_PID" 2>/dev/null || true
        SETTINGS_PID=""
    fi
}

set_key() {
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Set" "$1" "$2" >/dev/null
}

set_scheme() {
    set_key "appearance.colorScheme" "<'$1'>"
}

raise_dragonfruit() {
    local script="$SCRATCH/raise-dragonfruit.js"
    cat >"$script" <<'JS'
function raiseDragonfruit() {
    var wins = workspace.windowList();
    for (var i = 0; i < wins.length; ++i) {
        var w = wins[i];
        var cap = (w.caption || "").toString();
        var cls = (w.resourceClass || "").toString().toLowerCase();
        if (cap.indexOf("Dragonfruit") >= 0 || cls.indexOf("dragonfruit") >= 0) {
            w.minimized = false;
            if (workspace.activateWindow) { workspace.activateWindow(w); }
            else { workspace.activeWindow = w; }
        }
    }
}
raiseDragonfruit();
JS
    # df-allow-desktop-name: the host compositor's scripting D-Bus, used only
    # to raise the nested window for the active-window capture.
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$script" df_glass_capture >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1
}

start_demo() {
    local extra_env=("$@")
    rm -f "$SYNTH"
    LOG="$SCRATCH/demo.log"
    echo "capture-t20-liquid-glass: starting the nested demo (socket $SOCKET, ${extra_env[*]:-})"
    env "${extra_env[@]}" DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" DRAGONFRUIT_FRAME_TRACE=1 \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$LOG" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-t20-liquid-glass: synthetic socket never appeared; demo log:" >&2
        tail -40 "$LOG" >&2
        exit 1
    fi
    sleep "$SETTLE"
    if grep -q "glass-program=compiled" "$LOG"; then
        echo "capture-t20-liquid-glass: liquid-glass shader compiled"
    else
        echo "capture-t20-liquid-glass: WARNING — demo log has no glass-program=compiled" >&2
        grep -iE "glass|shader" "$LOG" | tail -5 >&2 || true
    fi
}

driver() {
    python3 scripts/t20-liquid-glass-driver.py --synth "$SYNTH" --outdir "$OUTDIR" --steps "$1"
}

echo "capture-t20-liquid-glass: starting the scratch settingsd"
XDG_CONFIG_HOME="$XDG_DIR" "$SETTINGS_BIN" >>"$SCRATCH/settingsd.log" 2>&1 &
SETTINGS_PID=$!
for _ in $(seq 1 200); do
    if gdbus call --session --dest org.freedesktop.DBus \
            --object-path /org/freedesktop/DBus \
            --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
        break
    fi
    sleep 0.05
done

# --- Phase 1: a menu-bar status dropdown (a Popup panel). -----------------
set_scheme light
start_demo DF_STATUS_FIXTURE=1
raise_dragonfruit
driver "move:$WIFI_X:$BAR_Y; click:$WIFI_X:$BAR_Y; sleep:1.0; shot:t20-liquid-glass-menubar-light"
set_scheme dark
raise_dragonfruit
driver "sleep:1.5; shot:t20-liquid-glass-menubar-dark"
stop_demo

# --- Phase 2: the Applications drawer card (a Drawer panel). ---------------
set_scheme light
start_demo DF_APPS_DRAWER_FIXTURE=1
raise_dragonfruit
driver "sleep:1.0; shot:t20-liquid-glass-drawer-light"
set_scheme dark
raise_dragonfruit
driver "sleep:1.5; shot:t20-liquid-glass-drawer-dark"
set_scheme light
driver "cmd:set degrade-tier reduced; sleep:1.5; shot:t20-liquid-glass-drawer-reduced"
driver "cmd:set degrade-tier full"
stop_demo

echo "capture-t20-liquid-glass: done"
echo "  ${OUTDIR}/t20-liquid-glass-menubar-{light,dark}.png"
echo "  ${OUTDIR}/t20-liquid-glass-drawer-{light,dark,reduced}.png"