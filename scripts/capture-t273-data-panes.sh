#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T273 headless Battery/Storage/Displays visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# and adapter fixtures, and writes light/dark stills for each pane's live,
# empty, and absence states at the measured 740×820 window:
#
#   docs/captures/t273-battery-live-<scheme>.png
#   docs/captures/t273-battery-empty-<scheme>.png
#   docs/captures/t273-battery-absence-<scheme>.png
#   docs/captures/t273-storage-live-<scheme>.png
#   docs/captures/t273-storage-empty-<scheme>.png
#   docs/captures/t273-storage-absence-<scheme>.png
#   docs/captures/t273-displays-single-<scheme>.png
#   docs/captures/t273-displays-dual-<scheme>.png
#   docs/captures/t273-displays-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_battery" ] \
        || [ ! -x "$TESTS/tst_settings_storage" ] \
        || [ ! -x "$TESTS/tst_settings_displays" ]; then
    echo "capture-t273-data-panes: pane tests not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

run() {
    local binary="$1"
    local input="$2"
    shift 2
    dbus-run-session -- env \
        QT_QPA_PLATFORM=offscreen \
        QT_QUICK_BACKEND=software \
        DF_SETTINGS_FIXTURE=1 \
        QML2_IMPORT_PATH="$BUILD/qml" \
        "$@" \
        "$TESTS/$binary" -input "$input"
}

run tst_settings_battery apps/settings/tests/tst_settings_t273_battery_capture.qml \
    DF_BATTERY_FIXTURE=1
run tst_settings_storage apps/settings/tests/tst_settings_t273_storage_capture.qml \
    DF_STORAGE_FIXTURE=1 DF_GENERAL_FIXTURE=1
run tst_settings_displays apps/settings/tests/tst_settings_t273_displays_capture.qml \
    DF_DISPLAY_FIXTURE=1

for scheme in light dark; do
    for image in \
        docs/captures/t273-battery-live-$scheme.png \
        docs/captures/t273-battery-empty-$scheme.png \
        docs/captures/t273-battery-absence-$scheme.png \
        docs/captures/t273-storage-live-$scheme.png \
        docs/captures/t273-storage-empty-$scheme.png \
        docs/captures/t273-storage-absence-$scheme.png \
        docs/captures/t273-displays-single-$scheme.png \
        docs/captures/t273-displays-dual-$scheme.png \
        docs/captures/t273-displays-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t273-data-panes: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t273-data-panes: wrote t273-{battery,storage,displays}-*.png"
