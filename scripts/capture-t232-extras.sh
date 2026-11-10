#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# T232: Wi-Fi, Bluetooth, Battery, Sound, Search, and Quick Settings extras.
#
# Headless offscreen/software, 1000x560 stage at 1x. The capture case lives in
# `shell/tests/tst_menubar.qml` and skips without DF_MENUBAR_CAPTURE_DIR, so a
# normal ctest run stays side-effect free. Each extra is captured in light and
# dark; the Quick Settings extra also captures the Control Center panel.
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
        >"$out_dir/t232-extras-capture.log" 2>&1
)

for name in wifi-light wifi-dark bluetooth-light bluetooth-dark \
            battery-light battery-dark volume-light volume-dark \
            search-light search-dark quick-settings-bar-light \
            quick-settings-bar-dark quick-settings-light quick-settings-dark; do
    test -s "$out_dir/t232-extras-$name.png" || {
        echo "missing capture: $out_dir/t232-extras-$name.png" >&2
        tail -40 "$out_dir/t232-extras-capture.log" >&2 || true
        exit 1
    }
done

cat >"$out_dir/t232-extras.txt" <<EOF
T232 menu-bar extras parity capture
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: shell/tests/tst_menubar.qml (headless offscreen/software)
stage: 1000x560 pt, 1x
artifacts:
  docs/captures/t232-extras-wifi-{light,dark}.png             current, known, other networks, details, settings
  docs/captures/t232-extras-bluetooth-{light,dark}.png        power, known devices, discovery, nearby, settings
  docs/captures/t232-extras-battery-{light,dark}.png          charge, health, power mode, settings
  docs/captures/t232-extras-volume-{light,dark}.png           output device, volume, mute, settings
  docs/captures/t232-extras-search-{light,dark}.png           Search status item
  docs/captures/t232-extras-quick-settings-bar-{light,dark}.png  Quick Settings status item
  docs/captures/t232-extras-quick-settings-{light,dark}.png   Control Center module state
EOF

echo "wrote $out_dir/t232-extras-*.png"
