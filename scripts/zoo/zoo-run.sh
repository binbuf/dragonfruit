#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-14.6a strange-app zoo run.
#
# Launches the nested Dragonfruit compositor with the synthetic-input harness,
# starts the strange-app zoo against its private socket, and records each
# app's raw identity, decoration tier, resolved desktop id, and global-menu
# tier into docs/captures/t14-zoo-matrix.{md,json}. The zoo is:
#
#   Firefox (X11) · xterm · a Steam stand-in · a GTK4 app (GNOME Calculator
#   via flatpak) · an SDL2 game sample · an Electron app
#
# Optional apps that cannot be installed are skipped and recorded as
# "not run". Requires a host Wayland session, `spectacle`, python3+Pillow,
# `dbus`/`dnf` for the xterm extraction, an Electron install (see
# ZOO_ELECTRON_DIR), and the built tree (`make build`).
set -uo pipefail

cd "$(dirname "$0")/../.."
ROOT=$PWD

OUTDIR="${OUTDIR:-docs/captures}"
PREFIX="${PREFIX:-t14-zoo}"
WAIT="${WAIT:-120}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-zoo.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"

COMPOSITOR="${DF_COMPOSITOR_BIN:-$ROOT/target/debug/dragonfruit-compositor}"
APP_INDEX="${DF_APP_INDEX_BIN:-$ROOT/target/debug/dragonfruit-app-index}"
MENU_BROKER="${DF_MENU_BROKER_BIN:-$ROOT/target/debug/dragonfruit-menu-broker}"
ELECTRON_DIR="${ZOO_ELECTRON_DIR:-/tmp/opencode/zoo-electron}"

mkdir -p "$SCRATCH"/{share/applications,logs,bin,ffprofile}

cleanup() {
    for pgid in ${PGIDS:-}; do
        kill -TERM -"$pgid" 2>/dev/null || true
    done
    sleep 1
    for pgid in ${PGIDS:-}; do
        kill -KILL -"$pgid" 2>/dev/null || true
    done
    rm -f "$SYNTH" "$XDG_RUNTIME_DIR/$SOCK" "$XDG_RUNTIME_DIR/$SOCK.lock" \
        "$XDG_RUNTIME_DIR/$SOCK.launch-token" "$XDG_RUNTIME_DIR/$SOCK.x11-display" 2>/dev/null
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

PGIDS=""

die() {
    echo "zoo: $*" >&2
    exit 1
}

for tool in python3 spectacle xprop; do
    command -v "$tool" >/dev/null 2>&1 || die "$tool not found"
done
python3 -c 'import PIL' 2>/dev/null || die "python3 Pillow not found"
[ -x "$COMPOSITOR" ] || die "compositor not built ($COMPOSITOR) — run make build"
[ -x "$APP_INDEX" ] || die "app-index not built ($APP_INDEX)"
[ -x "$MENU_BROKER" ] || die "menu-broker not built ($MENU_BROKER)"

# --- sample zoo clients -----------------------------------------------------
if command -v gcc >/dev/null 2>&1 && pkg-config --exists sdl2; then
    gcc -o "$SCRATCH/bin/sdl_zoo" scripts/zoo/sdl_zoo.c \
        $(pkg-config --cflags --libs sdl2) || die "failed to build sdl_zoo"
else
    echo "zoo: SDL2 development files missing; SDL game will be skipped"
fi
if command -v gcc >/dev/null 2>&1 && pkg-config --exists x11; then
    gcc -o "$SCRATCH/bin/x11_zoo" scripts/zoo/x11_zoo.c \
        $(pkg-config --cflags --libs x11) || die "failed to build x11_zoo"
else
    echo "zoo: X11 development files missing; the Steam stand-in will be skipped"
fi

# --- xterm (system, or extracted from the distro package) -------------------
XTERM_BIN=""
XTERM_SHARE=""
if command -v xterm >/dev/null 2>&1; then
    XTERM_BIN=$(command -v xterm)
    XTERM_SHARE=$(dirname "$(dirname "$XTERM_BIN")")/share
else
    echo "zoo: xterm not installed; trying the distro package (dnf download)"
    if (cd "$SCRATCH" && dnf download xterm >/dev/null 2>&1); then
        rpm=$(ls "$SCRATCH"/xterm-*.rpm 2>/dev/null | head -1)
        if [ -n "$rpm" ]; then
            mkdir -p "$SCRATCH/xterm-root"
            rpm2cpio "$rpm" | (cd "$SCRATCH/xterm-root" && cpio -idm --quiet) 2>/dev/null
            XTERM_BIN="$SCRATCH/xterm-root/usr/bin/xterm"
            XTERM_SHARE="$SCRATCH/xterm-root/usr/share"
        fi
    fi
fi
if [ -n "$XTERM_BIN" ] && [ -x "$XTERM_BIN" ]; then
    [ -n "$XTERM_SHARE" ] && cp "$XTERM_SHARE/applications/xterm.desktop" \
        "$SCRATCH/share/applications/" 2>/dev/null
else
    XTERM_BIN=""
    echo "zoo: xterm unavailable; skipping"
fi

# --- desktop entries for the apps with no installed entry -------------------
cp scripts/zoo/steam.desktop "$SCRATCH/share/applications/steam.desktop"

ELECTRON_BIN=""
for candidate in "$ELECTRON_DIR/node_modules/electron/dist/electron" \
                 "$(command -v electron 2>/dev/null || true)"; do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then
        ELECTRON_BIN="$candidate"
        break
    fi
done
if [ -n "$ELECTRON_BIN" ]; then
    cat > "$SCRATCH/share/applications/electron-zoo.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Electron Zoo
Exec=$ELECTRON_BIN
Icon=electron
Terminal=false
Categories=Utility;
EOF
fi

if [ -x "$SCRATCH/bin/sdl_zoo" ]; then
    cat > "$SCRATCH/share/applications/game.zoo.sdl.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=SDL Zoo Game
Exec=$SCRATCH/bin/sdl_zoo
Icon=applications-games
Terminal=false
Categories=Game;
StartupWMClass=game.zoo.sdl
EOF
fi

HAS_FIREFOX=0
command -v firefox >/dev/null 2>&1 && HAS_FIREFOX=1
HAS_CALCULATOR=0
flatpak info org.gnome.Calculator >/dev/null 2>&1 && HAS_CALCULATOR=1

SKIP_ARGS=()
[ "$HAS_FIREFOX" = "1" ] || SKIP_ARGS+=(--skip firefox)
[ -n "$XTERM_BIN" ] || SKIP_ARGS+=(--skip xterm)
[ -x "$SCRATCH/bin/x11_zoo" ] || SKIP_ARGS+=(--skip steam)
[ "$HAS_CALCULATOR" = "1" ] || SKIP_ARGS+=(--skip calculator)
[ -x "$SCRATCH/bin/sdl_zoo" ] || SKIP_ARGS+=(--skip sdl)
[ -n "$ELECTRON_BIN" ] || SKIP_ARGS+=(--skip electron)
if [ -n "${SKIP_ARGS[*]:-}" ]; then
    echo "zoo: skipping ${SKIP_ARGS[*]}"
fi

# The app-index the driver calls must see the zoo's .desktop corpus.
export XDG_DATA_DIRS="$SCRATCH/share${XTERM_SHARE:+:$XTERM_SHARE}:/usr/local/share:/usr/share"

# --- session -----------------------------------------------------------------
SOCK="dragonfruit-zoo-$$"
SYNTH="$XDG_RUNTIME_DIR/$SOCK.synth"
rm -f "$SYNTH"

echo "zoo: starting nested compositor (socket $SOCK)"
setsid env DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" "$COMPOSITOR" \
    --backend nested --socket-name "$SOCK" >"$SCRATCH/logs/compositor.log" 2>&1 &
PGIDS="$PGIDS $!"
for _ in $(seq 1 200); do
    [ -e "$XDG_RUNTIME_DIR/$SOCK" ] && [ -e "$SYNTH" ] && break
    sleep 0.1
done
for _ in $(seq 1 100); do
    [ -s "$XDG_RUNTIME_DIR/$SOCK.x11-display" ] && break
    sleep 0.1
done
XDISPLAY=$(cat "$XDG_RUNTIME_DIR/$SOCK.x11-display" 2>/dev/null || true)
[ -e "$SYNTH" ] || die "synthetic-input socket never appeared (see $SCRATCH/logs/compositor.log)"
# Wait until Xwayland accepts connections before launching X11 clients.
if [ -n "$XDISPLAY" ]; then
    for _ in $(seq 1 150); do
        DISPLAY="$XDISPLAY" xprop -root >/dev/null 2>&1 && break
        sleep 0.1
    done
fi
echo "zoo: private socket $SOCK, Xwayland DISPLAY=$XDISPLAY"

launch() {
    # launch <log-name> <env...> -- <cmd...>
    local name=$1; shift
    setsid "$@" >"$SCRATCH/logs/$name.log" 2>&1 &
    PGIDS="$PGIDS $!"
}

if [ "$HAS_FIREFOX" = "1" ]; then
    mkdir -p "$SCRATCH/ffprofile"
    # A seeded profile skips the first-run/default-browser/telemetry round
    # trips that otherwise delay a cold Firefox's first window by a minute.
    cat > "$SCRATCH/ffprofile/user.js" <<'FFPREFS'
user_pref("browser.shell.checkDefaultBrowser", false);
user_pref("browser.aboutwelcome.enabled", false);
user_pref("browser.startup.page", 0);
user_pref("datareporting.policy.dataSubmissionEnabled", false);
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("browser.safebrowsing.malware.enabled", false);
user_pref("browser.safebrowsing.phishing.enabled", false);
user_pref("network.captive-portal-service.enabled", false);
user_pref("toolkit.telemetry.enabled", false);
user_pref("browser.discovery.enabled", false);
user_pref("extensions.getAddons.showPane", false);
FFPREFS
    # An inherited host WAYLAND_DISPLAY makes Firefox probe the host compositor
    # first; unset it and pin GDK to X11 so it uses the nested Xwayland.
    launch firefox env -u WAYLAND_DISPLAY DISPLAY="$XDISPLAY" \
        GDK_BACKEND=x11 MOZ_ENABLE_WAYLAND=0 MOZ_DISABLE_CONTENT_SANDBOX=1 \
        firefox --no-remote --new-instance --profile "$SCRATCH/ffprofile" about:blank
fi
if [ -n "$XTERM_BIN" ]; then
    launch xterm env DISPLAY="$XDISPLAY" "$XTERM_BIN" \
        -geometry 70x14+20+40 -T "Zoo xterm" -e sleep 600
fi
if [ -x "$SCRATCH/bin/x11_zoo" ]; then
    launch steam env DISPLAY="$XDISPLAY" "$SCRATCH/bin/x11_zoo" \
        --title Steam --instance steam --class Steam --lifetime-ms 600000
fi
if [ -x "$SCRATCH/bin/sdl_zoo" ]; then
    # SDL runs on the nested Wayland socket. The sample presents a frame each
    # loop (T-14.6b); SDL's Wayland backend only attaches its first buffer
    # when the app draws, and the compositor only maps a toplevel once it has
    # a buffer. The T-14.6a run fell back to SDL's X11 driver because the
    # sample never drew.
    launch sdl env WAYLAND_DISPLAY="$SOCK" SDL_VIDEODRIVER=wayland \
        SDL_APP_ID=game.zoo.sdl "$SCRATCH/bin/sdl_zoo" --lifetime-ms 600000
fi
if [ -n "$ELECTRON_BIN" ]; then
    launch electron env WAYLAND_DISPLAY="$SOCK" "$ELECTRON_BIN" \
        --no-sandbox --ozone-platform=wayland --class=electron \
        "$ROOT/scripts/zoo/electron/main.js"
fi
if [ "$HAS_CALCULATOR" = "1" ]; then
    # --filesystem exposes the private socket inside the sandbox; --socket
    # alone only exposes the host's default wayland socket.
    launch calculator flatpak run --user --socket=wayland \
        --filesystem="$XDG_RUNTIME_DIR/$SOCK" \
        --env=WAYLAND_DISPLAY="$SOCK" --env=XDG_CURRENT_DESKTOP=Dragonfruit \
        org.gnome.Calculator
fi

echo "zoo: driving the matrix"
python3 scripts/zoo/zoo-driver.py \
    --synth "$SYNTH" --xdisplay "$XDISPLAY" \
    --outdir "$OUTDIR" --scratch "$SCRATCH" \
    --app-index "$APP_INDEX" --menu-broker "$MENU_BROKER" \
    --prefix "$PREFIX" --wait "$WAIT" "${SKIP_ARGS[@]}"
rc=$?

echo "zoo: done (rc=$rc, scratch $SCRATCH)"
exit $rc