#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.2c persistent-chrome material forced-tier capture.
#
# Runs the nested demo, forces Full/Reduced/Minimal through the compositor's
# synthetic-input harness in the light and dark schemes, and commits the Dock
# and menu-bar crops:
#
#   * docs/captures/t21-material-dock-{full,reduced,minimal}-{light,dark}.png
#   * docs/captures/t21-material-menubar-{full,reduced,minimal}-{light,dark}.png
#   * docs/captures/t21-material-{dock,menubar}-tiers-{light,dark}.png (review sheets)
#   * docs/captures/t21-material-tiers.txt (the forced `query degrade` state)
#
# The shell QML Theme follows settingsd, so the script reuses the running
# settingsd and restores its `appearance.colorScheme` afterwards; when no
# settingsd owns the bus it starts a scratch one. The stills are the
# compositor's own render (the T-21.1c capture fixture), so no host screenshot
# tool is needed. Not part of `make e2e`: CI has no host session.
#
# Requires: a host Wayland session, `gdbus`, python3 with Pillow, and the built
# tree (`make build`).
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-14}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-material.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t21-material}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t21-material: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
for tool in gdbus python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t21-material: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t21-material: python3 Pillow not found" >&2
    exit 1
}

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
SHOTS="$SCRATCH/shots"
XDG_DIR="$SCRATCH/xdg"
mkdir -p "$SHOTS"

settings_owned() {
    gdbus call --session --dest org.freedesktop.DBus \
        --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true
}

read_scheme() {
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Get" "appearance.colorScheme" 2>/dev/null \
        | sed -nE "s/.*<'([^']*)'>.*/\1/p"
}

set_scheme() {
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Set" "appearance.colorScheme" "<'$1'>" >/dev/null
}

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

cleanup() {
    stop_demo
    if [ -n "${ORIGINAL_SCHEME:-}" ]; then
        set_scheme "$ORIGINAL_SCHEME" || true
    fi
    stop_settingsd
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

if settings_owned; then
    ORIGINAL_SCHEME="$(read_scheme)"
    [ -n "$ORIGINAL_SCHEME" ] || ORIGINAL_SCHEME="auto"
    echo "capture-t21-material: using the running settingsd (restoring scheme '$ORIGINAL_SCHEME')"
else
    if [ ! -x "$SETTINGS_BIN" ]; then
        echo "capture-t21-material: $SETTINGS_BIN not built — run make build" >&2
        exit 1
    fi
    echo "capture-t21-material: starting a scratch settingsd"
    XDG_CONFIG_HOME="$XDG_DIR" "$SETTINGS_BIN" >>"$SCRATCH/settingsd.log" 2>&1 &
    SETTINGS_PID=$!
    for _ in $(seq 1 200); do
        settings_owned && break
        sleep 0.05
    done
fi

rm -f "$SYNTH"
LOG="$SCRATCH/demo.log"
echo "capture-t21-material: starting the nested demo (socket $SOCKET)"
env DF_STATUS_FIXTURE=1 \
    DF_SCREENSHOT_DIR="$SHOTS" \
    DF_SCREENSHOT_CAPTURE_FIXTURE="1000:$SCRATCH/capture.png" \
    DF_SCREENSHOT_CAPTURE_INTERVAL_MS=1500 \
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-t21-material: synthetic socket never appeared; demo log:" >&2
    tail -40 "$LOG" >&2
    exit 1
fi
sleep "$SETTLE"

{
    echo "T-21.2c forced persistent-chrome material tiers"
    echo "scheme and forced tier, then the compositor's query degrade"
    echo
} >"$OUTDIR/t21-material-tiers.txt"

for scheme in light dark; do
    set_scheme "$scheme"
    sleep 1.0
    python3 scripts/t21-material-tiers-driver.py \
        --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH" \
        --shots "$SHOTS" --scheme "$scheme" --compose
done

echo "capture-t21-material: done"
echo "  ${OUTDIR}/t21-material-{dock,menubar}-{full,reduced,minimal}-{light,dark}.png"
