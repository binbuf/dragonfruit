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
# The link farm points at target/debug and build/, so a normal `make build`
# is enough before a restart: no reinstall step is needed.
#
# Usage:
#   scripts/native-session.sh [start] [--no-build] [--no-logout] [--no-deps] [--autologin] [--force]
#   scripts/native-session.sh return [--force]
#   scripts/native-session.sh ready
#   scripts/native-session.sh recover
#   scripts/native-session.sh restart [--no-build] [--no-deps] [--force] [UNIT ...]
#   scripts/native-session.sh status
#   scripts/native-session.sh logs [-f] [UNIT]
#   scripts/native-session.sh install [--force] [--no-deps]
#   scripts/native-session.sh deps
#
# UNIT is a shorthand (`shell`, `settings`, `notifications`, `wallpaper`,
# `apps`, `menu`, `portal`, `services`, `all`) or a systemd user unit name.
# `all` deliberately excludes the compositor: it is the session anchor, so
# restarting it ends the session. Rebuild and use `return` + `start` for
# compositor changes (or use the nested `make dev` loop).
#
# DF_NATIVE_PREFIX overrides the install prefix (default /usr; useful for
# staging tests, not for a real display-manager login).

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

PREFIX="${DF_NATIVE_PREFIX:-/usr}"
PREFIX="${PREFIX%/}"
DEV_BIN="$root/target/debug/dragonfruit"
SESSION_BIN="$root/target/debug/dragonfruit-session"
PORTAL_BIN="$root/target/debug/xdg-desktop-portal-dragonfruit"

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

NO_BUILD=0
NO_LOGOUT=0
NO_DEPS=0
FORCE=0
AUTOLOGIN=0
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
  scripts/native-session.sh [start] [--no-build] [--no-logout] [--no-deps] [--autologin] [--force]
  scripts/native-session.sh return [--force]
  scripts/native-session.sh ready
  scripts/native-session.sh recover
  scripts/native-session.sh restart [--no-build] [--no-deps] [--force] [UNIT ...]
  scripts/native-session.sh status
  scripts/native-session.sh logs [-f] [UNIT]
  scripts/native-session.sh install [--force] [--no-deps]
  scripts/native-session.sh deps

UNIT is a shorthand (shell, settings, notifications, wallpaper, apps, menu,
portal, services, all) or a systemd user unit name. `all` deliberately
excludes the compositor: it is the session anchor, so restarting it ends the
session. Rebuild and use `return` + `start` for compositor changes (or use
the nested `make dev` loop).

Running the script with no command is `start`: it checks/installs the
distribution build dependencies, builds the tree, installs the session entry,
user units, app metadata, and portal data, links the built binaries into
place, then arms the next login for Dragonfruit and asks the host session
manager to log out. The installer uses sudo only for the system copy.
Rebuilds are picked up through the symlinks, so `restart` needs no reinstall.
The newer SDDM-derived login manager is preselected through a conf.d drop-in;
`return`/`recover` remove it again. `--no-logout` stops after
install/preselect, so you can log out by hand. `--no-deps` skips the
distribution package bootstrap; `deps` prints the detected package command.

DF_NATIVE_PREFIX overrides the install prefix (default /usr; useful for
staging tests, not for a real display-manager login).
EOF
}

require_user() {
    [ "$(id -u)" -ne 0 ] || die "run this as your desktop user, not root (the script uses sudo only for installation)"
}

binary_source() {
    case "$1" in
        dragonfruit-shell) printf '%s\n' "$root/build/shell/src/dragonfruit-shell" ;;
        dragonfruit-files) printf '%s\n' "$root/build/apps/files/dragonfruit-files" ;;
        dragonfruit-settings) printf '%s\n' "$root/build/apps/settings/dragonfruit-settings" ;;
        *) printf '%s\n' "$root/target/debug/$1" ;;
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
    say "building the workspace and Qt side"
    make build
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
    printf '%s\n' "${XDG_STATE_HOME:-$HOME/.local/state}/dragonfruit/dev-session.json"
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
# drop-in is enough to put Dragonfruit first without arming autologin.
write_plasma_login_preselect() {
    plasma_login_present || return 0
    local dropin tmp dir
    dropin="$(plasma_login_dropin)"
    dir="$(dirname "$dropin")"
    mkdir -p "$dir" 2>/dev/null || sudo mkdir -p "$dir"
    tmp="$(mktemp)"
    cat >"$tmp" <<'EOF'
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
    if [ "$(id -u)" -eq 0 ] || [ -w "$dir" ]; then
        rm -f "$dropin"
    else
        sudo rm -f "$dropin"
    fi
    say "removed the login-manager Dragonfruit preselect"
}

install_native() {
    require_built

    [ -d "$PREFIX" ] || mkdir -p "$PREFIX"

    STAGE="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-native.XXXXXX")"
    local stage_root="$STAGE/root"

    say "staging the session entry, systemd user units, app metadata, and portal data"
    "$SESSION_BIN" --install-session "$stage_root" >/dev/null
    "$PORTAL_BIN" --install-data "$stage_root" >/dev/null
    cmake --install build --prefix "$stage_root" >/dev/null

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
    install_native
}

cmd_start() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --autologin) AUTOLOGIN=1 ;;
            --force) FORCE=1 ;;
            --no-build) NO_BUILD=1 ;;
            --no-logout) NO_LOGOUT=1 ;;
            --no-deps) NO_DEPS=1 ;;
            -h|--help) usage; return 0 ;;
            *) die "start: unknown argument: $1" ;;
        esac
        shift
    done
    require_user
    command -v systemctl >/dev/null 2>&1 || die "the native session uses systemd user units; use 'make dev' on this host"
    systemctl --user show-environment >/dev/null 2>&1 || die "no systemd user manager; use 'make dev' on this host"
    build_all
    install_native
    write_plasma_login_preselect

    if [ "$NO_LOGOUT" -eq 1 ]; then
        say "installed and preselected; not logging out (--no-logout)"
        say "log out from the host desktop when ready, then choose Dragonfruit"
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
    say "after login, use '$0 restart [UNIT]' to rebuild/restart, or '$0 return' to go back"
    exec "$DEV_BIN" "${args[@]}"
}

cmd_return() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --force) FORCE=1 ;;
            -h|--help) usage; return 0 ;;
            *) die "return: unknown argument: $1" ;;
        esac
        shift
    done
    require_user
    if [ "${XDG_CURRENT_DESKTOP:-}" != "dragonfruit" ] && [ "$FORCE" -eq 0 ]; then
        die "not inside Dragonfruit; run this from the Dragonfruit session (or pass --force)"
    fi
    remove_plasma_login_preselect
    say "restoring the previous desktop selection and logging out"
    exec "$DEV_BIN" dev --real --return
}

cmd_ready() {
    require_user
    exec "$DEV_BIN" dev --real --ready
}

cmd_recover() {
    require_user
    remove_plasma_login_preselect
    exec "$DEV_BIN" dev --real --recover
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

    build_all

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
    if [ -f "$state" ]; then
        say "armed state: $state"
    else
        say "armed state: none ($state)"
    fi
    if plasma_login_present; then
        if [ -e "$(plasma_login_dropin)" ]; then
            say "login-manager preselect: installed ($(plasma_login_dropin))"
        else
            say "login-manager preselect: not installed"
        fi
    fi

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
        --restart) command=restart; shift ;;
        --status) command=status; shift ;;
        --logs) command=logs; shift ;;
        --help|-h) command=help; shift ;;
        --no-logout|--no-build|--no-deps|--autologin|--force) command=start ;;
        *) command="$1"; shift ;;
    esac
fi

case "$command" in
    start|up|switch) cmd_start "$@" ;;
    return|back|down) cmd_return "$@" ;;
    ready) cmd_ready "$@" ;;
    recover) cmd_recover "$@" ;;
    restart|reload) cmd_restart "$@" ;;
    status) cmd_status "$@" ;;
    logs|log) cmd_logs "$@" ;;
    install) cmd_install "$@" ;;
    deps|doctor) cmd_deps "$@" ;;
    help) usage ;;
    *) die "unknown command '$command' (try '$0 help')" ;;
esac
