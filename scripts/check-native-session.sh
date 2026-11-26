#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T343: offline acceptance check for the native install transaction and the
# session entry's stale-target handling.
#
# Everything runs under a temporary prefix and a temporary HOME with fake
# build artifacts, a fake `cmake`, a fake `systemctl`, and a fake session
# binary. It never writes to /usr, never uses root, and never talks to the
# host's systemd. The checks are:
#
#   1. a first install stages and writes the complete tree;
#   2. a second install is idempotent (same XCURSOR_PATH, no duplicates);
#   3. a conflict is refused before the prefix is touched;
#   4. a simulated mid-write failure restores the previous install;
#   5. an already-active target is stopped before the socket is removed;
#   6. a failed target start stops the target;
#   7. `recover` removes exactly what the install wrote and keeps unrelated
#      files.
#
# Usage: scripts/check-native-session.sh [--keep]

set -uo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

keep=0
[ "${1:-}" = "--keep" ] && keep=1

work=$(mktemp -d "${TMPDIR:-/tmp}/df-native-check.XXXXXX")
cleanup() {
    if [ "$keep" -eq 1 ]; then
        printf 'check-native-session: kept %s\n' "$work"
    else
        rm -rf "$work"
    fi
}
trap cleanup EXIT

pass=0
fail=0
ok() {
    printf 'ok   %s\n' "$*"
    pass=$((pass + 1))
}
bad() {
    printf 'FAIL %s\n' "$*"
    fail=$((fail + 1))
}
check_rc() {
    local want="$1" got="$2" label="$3"
    if [ "$got" -eq "$want" ]; then
        ok "$label"
    else
        bad "$label (rc $got, want $want)"
    fi
}
check_eq() {
    local want="$1" got="$2" label="$3"
    if [ "$want" = "$got" ]; then
        ok "$label"
    else
        bad "$label"
        printf '  want: %s\n  got:  %s\n' "$want" "$got"
    fi
}
check_file() {
    if [ -e "$1" ]; then ok "$2"; else bad "$2 ($1 is missing)"; fi
}
check_absent() {
    if [ -e "$1" ]; then bad "$2 ($1 exists)"; else ok "$2"; fi
}

# --- fake build tree ------------------------------------------------------

build_root="$work/build"
for profile in release; do
    mkdir -p "$build_root/target/$profile"
    for name in \
        dragonfruit dragonfruit-session dragonfruit-compositor dragonfruit-shell \
        dragonfruit-settingsd dragonfruit-menu-broker dragonfruit-app-index \
        dragonfruit-notifications dragonfruit-system-status dragonfruit-wallpaperd \
        xdg-desktop-portal-dragonfruit dragonfruit-pam-helper dragonfruit-files \
        dragonfruit-settings; do
        printf '#!/bin/sh\nexit 0\n' >"$build_root/target/$profile/$name"
        chmod 755 "$build_root/target/$profile/$name"
    done
done
mkdir -p "$build_root/build-release/shell/src" "$build_root/build-release/apps/files" \
    "$build_root/build-release/apps/settings"
for path in shell/src/dragonfruit-shell apps/files/dragonfruit-files apps/settings/dragonfruit-settings; do
    printf '#!/bin/sh\nexit 0\n' >"$build_root/build-release/$path"
    chmod 755 "$build_root/build-release/$path"
done
for module in Dragonfruit Dragonfruit/Settings Dragonfruit/Files; do
    mkdir -p "$build_root/build-release/qml/$module"
    printf 'module %s\n' "$(basename "$module")" >"$build_root/build-release/qml/$module/qmldir"
done

# The session and portal stubs write the files the real binaries install.
cat >"$build_root/target/release/dragonfruit-session" <<'SH'
#!/bin/sh
case "$1" in
    --install-session)
        prefix="$2"
        mkdir -p "$prefix/bin" "$prefix/share/wayland-sessions" \
            "$prefix/lib/systemd/user" "$prefix/share/dragonfruit/wallpapers"
        printf '#!/bin/sh\nexit 0\n' >"$prefix/bin/dragonfruit-session-entry"
        chmod 755 "$prefix/bin/dragonfruit-session-entry"
        printf '[Desktop Entry]\nName=Dragonfruit\nExec=/usr/bin/dragonfruit-session-entry\n' \
            >"$prefix/share/wayland-sessions/dragonfruit.desktop"
        printf '[Unit]\nDescription=Dragonfruit session\n' \
            >"$prefix/lib/systemd/user/dragonfruit-session.target"
        printf 'wallpaper\n' >"$prefix/share/dragonfruit/wallpapers/Default.jpg"
        ;;
    --print-env)
        printf 'XDG_CURRENT_DESKTOP=dragonfruit\n'
        printf 'XDG_SESSION_TYPE=wayland\n'
        printf 'WAYLAND_DISPLAY=dragonfruit-wayland\n'
        printf 'GTK_CSD=0\n'
        ;;
    --sync-toolkit) exit 0 ;;
esac
exit 0
SH
chmod 755 "$build_root/target/release/dragonfruit-session"

cat >"$build_root/target/release/xdg-desktop-portal-dragonfruit" <<'SH'
#!/bin/sh
case "$1" in
    --install-data)
        prefix="$2"
        mkdir -p "$prefix/share/xdg-desktop-portal/portals" \
            "$prefix/share/xdg-desktop-portal" \
            "$prefix/share/dbus-1/services"
        printf '[portal]\nDBusName=org.freedesktop.impl.portal.desktop.dragonfruit\n' \
            >"$prefix/share/xdg-desktop-portal/portals/dragonfruit.portal"
        printf '[preferred]\ndefault=dragonfruit\n' \
            >"$prefix/share/xdg-desktop-portal/portals.conf"
        printf '[D-BUS Service]\nName=org.freedesktop.impl.portal.desktop.dragonfruit\n' \
            >"$prefix/share/dbus-1/services/org.freedesktop.impl.portal.desktop.dragonfruit.service"
        ;;
esac
exit 0
SH
chmod 755 "$build_root/target/release/xdg-desktop-portal-dragonfruit"

# --- fake commands --------------------------------------------------------

fakebin="$work/fakebin"
mkdir -p "$fakebin"

cat >"$fakebin/cmake" <<'SH'
#!/bin/sh
prefix=""
previous=""
for arg in "$@"; do
    [ "$previous" = "--prefix" ] && prefix="$arg"
    previous="$arg"
done
[ -n "$prefix" ] || exit 2
mkdir -p "$prefix/share/dragonfruit/qml/Dragonfruit" \
    "$prefix/share/icons/capitaine-cursors"
printf 'module Dragonfruit\n' >"$prefix/share/dragonfruit/qml/Dragonfruit/qmldir"
printf 'Rectangle {}\n' >"$prefix/share/dragonfruit/qml/Dragonfruit/Thing.qml"
printf '[Icon Theme]\nName=capitaine-cursors\n' \
    >"$prefix/share/icons/capitaine-cursors/index.theme"
exit 0
SH
chmod 755 "$fakebin/cmake"

cat >"$fakebin/sudo" <<'SH'
#!/bin/sh
echo "check-native-session: sudo must not run" >&2
exit 99
SH
chmod 755 "$fakebin/sudo"

cat >"$fakebin/loginctl" <<'SH'
#!/bin/sh
# No sessions in the offline check.
[ "$1" = "list-sessions" ] && exit 0
exit 0
SH
chmod 755 "$fakebin/loginctl"

cat >"$fakebin/systemctl" <<'SH'
#!/usr/bin/env bash
log="${FAKE_SYSTEMCTL_LOG:?}"
printf 'systemctl %s\n' "$*" >>"$log"

cmd=""
unit=""
quiet=0
for arg in "$@"; do
    case "$arg" in
        --user) ;;
        --quiet) quiet=1 ;;
        --no-pager) ;;
        -*) ;;
        is-active|start|stop|reset-failed|daemon-reload|import-environment|show-environment|status|cat|restart)
            [ -z "$cmd" ] && cmd="$arg" ;;
        *)
            [ -z "$unit" ] && unit="$arg"
            ;;
    esac
done

state_file="${FAKE_TARGET_STATE:?}"
socket="${FAKE_SOCKET:?}"

case "$cmd" in
    is-active)
        case "$unit" in
            dragonfruit-session.target)
                state="$(cat "$state_file" 2>/dev/null || echo inactive)"
                [ "$quiet" -eq 1 ] && { [ "$state" = active ] && exit 0 || exit 3; }
                printf '%s\n' "$state"
                [ "$state" = active ] && exit 0 || exit 3
                ;;
            dragonfruit-compositor.service)
                ticks="$(cat "${FAKE_COMPOSITOR_TICKS:?}" 2>/dev/null || echo 0)"
                if [ "${ticks:-0}" -gt 0 ]; then
                    printf '%s\n' "$((ticks - 1))" >"$FAKE_COMPOSITOR_TICKS"
                    [ "$quiet" -eq 1 ] || printf 'active\n'
                    exit 0
                fi
                [ "$quiet" -eq 1 ] || printf 'inactive\n'
                exit 3
                ;;
            *)
                [ "$quiet" -eq 1 ] || printf 'inactive\n'
                exit 3
                ;;
        esac
        ;;
    start)
        if [ "${FAKE_START_FAIL:-0}" = "1" ]; then
            printf 'systemctl start failed (simulated)\n' >&2
            exit 1
        fi
        printf 'active\n' >"$state_file"
        : >"$socket"
        exit 0
        ;;
    stop)
        present=absent
        [ -e "$socket" ] && present=present
        printf 'stop %s socket=%s\n' "$unit" "$present" >>"$log"
        [ "$unit" = "dragonfruit-session.target" ] && printf 'inactive\n' >"$state_file"
        exit 0
        ;;
    *)
        exit 0
        ;;
esac
SH
chmod 755 "$fakebin/systemctl"

# --- fake home and environment -------------------------------------------

home="$work/home"
mkdir -p "$home/.config" "$home/.local/state" "$home/.local/share" "$work/runtime"
export HOME="$home"
export XDG_CONFIG_HOME="$home/.config"
export XDG_STATE_HOME="$home/.local/state"
export XDG_DATA_HOME="$home/.local/share"
export XDG_RUNTIME_DIR="$work/runtime"
unset XDG_CURRENT_DESKTOP
export PATH="$fakebin:$PATH"
export LD_LIBRARY_PATH=""
export DF_NATIVE_PREFIX="$work/prefix"
export DF_NATIVE_BUILD_ROOT="$build_root"
export DF_NATIVE_PROFILE=release
export DF_NATIVE_ENV_FILE="$home/.config/dragonfruit/native.env"
export DF_NATIVE_INSTALL_MANIFEST="$XDG_STATE_HOME/dragonfruit/native-install.json"
export DF_TOOLCHAIN="$work/toolchain"
export DF_DEVROOT="$work/devroot"
export DF_NATIVE_TEST_FAIL_WRITE=0
export FAKE_SYSTEMCTL_LOG="$work/systemctl.log"
export FAKE_TARGET_STATE="$work/target.state"
export FAKE_SOCKET="$work/runtime/dragonfruit-wayland"
export FAKE_COMPOSITOR_TICKS="$work/compositor.ticks"
mkdir -p "$DF_TOOLCHAIN/lib64" "$DF_DEVROOT"
: >"$FAKE_SYSTEMCTL_LOG"
printf 'inactive\n' >"$FAKE_TARGET_STATE"

prefix="$DF_NATIVE_PREFIX"
env_file="$DF_NATIVE_ENV_FILE"
manifest="$DF_NATIVE_INSTALL_MANIFEST"

native() {
    bash "$repo_root/scripts/native-session.sh" "$@"
}

tree_listing() {
    (
        cd "$1" || exit 1
        find . -mindepth 1 -printf '%P|%y|%l\n' | LC_ALL=C sort
        find . -type f -print0 | LC_ALL=C sort -z | xargs -0 -r sha256sum
    )
}

tree_fingerprint() {
    tree_listing "$1" | sha256sum | cut -d' ' -f1
}

# --- 1. first install -----------------------------------------------------

printf '== first install ==\n'
native install --no-build --no-deps >"$work/install1.log" 2>&1
check_rc 0 "$?" "first install succeeds"
check_file "$prefix/bin/dragonfruit-shell" "binary link farm installed"
check_file "$prefix/bin/dragonfruit-session-entry" "session entry installed"
check_file "$prefix/share/wayland-sessions/dragonfruit.desktop" "session desktop installed"
check_file "$prefix/lib/systemd/user/dragonfruit-session.target" "session target installed"
check_file "$prefix/share/dragonfruit/qml/Dragonfruit/qmldir" "QML modules installed"
check_file "$prefix/share/xdg-desktop-portal/portals/dragonfruit.portal" "portal data installed"
check_file "$manifest" "install manifest written"
if [ -L "$prefix/bin/dragonfruit-shell" ]; then
    ok "the binary is a dev symlink"
else
    bad "the binary is a dev symlink"
fi

# A file that is not part of the install must survive both install and recover.
printf 'host file\n' >"$prefix/share/keep-me.txt"

# --- 2. idempotent XCURSOR_PATH ------------------------------------------

printf '== idempotent environment ==\n'
x1="$(sed -n 's/^XCURSOR_PATH="\(.*\)"$/\1/p' "$env_file")"
if [ -n "$x1" ]; then ok "XCURSOR_PATH is written"; else bad "XCURSOR_PATH is written"; fi
XCURSOR_PATH="$x1" native install --no-build --no-deps >"$work/install2.log" 2>&1
check_rc 0 "$?" "second install succeeds"
x2="$(sed -n 's/^XCURSOR_PATH="\(.*\)"$/\1/p' "$env_file")"
check_eq "$x1" "$x2" "two runs produce the same XCURSOR_PATH"
deduped="$(printf '%s\n' "$x2" | tr ':' '\n' | awk 'NF' | awk '!seen[$0]++' | paste -sd: -)"
check_eq "$x2" "$deduped" "XCURSOR_PATH has no duplicate entries"

# --- 3. conflict leaves the previous install intact ----------------------

printf '== conflict is refused before writing ==\n'
rm -f "$prefix/bin/dragonfruit-shell"
printf 'packaged binary\n' >"$prefix/bin/dragonfruit-shell"
before="$(tree_fingerprint "$prefix")"
native install --no-build --no-deps >"$work/conflict.log" 2>&1
check_rc 1 "$?" "a conflicting binary is refused"
check_eq "packaged binary" "$(cat "$prefix/bin/dragonfruit-shell")" "the conflicting file is untouched"
after="$(tree_fingerprint "$prefix")"
check_eq "$before" "$after" "a refused install leaves the whole prefix intact"
if grep -q -- "--force" "$work/conflict.log"; then
    ok "the refusal points at --force"
else
    bad "the refusal points at --force"
fi
rm -f "$prefix/bin/dragonfruit-shell"
native install --no-build --no-deps >"$work/install3.log" 2>&1
check_rc 0 "$?" "install recovers after the conflict is removed"

# --- 3b. forced replacement keeps a backup -------------------------------

printf '== forced replacement ==\n'
rm -f "$prefix/bin/dragonfruit-shell"
printf 'packaged binary\n' >"$prefix/bin/dragonfruit-shell"
native install --no-build --no-deps --force >"$work/force.log" 2>&1
check_rc 0 "$?" "a forced install replaces the conflicting file"
check_file "$prefix/bin/dragonfruit-shell.pre-dfdev" "the replaced file is backed up"
check_eq "packaged binary" "$(cat "$prefix/bin/dragonfruit-shell.pre-dfdev")" \
    "the backup keeps the original content"
python3 "$repo_root/scripts/native-install.py" uninstall \
    --prefix "$prefix" --manifest "$manifest" >/dev/null 2>&1
check_rc 0 "$?" "uninstall succeeds"
check_eq "packaged binary" "$(cat "$prefix/bin/dragonfruit-shell")" \
    "uninstall restores the forced backup"
rm -f "$prefix/bin/dragonfruit-shell" "$manifest"
native install --no-build --no-deps >"$work/install3b.log" 2>&1
check_rc 0 "$?" "install succeeds again after the forced backup is restored"

# --- 4. a failed write rolls back ----------------------------------------

printf '== mid-write failure rolls back ==\n'
printf 'previous install\n' >"$prefix/share/wayland-sessions/dragonfruit.desktop"
rm -f "$prefix/share/icons/capitaine-cursors/index.theme"
before="$(tree_listing "$prefix")"
DF_NATIVE_TEST_FAIL_WRITE=1 native install --no-build --no-deps >"$work/rollback.log" 2>&1
check_rc 1 "$?" "a simulated write failure fails the install"
check_eq "previous install" "$(cat "$prefix/share/wayland-sessions/dragonfruit.desktop")" \
    "the replaced file is restored"
check_absent "$prefix/share/icons/capitaine-cursors/index.theme" \
    "a newly created file is removed again"
after="$(tree_listing "$prefix")"
if [ "$before" = "$after" ]; then
    ok "the rollback restores the whole prefix"
else
    bad "the rollback restores the whole prefix"
    diff <(printf '%s\n' "$before") <(printf '%s\n' "$after") | sed 's/^/  /'
fi
check_absent "$prefix/.df-native-stage" "no staging leftovers under the prefix"
native install --no-build --no-deps >"$work/install4.log" 2>&1
check_rc 0 "$?" "install succeeds after the rollback"

# --- 5. stale active target is stopped before the socket cleanup ---------

printf '== stale target ==\n'
entry_home="$work/entry-home"
mkdir -p "$entry_home"
: >"$FAKE_SYSTEMCTL_LOG"
printf 'active\n' >"$FAKE_TARGET_STATE"
printf '1\n' >"$FAKE_COMPOSITOR_TICKS"
: >"$FAKE_SOCKET"
HOME="$entry_home" XDG_RUNTIME_DIR="$work/runtime" \
    DF_SESSION_BIN="$build_root/target/release/dragonfruit-session" \
    DF_SYSTEMCTL="$fakebin/systemctl" \
    DF_SESSION_SOCKET=dragonfruit-wayland \
    DF_SESSION_START_TIMEOUT=5 \
    bash "$repo_root/services/session/dragonfruit-session-entry" \
    >"$work/entry.log" 2>&1
check_rc 0 "$?" "the session entry completes with a stale active target"
if grep -q '^stop dragonfruit-session.target socket=present$' "$FAKE_SYSTEMCTL_LOG"; then
    ok "the stale target is stopped while the socket still exists"
else
    bad "the stale target is stopped while the socket still exists"
    cat "$FAKE_SYSTEMCTL_LOG"
fi
if grep -q '^systemctl --user start dragonfruit-session.target$' "$FAKE_SYSTEMCTL_LOG"; then
    ok "the target is started again after the cleanup"
else
    bad "the target is started again after the cleanup"
fi
check_absent "$FAKE_SOCKET" "the entry removes its hand-off socket on exit"

# --- 6. a failed start stops the target ----------------------------------

printf '== failed start ==\n'
: >"$FAKE_SYSTEMCTL_LOG"
printf 'inactive\n' >"$FAKE_TARGET_STATE"
HOME="$entry_home" XDG_RUNTIME_DIR="$work/runtime" \
    DF_SESSION_BIN="$build_root/target/release/dragonfruit-session" \
    DF_SYSTEMCTL="$fakebin/systemctl" \
    DF_SESSION_SOCKET=dragonfruit-wayland \
    DF_SESSION_START_TIMEOUT=5 \
    FAKE_START_FAIL=1 \
    bash "$repo_root/services/session/dragonfruit-session-entry" \
    >"$work/entry-fail.log" 2>&1
check_rc 1 "$?" "a failed start fails the entry"
if grep -q '^stop dragonfruit-session.target socket=' "$FAKE_SYSTEMCTL_LOG"; then
    ok "a failed start stops the target"
else
    bad "a failed start stops the target"
    cat "$FAKE_SYSTEMCTL_LOG"
fi

# --- 7. a tampered manifest is refused -----------------------------------

printf '== manifest safety ==\n'
cat >"$work/bad-manifest.json" <<EOF
{"version": 1, "prefix": "$prefix", "files": {
  "../../etc/passwd": {},
  "/etc/passwd": {},
  "usr/bin/sh": {}
}}
EOF
python3 "$repo_root/scripts/native-install.py" uninstall \
    --prefix "$prefix" --manifest "$work/bad-manifest.json" \
    >"$work/bad-manifest.log" 2>&1
check_rc 1 "$?" "a manifest outside the install roots is refused"

# --- 8. recover removes exactly the install ------------------------------

printf '== recover uninstalls ==\n'
native recover --no-restart-dm >"$work/recover.log" 2>&1
check_rc 0 "$?" "recover succeeds"
check_absent "$prefix/share/wayland-sessions/dragonfruit.desktop" "the session desktop is removed"
check_absent "$prefix/bin/dragonfruit-session-entry" "the session entry is removed"
check_absent "$prefix/bin/dragonfruit-shell" "the binary link farm is removed"
check_absent "$manifest" "the install manifest is removed"
check_file "$prefix/share/keep-me.txt" "an unrelated host file is kept"
check_absent "$env_file" "the runtime environment is removed"

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
