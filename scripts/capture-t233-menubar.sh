#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T233: menu-bar auto-hide reveal (mid-slide and settled), the concealed
# resting state, and a measured overflow menu on a narrow display.
#
# Headless offscreen/software, 1000x560 stage at 1x. The capture case lives in
# `shell/tests/tst_menubar.qml` and skips without DF_MENUBAR_CAPTURE_DIR, so a
# normal ctest run stays side-effect free.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
build_dir=${BUILD_DIR:-"$repo/build"}
binary="$build_dir/shell/tests/tst_menubar"
out_dir="$repo/docs/captures"

if [[ ! -x "$binary" ]]; then
    echo "Menu-bar test binary not found: $binary (run \`make build\`)" >&2
    exit 1
fi

mkdir -p "$out_dir"

(
    cd "$repo/shell/tests"
    env QT_QPA_PLATFORM=offscreen \
        QT_QUICK_BACKEND=software \
        DF_MENUBAR_CAPTURE_DIR="$out_dir" \
        QML2_IMPORT_PATH="$build_dir/qml" \
        "$binary" -input "$repo/shell/tests/tst_menubar.qml" \
        >"$out_dir/t233-menubar-capture.log" 2>&1
)

for name in reveal revealed concealed overflow; do
    test -s "$out_dir/t233-menubar-$name.png" || {
        echo "missing capture: $out_dir/t233-menubar-$name.png" >&2
        tail -40 "$out_dir/t233-menubar-capture.log" >&2 || true
        exit 1
    }
done

cat >"$out_dir/t233-menubar.txt" <<EOF
T233 menu-bar visibility capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: shell/tests/tst_menubar.qml (headless offscreen/software)
stage: 1000x560 pt, 1x
axes: auto-hide reveal progress 0 / 0.35 / 1, narrow display 420 pt, overflow open
artifacts:
  docs/captures/t233-menubar-reveal.png      auto-hide reveal mid-slide (35%)
  docs/captures/t233-menubar-revealed.png    settled reveal with the app menu
  docs/captures/t233-menubar-concealed.png   concealed resting state, no strip
  docs/captures/t233-menubar-overflow.png    measured overflow menu on a narrow display
EOF

echo "wrote $out_dir/t233-menubar-*.png"
