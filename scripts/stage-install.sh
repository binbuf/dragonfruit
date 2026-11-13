#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Stage one Dragonfruit install tree for packaging (T-32/T164/T165).
#
# This is the single install layout every release asset uses: the session
# entry and user units, the portal discovery files, the CMake/Qt install tree
# (QML modules, cursors, app entries, icons, translations), and the runtime
# binaries. RPM, DEB, and the portable tar.gz consume the same root, so they
# cannot drift from each other.
#
# Usage:
#   scripts/stage-install.sh --destdir DIR [--prefix /usr] [--profile release]
#                            [--no-build] [--no-strip]
#
# `--destdir` is required on purpose: this script must never write into the
# live prefix. `--no-build` stages the already-built tree (CI can build once
# and package several formats).
#
# The dev-only `scripts/native-session.sh install` is deliberately not used
# here. It symlinks target/<profile> and build[-release]/ into /usr, writes a
# dev loader environment, and can preselect the display manager. A package
# carries real binaries and never edits the host login configuration.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

DESTDIR=""
PREFIX="/usr"
PROFILE="release"
BUILD=1
STRIP=1

say() {
    printf 'stage-install: %s\n' "$*"
}

die() {
    printf 'stage-install: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Stage a Dragonfruit install tree for packaging.

Usage:
  scripts/stage-install.sh --destdir DIR [--prefix /usr] [--profile release]
                           [--no-build] [--no-strip]

Options:
  --destdir DIR    staging root (required; the tree is written under DIR/PREFIX)
  --prefix PATH    install prefix inside the staging root (default /usr)
  --profile NAME   release (default) or debug
  --no-build       use the already-built target/<profile> and build[-release]/
  --no-strip       keep debug symbols in the staged binaries
  -h, --help       show this help

The staged layout is what scripts/package.sh turns into a tar.gz, DEB, and
RPM. It installs alongside another desktop and never preselects a display
manager or changes user defaults.
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --destdir)
            [ "$#" -ge 2 ] || die "--destdir needs a value"
            DESTDIR="$2"
            shift 2
            ;;
        --destdir=*)
            DESTDIR="${1#*=}"
            shift
            ;;
        --prefix)
            [ "$#" -ge 2 ] || die "--prefix needs a value"
            PREFIX="$2"
            shift 2
            ;;
        --prefix=*)
            PREFIX="${1#*=}"
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

[ -n "$DESTDIR" ] || die "--destdir is required (staging never writes to the live prefix)"

case "$PROFILE" in
    release)
        CARGO_FLAGS=(--release)
        QT_BUILD_DIR="$root/build-release"
        CMAKE_BUILD_TYPE=Release
        TARGET_DIR="$root/target/release"
        ;;
    debug)
        CARGO_FLAGS=()
        QT_BUILD_DIR="$root/build"
        CMAKE_BUILD_TYPE=Debug
        TARGET_DIR="$root/target/debug"
        ;;
    *)
        die "unknown profile: $PROFILE (expected release or debug)"
        ;;
esac

# Normalise the staging root without requiring the caller to pre-create it.
mkdir -p "$DESTDIR"
DESTDIR="$(cd "$DESTDIR" && pwd)"
PREFIX="${PREFIX%/}"
if [ -z "$PREFIX" ]; then
    PREFIX=""
fi
STAGE_PREFIX="${DESTDIR}${PREFIX}"

if [ "$BUILD" -eq 1 ]; then
    say "building the workspace and Qt side ($PROFILE)"
    cargo build --workspace "${CARGO_FLAGS[@]}"
    cmake -S "$root" -B "$QT_BUILD_DIR" -G Ninja \
        -DCMAKE_BUILD_TYPE="$CMAKE_BUILD_TYPE" \
        -DDF_BUILD_TESTS=OFF
    cmake --build "$QT_BUILD_DIR"
fi

RUST_BINS=(
    dragonfruit-compositor
    dragonfruit-session
    dragonfruit-settingsd
    dragonfruit-menu-broker
    dragonfruit-app-index
    dragonfruit-notifications
    dragonfruit-wallpaperd
    dragonfruit-system-status
    xdg-desktop-portal-dragonfruit
    dragonfruit-pam-helper
)
QT_BINS=(
    dragonfruit-shell
    dragonfruit-files
    dragonfruit-settings
)

qt_binary_source() {
    case "$1" in
        dragonfruit-shell) printf '%s\n' "$QT_BUILD_DIR/shell/src/dragonfruit-shell" ;;
        dragonfruit-files) printf '%s\n' "$QT_BUILD_DIR/apps/files/dragonfruit-files" ;;
        dragonfruit-settings) printf '%s\n' "$QT_BUILD_DIR/apps/settings/dragonfruit-settings" ;;
        *) die "no source rule for $1" ;;
    esac
}

for name in "${RUST_BINS[@]}"; do
    [ -x "$TARGET_DIR/$name" ] \
        || die "missing $TARGET_DIR/$name — build the workspace first"
done
for name in "${QT_BINS[@]}"; do
    src="$(qt_binary_source "$name")"
    [ -x "$src" ] || die "missing $src — build the Qt side first"
done

say "staging session entry, user units, portal data, and the CMake install tree"
"$TARGET_DIR/dragonfruit-session" --install-session "$STAGE_PREFIX" >/dev/null
"$TARGET_DIR/xdg-desktop-portal-dragonfruit" --install-data "$STAGE_PREFIX" >/dev/null
DESTDIR="$DESTDIR" cmake --install "$QT_BUILD_DIR" --prefix "$PREFIX" >/dev/null

bin_dir="$STAGE_PREFIX/bin"
install -d "$bin_dir"
for name in "${RUST_BINS[@]}"; do
    install -m 0755 "$TARGET_DIR/$name" "$bin_dir/$name"
done
for name in "${QT_BINS[@]}"; do
    install -m 0755 "$(qt_binary_source "$name")" "$bin_dir/$name"
done

doc_dir="$STAGE_PREFIX/share/doc/dragonfruit"
install -d "$doc_dir"
install -m 0644 "$root/LICENSE.md" "$doc_dir/LICENSE.md"
install -m 0644 "$root/NOTICE" "$doc_dir/NOTICE"
install -m 0644 "$root/README.md" "$doc_dir/README.md"
rm -rf "$doc_dir/LICENSES"
cp -a "$root/LICENSES" "$doc_dir/LICENSES"
find "$doc_dir/LICENSES" -type f -exec chmod 0644 {} +
find "$doc_dir/LICENSES" -type d -exec chmod 0755 {} +

if [ "$STRIP" -eq 1 ]; then
    if command -v strip >/dev/null 2>&1; then
        say "stripping staged binaries"
        # dragonfruit-session-entry is a shell script, not an ELF object.
        find "$bin_dir" -type f ! -name 'dragonfruit-session-entry' \
            -exec strip --strip-unneeded {} + 2>/dev/null || true
    else
        say "strip not found; keeping unstripped binaries"
    fi
fi

# The manifest is what the portable tar's uninstall.sh uses. Paths are
# absolute under the build prefix so the same manifest works with DESTDIR.
manifest="$doc_dir/manifest.txt"
manifest_tmp="$(mktemp "${TMPDIR:-/tmp}/dragonfruit-manifest.XXXXXX")"
{
    (
        cd "$STAGE_PREFIX"
        find . \( -type f -o -type l \) -printf '%P\n' \
            | while IFS= read -r rel; do
                printf '%s/%s\n' "$PREFIX" "$rel"
            done
    )
    # The manifest itself is written after the walk; list it explicitly so
    # uninstall.sh removes it too.
    printf '%s/share/doc/dragonfruit/manifest.txt\n' "$PREFIX"
} | LC_ALL=C sort -u >"$manifest_tmp"
mv "$manifest_tmp" "$manifest"

say "staged $(wc -l <"$manifest") files under $STAGE_PREFIX"
