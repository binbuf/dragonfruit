#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T267 headless System Settings accessibility/scale visual check.
#
# Renders the real shell with the deterministic settings fixture and writes:
#
#   docs/captures/t267-settings-a11y-baseline.png
#   docs/captures/t267-settings-a11y-high-contrast.png
#   docs/captures/t267-settings-a11y-reduced-transparency.png
#   docs/captures/t267-settings-a11y-reduced-motion.png
#   docs/captures/t267-settings-a11y-rtl.png
#   docs/captures/t267-settings-a11y-2x-740x820.png
#   docs/captures/t267-settings-a11y-2x-720x480.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no bridge host. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixture.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t267_capture.qml"
INPUT_2X="apps/settings/tests/tst_settings_t267_capture_2x.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t267-settings-accessibility: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

run() {
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
        "$@"
}

run "$BIN" -input "$INPUT"
run env QT_SCALE_FACTOR=2 "$BIN" -input "$INPUT_2X"

for image in docs/captures/t267-settings-a11y-baseline.png \
             docs/captures/t267-settings-a11y-high-contrast.png \
             docs/captures/t267-settings-a11y-reduced-transparency.png \
             docs/captures/t267-settings-a11y-reduced-motion.png \
             docs/captures/t267-settings-a11y-rtl.png \
             docs/captures/t267-settings-a11y-2x-740x820.png \
             docs/captures/t267-settings-a11y-2x-720x480.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t267-settings-accessibility: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t267-settings-accessibility: wrote t267-settings-a11y-*.png"
