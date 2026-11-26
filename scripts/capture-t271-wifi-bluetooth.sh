#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T271 headless Wi-Fi and Bluetooth visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# Wi-Fi, and Bluetooth fixtures, and writes light/dark stills for the live,
# scanning, connected, off, details, hotspot, and absence Wi-Fi states plus the
# live, device-sheet, and absence Bluetooth states:
#
#   docs/captures/t271-wifi-live-<scheme>.png
#   docs/captures/t271-wifi-scanning-<scheme>.png
#   docs/captures/t271-wifi-connected-<scheme>.png
#   docs/captures/t271-wifi-details-<scheme>.png
#   docs/captures/t271-wifi-hotspot-<scheme>.png
#   docs/captures/t271-wifi-off-<scheme>.png
#   docs/captures/t271-wifi-absence-<scheme>.png
#   docs/captures/t271-bluetooth-live-<scheme>.png
#   docs/captures/t271-bluetooth-device-<scheme>.png
#   docs/captures/t271-bluetooth-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_wifi"
INPUT="apps/settings/tests/tst_settings_t271_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t271-wifi-bluetooth: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_WIFI_FIXTURE=1 \
    DF_BLUETOOTH_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for scheme in light dark; do
    for image in \
        docs/captures/t271-wifi-live-$scheme.png \
        docs/captures/t271-wifi-scanning-$scheme.png \
        docs/captures/t271-wifi-connected-$scheme.png \
        docs/captures/t271-wifi-details-$scheme.png \
        docs/captures/t271-wifi-hotspot-$scheme.png \
        docs/captures/t271-wifi-off-$scheme.png \
        docs/captures/t271-wifi-absence-$scheme.png \
        docs/captures/t271-bluetooth-live-$scheme.png \
        docs/captures/t271-bluetooth-device-$scheme.png \
        docs/captures/t271-bluetooth-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t271-wifi-bluetooth: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t271-wifi-bluetooth: wrote t271-*.png"
