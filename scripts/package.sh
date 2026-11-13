#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Build the portable release assets from one staged install tree.
#
# Usage:
#   scripts/package.sh [--version VERSION] [--output DIR]
#                      [--profile release|debug]
#                      [--formats tar,deb,rpm] [--no-build] [--no-strip]
#
# Outputs (default: dist/):
#   dragonfruit-<version>-linux-<arch>.tar.gz
#   dragonfruit-desktop_<version>_<deb-arch>.deb
#   dragonfruit-desktop-<version>-1.<arch>.rpm
#   SHA256SUMS
#
# The tar.gz is a portable bundle with install.sh/uninstall.sh. The DEB and
# RPM are binary packages built from the same tree. Flatpak is intentionally
# not produced: a desktop session/compositor cannot be sandboxed as an app.
#
# These are early-testing binary assets, not official distro packages. T164
# and T165 still own Fedora review/COPR, Debian policy, and clean-VM
# validation.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

OUTPUT="$root/dist"
VERSION=""
PROFILE="release"
FORMATS="tar,deb,rpm"
BUILD=1
STRIP=1

say() {
    printf 'package: %s\n' "$*"
}

die() {
    printf 'package: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Build Dragonfruit release assets from one staged install tree.

Usage:
  scripts/package.sh [--version VERSION] [--output DIR]
                     [--profile release|debug]
                     [--formats tar,deb,rpm] [--no-build] [--no-strip]

Options:
  --version VERSION  package version (default: the exact git tag, or a
                     sanitised git describe value)
  --output DIR       output directory (default: dist)
  --profile NAME     release (default) or debug
  --formats LIST     comma-separated tar, deb, rpm, arch (default: tar,deb,rpm)
  --no-build         use the already-built tree
  --no-strip         keep debug symbols
  -h, --help         show this help

Build each native package on the oldest release of its family that still has
the Qt floor (currently Debian 13 / Fedora 43 / Arch): a binary built against
the oldest Qt runs on newer releases of the same family, and the dependency
version is derived from the build host, never hard-coded. The DEB needs
dpkg-deb (dpkg-dev on Debian/Ubuntu). The RPM needs rpmbuild (rpm-build on
Fedora). The Arch package needs bsdtar and zstd. Use --formats tar to build
only the portable bundle on a host without those tools.
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --version)
            [ "$#" -ge 2 ] || die "--version needs a value"
            VERSION="$2"
            shift 2
            ;;
        --version=*)
            VERSION="${1#*=}"
            shift
            ;;
        --output)
            [ "$#" -ge 2 ] || die "--output needs a value"
            OUTPUT="$2"
            shift 2
            ;;
        --output=*)
            OUTPUT="${1#*=}"
            shift
            ;;
        --profile)
            [ "$#" -ge 2 ] || die "--profile needs a value"
            PROFILE="$2"
            shift 2
            ;;
        --profile=*)
            PROFILE="${1#*=}"
            shift
            ;;
        --formats)
            [ "$#" -ge 2 ] || die "--formats needs a value"
            FORMATS="$2"
            shift 2
            ;;
        --formats=*)
            FORMATS="${1#*=}"
            shift
            ;;
        --no-build)
            BUILD=0
            shift
            ;;
        --no-strip)
            STRIP=0
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "unknown argument: $1"
            ;;
    esac
done

case "$PROFILE" in
    release|debug) ;;
    *) die "unknown profile: $PROFILE (expected release or debug)" ;;
esac

if [ -z "$VERSION" ]; then
    if tag="$(git describe --tags --exact-match 2>/dev/null)"; then
        VERSION="${tag#v}"
    elif describe="$(git describe --tags --always --dirty 2>/dev/null)"; then
        VERSION="${describe#v}"
        VERSION="${VERSION//-/.}"
    else
        VERSION="0.0.0"
    fi
fi
VERSION="${VERSION#v}"

case "$VERSION" in
    *[!A-Za-z0-9.+~-]*|'')
        die "invalid version: $VERSION (use letters, digits, '.', '+', '~', or '-')"
        ;;
esac

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        DEB_ARCH=amd64
        ;;
    aarch64|arm64)
        DEB_ARCH=arm64
        ;;
    armv7l|armhf)
        DEB_ARCH=armhf
        ;;
    *)
        DEB_ARCH="$ARCH"
        ;;
esac

# RPM versions cannot contain '-'. A tag like 0.2.0-rc1 becomes 0.2.0~rc1,
# which sorts before the final 0.2.0 release.
RPM_VERSION="${VERSION//-/\~}"
RPM_RELEASE=1

# The binary packages must declare the Qt they were actually built against.
# Each CI job builds on the oldest supported release of its family, so this
# stays honest and never hard-codes a version the build host does not provide.
QT_MIN="$(pkg-config --modversion Qt6Core 2>/dev/null | cut -d. -f1,2 || true)"
[ -n "$QT_MIN" ] || QT_MIN="6.8"

mkdir -p "$OUTPUT"
OUTPUT="$(cd "$OUTPUT" && pwd)"
# Replace assets from a previous version in this output directory; leave any
# unrelated files the user keeps there alone.
rm -f "$OUTPUT"/*.tar.gz "$OUTPUT"/*.deb "$OUTPUT"/*.rpm \
    "$OUTPUT"/*.pkg.tar.zst "$OUTPUT"/SHA256SUMS
STAGE="$OUTPUT/stage"
rm -rf "$STAGE"
mkdir -p "$STAGE"

stage_args=(
    --destdir "$STAGE"
    --prefix /usr
    --profile "$PROFILE"
)
[ "$BUILD" -eq 1 ] || stage_args+=(--no-build)
[ "$STRIP" -eq 1 ] || stage_args+=(--no-strip)

"$root/scripts/stage-install.sh" "${stage_args[@]}"

required=(
    usr/bin/dragonfruit-compositor
    usr/bin/dragonfruit-session
    usr/bin/dragonfruit-session-entry
    usr/bin/dragonfruit-shell
    usr/bin/dragonfruit-settingsd
    usr/bin/dragonfruit-menu-broker
    usr/bin/dragonfruit-app-index
    usr/bin/dragonfruit-notifications
    usr/bin/dragonfruit-wallpaperd
    usr/bin/dragonfruit-system-status
    usr/bin/xdg-desktop-portal-dragonfruit
    usr/bin/dragonfruit-pam-helper
    usr/bin/dragonfruit-files
    usr/bin/dragonfruit-settings
    usr/lib/systemd/user/dragonfruit-session.target
    usr/share/wayland-sessions/dragonfruit.desktop
    usr/share/applications/org.dragonfruit.Files.desktop
    usr/share/applications/org.dragonfruit.Settings.desktop
    usr/share/dragonfruit/qml
    usr/share/dragonfruit/translations
    usr/share/dragonfruit/wallpapers/Default.jpg
    usr/share/icons/capitaine-cursors
    usr/share/dbus-1/services/org.dragonfruit.Files1.service
    usr/share/dbus-1/services/org.freedesktop.impl.portal.desktop.dragonfruit.service
    usr/share/xdg-desktop-portal/portals/dragonfruit.portal
)
for rel in "${required[@]}"; do
    [ -e "$STAGE/$rel" ] || die "staged tree is incomplete: $rel"
done

IFS=',' read -r -a format_list <<<"$FORMATS"
for format in "${format_list[@]}"; do
    case "$format" in
        tar|deb|rpm|arch) ;;
        *) die "unknown format: $format (expected tar, deb, rpm, or arch)" ;;
    esac
done

build_tar() {
    local top="dragonfruit-${VERSION}-linux-${ARCH}"
    local tmp="$OUTPUT/.tar"

    say "building $top.tar.gz"
    rm -rf "$tmp"
    mkdir -p "$tmp/$top"
    cp -a "$STAGE/usr" "$tmp/$top/"
    install -m 0644 "$root/LICENSE.md" "$tmp/$top/LICENSE.md"
    install -m 0644 "$root/NOTICE" "$tmp/$top/NOTICE"
    install -m 0644 "$root/README.md" "$tmp/$top/README.md"
    install -m 0755 "$root/packaging/tar/install.sh" "$tmp/$top/install.sh"
    install -m 0755 "$root/packaging/tar/uninstall.sh" "$tmp/$top/uninstall.sh"
    install -m 0644 "$root/packaging/tar/README.md" "$tmp/$top/INSTALL.md"
    tar -C "$tmp" -czf "$OUTPUT/$top.tar.gz" "$top"
    say "built $top.tar.gz"
    rm -rf "$tmp"
}

build_deb() {
    command -v dpkg-deb >/dev/null 2>&1 \
        || die "dpkg-deb not found; install dpkg-dev or use --formats tar"

    local debroot="$OUTPUT/.deb"
    local debname="dragonfruit-desktop_${VERSION}_${DEB_ARCH}.deb"
    local size
    local depends
    local recommends
    local suggests

    say "building $debname"
    rm -rf "$debroot"
    mkdir -p "$debroot/DEBIAN"
    cp -a "$STAGE/usr" "$debroot/"

    size="$(du -sk "$debroot/usr" | awk '{print $1}')"
    # The QML modules the shell imports at startup are runtime requirements,
    # not suggestions: a minimal install without them cannot load the shell.
    depends="libc6, libgomp1, libstdc++6, libqt6core6t64 (>= $QT_MIN) | libqt6core6 (>= $QT_MIN), libqt6gui6 (>= $QT_MIN), libqt6quick6 (>= $QT_MIN), libqt6qml6 (>= $QT_MIN), libqt6qmlmeta6 (>= $QT_MIN), libqt6qmlmodels6 (>= $QT_MIN), libqt6qmlworkerscript6 (>= $QT_MIN), libqt6opengl6 (>= $QT_MIN), libqt6dbus6 (>= $QT_MIN), libqt6network6 (>= $QT_MIN), qt6-wayland (>= $QT_MIN), qml6-module-qtqml (>= $QT_MIN), qml6-module-qtqml-workerscript (>= $QT_MIN), qml6-module-qtquick (>= $QT_MIN), qml6-module-qtquick-window (>= $QT_MIN), qml6-module-qtquick-shapes (>= $QT_MIN), qml6-module-qtquick-vectorimage (>= $QT_MIN), libwayland-client0, libxkbcommon0, libdrm2, libgbm1, libinput10, libseat1, libudev1, libsystemd0, libdbus-1-3, libegl1, libgl1, libopengl0, libpam0g, systemd, dbus, xdg-desktop-portal, xwayland"
    recommends="xdg-desktop-portal-gtk, network-manager, bluez, pipewire, wireplumber, upower, udisks2, polkitd, flatpak, cups, sane-utils, colord, fprintd, accountsservice, gnome-online-accounts, orca, speech-dispatcher, ibus, fcitx5"  # df-allow-desktop-name: distro package names
    suggests="gnome-software, plasma-discover, snapd, ufw, firewalld, restic, pika-backup, timeshift, tracker, baloo-kf5, recoll, espeak-ng, localsend"  # df-allow-desktop-name: distro package names

    sed \
        -e "s|@VERSION@|$VERSION|g" \
        -e "s|@ARCH@|$DEB_ARCH|g" \
        -e "s|@SIZE@|$size|g" \
        -e "s|@DEPENDS@|$depends|g" \
        -e "s|@RECOMMENDS@|$recommends|g" \
        -e "s|@SUGGESTS@|$suggests|g" \
        "$root/packaging/debian/control.in" >"$debroot/DEBIAN/control"

    install -m 0755 "$root/packaging/debian/postinst" "$debroot/DEBIAN/postinst"
    install -m 0755 "$root/packaging/debian/postrm" "$debroot/DEBIAN/postrm"
    (
        cd "$debroot"
        find usr -type f -print0 \
            | LC_ALL=C sort -z \
            | xargs -0 md5sum >DEBIAN/md5sums
    )
    say "md5sums written"

    say "running dpkg-deb"
    dpkg-deb --build --root-owner-group "$debroot" "$OUTPUT/$debname"
    say "built $debname"
    rm -rf "$debroot"
}

build_rpm() {
    command -v rpmbuild >/dev/null 2>&1 \
        || die "rpmbuild not found; install rpm-build (Fedora) or rpm (Debian/Ubuntu), or use --formats tar"

    local topdir="$OUTPUT/.rpmbuild"
    local rpm_file

    say "building binary RPM"
    rm -rf "$topdir"
    mkdir -p "$topdir/BUILD" "$topdir/BUILDROOT" "$topdir/RPMS" \
        "$topdir/SOURCES" "$topdir/SPECS" "$topdir/SRPMS"

    rpmbuild -bb "$root/packaging/fedora/dragonfruit.spec" \
        --define "_topdir $topdir" \
        --define "df_version $RPM_VERSION" \
        --define "df_release $RPM_RELEASE" \
        --define "df_qt_min $QT_MIN" \
        --define "df_stage $STAGE" \
        --define "df_date $(date -u '+%a %b %d %Y')"

    rpm_file="$(find "$topdir/RPMS" -type f -name '*.rpm' -print -quit)"
    [ -n "$rpm_file" ] || die "rpmbuild produced no RPM"
    cp "$rpm_file" "$OUTPUT/"
    rm -rf "$topdir"
}

build_arch() {
    command -v bsdtar >/dev/null 2>&1 \
        || die "bsdtar not found; install libarchive (bsdtar) or use another format"
    command -v zstd >/dev/null 2>&1 \
        || die "zstd not found; install zstd or use another format"

    local pkgdir="$OUTPUT/.arch"
    # Arch pkgver must not contain '-'; reuse the RPM-sanitised form.
    local pkgname="dragonfruit-desktop-${RPM_VERSION}-1-${ARCH}.pkg.tar.zst"
    local size

    say "building $pkgname"
    rm -rf "$pkgdir"
    mkdir -p "$pkgdir"
    cp -a "$STAGE/usr" "$pkgdir/"
    size="$(du -sb "$pkgdir/usr" | awk '{print $1}')"

    cat >"$pkgdir/.PKGINFO" <<EOF
# Generated by scripts/package.sh. Do not edit.
pkgname = dragonfruit-desktop
pkgbase = dragonfruit-desktop
pkgver = ${RPM_VERSION}-1
pkgdesc = Modern Wayland desktop environment
url = https://github.com/dragonfruit-desktop/dragonfruit
builddate = $(date +%s)
packager = Dragonfruit Developers <dragonfruit@users.noreply.github.com>
size = ${size}
arch = ${ARCH}
license = MIT
depend = glibc
depend = gcc-libs
depend = qt6-base
depend = qt6-declarative
depend = qt6-wayland
depend = wayland
depend = libxkbcommon
depend = libdrm
depend = mesa
depend = libinput
depend = seatd
depend = systemd-libs
depend = dbus
depend = pam
depend = xdg-desktop-portal
depend = xorg-xwayland
EOF

    # Pacman verifies .MTREE on install; makepkg builds it the same way.
    ( cd "$pkgdir" \
        && bsdtar -czf .MTREE --format=mtree \
            --options='!all,use-set,type,uid,gid,mode,time,size,md5,sha256,link' \
            .PKGINFO usr )
    ( cd "$pkgdir" \
        && tar --zstd -cf "$OUTPUT/$pkgname" .PKGINFO .MTREE usr )
    rm -rf "$pkgdir"
}

for format in "${format_list[@]}"; do
    case "$format" in
        tar) build_tar ;;
        deb) build_deb ;;
        rpm) build_rpm ;;
        arch) build_arch ;;
    esac
done

say "writing SHA256SUMS"
(
    cd "$OUTPUT"
    find . -maxdepth 1 -type f \
        \( -name '*.tar.gz' -o -name '*.deb' -o -name '*.rpm' \
           -o -name '*.pkg.tar.zst' \) \
        -printf '%P\n' \
        | LC_ALL=C sort \
        | xargs -r sha256sum >SHA256SUMS
)

say "assets in $OUTPUT"
find "$OUTPUT" -maxdepth 1 -type f \
    \( -name '*.tar.gz' -o -name '*.deb' -o -name '*.rpm' \
       -o -name '*.pkg.tar.zst' -o -name 'SHA256SUMS' \) \
    -printf '  %f\n' | LC_ALL=C sort
