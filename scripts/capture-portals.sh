#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-13.7 portal track capture: a real Flatpak browser against the Dragonfruit
# xdg-desktop-portal backend.
#
# Produces the T-13 artifacts in docs/captures/:
#
#   * t13-portals.txt  the automated round-trip transcript. A private session
#                      bus runs the real `xdg-desktop-portal` frontend with the
#                      Dragonfruit backend, a host-side presenter (the shell's
#                      role), and a Python client executed *inside*
#                      `flatpak run org.mozilla.firefox`. FileChooser,
#                      Screenshot, and the ScreenCast CreateSession/
#                      SelectSources/Start flow each cross the sandbox D-Bus
#                      proxy and return a result. The frontend's advertised
#                      ScreenCast/Screenshot versions are recorded too.
#   * t13-portals.png  the nested Dragonfruit session with the shell's
#                      FileChooser picker raised by that same Flatpak client:
#                      the browser really file-chooses and the Dragonfruit
#                      picker answers. Active-window still, trimmed to the
#                      1920x1200 nested output.
#
# Requires: a host Wayland session (KWin for the raise script, or any
# compositor where the active-window capture works), `flatpak` with a browser
# (default org.mozilla.firefox), python3 with PyGObject, `spectacle`, Pillow,
# and the built tree (`make build`). Not part of `make e2e`: CI has no host
# session, Flatpak, or seat.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

OUTDIR="${OUTDIR:-docs/captures}"
APP="${DF_T99_FLATPAK_APP:-org.mozilla.firefox}"
SOCKET="${SOCKET:-dragonfruit-t99}"
SETTLE="${SETTLE:-12}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-portals.XXXXXX")}"
DRIVER="$root/scripts/flatpak-portal-driver.py"

mkdir -p "$OUTDIR"
[ -f "$DRIVER" ] || { echo "capture-portals: driver missing at $DRIVER" >&2; exit 1; }
command -v flatpak >/dev/null 2>&1 || { echo "capture-portals: flatpak not found" >&2; exit 1; }
flatpak info "$APP" >/dev/null 2>&1 || { echo "capture-portals: $APP is not installed" >&2; exit 1; }

# Keep the Qt toolchain loader path consistent with the other captures.
if [ -d "$HOME/.local/df-toolchain/usr/lib64" ]; then
    export LD_LIBRARY_PATH="$HOME/.local/df-toolchain/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

# --- private portal session ------------------------------------------------
BUS_PID=""
BACKEND_PID=""
FRONTEND_PID=""
SETTINGSD_PID=""
NOTIF_PID=""
PRESENTER_PID=""
DEMO_PGID=""
TRIGGER_PID=""

cleanup() {
    for pid in "$TRIGGER_PID" "$PRESENTER_PID" "$FRONTEND_PID" "$BACKEND_PID" "$NOTIF_PID" "$SETTINGSD_PID"; do
        [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
    done
    if [ -n "$DEMO_PGID" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    [ -n "$BUS_PID" ] && kill "$BUS_PID" 2>/dev/null || true
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

echo "capture-portals: private session bus"
dbus-daemon --session --nofork --print-address=1 >"$SCRATCH/bus.addr" 2>/dev/null &
BUS_PID=$!
for _ in $(seq 1 100); do [ -s "$SCRATCH/bus.addr" ] && break; sleep 0.1; done
export DBUS_SESSION_BUS_ADDRESS="$(cat "$SCRATCH/bus.addr")"

# A private data prefix so the real frontend discovers only our backend plus
# the system fallbacks. `--install-data` writes the .portal descriptor,
# portals.conf, and the D-Bus activation file.
./target/debug/xdg-desktop-portal-dragonfruit --install-data "$SCRATCH/prefix" >/dev/null
export XDG_DATA_DIRS="$SCRATCH/prefix/share:/usr/share"
export XDG_CURRENT_DESKTOP=dragonfruit
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"

# The shell needs its settings owner to finish startup; notifications keep the
# menu bar's Focus/DND path honest.
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

harness_dir="$SCRATCH/harness"
mkdir -p "$harness_dir"
printf 'picked\n' >"$harness_dir/picked.txt"
printf '\x89PNG\r\n\x1a\n' >"$harness_dir/shot.png"
export DF_T99_PICKED="$harness_dir/picked.txt"
export DF_T99_SHOT="$harness_dir/shot.png"

flatpak_run() {
    # The sandbox sees the driver and the harness dir; the private bus comes in
    # through DBUS_SESSION_BUS_ADDRESS, so every portal call in the sandbox
    # crosses its D-Bus proxy to the real frontend.
    flatpak run --filesystem="$root/scripts" --filesystem="$harness_dir" \
        --command=python3 "$APP" "$@"
}

# --- 1. automated Flatpak round-trips -------------------------------------
{
    echo "T-13.7 Flatpak portal walkthrough"
    echo "app: $APP"
    echo "backend: org.freedesktop.impl.portal.desktop.dragonfruit"
    echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo
    echo "== real frontend reads the standard interface versions =="
    for iface in org.freedesktop.portal.ScreenCast org.freedesktop.portal.Screenshot; do
        printf '%s.version = ' "$iface"
        gdbus call --session --dest org.freedesktop.portal.Desktop \
            --object-path /org/freedesktop/portal/desktop \
            --method org.freedesktop.DBus.Properties.Get "$iface" version
    done
    printf 'org.freedesktop.portal.ScreenCast.AvailableCursorModes = '
    gdbus call --session --dest org.freedesktop.portal.Desktop \
        --object-path /org/freedesktop/portal/desktop \
        --method org.freedesktop.DBus.Properties.Get \
        org.freedesktop.portal.ScreenCast AvailableCursorModes
    echo
    echo "== host presenter + Flatpak client =="
} >"$OUTDIR/t13-portals.txt"

PRESENTER_SECONDS=180 python3 "$DRIVER" presenter >"$SCRATCH/presenter.log" 2>&1 &
PRESENTER_PID=$!
sleep 1
set +e
flatpak_run "$DRIVER" client 2>&1 | grep -v "Can't find a11y" | tee -a "$OUTDIR/t13-portals.txt"
client_status=${PIPESTATUS[0]}
set -e
echo >>"$OUTDIR/t13-portals.txt"
echo "-- presenter --" >>"$OUTDIR/t13-portals.txt"
cat "$SCRATCH/presenter.log" >>"$OUTDIR/t13-portals.txt"
kill "$PRESENTER_PID" 2>/dev/null || true
PRESENTER_PID=""
if [ "$client_status" -ne 0 ]; then
    echo "capture-portals: Flatpak round-trips failed (see $OUTDIR/t13-portals.txt)" >&2
fi

# --- 2. the live picker still ---------------------------------------------
if command -v spectacle >/dev/null 2>&1; then
    echo "capture-portals: nested session + Flatpak-driven picker still"
    rm -f "$XDG_RUNTIME_DIR/$SOCKET"
    setsid make demo DEMO_ARGS="--socket-name $SOCKET" >"$SCRATCH/demo.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 1200); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] || { echo "capture-portals: no nested socket" >&2; tail -30 "$SCRATCH/demo.log" >&2; exit 1; }
    sleep "$SETTLE"

    # The Flatpak browser opens a file; the shell (connected to the private
    # bus) is the presenter, so the picker stays mapped while we photograph it.
    flatpak_run "$DRIVER" trigger >"$SCRATCH/trigger.log" 2>&1 &
    TRIGGER_PID=$!
    sleep 6

    # Raise the nested window so Spectacle's active-window capture targets it,
    # then trim to the 1920x1200 nested output (the T-07 pattern).
    raise_script="$SCRATCH/raise-dragonfruit.js"
    cat >"$raise_script" <<'JS'
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
    # The host compositor's scripting D-Bus is used only to raise the nested
    # window for the active-window capture; the marker exempts the desktop name.
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$raise_script" df_t99_raise >/dev/null 2>&1 || true  # df-allow-desktop-name
    gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1 || true  # df-allow-desktop-name
    sleep 1

    raw="$SCRATCH/t13-raw.png"
    env DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus" \
        spectacle -b -n -a -o "$raw" || true
    python3 - "$raw" "$OUTDIR/t13-portals.png" <<'PY'
import sys
from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
raw, dest = sys.argv[1], sys.argv[2]
im = Image.open(raw)
if im.mode == "RGBA":
    alpha = im.getchannel("A")
    minx, miny, maxx, maxy = alpha.getbbox() or (0, 0, im.width, im.height)
    if maxx > minx:
        im = im.crop((minx, miny, maxx, maxy))
if im.width >= NESTED_W and im.height >= NESTED_H:
    x0 = (im.width - NESTED_W) // 2
    y0 = im.height - NESTED_H
    im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
im.convert("RGB").save(dest)
print(f"capture-portals: saved {dest} ({im.width}x{im.height})")
PY

    kill "$TRIGGER_PID" 2>/dev/null || true
    TRIGGER_PID=""
    kill -TERM -"$DEMO_PGID" 2>/dev/null || true
    sleep 1
    kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    DEMO_PGID=""
else
    echo "capture-portals: spectacle not found — skipping the live still" >&2
fi

echo "capture-portals: wrote $OUTDIR/t13-portals.txt and $OUTDIR/t13-portals.png"
[ "$client_status" -eq 0 ]