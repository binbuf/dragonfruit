#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T289 Battery/Displays visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# battery, and display fixtures, and writes light/dark stills for both panes in
# their present, sheet-open, projector, and absent states:
#
#   docs/captures/t289-battery-{present,options,absent}-<scheme>.png
#   docs/captures/t289-displays-{present,advanced,projector,absent}-<scheme>.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no host bridge. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_displays"
INPUT="apps/settings/tests/tst_settings_t289_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t289-settings-battery-displays: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_BATTERY_FIXTURE=1 \
    DF_DISPLAY_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for scheme in light dark; do
    for name in \
        battery-present battery-options battery-absent \
        displays-present displays-advanced displays-projector displays-absent; do
        image="docs/captures/t289-$name-$scheme.png"
        if [ ! -s "$image" ]; then
            echo "capture-t289-settings-battery-displays: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
