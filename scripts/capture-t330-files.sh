#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# T330: Files copy/cut/paste stills, including the conflict sheet. Headless
# offscreen/software, 900x600 pt at 1x.
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
log="$out_dir/t330-files-capture.log"

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

for name in paste-copy paste-conflict paste-keep-both paste-cut; do
    test -s "$out_dir/t330-files-$name.png" || {
        echo "missing capture: $out_dir/t330-files-$name.png" >&2
        tail -40 "$log" >&2 || true
        exit 1
    }
done

paths="$out_dir/t330-files-paths.txt"
fixture=$(sed -n 's/^mutation fixture: //p' "$paths" 2>/dev/null | head -1)
view_fixture=$(sed -n 's/^view fixture: //p' "$paths" 2>/dev/null | head -1)

cat >"$out_dir/t330-files.txt" <<EOF
T330 Files copy, cut, paste and conflict capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: apps/files/tests/tst_files_shell.qml (headless offscreen/software)
window: 900x600 pt, 1x
view fixture: ${view_fixture:-unknown}
mutation fixture (temporary tree): ${fixture:-unknown}
scenarios:
  copy: CaptureSrc (report.txt + Folder/leaf.txt) pasted into CaptureDst
  conflict: CaptureSrc/report.txt pasted over CaptureConflict/report.txt;
            the sheet offers Replace / Skip / Keep Both / Cancel
  keep both: CaptureConflict lists report.txt and report.txt 2
  cut: CaptureCut/move-me.txt cut and pasted into CaptureCutDest
status bar: shows "Copying N of M" while a batch runs, then the summary
            ("Copied N" / "Moved N" / "Skipped N" / "N failed")
artifacts:
  docs/captures/t330-files-paste-copy.png
  docs/captures/t330-files-paste-conflict.png
  docs/captures/t330-files-paste-keep-both.png
  docs/captures/t330-files-paste-cut.png
EOF

echo "wrote $out_dir/t330-files-paste-{copy,conflict,keep-both,cut}.png"
