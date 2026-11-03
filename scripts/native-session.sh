#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Native development session helper.
#
# This is the turnkey wrapper around the T-12.6b display-manager round trip.
# It installs the session entry, user units, app metadata, and portal data,
# links the built binaries into place, arms the next login for Dragonfruit,
# and asks the host session manager to log out. From inside Dragonfruit the
# same script can rebuild/restart units without a logout, or restore the
# previous desktop and log out.
#
# The link farm points at target/<profile> and build[-release]/, so a normal
# `make build` (or the release build) is enough before a restart: no reinstall
# step is needed.
#
# Usage:
#   scripts/native-session.sh [start] [--no-build] [--no-logout] [--no-deps] [--autologin] [--force] [--skip-preflight]
#   scripts/native-session.sh return [--restart-dm|--no-restart-dm] [--force]
#   scripts/native-session.sh recover [--restart-dm|--no-restart-dm] [--force]
#   scripts/native-session.sh ready
#   scripts/native-session.sh restart [--no-build] [--no-deps] [--force] [UNIT ...]
#   scripts/native-session.sh status
#   scripts/native-session.sh capture
#   scripts/native-session.sh logs [-f] [UNIT]
#   scripts/native-session.sh install [--force] [--no-deps]
#   scripts/native-session.sh deps
#
# `return` and `recover` are the same safety path. They work from inside the
# Dragonfruit session, from another desktop's terminal, and from a plain text
# console (Ctrl+Alt+F3): they remove the dev preselect, restore the saved
# display-manager files, clear the round-trip state, stop the failed session,
# and — when run from a text console — restart the login manager so the host
# desktop is selectable again.
#
# UNIT is a shorthand (`shell`, `settings`, `notifications`, `wallpaper`,
# `apps`, `menu`, `portal`, `services`, `all`) or a systemd user unit name.
# `all` deliberately excludes the compositor: it is the session anchor, so
# restarting it ends the session. Rebuild and use `return` + `start` for
# compositor changes (or use the nested `make dev` loop).
#
# DF_NATIVE_PREFIX overrides the install prefix (default /usr; useful for
# staging tests, not for a real display-manager login).
# DF_NATIVE_PROFILE=debug|release selects the build. The default is **release**:
# the native session is meant to be dogfooded as a real desktop, so it uses the
# optimized build unless the developer explicitly asks for debug with `--debug`
# (or DF_NATIVE_PROFILE=debug). `--release` is the explicit inverse. The CLI
# flag wins over the environment variable; the environment wins over the
# default. A debug build is still available for fast iteration.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

PREFIX="${DF_NATIVE_PREFIX:-/usr}"
PREFIX="${PREFIX%/}"
# The Makefile uses these paths too; default them so the script also works when
# invoked directly (make variables are exported by the native targets).
DF_TOOLCHAIN="${DF_TOOLCHAIN:-$HOME/.local/df-toolchain/usr}"
DF_DEVROOT="${DF_DEVROOT:-$HOME/.local/df-devroot/lib64}"

# Build profile for the native session. `release` is the default because the
# native session is the dogfooding/deployment rail: the 4K material/damage path
# is CPU-bound enough that the unoptimized build caps animated clients well
# below 60 fps and pushes the material degrade ladder down. `debug` is the fast
# iteration build, opt-in with `--debug` (or DF_NATIVE_PROFILE=debug).
#
# Precedence: CLI `--debug`/`--release` > DF_NATIVE_PROFILE > `release`.
# Strip the global profile flags before the command parsers see the arguments.
PROFILE_FLAG=""
for arg in "$@"; do
    case "$arg" in
        --debug) PROFILE_FLAG=debug ;;
        --release) PROFILE_FLAG=release ;;
    esac
done
if [ -n "$PROFILE_FLAG" ]; then
    args=()
    for arg in "$@"; do
        case "$arg" in
            --debug|--release) ;;
            *) args+=("$arg") ;;
        esac
    done
    set -- "${args[@]}"
fi
PROFILE="${PROFILE_FLAG:-${DF_NATIVE_PROFILE:-release}}"
case "$PROFILE" in
    debug) QT_BUILD_DIR="$root/build" ;;
    release) QT_BUILD_DIR="$root/build-release" ;;
    *) echo "native-session: DF_NATIVE_PROFILE must be debug or release" >&2; exit 2 ;;
esac
DEV_BIN="$root/target/$PROFILE/dragonfruit"
SESSION_BIN="$root/target/$PROFILE/dragonfruit-session"
PORTAL_BIN="$root/target/$PROFILE/xdg-desktop-portal-dragonfruit"

BIN_NAMES=(
    dragonfruit
    dragonfruit-session
    dragonfruit-compositor
    dragonfruit-shell
    dragonfruit-settingsd
    dragonfruit-menu-broker
    dragonfruit-app-index
    dragonfruit-notifications
    dragonfruit-wallpaperd
    xdg-desktop-portal-dragonfruit
    dragonfruit-pam-helper
    dragonfruit-files
    dragonfruit-settings
)

NATIVE_ENV_FILE="${DF_NATIVE_ENV_FILE:-$HOME/.config/dragonfruit/native.env}"

# Dev-only diagnostics. The unit and target drop-in are per-user, installed by
# `make native` and removed by `native-recover`/`native-return`; the capture
# reports themselves stay under the user state directory for later analysis.
CAPTURE_UNIT="dragonfruit-native-capture.service"
CAPTURE_TARGET_DROPIN="$HOME/.config/systemd/user/dragonfruit-session.target.d/20-dragonfruit-native-capture.conf"
CAPTURE_STATE_DIR="${DF_CAPTURE_ROOT:-${XDG_STATE_HOME:-$HOME/.local/state}/dragonfruit/native-captures}"

# The units that need the dev-build loader path. The Qt shell pulls in the
# toolchain Qt; the Qt apps it launches inherit this environment. The
# compositor pulls in the user-space DRM sysroot and the local libxkbcommon.
# Keep the path off the Rust session services so it cannot shadow their libs.
ENV_UNITS=(
    dragonfruit-compositor.service
    dragonfruit-shell.service
)

NO_BUILD=0
NO_LOGOUT=0
NO_DEPS=0
FORCE=0
AUTOLOGIN=0
SKIP_PREFLIGHT=0
STAGE=""

cleanup() {
    if [ -n "$STAGE" ]; then
        rm -rf "$STAGE"
    fi
}
trap cleanup EXIT

say() {
    printf 'native-session: %s\n' "$*"
}

die() {
    printf 'native-session: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Native development session helper.

Usage:
  scripts/native-session.sh [--debug|--release] [start] [--no-build] [--no-logout] [--no-deps] [--autologin] [--force] [--skip-preflight]
  scripts/native-session.sh [--debug|--release] return [--restart-dm|--no-restart-dm] [--force]
  scripts/native-session.sh [--debug|--release] recover [--restart-dm|--no-restart-dm] [--force]
  scripts/native-session.sh [--debug|--release] ready
  scripts/native-session.sh [--debug|--release] rebuild [--reboot] [--no-build] [--no-deps] [--no-install] [--no-logout] [--force]
  scripts/native-session.sh [--debug|--release] restart [--no-build] [--no-deps] [--force] [UNIT ...]
  scripts/native-session.sh [--debug|--release] status
  scripts/native-session.sh [--debug|--release] capture
  scripts/native-session.sh [--debug|--release] logs [-f] [UNIT]
  scripts/native-session.sh [--debug|--release] install [--force] [--no-deps]
  scripts/native-session.sh [--debug|--release] deps

Profile:
  release (default) — optimized daily-driver build; use this for dogfooding
                      and for any native session you intend to judge visually.
  --debug           — fast iteration build (build/, target/debug); also
                      selectable with DF_NATIVE_PROFILE=debug.
  --release         — explicit inverse; the default.
  CLI flags win over DF_NATIVE_PROFILE; the environment wins over the default.

UNIT is a shorthand (shell, settings, notifications, wallpaper, apps, menu,
portal, services, all) or a systemd user unit name. `all` deliberately
excludes the compositor: it is the session anchor, so restarting it ends the
session. Rebuild and use `return` + `start` for compositor changes (or use
the nested `make dev` loop).

Running the script with no command is `start`: it checks/installs the
distribution build dependencies, builds the tree (release by default), installs
the session entry, user units, app metadata, and portal data, links the built
binaries into place, then arms the next login for Dragonfruit and asks the host
session manager to log out. The installer uses sudo only for the system copy.
Rebuilds are picked up through the symlinks, so `restart` needs no reinstall
when the selected profile matches the installed one. It refuses a mismatch
because it cannot replace the running compositor; use `rebuild`/`reboot` to
switch profiles. The newer SDDM-derived login manager is preselected through a
conf.d drop-in; `return`/`recover` remove it again.
`--no-logout` stops after install/preselect, so you can log out by hand.
`--no-deps` skips the distribution package bootstrap; `deps` prints the
detected package command.

`rebuild` is the full-session cycle for when a change needs a fresh session
(the compositor, or you just want every app restarted): it rebuilds, refreshes
the install links/units (skip with `--no-install`), keeps Dragonfruit
preselected, then ends the current login session so the display manager comes
back with the new binaries. `--reboot` reboots the machine instead of only
logging out.

Before arming the login, `start` writes the per-user runtime environment the
dev build needs (the Qt toolchain and user-space sysroot library paths) and
checks every installed binary against it. If a binary cannot load at a
display-manager login, the script stops before the preselect/logout instead
of leaving a session that cannot start. `--skip-preflight` bypasses that gate.

If the Dragonfruit login does not come up, switch to a text console
(Ctrl+Alt+F3) and run `bash scripts/native-session.sh recover` (or
`make native-recover`). It works without a running Dragonfruit session, so it
is the recovery path when the greeter cannot start it.

DF_NATIVE_PREFIX overrides the install prefix (default /usr; useful for
staging tests, not for a real display-manager login).
EOF
}

require_user() {
    [ "$(id -u)" -ne 0 ] || die "run this as your desktop user, not root (the script uses sudo only for installation)"
}

binary_source() {
    case "$1" in
        dragonfruit-shell) printf '%s\n' "$QT_BUILD_DIR/shell/src/dragonfruit-shell" ;;
        dragonfruit-files) printf '%s\n' "$QT_BUILD_DIR/apps/files/dragonfruit-files" ;;
        dragonfruit-settings) printf '%s\n' "$QT_BUILD_DIR/apps/settings/dragonfruit-settings" ;;
        *) printf '%s\n' "$root/target/$PROFILE/$1" ;;
    esac
}

# Which profile the installed link farm currently points at. `status` reports
# it; `restart` uses it to refuse a profile mismatch (restart cannot replace
# the running compositor).
installed_profile() {
    local installed
    installed="$(readlink -f "$PREFIX/bin/dragonfruit-compositor" 2>/dev/null || true)"
    case "$installed" in
        */target/release/*) printf 'release\n' ;;
        */target/debug/*) printf 'debug\n' ;;
        *) printf 'unknown\n' ;;
    esac
}

require_built() {
    local name src
    for name in "${BIN_NAMES[@]}"; do
        src="$(binary_source "$name")"
        [ -x "$src" ] || die "missing $src — run 'make build' first"
    done
    [ -x "$SESSION_BIN" ] || die "missing $SESSION_BIN — run 'make build' first"
    [ -x "$PORTAL_BIN" ] || die "missing $PORTAL_BIN — run 'make build' first"
    # The first-party apps load QML modules from the build tree (the dev
    # installer symlinks binaries but does not install QML). Check the corpus
    # before arming a login: without it Settings and Files start and exit
    # before mapping a window, which looks like "the Dock does nothing".
    local module
    for module in Dragonfruit Dragonfruit/Settings Dragonfruit/Files; do
        [ -f "$QT_BUILD_DIR/qml/$module/qmldir" ] \
            || die "missing $QT_BUILD_DIR/qml/$module/qmldir — build the $PROFILE tree first"
    done
}

# --- the login runtime environment -----------------------------------------
#
# The dev tree links the Qt shell/apps against the pinned toolchain at
# $DF_TOOLCHAIN/usr and the DRM stack against the user-space sysroot. A shell
# started by `make` inherits the Makefile's LD_LIBRARY_PATH, but a
# display-manager login does not: the session entry is the only process the
# greeter starts, and the systemd user units inherit the *user manager's*
# environment. Without these paths the Qt shell exits at startup with
# "error while loading shared libraries" and the login shows a black desktop.
#
# `start` therefore writes the paths to a per-user file; the shipped session
# entry sources it (when present) and imports LD_LIBRARY_PATH into the user
# manager. `recover` removes it again.

session_library_path() {
    local dir out="" first=1
    local -a dirs=(
        "${DF_TOOLCHAIN}/lib64"
        "${DF_TOOLCHAIN}/lib"
        "${DF_DEVROOT}"
        "${HOME}/.local/df-devroot/lib64"
        "${HOME}/.local/lib"
    )
    if [ "$PREFIX" != "/usr" ]; then
        dirs+=("${PREFIX}/lib64" "${PREFIX}/lib")
    fi
    if [ -n "${LD_LIBRARY_PATH:-}" ]; then
        local -a inherited=()
        local old_ifs="$IFS"
        IFS=':'
        read -r -a inherited <<<"$LD_LIBRARY_PATH"
        IFS="$old_ifs"
        dirs+=("${inherited[@]}")
    fi
    for dir in "${dirs[@]}"; do
        [ -n "$dir" ] || continue
        [ -d "$dir" ] || continue
        case ":$out:" in
            *":$dir:"*) continue ;;
        esac
        if [ "$first" -eq 1 ]; then
            out="$dir"
            first=0
        else
            out="$out:$dir"
        fi
    done
    printf '%s\n' "$out"
}

# Quote one value for the POSIX-sh environment file the session entry sources.
env_file_value() {
    printf '%s' "$1" | sed 's/[\\"$`]/\\&/g'
}

# Quote one value for a systemd unit `Environment=` line.
systemd_env_value() {
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//%/%%}"
    printf '%s' "$value"
}

write_session_environment() {
    local libs qml_dir cursor_dir unit dropin_dir tmp
    local -a env_names=() env_values=()
    libs="$(session_library_path)"
    qml_dir="$QT_BUILD_DIR/qml"
    cursor_dir="$root/assets/cursors"

    if [ -n "$libs" ]; then
        env_names+=(LD_LIBRARY_PATH)
        env_values+=("$libs")
    fi
    # The dev tree builds its QML modules under build/qml and does not install
    # them (the `.desktop` entries are installed, the binaries are symlinks).
    # A shell started by systemd has no `make`/dev-tool environment, so a
    # launched first-party app cannot resolve `Dragonfruit.Settings` and exits
    # before mapping a window. Export the built module root on the shell unit;
    # every app it launches inherits it.
    if [ -d "$qml_dir" ]; then
        env_names+=(QML_IMPORT_PATH QML2_IMPORT_PATH)
        env_values+=("$qml_dir" "$qml_dir")
    fi
    # Let first-party clients resolve the same cursor theme the compositor
    # draws, even before the theme is installed system-wide. Keep the standard
    # roots (and any inherited XCURSOR_PATH) so a shape this theme lacks still
    # falls back instead of losing the pointer.
    if [ -d "$cursor_dir/capitaine-cursors" ]; then
        local cursor_search
        cursor_search="$cursor_dir:${HOME}/.icons:${XDG_DATA_HOME:-${HOME}/.local/share}/icons:/usr/local/share/icons:/usr/share/icons:/usr/share/pixmaps"
        if [ -n "${XCURSOR_PATH:-}" ]; then
            cursor_search="$cursor_search:$XCURSOR_PATH"
        fi
        env_names+=(XCURSOR_PATH XCURSOR_THEME)
        env_values+=("$cursor_search" "capitaine-cursors")
    fi
    # P2 defaults on. These are opt-outs for diagnostics or a quick rollback:
    #   DRAGONFRUIT_DRM_MATERIAL=0 make native-rebuild
    #   DRAGONFRUIT_DRM_GLASS=0 make native-rebuild
    # Only an explicit value is persisted; unset keeps the compositor default.
    if [ -n "${DRAGONFRUIT_DRM_MATERIAL:-}" ]; then
        env_names+=(DRAGONFRUIT_DRM_MATERIAL)
        env_values+=("$DRAGONFRUIT_DRM_MATERIAL")
    fi
    if [ -n "${DRAGONFRUIT_DRM_GLASS:-}" ]; then
        env_names+=(DRAGONFRUIT_DRM_GLASS)
        env_values+=("$DRAGONFRUIT_DRM_GLASS")
    fi

    if [ "${#env_names[@]}" -eq 0 ]; then
        remove_session_environment
        say "no local runtime environment needed; using the system paths"
        return 0
    fi

    mkdir -p "$(dirname "$NATIVE_ENV_FILE")"
    tmp="$(mktemp)"
    {
        printf '# Generated by scripts/native-session.sh for the Dragonfruit native session.\n'
        printf '# Removed again by `scripts/native-session.sh recover`.\n'
        # Plain shell assignment with the value quoted: the session entry is a
        # POSIX sh script, so no %q/$'...' bashisms.
        local i
        for i in "${!env_names[@]}"; do
            printf '%s="%s"\n' "${env_names[$i]}" "$(env_file_value "${env_values[$i]}")"
        done
    } >"$tmp"
    mv "$tmp" "$NATIVE_ENV_FILE"

    # Also pin the values on every unit. The session entry imports the session
    # identity into the user manager, but a per-user drop-in keeps
    # `make native-restart` and a hand-started target working even if the
    # manager environment was not updated.
    for unit in "${ENV_UNITS[@]}"; do
        dropin_dir="$HOME/.config/systemd/user/$unit.d"
        mkdir -p "$dropin_dir"
        {
            printf '[Service]\n'
            local i
            for i in "${!env_names[@]}"; do
                printf 'Environment="%s=%s"\n' "${env_names[$i]}" \
                    "$(systemd_env_value "${env_values[$i]}")"
            done
        } >"$dropin_dir/10-dragonfruit-native-env.conf"
    done
    systemctl --user daemon-reload >/dev/null 2>&1 || true
    say "session runtime library path: ${libs:-<system>}"
    if [ -d "$qml_dir" ]; then
        say "session QML import path: $qml_dir"
    fi
    if [ -d "$cursor_dir/capitaine-cursors" ]; then
        say "session cursor theme: capitaine-cursors ($cursor_dir)"
    fi
}

remove_session_environment() {
    local unit dropin_dir removed=0
    for unit in "${ENV_UNITS[@]}"; do
        dropin_dir="$HOME/.config/systemd/user/$unit.d"
        if [ -e "$dropin_dir/10-dragonfruit-native-env.conf" ]; then
            rm -f "$dropin_dir/10-dragonfruit-native-env.conf"
            rmdir "$dropin_dir" 2>/dev/null || true
            removed=1
        fi
    done
    if [ -e "$NATIVE_ENV_FILE" ]; then
        rm -f "$NATIVE_ENV_FILE"
        rmdir "$(dirname "$NATIVE_ENV_FILE")" 2>/dev/null || true
        removed=1
    fi
    if [ "$removed" -eq 1 ]; then
        systemctl --user daemon-reload >/dev/null 2>&1 || true
        say "removed the native session runtime environment"
    fi
}

install_native_capture() {
    local unit_dir="$HOME/.config/systemd/user"
    local unit_file="$unit_dir/$CAPTURE_UNIT"
    local target_dir
    target_dir="$(dirname "$CAPTURE_TARGET_DROPIN")"

    [ -x "$root/scripts/native-capture.sh" ] || return 0
    mkdir -p "$unit_dir" "$target_dir"
    cat >"$unit_file" <<EOF
[Unit]
Description=Dragonfruit native session capture (dev)
Documentation=https://github.com/dragonfruit-desktop/dragonfruit
After=dragonfruit-compositor.service
PartOf=dragonfruit-session.target

[Service]
Type=simple
ExecStart=/bin/bash "$root/scripts/native-capture.sh" --watch
Restart=no
Nice=19
IOSchedulingClass=idle

[Install]
WantedBy=dragonfruit-session.target
EOF
    cat >"$CAPTURE_TARGET_DROPIN" <<EOF
[Unit]
Wants=$CAPTURE_UNIT
EOF
    systemctl --user daemon-reload >/dev/null 2>&1 || true
    say "native capture: reports under $CAPTURE_STATE_DIR"
}

remove_native_capture() {
    local unit_file="$HOME/.config/systemd/user/$CAPTURE_UNIT"
    local removed=0

    if systemctl --user cat "$CAPTURE_UNIT" >/dev/null 2>&1; then
        systemctl --user stop "$CAPTURE_UNIT" >/dev/null 2>&1 || true
    fi
    if [ -e "$unit_file" ] || [ -e "$CAPTURE_TARGET_DROPIN" ]; then
        rm -f "$unit_file" "$CAPTURE_TARGET_DROPIN"
        rmdir "$(dirname "$CAPTURE_TARGET_DROPIN")" 2>/dev/null || true
        removed=1
    fi
    if [ "$removed" -eq 1 ]; then
        systemctl --user daemon-reload >/dev/null 2>&1 || true
        say "removed the native capture unit"
    fi
}

# Refuse to arm a login whose binaries cannot resolve their shared libraries
# in the environment the session entry will run with. This is the check that
# prevents "I logged out and now every login is a black screen".
preflight_login_environment() {
    local libs name src output missing=0
    libs="$(session_library_path)"
    if ! command -v ldd >/dev/null 2>&1; then
        say "preflight: ldd is unavailable; skipping the loader check"
        return 0
    fi
    for name in "${BIN_NAMES[@]}"; do
        src="$(binary_source "$name")"
        if [ ! -x "$src" ]; then
            say "preflight: missing executable $src"
            missing=1
            continue
        fi
        output="$(env -i HOME="$HOME" PATH="${PATH:-/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin}" \
            LD_LIBRARY_PATH="$libs" ldd "$src" 2>&1 || true)"
        if printf '%s\n' "$output" | grep -q 'not found'; then
            say "preflight: $name cannot start at a login; unresolved libraries:"
            printf '%s\n' "$output" | grep 'not found' | sed 's/^/  /' >&2
            missing=1
        fi
    done
    if [ "$missing" -ne 0 ]; then
        say "session LD_LIBRARY_PATH: ${libs:-<empty>}"
        die "refusing to arm the login until the libraries above resolve (pass --skip-preflight to override)"
    fi
    say "preflight: built binaries resolve with the login environment"
}

# The installed symlinks are what the units exec; verify the install actually
# put them in place before a logout makes that hard to fix.
verify_installed_links() {
    local name
    for name in "${BIN_NAMES[@]}"; do
        [ -x "$PREFIX/bin/$name" ] || die "install is incomplete: $PREFIX/bin/$name is missing or not executable"
    done
    [ -x "$PREFIX/bin/dragonfruit-session-entry" ] \
        || die "install is incomplete: $PREFIX/bin/dragonfruit-session-entry is missing"
    [ -f "$PREFIX/share/wayland-sessions/dragonfruit.desktop" ] \
        || die "install is incomplete: the Dragonfruit session entry is missing"
}

package_manager() {
    local manager
    for manager in dnf apt-get pacman zypper apk; do
        if command -v "$manager" >/dev/null 2>&1; then
            printf '%s\n' "$manager"
            return 0
        fi
    done
    return 1
}

packages_for() {
    case "$1" in
        dnf|yum)
            printf '%s\n' "gcc gcc-c++ make cmake ninja-build pkgconf-pkg-config \
                qt6-qtbase-devel qt6-qtdeclarative-devel wayland-devel \
                libxkbcommon-devel libdrm-devel mesa-libgbm-devel \
                libinput-devel libseat-devel systemd-devel dbus-daemon \
                rust cargo curl"
            ;;
        apt-get)
            printf '%s\n' "build-essential pkg-config cmake ninja-build \
                qt6-base-dev qt6-declarative-dev qt6-wayland-dev \
                libxkbcommon-dev libwayland-dev libdrm-dev libgbm-dev \
                libinput-dev libseat-dev libsystemd-dev libdbus-1-dev \
                libegl-dev libgl-dev cargo rustc curl"
            ;;
        pacman)
            printf '%s\n' "base-devel pkgconf cmake ninja qt6-base qt6-declarative \
                qt6-wayland wayland libxkbcommon libdrm mesa libinput seatd \
                systemd-libs dbus rust curl"
            ;;
        zypper)
            printf '%s\n' "gcc gcc-c++ make cmake ninja pkg-config \
                qt6-base-devel qt6-declarative-devel wayland-devel \
                libxkbcommon-devel libdrm-devel Mesa-libgbm-devel \
                libinput-devel libseat-devel systemd-devel dbus-1-devel \
                rust cargo curl"
            ;;
        apk)
            printf '%s\n' "build-base pkgconf cmake ninja qt6-qtbase-dev \
                qt6-qtdeclarative-dev wayland-dev libxkbcommon-dev libdrm-dev \
                mesa-dev libinput-dev libseat-dev dbus-dev rust cargo curl"
            ;;
        *)
            return 1
            ;;
    esac
}

missing_prereqs() {
    local missing=() command
    for command in cc c++ make cmake ninja pkg-config cargo rustc systemctl; do
        if ! command -v "$command" >/dev/null 2>&1; then
            missing+=("$command")
        fi
    done
    if command -v pkg-config >/dev/null 2>&1; then
        if ! pkg-config --exists Qt6Core Qt6Quick Qt6Qml Qt6Test Qt6DBus 2>/dev/null; then
            missing+=("Qt6 development files")
        fi
    fi
    if [ "${#missing[@]}" -gt 0 ]; then
        printf '%s\n' "${missing[@]}"
    fi
}

install_prereqs_with() {
    local manager="$1"
    shift
    local -a as_root=()
    if [ "$(id -u)" -ne 0 ]; then
        command -v sudo >/dev/null 2>&1 || die "sudo is required to install build dependencies"
        as_root=(sudo)
    fi

    case "$manager" in
        dnf|yum)
            "${as_root[@]}" "$manager" install -y "$@"
            ;;
        apt-get)
            "${as_root[@]}" apt-get update
            "${as_root[@]}" apt-get install -y "$@"
            ;;
        pacman)
            "${as_root[@]}" pacman -S --needed --noconfirm "$@"
            ;;
        zypper)
            "${as_root[@]}" zypper --non-interactive install "$@"
            ;;
        apk)
            "${as_root[@]}" apk add "$@"
            ;;
        *)
            die "unsupported package manager: $manager"
            ;;
    esac
}

ensure_prereqs() {
    local missing
    missing="$(missing_prereqs)"
    [ -z "$missing" ] && return 0

    say "missing build prerequisites:"
    while IFS= read -r line; do
        printf '  %s\n' "$line"
    done <<<"$missing"

    if [ "$NO_DEPS" -eq 1 ]; then
        die "missing build prerequisites; install them or rerun without --no-deps"
    fi

    local manager
    manager="$(package_manager)" || die "no supported package manager found; install the prerequisites manually"
    local -a packages=()
    read -r -a packages <<<"$(packages_for "$manager")"
    say "installing with $manager: ${packages[*]}"
    install_prereqs_with "$manager" "${packages[@]}"
    hash -r
}

cmd_deps() {
    local missing
    missing="$(missing_prereqs)"
    if [ -z "$missing" ]; then
        say "all build prerequisites present"
        return 0
    fi
    say "missing build prerequisites:"
    while IFS= read -r line; do
        printf '  %s\n' "$line"
    done <<<"$missing"
    local manager
    manager="$(package_manager)" || die "no supported package manager found"
    local -a packages=()
    read -r -a packages <<<"$(packages_for "$manager")"
    case "$manager" in
        apt-get) printf 'sudo apt-get update && sudo apt-get install -y %s\n' "${packages[*]}" ;;
        pacman) printf 'sudo pacman -S --needed --noconfirm %s\n' "${packages[*]}" ;;
        zypper) printf 'sudo zypper --non-interactive install %s\n' "${packages[*]}" ;;
        apk) printf 'sudo apk add %s\n' "${packages[*]}" ;;
        *) printf 'sudo %s install -y %s\n' "$manager" "${packages[*]}" ;;
    esac
}

build_all() {
    if [ "$NO_BUILD" -eq 1 ]; then
        say "skipping build (--no-build)"
        return
    fi
    ensure_prereqs
    say "building the workspace and Qt side ($PROFILE)"
    if [ "$PROFILE" = release ]; then
        cargo build --workspace --release
        cmake -S "$root" -B "$QT_BUILD_DIR" -G Ninja -DCMAKE_BUILD_TYPE=Release
        cmake --build "$QT_BUILD_DIR"
    else
        make build
    fi
}

elevate() {
    if [ "$(id -u)" -eq 0 ] || [ -w "$PREFIX" ]; then
        "$@"
    else
        sudo "$@"
    fi
}

unit_for() {
    case "$1" in
        shell) printf '%s\n' dragonfruit-shell.service ;;
        settings|settingsd) printf '%s\n' dragonfruit-settingsd.service ;;
        notifications|notification) printf '%s\n' dragonfruit-notifications.service ;;
        wallpaper|wallpaperd) printf '%s\n' dragonfruit-wallpaperd.service ;;
        apps|app-index|index) printf '%s\n' dragonfruit-app-index.service ;;
        menu|menu-broker) printf '%s\n' dragonfruit-menu-broker.service ;;
        portal) printf '%s\n' dragonfruit-portal.service ;;
        compositor) printf '%s\n' dragonfruit-compositor.service ;;
        *.service|*.target) printf '%s\n' "$1" ;;
        *) return 1 ;;
    esac
}

state_file() {
    local primary fallback
    primary="${XDG_STATE_HOME:-$HOME/.local/state}/dragonfruit/dev-session.json"
    fallback="$HOME/.local/state/dragonfruit/dev-session.json"
    # A text-console shell may not carry the desktop's XDG_STATE_HOME. The arm
    # wrote to whichever path was active then; prefer an existing file.
    if [ -f "$primary" ] || [ ! -f "$fallback" ]; then
        printf '%s\n' "$primary"
    else
        printf '%s\n' "$fallback"
    fi
}

plasma_login_dropin() {
    printf '%s\n' "${DF_PLASMALOGIN_DROPIN:-/etc/plasmalogin.conf.d/99-dragonfruit-dev.conf}"
}

plasma_login_present() {
    command -v plasmalogin >/dev/null 2>&1 \
        || [ -d /etc/plasmalogin.conf.d ] \
        || [ -e /etc/plasmalogin.conf ]
}

# The newer SDDM-derived login manager is not part of the GDM/SDDM/LightDM
# adapter set yet. Its greeter reads `[Greeter] PreselectedSession`, so a
# drop-in is enough to put Dragonfruit first without arming autologin. The
# marker line is what lets `recover` remove only the file this script wrote.
plasma_login_marker='# Generated by Dragonfruit native-session.sh; remove with native-session.sh recover.'

# Read a root-owned file without prompting for sudo when the user can read it.
file_contains() {
    local pattern="$1" file="$2"
    if [ -r "$file" ]; then
        grep -qF "$pattern" "$file" 2>/dev/null
    else
        sudo grep -qF "$pattern" "$file" 2>/dev/null
    fi
}

file_assignment_count() {
    local file="$1"
    if [ -r "$file" ]; then
        grep -c '=' "$file" 2>/dev/null || true
    else
        sudo grep -c '=' "$file" 2>/dev/null || true
    fi
}

# Whether an existing preselect drop-in is one this script wrote (including the
# pre-marker shape from the first version, so recovery works on a host armed by
# it). A user's own file at the same path is never touched without --force.
plasma_preselect_is_ours() {
    local file="$1" count
    if file_contains "$plasma_login_marker" "$file"; then
        return 0
    fi
    if file_contains 'PreselectedSession=dragonfruit.desktop' "$file"; then
        count="$(file_assignment_count "$file" | head -n1 || true)"
        [ "${count:-0}" -le 1 ] && return 0
    fi
    return 1
}

write_plasma_login_preselect() {
    plasma_login_present || return 0
    local dropin tmp dir
    dropin="$(plasma_login_dropin)"
    dir="$(dirname "$dropin")"
    if [ -e "$dropin" ] && ! plasma_preselect_is_ours "$dropin"; then
        if [ "$FORCE" -eq 0 ]; then
            die "$dropin exists and was not written by this script; pass --force to replace it"
        fi
        say "replacing an existing $dropin (--force)"
    fi
    mkdir -p "$dir" 2>/dev/null || sudo mkdir -p "$dir"
    tmp="$(mktemp)"
    cat >"$tmp" <<EOF
$plasma_login_marker
[Greeter]
PreselectedSession=dragonfruit.desktop
EOF
    if [ "$(id -u)" -eq 0 ] || [ -w "$dir" ]; then
        install -m 0644 "$tmp" "$dropin"
    else
        sudo install -m 0644 "$tmp" "$dropin"
    fi
    rm -f "$tmp"
    say "preselected Dragonfruit in the login-manager greeter"
}

remove_plasma_login_preselect() {
    plasma_login_present || return 0
    local dropin dir
    dropin="$(plasma_login_dropin)"
    dir="$(dirname "$dropin")"
    [ -e "$dropin" ] || return 0
    if ! plasma_preselect_is_ours "$dropin"; then
        say "$dropin was not written by this script; leaving it alone"
        return 0
    fi
    if [ "$(id -u)" -eq 0 ] || [ -w "$dir" ]; then
        rm -f "$dropin"
    else
        sudo rm -f "$dropin"
    fi
    say "removed the login-manager Dragonfruit preselect"
}

# The SDDM-derived login manager also remembers the last *logged-in* session
# in its own state file (not in the preselect drop-in). A failed Dragonfruit
# login is recorded there, so removing the preselect alone would leave the
# greeter falling back to Dragonfruit. Clear that memory on recovery.
clear_login_manager_last_session() {
    command -v sudo >/dev/null 2>&1 || return 0
    local dir files file cleared=0
    for dir in /var/lib/plasmalogin /var/lib/plasmalogin-greeter /var/cache/plasmalogin; do
        [ -d "$dir" ] || continue
        files="$(sudo grep -rl 'LastLoggedInSession' "$dir" 2>/dev/null || true)"
        [ -n "$files" ] || continue
        while IFS= read -r file; do
            [ -n "$file" ] || continue
            sudo rm -f "$file"
            cleared=1
        done <<<"$files"
    done
    if [ "$cleared" -eq 1 ]; then
        say "cleared the login manager's last-session memory"
    fi
}

# Restore the display-manager files the round trip snapshotted. This runs
# under sudo because GDM/SDDM/LightDM keep them root-owned, and it deliberately
# does not depend on the built dev tool: recovery must work when the build (or
# the session) is what broke.
restore_round_trip_state() {
    local state
    state="$(state_file)"
    if [ ! -f "$state" ]; then
        say "no armed round trip state to restore"
        return 0
    fi
    if ! command -v python3 >/dev/null 2>&1; then
        if [ -x "$DEV_BIN" ]; then
            "$DEV_BIN" dev --real --recover --user "${USER:-$(id -un)}"
            return $?
        fi
        say "python3 is required to restore the display-manager files; install it and rerun"
        return 1
    fi
    local -a as_root=()
    if [ "$(id -u)" -ne 0 ]; then
        command -v sudo >/dev/null 2>&1 || die "sudo is required to restore root-owned display-manager files"
        as_root=(sudo)
    fi
    "${as_root[@]}" python3 - "$state" "$HOME" "${USER:-$(id -un)}" <<'PY'
import json
import os
import sys

state_path, home, _user = sys.argv[1:4]
try:
    with open(state_path, encoding="utf-8") as handle:
        state = json.load(handle)
except (OSError, ValueError) as error:
    print(f"native-session: cannot read {state_path}: {error}", file=sys.stderr)
    sys.exit(1)
if not isinstance(state, dict):
    print(f"native-session: {state_path} is not a dev-session object", file=sys.stderr)
    sys.exit(1)

# Only the display-manager files the harness is allowed to edit. The state
# file is user-writable, so an unvalidated path would turn `recover` (which
# runs as root) into an arbitrary-file writer.
prefixes = (
    "/var/lib/AccountsService/users/",
    "/etc/gdm/",
    "/etc/gdm3/",
    "/var/lib/sddm/",
    "/etc/sddm.conf",
    "/etc/sddm.conf.d/",
    "/etc/lightdm/",
    "/etc/plasmalogin.conf.d/",
    "/etc/plasmalogin.conf",
)
dmrc = os.path.realpath(os.path.join(home, ".dmrc"))


def allowed(raw):
    path = os.path.realpath(raw)
    if path == dmrc:
        return path
    for prefix in prefixes:
        if path == prefix or path.startswith(prefix.rstrip("/") + "/"):
            return path
    return None


errors = 0
restored = 0
backups = state.get("backups", [])
if not isinstance(backups, list):
    backups = []
for backup in backups:
    if not isinstance(backup, dict):
        print("native-session: ignoring a malformed backup entry", file=sys.stderr)
        errors += 1
        continue
    raw = backup.get("path") or ""
    path = allowed(raw)
    if path is None:
        print(f"native-session: refusing to restore unexpected path {raw!r}", file=sys.stderr)
        errors += 1
        continue
    contents = backup.get("contents")
    try:
        if contents is None:
            try:
                os.remove(path)
            except FileNotFoundError:
                pass
            print(f"native-session: removed {path}")
        else:
            parent = os.path.dirname(path)
            if parent:
                os.makedirs(parent, exist_ok=True)
            flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
            if hasattr(os, "O_NOFOLLOW"):
                # Do not follow a final-component symlink in a user-writable
                # location: the whitelist already realpaths the path, but this
                # closes the race.
                flags |= os.O_NOFOLLOW
            fd = os.open(path, flags, 0o644)
            with os.fdopen(fd, "w", encoding="utf-8") as handle:
                handle.write(contents)
            print(f"native-session: restored {path}")
        restored += 1
    except OSError as error:
        print(f"native-session: cannot restore {path}: {error}", file=sys.stderr)
        errors += 1

if errors:
    print(
        "native-session: restore incomplete; the state file is kept for a retry",
        file=sys.stderr,
    )
    sys.exit(1)

previous = state.get("previous_session")
if previous and not backups:
    print(
        f"native-session: previous desktop was {previous!r}; no file snapshots were recorded",
        file=sys.stderr,
    )

try:
    os.remove(state_path)
except FileNotFoundError:
    pass
print(f"native-session: restored {restored} display-manager file(s) and cleared the round-trip state")
PY
}

# Stop the failed session without touching the user's text console. The user
# manager is shared per UID, so this works from a TTY/SSH login too.
stop_native_session() {
    systemctl --user stop dragonfruit-session.target >/dev/null 2>&1 || true
    systemctl --user reset-failed dragonfruit-session.target >/dev/null 2>&1 || true
    if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
        rm -f \
            "$XDG_RUNTIME_DIR/dragonfruit-wayland" \
            "$XDG_RUNTIME_DIR/dragonfruit-wayland.lock" \
            "$XDG_RUNTIME_DIR/dragonfruit-wayland.x11-display" \
            "$XDG_RUNTIME_DIR/dragonfruit-wayland.launch-token" \
            "$XDG_RUNTIME_DIR/dragonfruit-wayland.desktop-launch-token"
    fi
}

# A wedged graphical session may outlive the user units; terminate only
# sessions whose desktop is Dragonfruit. Never the text console we run from.
terminate_dragonfruit_sessions() {
    command -v loginctl >/dev/null 2>&1 || return 0
    local id desktop
    for id in $(loginctl list-sessions --no-legend 2>/dev/null | awk '{print $1}'); do
        desktop="$(loginctl show-session "$id" -p Desktop --value 2>/dev/null || true)"
        [ "$desktop" = "dragonfruit" ] || continue
        say "terminating the failed Dragonfruit session $id"
        loginctl terminate-session "$id" >/dev/null 2>&1 || true
    done
}

# Whether the current user still has a working graphical session that is not
# the failed Dragonfruit one. Used to avoid restarting a login manager out
# from under a desktop when recovery is run over SSH.
graphical_non_dragonfruit_session_for_user() {
    command -v loginctl >/dev/null 2>&1 || return 1
    local id name type desktop user
    user="${USER:-$(id -un)}"
    for id in $(loginctl list-sessions --no-legend 2>/dev/null | awk '{print $1}'); do
        name="$(loginctl show-session "$id" -p Name --value 2>/dev/null || true)"
        [ "$name" = "$user" ] || continue
        type="$(loginctl show-session "$id" -p Type --value 2>/dev/null || true)"
        case "$type" in
            wayland|x11) ;;
            *) continue ;;
        esac
        desktop="$(loginctl show-session "$id" -p Desktop --value 2>/dev/null || true)"
        # An empty Desktop is not proof of a working host desktop; only a named,
        # non-Dragonfruit one is. Being conservative here restarts the login
        # manager when the failed session is still lingering.
        [ -n "$desktop" ] || continue
        [ "$desktop" = "dragonfruit" ] && continue
        return 0
    done
    return 1
}

restart_display_manager() {
    local unit
    for unit in display-manager.service plasmalogin.service sddm.service gdm.service lightdm.service; do
        if systemctl cat "$unit" >/dev/null 2>&1; then
            say "restarting $unit"
            if sudo systemctl restart "$unit"; then
                say "the login manager is back; switch to its VT (usually Ctrl+Alt+F1/F2)"
                return 0
            fi
        fi
    done
    say "could not restart a display manager automatically; log out and back in (or reboot)"
    return 1
}

install_native() {
    require_built

    [ -d "$PREFIX" ] || mkdir -p "$PREFIX"

    STAGE="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-native.XXXXXX")"
    local stage_root="$STAGE/root"

    say "staging the session entry, systemd user units, app metadata, and portal data"
    "$SESSION_BIN" --install-session "$stage_root" >/dev/null
    "$PORTAL_BIN" --install-data "$stage_root" >/dev/null
    cmake --install "$QT_BUILD_DIR" --prefix "$stage_root" >/dev/null

    say "installing under $PREFIX"
    elevate cp -a --no-preserve=ownership,context "$stage_root/." "$PREFIX/"

    local name src dest
    for name in "${BIN_NAMES[@]}"; do
        src="$(binary_source "$name")"
        dest="$PREFIX/bin/$name"
        if [ -e "$dest" ] && [ ! -L "$dest" ]; then
            if [ "$FORCE" -eq 1 ]; then
                elevate mv "$dest" "$dest.pre-dfdev"
                say "backed up $dest to $dest.pre-dfdev"
            else
                die "$dest exists and is not a dev symlink; pass --force to replace it (a .pre-dfdev backup is kept)"
            fi
        fi
        elevate ln -sfn "$src" "$dest"
    done

    systemctl --user daemon-reload 2>/dev/null || true
    say "installed; the display manager will list Dragonfruit at the next login"
}

cmd_install() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --force) FORCE=1 ;;
            --no-build) NO_BUILD=1 ;;
            --no-deps) NO_DEPS=1 ;;
            -h|--help) usage; return 0 ;;
            *) die "install: unknown argument: $1" ;;
        esac
        shift
    done
    require_user
    build_all
    write_session_environment
    install_native
    install_native_capture
    verify_installed_links
}

cmd_start() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --autologin) AUTOLOGIN=1 ;;
            --force) FORCE=1 ;;
            --no-build) NO_BUILD=1 ;;
            --no-logout) NO_LOGOUT=1 ;;
            --no-deps) NO_DEPS=1 ;;
            --skip-preflight) SKIP_PREFLIGHT=1 ;;
            -h|--help) usage; return 0 ;;
            *) die "start: unknown argument: $1" ;;
        esac
        shift
    done
    require_user
    command -v systemctl >/dev/null 2>&1 || die "the native session uses systemd user units; use 'make dev' on this host"
    systemctl --user show-environment >/dev/null 2>&1 || die "no systemd user manager; use 'make dev' on this host"
    build_all
    # The login environment check comes before the install/preselect: if the
    # session cannot start, the host desktop must not be changed yet.
    if [ "$SKIP_PREFLIGHT" -eq 0 ]; then
        preflight_login_environment
    else
        say "skipping the login preflight (--skip-preflight)"
    fi
    write_session_environment
    install_native
    install_native_capture
    verify_installed_links
    write_plasma_login_preselect

    if [ "$NO_LOGOUT" -eq 1 ]; then
        say "installed and preselected; not logging out (--no-logout)"
        say "log out from the host desktop when ready, then choose Dragonfruit"
        say "recover from a text console with: bash $root/scripts/native-session.sh recover"
        return 0
    fi

    local -a args=(dev --real --round-trip)
    if [ "$AUTOLOGIN" -eq 1 ]; then
        if plasma_login_present; then
            say "this login manager does not support the harness autologin; Dragonfruit is preselected and you log in manually"
        else
            args+=(--autologin)
        fi
    fi

    say "arming the next login and asking the host to log out"
    say "after login, use 'bash $root/scripts/native-session.sh restart [UNIT]' to rebuild/restart"
    say "if the login does not come up, switch to a text console (Ctrl+Alt+F3) and run:"
    say "  bash $root/scripts/native-session.sh recover    # or: make native-recover"
    exec "$DEV_BIN" "${args[@]}"
}

cmd_ready() {
    require_user
    exec "$DEV_BIN" dev --real --ready
}

cmd_capture() {
    require_user
    exec "$root/scripts/native-capture.sh" "${1:-once}"
}

# Undo everything the native loop changed outside the repo: the login-manager
# preselect, the dev runtime environment, and the saved display-manager files.
# Safe to run when nothing is armed (idempotent) and from a plain text console.
recover_host() {
    local inside="$1"
    local restore_status=0

    remove_plasma_login_preselect
    remove_native_capture
    remove_session_environment
    clear_login_manager_last_session
    if ! restore_round_trip_state; then
        restore_status=1
        say "the display-manager files could not be fully restored; the state file is kept for a retry"
    fi
    if [ "$inside" -eq 0 ]; then
        stop_native_session
        terminate_dragonfruit_sessions
    fi
    return "$restore_status"
}

cmd_recover() {
    local inside=0 restart_dm=auto
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --force) FORCE=1 ;;
            --restart-dm) restart_dm=yes ;;
            --no-restart-dm) restart_dm=no ;;
            -h|--help) usage; return 0 ;;
            *) die "recover: unknown argument: $1" ;;
        esac
        shift
    done
    require_user
    if [ "${XDG_CURRENT_DESKTOP:-}" = "dragonfruit" ]; then
        inside=1
    fi

    say "restoring the host login configuration (inside Dragonfruit: $([ "$inside" -eq 1 ] && echo yes || echo no))"
    recover_host "$inside" || true
    say "to reinstall the session later without logging out: bash $root/scripts/native-session.sh --no-logout"

    if [ "$inside" -eq 1 ]; then
        say "ending the Dragonfruit session; the display manager will return"
        systemctl --user stop dragonfruit-session.target >/dev/null 2>&1 || true
        return 0
    fi

    if [ "$restart_dm" = no ]; then
        say "restore complete; return to the login screen (Ctrl+Alt+F1/F2) and log in normally"
        return 0
    fi
    # Do not restart a login manager out from under a working desktop; on a
    # text console (the broken-login case) restart it so the restored default
    # is picked up immediately.
    if [ "$restart_dm" = auto ] \
        && { [ -n "${XDG_CURRENT_DESKTOP:-}" ] || graphical_non_dragonfruit_session_for_user; }; then
        say "restore complete; log out and back in to pick up the restored default"
        return 0
    fi
    restart_display_manager
}

cmd_return() {
    cmd_recover "$@"
}

cmd_rebuild() {
    local reboot=0
    local no_install=0
    local no_logout=0

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --reboot) reboot=1 ;;
            --no-build) NO_BUILD=1 ;;
            --no-deps) NO_DEPS=1 ;;
            --no-install) no_install=1 ;;
            --no-logout) no_logout=1 ;;
            --force) FORCE=1 ;;
            -h|--help) usage; return 0 ;;
            *) die "rebuild: unknown argument: $1" ;;
        esac
        shift
    done

    require_user
    if [ "${XDG_CURRENT_DESKTOP:-}" != "dragonfruit" ] && [ "$FORCE" -eq 0 ]; then
        die "rebuild ends the current session; run it inside Dragonfruit (or pass --force)"
    fi

    build_all
    write_session_environment
    if [ "$no_install" -eq 0 ]; then
        install_native
        install_native_capture
        verify_installed_links
        write_plasma_login_preselect
    else
        say "skipping install refresh (--no-install)"
    fi

    if [ "$no_logout" -eq 1 ]; then
        say "rebuild complete; not ending the session (--no-logout)"
        return 0
    fi

    if [ "$reboot" -eq 1 ]; then
        say "rebuild complete; rebooting into the updated Dragonfruit session"
        elevate systemctl reboot
        return
    fi

    say "rebuild complete; ending the Dragonfruit session"
    say "the display manager will return with Dragonfruit preselected; log back in to pick up the new binaries"
    local session_id="${XDG_SESSION_ID:-}"
    if command -v loginctl >/dev/null 2>&1 && [ -n "$session_id" ]; then
        if ! loginctl terminate-session "$session_id"; then
            say "loginctl could not end the session; stopping the Dragonfruit session target"
            systemctl --user stop dragonfruit-session.target >/dev/null 2>&1 || true
        fi
    else
        say "no logind session id; stopping the Dragonfruit session target"
        systemctl --user stop dragonfruit-session.target >/dev/null 2>&1 || true
        say "if the greeter does not return, switch to a text console and run: make native-recover"
    fi
}

cmd_restart() {
    local -a units=()
    local arg unit

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --no-build) NO_BUILD=1 ;;
            --no-deps) NO_DEPS=1 ;;
            --force) FORCE=1 ;;
            -h|--help) usage; return 0 ;;
            all)
                units+=(
                    dragonfruit-shell.service
                    dragonfruit-settingsd.service
                    dragonfruit-menu-broker.service
                    dragonfruit-app-index.service
                    dragonfruit-notifications.service
                    dragonfruit-wallpaperd.service
                    dragonfruit-portal.service
                )
                ;;
            services)
                units+=(
                    dragonfruit-settingsd.service
                    dragonfruit-menu-broker.service
                    dragonfruit-app-index.service
                    dragonfruit-notifications.service
                    dragonfruit-wallpaperd.service
                    dragonfruit-portal.service
                )
                ;;
            *)
                unit="$(unit_for "$1")" || die "restart: unknown unit '$1'"
                units+=("$unit")
                ;;
        esac
        shift
    done


    if [ "${#units[@]}" -eq 0 ]; then
        units=(dragonfruit-shell.service)
    fi

    if [ "${XDG_CURRENT_DESKTOP:-}" != "dragonfruit" ] && [ "$FORCE" -eq 0 ]; then
        die "not inside Dragonfruit; restart only applies to the running native session (or pass --force)"
    fi

    local installed
    installed="$(installed_profile)"
    if [ "$installed" != "$PROFILE" ]; then
        say "installed profile is $installed; selected profile is $PROFILE."
        say "restart cannot replace the running compositor; use 'make native-rebuild'"
        say "(or 'make native-reboot') to switch profiles."
        if [ "$installed" = debug ] || [ "$installed" = release ]; then
            say "or pass --$installed to keep the installed profile for this restart."
        fi
        exit 1
    fi

    build_all
    write_session_environment

    for unit in "${units[@]}"; do
        if [ "$unit" = "dragonfruit-compositor.service" ] && [ "$FORCE" -eq 0 ]; then
            die "the compositor is the session anchor; restarting it ends the session. Use 'return' then 'start', or pass --force"
        fi
        say "restarting $unit"
        systemctl --user restart "$unit"
    done
    say "restarted: ${units[*]}"
}

cmd_status() {
    local state unit active
    state="$(state_file)"

    say "host desktop: ${XDG_CURRENT_DESKTOP:-<unset>}"
    say "install prefix: $PREFIX"
    say "selected profile: $PROFILE"
    say "installed profile: $(installed_profile)"
    if [ -f "$state" ]; then
        say "armed state: $state"
    else
        say "armed state: none ($state)"
    fi
    if [ -f "$NATIVE_ENV_FILE" ]; then
        say "session environment: $NATIVE_ENV_FILE"
    else
        say "session environment: none ($NATIVE_ENV_FILE)"
    fi
    if plasma_login_present; then
        if [ -e "$(plasma_login_dropin)" ]; then
            say "login-manager preselect: installed ($(plasma_login_dropin))"
        else
            say "login-manager preselect: not installed"
        fi
    fi
    if [ -e "$HOME/.config/systemd/user/$CAPTURE_UNIT" ]; then
        say "native capture: installed ($CAPTURE_STATE_DIR)"
    else
        say "native capture: not installed"
    fi
    if [ -d "$CAPTURE_STATE_DIR" ]; then
        local latest
        latest="$(find "$CAPTURE_STATE_DIR" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' 2>/dev/null \
            | sort -rn | head -n 1 | cut -d' ' -f2-)"
        [ -n "$latest" ] && say "latest capture: $latest"
    fi
    say "recovery: bash $root/scripts/native-session.sh recover"

    printf '  %-34s %s\n' UNIT STATE
    for unit in \
        dragonfruit-session.target \
        dragonfruit-compositor.service \
        dragonfruit-shell.service \
        dragonfruit-settingsd.service \
        dragonfruit-menu-broker.service \
        dragonfruit-app-index.service \
        dragonfruit-notifications.service \
        dragonfruit-wallpaperd.service \
        dragonfruit-portal.service; do
        active="$(systemctl --user is-active "$unit" 2>/dev/null || true)"
        printf '  %-34s %s\n' "$unit" "${active:-unknown}"
    done
}

cmd_logs() {
    local follow=0 unit=""

    while [ "$#" -gt 0 ]; do
        case "$1" in
            -f|--follow) follow=1 ;;
            -h|--help) usage; return 0 ;;
            -*) die "logs: unknown argument: $1" ;;
            *)
                [ -z "$unit" ] || die "logs: only one unit may be given"
                unit="$1"
                ;;
        esac
        shift
    done

    if [ -z "$unit" ]; then
        unit="shell"
    fi
    local requested="$unit"
    unit="$(unit_for "$requested")" || die "logs: unknown unit '$requested'"

    local -a args=(--user -u "$unit" -n "${LINES:-200}")
    if [ "$follow" -eq 1 ]; then
        args+=(-f)
    fi
    exec journalctl "${args[@]}"
}

if [ "$#" -eq 0 ]; then
    command=start
else
    case "$1" in
        --return) command=return; shift ;;
        --recover) command=recover; shift ;;
        --restart) command=restart; shift ;;
        --status) command=status; shift ;;
        --logs) command=logs; shift ;;
        --help|-h) command=help; shift ;;
        --no-logout|--no-build|--no-deps|--autologin|--force|--skip-preflight) command=start ;;
        *) command="$1"; shift ;;
    esac
fi

case "$command" in
    start|up|switch) cmd_start "$@" ;;
    return|back|down) cmd_return "$@" ;;
    ready) cmd_ready "$@" ;;
    recover) cmd_recover "$@" ;;
    rebuild) cmd_rebuild "$@" ;;
    restart|reload) cmd_restart "$@" ;;
    status) cmd_status "$@" ;;
    capture|record) cmd_capture "$@" ;;
    logs|log) cmd_logs "$@" ;;
    install) cmd_install "$@" ;;
    deps|doctor) cmd_deps "$@" ;;
    help) usage ;;
    *) die "unknown command '$command' (try '$0 help')" ;;
esac
