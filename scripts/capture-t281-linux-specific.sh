#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T281 Linux-specific additions headless visual check.
#
# Renders the owning pane for every new T281 row at the measured 740×820
# window in light and dark, scrolled to the row:
#
#   docs/captures/t281-wallpaper-slideshow-<scheme>.png
#   docs/captures/t281-desktop-window-rules-<scheme>.png
#   docs/captures/t281-keyboard-input-method-<scheme>.png
#   docs/captures/t281-lock-idle-stages-<scheme>.png
#   docs/captures/t281-search-clipboard-<scheme>.png
#   docs/captures/t281-menubar-global-menu-<scheme>.png
#   docs/captures/t281-overview-workspaces-<scheme>.png
#   docs/captures/t281-appearance-window-controls-<scheme>.png
#   docs/captures/t281-displays-night-light-<scheme>.png
#   docs/captures/t281-sound-per-app-<scheme>.png
#   docs/captures/t281-trackpad-gestures-<scheme>.png
#   docs/captures/t281-battery-charge-limit-<scheme>.png
#   docs/captures/t281-network-firewall-<scheme>.png
#   docs/captures/t281-vpn-wireguard-<scheme>.png
#   docs/captures/t281-privacy-usb-accessories-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixture for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_map" ]; then
    echo "capture-t281-linux-specific: pane tests not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_SOUND_FIXTURE=1 \
    DF_BATTERY_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_map" \
    -input apps/settings/tests/tst_settings_t281_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t281-wallpaper-slideshow-$scheme.png \
        docs/captures/t281-desktop-window-rules-$scheme.png \
        docs/captures/t281-keyboard-input-method-$scheme.png \
        docs/captures/t281-lock-idle-stages-$scheme.png \
        docs/captures/t281-search-clipboard-$scheme.png \
        docs/captures/t281-menubar-global-menu-$scheme.png \
        docs/captures/t281-overview-workspaces-$scheme.png \
        docs/captures/t281-appearance-window-controls-$scheme.png \
        docs/captures/t281-displays-night-light-$scheme.png \
        docs/captures/t281-sound-per-app-$scheme.png \
        docs/captures/t281-trackpad-gestures-$scheme.png \
        docs/captures/t281-battery-charge-limit-$scheme.png \
        docs/captures/t281-network-firewall-$scheme.png \
        docs/captures/t281-vpn-wireguard-$scheme.png \
        docs/captures/t281-privacy-usb-accessories-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t281-linux-specific: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
