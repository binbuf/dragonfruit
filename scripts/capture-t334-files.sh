#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# T334: Files Open With and menu publication stills — the picker with the
# installed handlers and its default selection, a chosen non-default handler,
# the Set as Default result, and the File menu rendered from the same published
# model the menu-broker receives. Headless offscreen/software, 900x600 pt at
# 1x.
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
log="$out_dir/t334-files-capture.log"

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
        >"$log" 2>&1
)

for name in open-with open-with-selected open-with-default file-menu; do
    test -s "$out_dir/t334-files-$name.png" || {
        echo "missing capture: $out_dir/t334-files-$name.png" >&2
        tail -40 "$log" >&2 || true
        exit 1
    }
done

paths="$out_dir/t330-files-paths.txt"
fixture=$(sed -n 's/^mutation fixture: //p' "$paths" 2>/dev/null | head -1)

cat >"$out_dir/t334-files.txt" <<EOF
T334 Files Open With and menu publication capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: apps/files/tests/tst_files_shell.qml (headless offscreen/software)
window: 900x600 pt, 1x
fixture: ${fixture:-unknown}
scenarios:
  open with:           the picker for alpha.txt with the two installed
                       handlers; Text Editor is the checked default
  open with selected:  Code Editor chosen; the Open button targets it
  open with default:   Set as Default applied; the notice confirms the
                       write and the default marker moves to Code Editor
  file menu:           the File menu rendered from FilesMenu.publishedModel,
                       the same rows the menu-broker resolves for the focused
                       Files window (native tier)
artifacts:
  docs/captures/t334-files-open-with.png
  docs/captures/t334-files-open-with-selected.png
  docs/captures/t334-files-open-with-default.png
  docs/captures/t334-files-file-menu.png
EOF

echo "wrote $out_dir/t334-files-{open-with,open-with-selected,open-with-default,file-menu}.png"
