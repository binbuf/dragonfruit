#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T234: the measured Notification Center ramp, banner stack, widget stack and
# the separate widget gallery, in light and dark.
#
# Headless offscreen/software, 800x480 stage at 1x; the item grabs are the
# measured panel sizes (344 pt Center, 1027x552 pt gallery). The capture case
# lives in `shell/tests/tst_notifications.qml` and skips without
# DF_NOTIFICATIONS_CAPTURE_DIR, so a normal ctest run stays side-effect free.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
build_dir=${BUILD_DIR:-"$repo/build"}
binary="$build_dir/shell/tests/tst_notifications"
out_dir="$repo/docs/captures"

if [[ ! -x "$binary" ]]; then
    echo "Notification test binary not found: $binary (run \`make build\`)" >&2
    exit 1
fi

mkdir -p "$out_dir"

(
    cd "$repo/shell/tests"
    env QT_QPA_PLATFORM=offscreen \
        QT_QUICK_BACKEND=software \
        DF_NOTIFICATIONS_CAPTURE_DIR="$out_dir" \
        QML2_IMPORT_PATH="$build_dir/qml" \
        "$binary" -input "$repo/shell/tests/tst_notifications.qml" \
        >"$out_dir/t234-notifications-capture.log" 2>&1
)

for state in banner center-populated center-empty widget-gallery; do
    for scheme in light dark; do
        test -s "$out_dir/t234-$state-$scheme.png" || {
            echo "missing capture: $out_dir/t234-$state-$scheme.png" >&2
            tail -40 "$out_dir/t234-notifications-capture.log" >&2 || true
            exit 1
        }
    done
done

cat >"$out_dir/t234-notifications.txt" <<EOF
T234 Notification Center / banner / widget gallery capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: shell/tests/tst_notifications.qml (headless offscreen/software)
stage: 800x480 pt, 1x
axes: light/dark; banner 380x132 with actions and a two-banner stack; Center
      344x620 with the banner stack, grouping, a Focus-suppressed row, the live
      164 pt Calendar widget and the lower panel; empty Center; separate
      1027x552 widget gallery (240 pt sidebar, 19 categories, 102x102 gallery
      thumbnails)
measured bands: banner alpha 0.549 / tint 0.976; stack 0.551 / 0.706;
                lower 0.407 / 0.852 (Light; Dark stack 0.507 / 0.550)
artifacts:
  docs/captures/t234-banner-{light,dark}.png
  docs/captures/t234-center-populated-{light,dark}.png
  docs/captures/t234-center-empty-{light,dark}.png
  docs/captures/t234-widget-gallery-{light,dark}.png
EOF

echo "wrote $out_dir/t234-*.png"
