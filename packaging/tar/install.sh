#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Install the portable Dragonfruit bundle under /usr.
#
# This is intentionally a file copy, not a desktop installer:
#   * it never removes another desktop;
#   * it never preselects a display manager or arms autologin;
#   * it never changes default apps or other user settings.
#
# Choose Dragonfruit at the login screen after installing. User-level setup
# (default apps, optional services, autologin) belongs in the running session.
#
# Usage:
#   sudo ./install.sh
#   DESTDIR=/tmp/root ./install.sh   # stage a tree without touching the host

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
destdir="${DESTDIR:-}"

if [ -z "$destdir" ] && [ "$(id -u)" -ne 0 ]; then
    exec sudo -- "$0" "$@"
fi

die() {
    printf 'install.sh: %s\n' "$*" >&2
    exit 1
}

[ -x "$root/usr/bin/dragonfruit-compositor" ] \
    || die "this does not look like a Dragonfruit bundle (missing usr/bin/dragonfruit-compositor)"
[ -f "$root/usr/share/wayland-sessions/dragonfruit.desktop" ] \
    || die "this does not look like a Dragonfruit bundle (missing the session entry)"

mkdir -p "$destdir/usr"
# Preserve the staged modes; the files are owned by the invoking user unless
# this script is running through sudo.
cp -a --no-preserve=ownership,context "$root/usr/." "$destdir/usr/"

if [ -z "$destdir" ]; then
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q /usr/share/applications >/dev/null 2>&1 || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
    fi
fi

if [ -n "$destdir" ]; then
    uninstall_hint="DESTDIR=$destdir $root/uninstall.sh"
else
    uninstall_hint="sudo $root/uninstall.sh"
fi

cat <<EOF
Dragonfruit is installed under ${destdir}/usr.

Log out, then choose "Dragonfruit" at the login screen. Your current desktop
is still installed; this bundle does not change the display-manager selection,
autologin, default applications, or remove anything.

To uninstall, run:
  $uninstall_hint
EOF
