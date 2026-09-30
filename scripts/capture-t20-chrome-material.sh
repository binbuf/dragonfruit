#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-20.3 chrome-material rollout live capture.
#
# Runs the nested demo and captures every chrome surface the T-20.3 rollout
# puts the sampled blur + liquid-glass material behind, in light and dark:
#
#   * docs/captures/t20-chrome-dock-{light,dark}.png
#   * docs/captures/t20-chrome-menubar-{light,dark}.png
#   * docs/captures/t20-chrome-contextmenu-{light,dark}.png
#   * docs/captures/t20-chrome-controlcenter-{light,dark}.png
#   * docs/captures/t20-chrome-osd-{light,dark}.png
#   * docs/captures/t20-chrome-notification-{light,dark}.png
#   * docs/captures/t20-chrome-drawer-{light,dark}.png
#
# A scratch-home settingsd drives `appearance.colorScheme` (so the compositor
# material and the shell QML agree on the scheme). `DRAGONFRUIT_FRAME_TRACE=1`
# records `glass-program=compiled` in the demo log as the shader-compile
# evidence. Not part of `make e2e`: CI has no host session and no screenshot
# tool.
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
# The synthetic pointer needs the full chrome to have settled before a click
# lands on a status item; the OSD fixture fires 1.5 s after chrome creation, so
# it uses the poll path instead.
SETTLE="${SETTLE:-16}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t20-chrome.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t20-chrome}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"
# The bar's centre row (output pixels). The dropdown is opened by probing the
# right-anchored status row (see the driver's `menu` step) because its x
# shifts with the clock width.
BAR_Y=14
# The bottom Dock band, centre artwork y (T-14.7b).
DOCK_X=960
DOCK_Y=1155
# The banner sits top-right below the bar; clicking the body dismisses it.
BANNER_DISMISS_X=1730
BANNER_DISMISS_Y=80
# Control-Option-C opens the Control Center (evdev codes).
CTRL=29
ALT=56
KEY_C=46

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t20-chrome: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-t20-chrome: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle gdbus python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t20-chrome: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t20-chrome: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-t20-chrome: $DBUS_DEST already owned — stop the running settingsd" >&2
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

# The nested demo's host window is the only window and is focused by default,
# so `spectacle -a` captures it without an explicit raise. An explicit KWin
# raise is deliberately *not* done: activating the host window after the demo
# settles swallows the first synthetic click on the menu bar, so the status
# dropdown would not open.
raise_dragonfruit() {
    return 0
}

# Start the nested demo with `extra_env`, wait for the synthetic socket, then
# either settle for `settle` seconds or (when `settle` is `poll`) return at once
# so the caller can wait on a specific demo-log marker.
start_demo() {
    local settle="$1"; shift
    local extra_env=("$@")
    rm -f "$SYNTH"
    LOG="$SCRATCH/demo.log"
    : >"$LOG"
    echo "capture-t20-chrome: starting the nested demo (socket $SOCKET, ${extra_env[*]:-})"
    env "${extra_env[@]}" DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" DRAGONFRUIT_FRAME_TRACE=1 \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >>"$LOG" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-t20-chrome: synthetic socket never appeared; demo log:" >&2
        tail -40 "$LOG" >&2
        exit 1
    fi
    [ "$settle" = "poll" ] || sleep "$settle"
    if grep -q "glass-program=compiled" "$LOG"; then
        echo "capture-t20-chrome: liquid-glass shader compiled"
    else
        echo "capture-t20-chrome: WARNING — demo log has no glass-program=compiled" >&2
        grep -iE "glass|shader" "$LOG" | tail -5 >&2 || true
    fi
}

driver() {
    python3 scripts/t20-chrome-material-driver.py --synth "$SYNTH" --outdir "$OUTDIR" --steps "$1"
}

# The OSD fixture presents the brief overlay 1.5 s after chrome creation and it
# auto-dismisses after 1.4 s, so a settled demo has already missed it. Start,
# raise, then shoot as soon as `renderOsd` logs its (one-shot) commit marker.
capture_osd() {
    local scheme="$1" name="$2"
    set_scheme "$scheme"
    start_demo poll DF_OSD_FIXTURE=1
    raise_dragonfruit
    for _ in $(seq 1 400); do
        grep -q "OSD scene-graph commit path active" "$LOG" && break
        sleep 0.05
    done
    driver "shot:$name"
    stop_demo
}

echo "capture-t20-chrome: starting the scratch settingsd"
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

# --- Phase 1a: the Dock (Dock role material). -------------------------------
set_scheme light
start_demo "$SETTLE" DF_STATUS_FIXTURE=1
driver "sleep:1.0; shot:t20-chrome-dock-light"
set_scheme dark
driver "sleep:1.5; shot:t20-chrome-dock-dark"
stop_demo

# --- Phase 1b: a menu-bar dropdown (Chrome popup overlay). ------------------
set_scheme light
start_demo "$SETTLE" DF_STATUS_FIXTURE=1 DF_MENUBAR_MENU_FIXTURE=0
driver "shot:t20-chrome-menubar-light"
set_scheme dark
driver "sleep:1.5; shot:t20-chrome-menubar-dark"
stop_demo

# --- Phase 2: a Dock entry context menu (Popup). ----------------------------
set_scheme light
start_demo "$SETTLE" DF_STATUS_FIXTURE=1
raise_dragonfruit
driver "rclick:$DOCK_X:$DOCK_Y; sleep:1.0; shot:t20-chrome-contextmenu-light"
set_scheme dark
raise_dragonfruit
driver "sleep:1.5; shot:t20-chrome-contextmenu-dark"
stop_demo

# --- Phase 3: Control Center (Popup overlay panel). -------------------------
set_scheme light
start_demo "$SETTLE" DF_STATUS_FIXTURE=1 DF_NOTIFY_FIXTURE=1
raise_dragonfruit
driver "click:$BANNER_DISMISS_X:$BANNER_DISMISS_Y; sleep:0.4; chord:$CTRL,$ALT,$KEY_C; sleep:1.2; shot:t20-chrome-controlcenter-light"
set_scheme dark
raise_dragonfruit
driver "sleep:1.5; shot:t20-chrome-controlcenter-dark"
stop_demo

# --- Phase 4: the OSD (Popup overlay card). ---------------------------------
capture_osd light t20-chrome-osd-light
capture_osd dark t20-chrome-osd-dark

# --- Phase 5: a notification banner (Popup). --------------------------------
set_scheme light
start_demo "$SETTLE" DF_NOTIFY_FIXTURE=1
raise_dragonfruit
driver "sleep:1.0; shot:t20-chrome-notification-light"
set_scheme dark
raise_dragonfruit
driver "sleep:1.5; shot:t20-chrome-notification-dark"
stop_demo

# --- Phase 6: the Applications drawer (Drawer). -----------------------------
set_scheme light
start_demo "$SETTLE" DF_APPS_DRAWER_FIXTURE=1
raise_dragonfruit
driver "shot:t20-chrome-drawer-light"
set_scheme dark
raise_dragonfruit
driver "sleep:1.5; shot:t20-chrome-drawer-dark"
stop_demo

echo "capture-t20-chrome: done"
echo "  ${OUTDIR}/t20-chrome-{dock,menubar,contextmenu,controlcenter,osd,notification,drawer}-{light,dark}.png"