#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.7w Dock icon squircle-mask capture.
#
# Runs the nested demo on the current session bus with a scratch app-index
# corpus (three apps whose themed icon files are a full-bleed square, a padded
# square, and a circle) and a scratch settingsd that pins exactly those three,
# so the Dock's real tile path renders them side by side. Writes:
#
#   * docs/captures/t14-dock-icon-mask-light.png
#   * docs/captures/t14-dock-icon-mask-dark.png
#
# Requires: a host Wayland session, `spectacle`, `gdbus`, python3 with Pillow,
# and the built tree (`make build`). Not part of `make e2e`. The three source
# icons are the shipped unit-test assets under shell/tests/data/.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-9}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t14-iconmask.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t14-iconmask}"
SETTINGS_BIN="$PWD/target/debug/dragonfruit-settingsd"
DBUS_DEST="org.dragonfruit.Settings1"
DBUS_PATH="/org/dragonfruit/Settings1"
DBUS_IFACE="org.dragonfruit.Settings1"

mkdir -p "$OUTDIR" "$SCRATCH"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-dock-icon-mask: XDG_RUNTIME_DIR unset — needs a host session" >&2
    exit 1
fi
if [ ! -x "$SETTINGS_BIN" ]; then
    echo "capture-dock-icon-mask: $SETTINGS_BIN not built — run make build" >&2
    exit 1
fi
for tool in spectacle gdbus python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-dock-icon-mask: $tool not found — needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-dock-icon-mask: python3 Pillow not found" >&2
    exit 1
}
if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$DBUS_DEST" 2>/dev/null | grep -q true; then
    echo "capture-dock-icon-mask: $DBUS_DEST is already owned — stop the running settingsd first" >&2
    exit 1
fi

ICON_DIR="$SCRATCH/icons"
APPS_DIR="$SCRATCH/xdg-data/applications"
XDG_DIR="$SCRATCH/xdg-config"
mkdir -p "$ICON_DIR" "$APPS_DIR" "$XDG_DIR"
cp shell/tests/data/icon-square.svg "$ICON_DIR/square.svg"
cp shell/tests/data/icon-padded.svg "$ICON_DIR/padded.svg"
cp shell/tests/data/icon-round.svg "$ICON_DIR/round.svg"

make_desktop() {
    local id="$1" name="$2" icon="$3"
    cat >"$APPS_DIR/$id.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=$name
Comment=Dock icon mask capture fixture
Exec=/bin/true
Icon=$ICON_DIR/$icon
NoDisplay=false
Terminal=false
EOF
}
make_desktop icon-square "Square Icon" square.svg
make_desktop icon-padded "Padded Icon" padded.svg
make_desktop icon-round "Round Icon" round.svg

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

set_key() {
    gdbus call --session --dest "$DBUS_DEST" --object-path "$DBUS_PATH" \
        --method "$DBUS_IFACE.Set" "$1" "$2" >/dev/null
}

echo "capture-dock-icon-mask: starting the scratch settingsd"
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
set_key "dock.position" "<'bottom'>"
set_key "appearance.colorScheme" "<'dark'>"
set_key "dock.pinned" "<['icon-square.desktop', 'icon-padded.desktop', 'icon-round.desktop']>" || true

echo "capture-dock-icon-mask: starting the nested demo (socket $SOCKET)"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
XDG_DATA_HOME="$SCRATCH/xdg-data" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
    >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-dock-icon-mask: synthetic socket never appeared; demo log:" >&2
    tail -40 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

# The shell seeds its default pins on the first empty settings snapshot, which
# can clobber the pre-set list; re-assert it now that the shell is live so only
# the three capture icons are pinned.
set_key "dock.pinned" "<['icon-square.desktop', 'icon-padded.desktop', 'icon-round.desktop']>" || true
sleep 2

python3 scripts/capture-dock-icon-mask-driver.py \
    --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH" \
    --demo-log "$SCRATCH/demo.log"

echo "capture-dock-icon-mask: done"