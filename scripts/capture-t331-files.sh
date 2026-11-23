#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T331: Files undo stills — the Edit menu naming the next undo step, and the
# listing after Cmd+Z put the name back. Headless offscreen/software, 900x600
# pt at 1x.
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
log="$out_dir/t331-files-capture.log"

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

for name in edit-menu undone; do
    test -s "$out_dir/t331-files-$name.png" || {
        echo "missing capture: $out_dir/t331-files-$name.png" >&2
        tail -40 "$log" >&2 || true
        exit 1
    }
done

paths="$out_dir/t331-files-paths.txt"
fixture=$(sed -n 's/^t331 undo fixture: //p' "$paths" 2>/dev/null | head -1)

cat >"$out_dir/t331-files.txt" <<EOF
T331 Files undo capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: apps/files/tests/tst_files_shell.qml (headless offscreen/software)
window: 900x600 pt, 1x
undo fixture (temporary tree): ${fixture:-unknown}
scenarios:
  edit menu: UndoCapture/alpha.txt renamed to beta.txt; the Edit menu's
             Undo row names the next step ("Undo Rename "alpha.txt"")
  undone:    Cmd+Z (the menu's Undo row) renamed it back; the inline notice
             states "Undid "Rename "alpha.txt""
artifacts:
  docs/captures/t331-files-edit-menu.png
  docs/captures/t331-files-undone.png
EOF

echo "wrote $out_dir/t331-files-{edit-menu,undone}.png"
