#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T266 headless System Settings component visual check.
#
# Renders the real shell/components with the deterministic settings fixture and
# writes:
#
#   docs/captures/t266-settings-components-long-string.png
#   docs/captures/t266-settings-components-popup.png
#   docs/captures/t266-settings-components-dialog.png
#   docs/captures/t266-settings-components-focus.png
#   docs/captures/t266-settings-components-scroll-modes.png
#   docs/captures/t266-settings-components-date-time-timezone.png
#   docs/captures/t266-settings-components-notifications-long-name.png
#   docs/captures/t266-settings-components-lock-screen-dialog.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no bridge host. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixture.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t266_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t266-settings-components: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

QT_QPA_PLATFORM=offscreen \
QT_QUICK_BACKEND=software \
DF_SETTINGS_FIXTURE=1 \
QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t266-settings-components-long-string.png \
             docs/captures/t266-settings-components-popup.png \
             docs/captures/t266-settings-components-dialog.png \
             docs/captures/t266-settings-components-focus.png \
             docs/captures/t266-settings-components-scroll-modes.png \
             docs/captures/t266-settings-components-date-time-timezone.png \
             docs/captures/t266-settings-components-notifications-long-name.png \
             docs/captures/t266-settings-components-lock-screen-dialog.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t266-settings-components: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t266-settings-components: wrote t266-settings-components-*.png"
