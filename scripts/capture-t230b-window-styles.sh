#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T230b: the measured window-chrome style capture.
#
# Reuses the design-system gallery snapshot driver (the same surfaces the
# visual-regression gate renders) and keeps the 32 pt titlebar, 52 pt unified,
# and 88 pt preference windows from the `WindowStyles` page. Headless; no host
# Wayland session or screenshot tool is required.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
build_dir=${BUILD_DIR:-"$repo/build"}
binary="$build_dir/design-system/gallery/dragonfruit-gallery-app"
out_dir="$repo/docs/captures"

if [[ ! -x "$binary" ]]; then
    echo "gallery binary not found: $binary (run \`make build\`)" >&2
    exit 1
fi

mkdir -p "$out_dir"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t230b-XXXXXX")
trap 'rm -rf "$tmp"' EXIT

QT_QPA_PLATFORM=offscreen \
QT_QUICK_BACKEND=software \
DRAGONFRUIT_GALLERY_SNAPSHOT="$tmp" \
    "$binary" >"$tmp/gallery.log" 2>&1

for scheme in light dark; do
    src="$tmp/windowstyles_${scheme}.png"
    if [[ ! -f "$src" ]]; then
        echo "missing gallery snapshot: $src" >&2
        sed -n '1,80p' "$tmp/gallery.log" >&2 || true
        exit 1
    fi
    cp "$src" "$out_dir/t230b-window-styles-${scheme}.png"
done

cat >"$out_dir/t230b-window-styles.txt" <<EOF
T230b window chrome style capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: design-system gallery WindowStyles page (headless offscreen/software)
light: docs/captures/t230b-window-styles-light.png
dark:  docs/captures/t230b-window-styles-dark.png
rows:
  titlebar    32 pt utility window
  unified     52 pt Files-style window
  preference  88 pt Settings-style window
geometry source: design-system/tokens/tokens.json component.windowStyle
EOF

echo "wrote $out_dir/t230b-window-styles-{light,dark}.png"
echo "wrote $out_dir/t230b-window-styles.txt"
