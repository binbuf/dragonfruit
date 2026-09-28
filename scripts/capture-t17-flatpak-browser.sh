#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-17.1c Flatpak/browser end-to-end capture.
#
# Runs a real Flatpak browser (`org.mozilla.firefox`) against the live nested
# Dragonfruit session and its real `xdg-desktop-portal` backend, and completes
# the three portal flows in the live shell presenter by synthetic input:
#
#   * file-choose  — `FileChooser.OpenFile` raises the shell's picker; the
#                    driver clicks a row and presses Return.
#   * screenshot   — an interactive `Screenshot.Screenshot` raises the shell's
#                    selection overlay; the driver drags a region.
#   * screen-share — ScreenCast `CreateSession`/`SelectSources` raises the
#                    shell's source picker; the driver picks a source, then
#                    `Start` negotiates a stream.
#
# Every flow crosses the sandbox D-Bus proxy, the real frontend, and the
# Dragonfruit backend; each client prints its portal response and `RESULT:
# PASS`. The nested stills land in docs/captures/t17-flatpak-browser*.
#
# Requires: a host Wayland session (KWin for the active-window raise),
# `flatpak` with a browser, `/usr/libexec/xdg-desktop-portal`, `spectacle`,
# python3 (host and sandbox) with PyGObject/Pillow, and the built tree
# (`make build`). Not part of `make e2e`: CI has no host session or Flatpak.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

OUTDIR="${OUTDIR:-docs/captures}"
APP="${DF_T154_FLATPAK_APP:-org.mozilla.firefox}"
SOCKET="${SOCKET:-dragonfruit-t154}"
SETTLE="${SETTLE:-12}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t154.XXXXXX")}"
DRIVER="$root/scripts/t17-flatpak-browser-driver.py"
FOLDER="${FOLDER:-$SCRATCH/picked}"

mkdir -p "$OUTDIR" "$FOLDER"
[ -f "$DRIVER" ] || { echo "capture-t17-flatpak: driver missing at $DRIVER" >&2; exit 1; }
command -v flatpak >/dev/null 2>&1 || { echo "capture-t17-flatpak: flatpak not found" >&2; exit 1; }
flatpak info "$APP" >/dev/null 2>&1 || { echo "capture-t17-flatpak: $APP is not installed" >&2; exit 1; }
command -v spectacle >/dev/null 2>&1 || { echo "capture-t17-flatpak: spectacle not found" >&2; exit 1; }
python3 -c 'import PIL, gi' 2>/dev/null || { echo "capture-t17-flatpak: host python3 needs Pillow + PyGObject" >&2; exit 1; }
[ -n "${XDG_RUNTIME_DIR:-}" ] || { echo "capture-t17-flatpak: XDG_RUNTIME_DIR unset" >&2; exit 1; }

SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
# The host session bus (KWin + Spectacle) before we switch to the private one.
HOST_BUS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
if [ -d "$HOME/.local/df-toolchain/usr/lib64" ]; then
    export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

BUS_PID=""
BACKEND_PID=""
FRONTEND_PID=""
SETTINGSD_PID=""
NOTIF_PID=""
DEMO_PGID=""

cleanup() {
    for pid in "$FRONTEND_PID" "$BACKEND_PID" "$NOTIF_PID" "$SETTINGSD_PID"; do
        [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
    done
    if [ -n "$DEMO_PGID" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    [ -n "$BUS_PID" ] && kill "$BUS_PID" 2>/dev/null || true
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

echo "capture-t17-flatpak: private session bus"
dbus-daemon --session --nofork --print-address=1 >"$SCRATCH/bus.addr" 2>/dev/null &
BUS_PID=$!
for _ in $(seq 1 100); do [ -s "$SCRATCH/bus.addr" ] && break; sleep 0.1; done
export DBUS_SESSION_BUS_ADDRESS="$(cat "$SCRATCH/bus.addr")"

./target/debug/xdg-desktop-portal-dragonfruit --install-data "$SCRATCH/prefix" >/dev/null
export XDG_DATA_DIRS="$SCRATCH/prefix/share:/usr/share"
export XDG_CURRENT_DESKTOP=dragonfruit
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"

./target/debug/dragonfruit-settingsd >"$SCRATCH/settingsd.log" 2>&1 &
SETTINGSD_PID=$!
./target/debug/dragonfruit-notifications >"$SCRATCH/notifications.log" 2>&1 &
NOTIF_PID=$!
./target/debug/xdg-desktop-portal-dragonfruit >"$SCRATCH/backend.log" 2>&1 &
BACKEND_PID=$!
sleep 1
/usr/libexec/xdg-desktop-portal -r >"$SCRATCH/frontend.log" 2>&1 &
FRONTEND_PID=$!
for _ in $(seq 1 100); do
    gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner org.freedesktop.portal.Desktop 2>/dev/null \
        | grep -q true && break
    sleep 0.1
done

echo "capture-t17-flatpak: starting the nested demo (socket $SOCKET)"
rm -f "$SYNTH" "$XDG_RUNTIME_DIR/$SOCKET"
DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" >"$SCRATCH/demo.log" 2>&1 &
DEMO_PGID=$!
for _ in $(seq 1 1200); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
for _ in $(seq 1 1200); do [ -e "$SYNTH" ] && break; sleep 0.1; done
if [ ! -e "$SYNTH" ]; then
    echo "capture-t17-flatpak: synthetic socket never appeared; demo log:" >&2
    tail -40 "$SCRATCH/demo.log" >&2
    exit 1
fi
sleep "$SETTLE"

echo "capture-t17-flatpak: driving the Flatpak browser walkthrough"
python3 "$DRIVER" \
    --synth "$SYNTH" --outdir "$OUTDIR" --scratch "$SCRATCH" \
    --prefix t17-flatpak-browser --app "$APP" --root "$root" --folder "$FOLDER" \
    --host-bus "$HOST_BUS"

if [ -f "$SCRATCH/transcript.txt" ]; then
    cp "$SCRATCH/transcript.txt" "$OUTDIR/t17-flatpak-browser.txt"
    echo "capture-t17-flatpak: wrote $OUTDIR/t17-flatpak-browser.txt"
fi

if command -v ffmpeg >/dev/null 2>&1; then
    STILLS=(
        "$OUTDIR/t17-flatpak-browser.png"
        "$OUTDIR/t17-flatpak-browser-file-choose.png"
        "$OUTDIR/t17-flatpak-browser-file-choose-accepted.png"
        "$OUTDIR/t17-flatpak-browser-screenshot.png"
        "$OUTDIR/t17-flatpak-browser-screenshot-accepted.png"
        "$OUTDIR/t17-flatpak-browser-screen-share.png"
        "$OUTDIR/t17-flatpak-browser-screen-share-accepted.png"
    )
    LIST="$SCRATCH/concat.txt"
    : >"$LIST"
    for still in "${STILLS[@]}"; do
        [ -f "$still" ] || continue
        printf "file '%s'\nduration 1.4\n" "$(realpath "$still")" >>"$LIST"
    done
    if [ -s "$LIST" ]; then
        last=$(grep "^file " "$LIST" | tail -1 | sed "s/^file '//; s/'$//")
        printf "file '%s'\n" "$last" >>"$LIST"
        ffmpeg -y -loglevel error -f concat -safe 0 -i "$LIST" \
            -vf "scale=1280:-2,format=yuv420p" -c:v mpeg4 -crf 28 \
            -movflags +faststart "$OUTDIR/t17-flatpak-browser.mp4"
        echo "capture-t17-flatpak: wrote $OUTDIR/t17-flatpak-browser.mp4"
    fi
fi

cat "$SCRATCH/walkthrough.txt" 2>/dev/null || true
echo "capture-t17-flatpak: done (scratch $SCRATCH)"
[ "${KEEP_SCRATCH:-0}" = "1" ] && echo "capture-t17-flatpak: kept scratch $SCRATCH" || true