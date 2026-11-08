#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T229a headless Settings visual check.
#
# Renders the real Settings shell offscreen with the deterministic fixtures and
# writes:
#
#   docs/captures/t229a-wallpaper-light.png          current preview + links
#   docs/captures/t229a-wallpaper-dark.png           the same in dark
#   docs/captures/t229a-notifications-light.png      prefs + per-app list
#   docs/captures/t229a-notifications-dark.png       the same in dark
#   docs/captures/t229a-notifications-app-light.png  the per-app detail sheet
#   docs/captures/t229a-notifications-app-dark.png   the same in dark
#   docs/captures/t229a-focus-light.png              modes/schedule/shortcut
#   docs/captures/t229a-focus-dark.png               the same in dark
#   docs/captures/t229a-lock-screen-light.png        timing/display/login
#   docs/captures/t229a-lock-screen-dark.png         the same in dark
#   docs/captures/t229a-wallpaper-offline.png        no provider: available soon
#   docs/captures/t229a-notifications-absence.png    no bridge host
#   docs/captures/t229a-focus-absence.png            no bridge host
#   docs/captures/t229a-lock-screen-absence.png      no settingsd
#
# Host: Linux. Build: offscreen QPA + software scene graph, private session
# bus, no real notification service/wallpaper provider/settingsd. This is the
# repeatable half of the live check; a host Wayland session can open the real
# app with the same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t229a_capture.qml"
ABSENCE_INPUT="apps/settings/tests/tst_settings_t229a_absence_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t229a-settings-panes: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_NOTIFICATIONS_FIXTURE=1 \
    DF_WALLPAPER_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

# The absence pass has no settingsd, no bridge host, and no wallpaper provider.
dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$ABSENCE_INPUT"

for image in docs/captures/t229a-wallpaper-light.png \
             docs/captures/t229a-wallpaper-dark.png \
             docs/captures/t229a-notifications-light.png \
             docs/captures/t229a-notifications-dark.png \
             docs/captures/t229a-notifications-app-light.png \
             docs/captures/t229a-notifications-app-dark.png \
             docs/captures/t229a-focus-light.png \
             docs/captures/t229a-focus-dark.png \
             docs/captures/t229a-lock-screen-light.png \
             docs/captures/t229a-lock-screen-dark.png \
             docs/captures/t229a-wallpaper-offline.png \
             docs/captures/t229a-notifications-absence.png \
             docs/captures/t229a-focus-absence.png \
             docs/captures/t229a-lock-screen-absence.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t229a-settings-panes: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t229a-settings-panes: wrote t229a-*.png"
