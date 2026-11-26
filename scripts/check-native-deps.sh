#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T342: check the native Qt 6 Wayland platform plugin before a source build
# arms a native login or CI relies on the shell's GPU chrome rail.
#
# The shell asks Qt for the Wayland QPA when GPU chrome is available
# (shell/src/main.cpp, ADR 0195). The plugin is a separate distro package on
# several distributions, so a host can have Qt 6 development files and still
# fail to start the shell. This check is read-only and never uses root: it
# reports the detected distro, the plugin it looked for, and the package
# command the user can run. The shell itself falls back to the offscreen QPA
# with a warning (a diagnostic, not a supported long-term mode).
#
# Usage:
#   scripts/check-native-deps.sh [--plugin-dir DIR] [--qt-prefix DIR] [--quiet]
#
# --plugin-dir restricts the search to the given platform-plugin directory
# (repeatable); it is also the deterministic way to exercise the missing path.
# --qt-prefix adds a Qt installation prefix (repeatable). Without either, the
# script reads QT_QPA_PLATFORM_PLUGIN_PATH, QT_PLUGIN_PATH, qtpaths/qmake,
# pkg-config, DF_TOOLCHAIN, QTDIR, and the common system Qt plugin directories.
# --quiet prints nothing; use the exit status.

set -uo pipefail

explicit_plugin_dirs=()
qt_prefixes=()
quiet=0

usage() {
    sed -n '2,23p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --plugin-dir)
            [ "$#" -ge 2 ] || { echo "check-native-deps: --plugin-dir needs a value" >&2; exit 2; }
            explicit_plugin_dirs+=("$2")
            shift 2
            ;;
        --qt-prefix)
            [ "$#" -ge 2 ] || { echo "check-native-deps: --qt-prefix needs a value" >&2; exit 2; }
            qt_prefixes+=("$2")
            shift 2
            ;;
        --quiet) quiet=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "check-native-deps: unknown argument: $1" >&2; exit 2 ;;
    esac
done

say() {
    [ "$quiet" -eq 1 ] && return 0
    printf 'check-native-deps: %s\n' "$*"
}

# --- distro and package manager -------------------------------------------

distro_id=""
distro_like=""
distro_name=""
if [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    distro_id="${ID:-}"
    distro_like="${ID_LIKE:-}"
    distro_name="${PRETTY_NAME:-${NAME:-$distro_id}}"
fi

manager=""
case " ${distro_id} ${distro_like} " in
    *" fedora "*|*" rhel "*|*" centos "*) command -v dnf >/dev/null 2>&1 && manager=dnf ;;
    *" opensuse "*|*" suse "*|*" sles "*) command -v zypper >/dev/null 2>&1 && manager=zypper ;;
    *" debian "*|*" ubuntu "*) command -v apt-get >/dev/null 2>&1 && manager=apt-get ;;
    *" arch "*) command -v pacman >/dev/null 2>&1 && manager=pacman ;;
    *" alpine "*) command -v apk >/dev/null 2>&1 && manager=apk ;;
esac
if [ -z "$manager" ]; then
    for candidate in dnf apt-get pacman zypper apk yum; do
        if command -v "$candidate" >/dev/null 2>&1; then
            manager="$candidate"
            break
        fi
    done
fi

package_for() {
    case "$1" in
        dnf|yum) printf '%s\n' qt6-qtwayland ;;
        apt-get) printf '%s\n' qt6-wayland ;;
        zypper) printf '%s\n' qt6-wayland ;;
        pacman) printf '%s\n' qt6-wayland ;;
        apk) printf '%s\n' qt6-qtwayland ;;
        *) printf '%s\n' "" ;;
    esac
}

package="$(package_for "$manager")"

install_hint() {
    if [ -z "$package" ]; then
        printf '%s\n' "install your distro's Qt 6 Wayland platform plugin (Fedora: qt6-qtwayland; Debian/Arch/openSUSE: qt6-wayland; Alpine: qt6-qtwayland)"
        return
    fi
    case "$manager" in
        dnf|yum) printf 'sudo dnf install %s\n' "$package" ;;
        apt-get) printf 'sudo apt-get install %s\n' "$package" ;;
        zypper) printf 'sudo zypper install %s\n' "$package" ;;
        pacman) printf 'sudo pacman -S %s\n' "$package" ;;
        apk) printf 'sudo apk add %s\n' "$package" ;;
    esac
}

# --- plugin lookup ---------------------------------------------------------

plugin_names=(libqwayland.so libqwayland-generic.so libqwayland-egl.so)
directories=()

add_directory() {
    local path="${1:-}"
    [ -n "$path" ] || return 0
    path="${path%/}"
    [ -n "$path" ] || path="/"
    local existing
    for existing in "${directories[@]}"; do
        [ "$existing" = "$path" ] && return 0
    done
    directories+=("$path")
}

add_path_list() {
    local value="${1:-}"
    local suffix="${2:-}"
    [ -n "$value" ] || return 0
    local -a entries=()
    IFS=':' read -r -a entries <<<"$value"
    local entry
    for entry in "${entries[@]}"; do
        [ -n "$entry" ] || continue
        add_directory "${entry}${suffix}"
    done
}

add_qt_prefix() {
    local prefix="${1:-}"
    [ -n "$prefix" ] || return 0
    add_directory "${prefix}/plugins/platforms"
    add_directory "${prefix}/lib64/qt6/plugins/platforms"
    add_directory "${prefix}/lib/qt6/plugins/platforms"
    add_directory "${prefix}/lib/x86_64-linux-gnu/qt6/plugins/platforms"
    add_directory "${prefix}/lib/aarch64-linux-gnu/qt6/plugins/platforms"
}

if [ "${#explicit_plugin_dirs[@]}" -gt 0 ]; then
    for directory in "${explicit_plugin_dirs[@]}"; do
        add_directory "$directory"
    done
else
    add_path_list "${QT_QPA_PLATFORM_PLUGIN_PATH:-}"
    add_path_list "${QT_PLUGIN_PATH:-}" "/platforms"

    for tool in qtpaths6 qtpaths; do
        command -v "$tool" >/dev/null 2>&1 || continue
        plugins="$("$tool" -query QT_INSTALL_PLUGINS 2>/dev/null || true)"
        [ -n "$plugins" ] && add_directory "${plugins}/platforms"
    done
    for tool in qmake6 qmake; do
        command -v "$tool" >/dev/null 2>&1 || continue
        plugins="$("$tool" -query QT_INSTALL_PLUGINS 2>/dev/null || true)"
        [ -n "$plugins" ] && add_directory "${plugins}/platforms"
    done

    if command -v pkg-config >/dev/null 2>&1; then
        prefix="$(pkg-config --variable=prefix Qt6Core 2>/dev/null || true)"
        [ -n "$prefix" ] && add_qt_prefix "$prefix"
        plugindir="$(pkg-config --variable=plugindir Qt6Core 2>/dev/null || true)"
        [ -n "$plugindir" ] && add_directory "${plugindir}/platforms"
    fi

    for prefix in "${qt_prefixes[@]}" "${DF_TOOLCHAIN:-}" "${QTDIR:-}" \
        "${HOME:-}/.local/df-toolchain/usr"; do
        add_qt_prefix "$prefix"
    done

    add_directory /usr/lib64/qt6/plugins/platforms
    add_directory /usr/lib/qt6/plugins/platforms
    add_directory /usr/local/lib64/qt6/plugins/platforms
    add_directory /usr/local/lib/qt6/plugins/platforms
    while IFS= read -r directory; do
        add_directory "$directory"
    done < <(compgen -G "/usr/lib/*-linux-gnu/qt6/plugins/platforms" || true)
    while IFS= read -r directory; do
        add_directory "$directory"
    done < <(compgen -G "${HOME:-/nonexistent}/Qt/*/gcc_64/plugins/platforms" || true)
fi

find_plugin() {
    local name directory
    for directory in "${directories[@]}"; do
        for name in "${plugin_names[@]}"; do
            if [ -f "${directory}/${name}" ]; then
                printf '%s\n' "${directory}/${name}"
                return 0
            fi
        done
    done
    return 1
}

wayland_plugin="$(find_plugin || true)"
offscreen_plugin=""
for directory in "${directories[@]}"; do
    if [ -f "${directory}/libqoffscreen.so" ]; then
        offscreen_plugin="${directory}/libqoffscreen.so"
        break
    fi
done

if [ -n "$wayland_plugin" ]; then
    say "distro: ${distro_name:-unknown} (package manager: ${manager:-unknown})"
    say "Qt 6 Wayland platform plugin: ${wayland_plugin}"
    if [ -n "$offscreen_plugin" ]; then
        say "offscreen fallback plugin: ${offscreen_plugin}"
    else
        say "warning: the offscreen fallback plugin (libqoffscreen.so) was not found"
    fi
    say "OK"
    exit 0
fi

say "distro: ${distro_name:-unknown} (package manager: ${manager:-unknown})"
say "Qt 6 Wayland platform plugin: MISSING"
say "looked for: ${plugin_names[*]}"
for directory in "${directories[@]}"; do
    say "  searched: ${directory}"
done
if [ -n "$offscreen_plugin" ]; then
    say "offscreen fallback plugin: ${offscreen_plugin} (the shell would start degraded)"
else
    say "warning: the offscreen fallback plugin (libqoffscreen.so) is also missing"
fi
say "missing package: ${package:-the distro Qt 6 Wayland platform plugin}"
say "install with: $(install_hint)"
exit 1
