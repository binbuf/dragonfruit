#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T263 headless System Settings window visual check.
#
# Renders the real shell with the deterministic settings fixture and writes:
#
#   docs/captures/t263-settings-740x820-light.png
#   docs/captures/t263-settings-740x820-dark.png
#   docs/captures/t263-settings-720x480-light.png
#   docs/captures/t263-settings-720x480-dark.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no bridge host. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixture.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t263_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t263-settings-window: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t263-settings-740x820-light.png \
             docs/captures/t263-settings-740x820-dark.png \
             docs/captures/t263-settings-720x480-light.png \
             docs/captures/t263-settings-720x480-dark.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t263-settings-window: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t263-settings-window: wrote t263-settings-*.png"
