#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T278 Wallpaper/Screen Saver/Notifications/Focus/Lock Screen headless visual
# check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# wallpaper, and notifications fixtures and writes every pane plus the per-app
# sheet in light and dark at the measured 740×820 window. A second pass with no
# settings daemon writes the absence stills:
#
#   docs/captures/t278-wallpaper-<scheme>.png
#   docs/captures/t278-screen-saver-<scheme>.png
#   docs/captures/t278-notifications-<scheme>.png
#   docs/captures/t278-notifications-sheet-<scheme>.png
#   docs/captures/t278-focus-<scheme>.png
#   docs/captures/t278-lock-screen-<scheme>.png
#   docs/captures/t278-<pane>-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_notifications" ] \
        || [ ! -x "$TESTS/tst_settings_absence" ]; then
    echo "capture-t278-personalization: pane tests not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_WALLPAPER_FIXTURE=1 \
    DF_NOTIFICATIONS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_notifications" \
    -input apps/settings/tests/tst_settings_t278_capture.qml

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_absence" \
    -input apps/settings/tests/tst_settings_t278_absence_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t278-wallpaper-$scheme.png \
        docs/captures/t278-screen-saver-$scheme.png \
        docs/captures/t278-notifications-$scheme.png \
        docs/captures/t278-notifications-sheet-$scheme.png \
        docs/captures/t278-focus-$scheme.png \
        docs/captures/t278-lock-screen-$scheme.png \
        docs/captures/t278-wallpaper-absence-$scheme.png \
        docs/captures/t278-screen-saver-absence-$scheme.png \
        docs/captures/t278-notifications-absence-$scheme.png \
        docs/captures/t278-focus-absence-$scheme.png \
        docs/captures/t278-lock-screen-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t278-personalization: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t278-personalization: wrote t278-*.png"
