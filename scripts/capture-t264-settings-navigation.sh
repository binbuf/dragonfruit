#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T264 headless System Settings navigation visual check.
#
# Renders the real shell with the deterministic settings fixture and writes the
# General -> About -> Back journey plus a row deep link (Language -> Region):
#
#   docs/captures/t264-settings-general-{light,dark}.png
#   docs/captures/t264-settings-about-{light,dark}.png
#   docs/captures/t264-settings-back-{light,dark}.png
#   docs/captures/t264-settings-deep-link-{light,dark}.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no bridge host. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixture.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t264_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t264-settings-navigation: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_UPDATES_FIXTURE=1 \
    DF_GENERAL_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t264-settings-general-light.png \
             docs/captures/t264-settings-general-dark.png \
             docs/captures/t264-settings-about-light.png \
             docs/captures/t264-settings-about-dark.png \
             docs/captures/t264-settings-back-light.png \
             docs/captures/t264-settings-back-dark.png \
             docs/captures/t264-settings-deep-link-light.png \
             docs/captures/t264-settings-deep-link-dark.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t264-settings-navigation: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t264-settings-navigation: wrote t264-settings-*.png"
