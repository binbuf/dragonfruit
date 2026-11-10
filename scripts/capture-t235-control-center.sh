#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T235: the measured Control Center grid, expanded sheets, Edit Controls
# catalogue, pages, resized controls and explicit absence.
#
# Headless offscreen/software, 760x1200 stage at 1x. The capture case lives in
# `shell/tests/tst_controlcenter.qml` and skips without
# DF_CONTROL_CENTER_CAPTURE_DIR, so a normal ctest run stays side-effect free.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
build_dir=${BUILD_DIR:-"$repo/build"}
binary="$build_dir/shell/tests/tst_controlcenter"
out_dir="$repo/docs/captures"

if [[ ! -x "$binary" ]]; then
    echo "Control Center test binary not found: $binary (run \`make build\`)" >&2
    exit 1
fi

mkdir -p "$out_dir"

(
    cd "$repo/shell/tests"
    env QT_QPA_PLATFORM=offscreen \
        QT_QUICK_BACKEND=software \
        DF_CONTROL_CENTER_CAPTURE_DIR="$out_dir" \
        QML2_IMPORT_PATH="$build_dir/qml" \
        "$binary" -input "$repo/shell/tests/tst_controlcenter.qml" \
        >"$out_dir/t235-control-center-capture.log" 2>&1
)

for name in grid-light grid-dark expanded-wifi expanded-bluetooth \
            expanded-focus expanded-now-playing expanded-sound \
            expanded-display expanded-battery page-2 resized absent; do
    test -s "$out_dir/t235-control-center-$name.png" || {
        echo "missing capture: $out_dir/t235-control-center-$name.png" >&2
        tail -60 "$out_dir/t235-control-center-capture.log" >&2 || true
        exit 1
    }
done
for name in light dark; do
    test -s "$out_dir/t235-edit-controls-$name.png" || {
        echo "missing capture: $out_dir/t235-edit-controls-$name.png" >&2
        tail -60 "$out_dir/t235-control-center-capture.log" >&2 || true
        exit 1
    }
done

cat >"$out_dir/t235-control-center.txt" <<EOF
T235 measured Control Center grid + Edit Controls capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: shell/tests/tst_controlcenter.qml (headless offscreen/software)
stage: 760x1200 pt, 1x
geometry:
  module field: 292 pt = 4x64 + 3x12
  spans: 64x64 (1x1), 140x64 (2x1), 140x140 (2x2), 292x64 (4x1)
  expanded sheet: 308 pt wide
  Edit Controls: 736x900 pt, 245 pt sidebar, 22 categories
artifacts:
  docs/captures/t235-control-center-grid-{light,dark}.png
  docs/captures/t235-control-center-expanded-{wifi,bluetooth,focus,now-playing,sound,display,battery}.png
  docs/captures/t235-control-center-page-2.png
  docs/captures/t235-control-center-resized.png
  docs/captures/t235-control-center-absent.png
  docs/captures/t235-edit-controls-{light,dark}.png
EOF

echo "wrote $out_dir/t235-control-center-*.png"
