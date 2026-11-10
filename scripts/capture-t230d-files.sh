#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T230d: Files window chrome, status bar, toolbar-hidden mode and Get Info
# inspector stills. Headless offscreen/software, 900×600 pt at 1×.
#
# The capture case is part of the Files QML suite and skips without
# DF_FILES_CAPTURE_DIR, so a normal ctest run stays side-effect free. This
# runner sets the directory and checks the artifacts on disk (Qt's QML
# QImage.save wrapper reports false on this build even when the file lands).
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
build_dir=${BUILD_DIR:-"$repo/build"}
binary="$build_dir/apps/files/tests/tst_files_shell"
out_dir="$repo/docs/captures"

if [[ ! -x "$binary" ]]; then
    echo "Files test binary not found: $binary (run \`make build\`)" >&2
    exit 1
fi

mkdir -p "$out_dir"

(
    cd "$repo/apps/files/tests"
    env QT_QPA_PLATFORM=offscreen \
        QT_QUICK_BACKEND=software \
        DF_FILES_FIXTURE=1 \
        DF_FILES_CAPTURE_DIR="$out_dir" \
        QML2_IMPORT_PATH="$build_dir/qml" \
        "$binary" -input "$repo/apps/files/tests/tst_files_shell.qml" \
        >"$out_dir/t230d-files-capture.log" 2>&1
)

for name in chrome-light chrome-dark toolbar-hidden-light toolbar-hidden-dark \
            get-info-light get-info-dark get-info-multi; do
    test -s "$out_dir/t230d-files-$name.png" || {
        echo "missing capture: $out_dir/t230d-files-$name.png" >&2
        tail -40 "$out_dir/t230d-files-capture.log" >&2 || true
        exit 1
    }
done

cat >"$out_dir/t230d-files.txt" <<EOF
T230d Files window chrome and Get Info capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: apps/files/tests/tst_files_shell.qml (headless offscreen/software)
window: 900x600 pt, 1x
geometry: 52 pt toolbar, 198 pt sidebar, 30 pt path bar, 28 pt status bar
toolbar hidden: 64 pt content inset, sidebar hidden, full-width status bar
get-info: 328x1034 pt, 32 pt chrome, seven single-column sections
artifacts:
  docs/captures/t230d-files-chrome-{light,dark}.png
  docs/captures/t230d-files-toolbar-hidden-{light,dark}.png
  docs/captures/t230d-files-get-info-{light,dark,multi}.png
EOF

echo "wrote $out_dir/t230d-files-{chrome,toolbar-hidden}-{light,dark}.png"
echo "wrote $out_dir/t230d-files-get-info-{light,dark,multi}.png"
