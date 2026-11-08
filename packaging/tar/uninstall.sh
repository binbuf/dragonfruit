#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Remove the files installed by the portable Dragonfruit bundle.
#
# It reads the manifest written by the staging step, so it only removes paths
# this bundle installed. User settings under ~/.config/dragonfruit and
# ~/.local/state/dragonfruit are deliberately kept.
#
# Usage:
#   sudo ./uninstall.sh
#   DESTDIR=/tmp/root ./uninstall.sh

set -euo pipefail

destdir="${DESTDIR:-}"

if [ -z "$destdir" ] && [ "$(id -u)" -ne 0 ]; then
    exec sudo -- "$0" "$@"
fi

die() {
    printf 'uninstall.sh: %s\n' "$*" >&2
    exit 1
}

manifest="${destdir}/usr/share/doc/dragonfruit/manifest.txt"
[ -f "$manifest" ] \
    || die "manifest not found at $manifest; was the bundle installed with install.sh?"

removed=0
while IFS= read -r path; do
    case "$path" in
        /usr/*)
            if [ -e "${destdir}${path}" ] || [ -L "${destdir}${path}" ]; then
                rm -f -- "${destdir}${path}"
                removed=$((removed + 1))
            fi
            ;;
    esac
done <"$manifest"

# Remove only directories this bundle created; never remove a non-empty
# shared directory such as /usr/share/applications or /usr/bin.
for tree in \
    /usr/share/doc/dragonfruit \
    /usr/share/dragonfruit \
    /usr/share/icons/capitaine-cursors \
    /usr/share/icons/hicolor/scalable/apps \
    /usr/share/xdg-desktop-portal/portals \
    /usr/share/xdg-desktop-portal \
    /usr/share/dbus-1/services \
    /usr/lib/systemd/user \
    /usr/share/wayland-sessions; do
    [ -d "${destdir}${tree}" ] \
        && find "${destdir}${tree}" -depth -type d -empty -delete 2>/dev/null || true
done
for dir in \
    /usr/share/applications \
    /usr/share/icons/hicolor/scalable \
    /usr/share/icons/hicolor \
    /usr/share/dbus-1 \
    /usr/lib/systemd \
    /usr/bin; do
    rmdir --ignore-fail-on-non-empty "${destdir}${dir}" 2>/dev/null || true
done

cat <<EOF
Removed $removed Dragonfruit files.

Kept on purpose:
  ~/.config/dragonfruit/
  ~/.local/state/dragonfruit/

Log out before removing the session from the display manager (the bundle
never changed the login selection).
EOF
