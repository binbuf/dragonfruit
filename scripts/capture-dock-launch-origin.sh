#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7l Dock launch-origin tile hand-off capture.
#
# Runs the nested demo with a scratch settingsd (Settings pinned, bottom Dock,
# dark), drives the Dock launch through the real click tree using the
# shell's `DF_DOCK_ACTIVATION_FIXTURE=launch` seam, and checks the compositor's
# `query motion` record: the launched window's appear must originate at the
# pinned entry's icon (the bottom Dock band), not the centered fallback. The
# identical fixture path the previous capture used now also publishes the
# entry tile, so this exercises the whole QML -> shell -> protocol hand-off.
#
# Stills:
#
#   * docs/captures/t14-dock-launch-origin.png        (annotated appear)
#   * docs/captures/t14-dock-launch-origin-trace.txt  (recorded origin/target)
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-8}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-launch-origin.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-launch-origin}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"
SETTINGS_APP="org.dragonfruit.Settings.desktop"

mkdir -p "$OUTDIR" "$SCRATCH" "$SCRATCH/xdg"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-launch-origin: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-launch-origin: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-launch-origin: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-launch-origin: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-launch-origin: $DBUS_DEST already owned — stop the running settingsd" >&2
    exit 1
fi

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
rm -f "$SYNTH"

cleanup() {
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    if [ -n "${SETTINGS_PID:-}" ]; then
        kill -KILL "$SETTINGS_PID" 2>/dev/null || true
        wait "$SETTINGS_PID" 2>/dev/null || true
    fi
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

echo "capture-dock-launch-origin: starting the scratch settingsd"
XDG_CONFIG_HOME="$SCRATCH/xdg" "$SETTINGS_BIN" >>"$SCRATCH/settingsd.log" 2>&1 &
SETTINGS_PID=$!
for _ in $(seq 1 200); do
    if gdbus call --session --dest org.freedesktop.DBus \
            --object-path /org/freedesktop/DBus \
            --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
        break
    fi
    sleep 0.05
done
gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
    --method "$DBUS_IFACE.Set" "dock.pinned" "<['$SETTINGS_APP']>" >/dev/null
gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
    --method "$DBUS_IFACE.Set" "dock.position" "<'bottom'>" >/dev/null
gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
    --method "$DBUS_IFACE.Set" "appearance.colorScheme" "<'dark'>" >/dev/null

echo "capture-dock-launch-origin: starting the nested demo ($SOCKET)"
env DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" DF_DEMO_QT_APP=/bin/true \
    DF_DOCK_ACTIVATION_FIXTURE=launch \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-dock-launch-origin: synthetic socket never appeared" >&2
    tail -40 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

echo "capture-dock-launch-origin: checking the recorded appear origin"
python3 scripts/capture-dock-launch-origin-driver.py \
    --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH"

echo "capture-dock-launch-origin: done"