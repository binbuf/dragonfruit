#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# T333: Files tabs and New Window stills — two tabs with independent
# locations, the second tab active, one tab after closing the second, and the
# second window surface a New Window shows. Headless offscreen/software,
# 900x600 pt at 1x.
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
log="$out_dir/t333-files-capture.log"

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

for name in two-tabs second-tab new-window second-window; do
    test -s "$out_dir/t333-files-$name.png" || {
        echo "missing capture: $out_dir/t333-files-$name.png" >&2
        tail -40 "$log" >&2 || true
        exit 1
    }
done

paths="$out_dir/t330-files-paths.txt"
fixture=$(sed -n 's/^mutation fixture: //p' "$paths" 2>/dev/null | head -1)

cat >"$out_dir/t333-files.txt" <<EOF
T333 Files tabs and New Window capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: apps/files/tests/tst_files_shell.qml (headless offscreen/software)
window: 900x600 pt, 1x
mutation fixture (temporary tree): ${fixture:-unknown}
scenarios:
  two tabs:   TabsCapture/1 and TabsCapture/2 open, the first tab active;
              the tab bar sits below the unified toolbar
  second tab: the second tab is active and lists TabsCapture/2 (two.txt)
  new window: the second tab closed; one tab remains and the bar stays open
  second window: a second Files window surface on TabsCapture/2 with its one
              tab and the bar shown (a single tab hides the bar unless it is
              shown). Production New Window launches a second process; the
              offscreen harness renders the same tabbed shell twice because it
              cannot photograph another process's surface.
artifacts:
  docs/captures/t333-files-two-tabs.png
  docs/captures/t333-files-second-tab.png
  docs/captures/t333-files-new-window.png
  docs/captures/t333-files-second-window.png
EOF

echo "wrote $out_dir/t333-files-{two-tabs,second-tab,new-window,second-window}.png"
