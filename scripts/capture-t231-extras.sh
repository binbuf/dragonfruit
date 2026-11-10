#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T231: menu-bar extras registry and Date & Time/calendar stills.
#
# Headless offscreen/software, 1000x560 stage at 1x. The capture case lives in
# `shell/tests/tst_menubar.qml` and skips without DF_MENUBAR_CAPTURE_DIR, so a
# normal ctest run stays side-effect free. This runner sets the directory and
# checks the artifacts on disk (Qt's QML image wrapper can report a save
# failure on this build even when the file lands).
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
        >"$out_dir/t231-extras-capture.log" 2>&1
)

for name in clock calendar removed overflow tray; do
    test -s "$out_dir/t231-extras-$name.png" || {
        echo "missing capture: $out_dir/t231-extras-$name.png" >&2
        tail -40 "$out_dir/t231-extras-capture.log" >&2 || true
        exit 1
    }
done

cat >"$out_dir/t231-extras.txt" <<EOF
T231 menu-bar extras registry and Date & Time/calendar capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: shell/tests/tst_menubar.qml (headless offscreen/software)
stage: 1000x560 pt, 1x
artifacts:
  docs/captures/t231-extras-clock.png       seconds + 24-hour clock
  docs/captures/t231-extras-calendar.png    Date & Time panel open
  docs/captures/t231-extras-removed.png     Wi-Fi item removed by policy
  docs/captures/t231-extras-overflow.png    third-party items in overflow
  docs/captures/t231-extras-tray.png        a third-party item's DBusMenu
EOF

echo "wrote $out_dir/t231-extras-{clock,calendar,removed,overflow,tray}.png"
