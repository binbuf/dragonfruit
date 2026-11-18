#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T288 connectivity visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# Wi-Fi, Bluetooth, NetworkManager, and VPN fixtures, and writes light/dark
# stills for every connectivity pane in its connected, empty, and absent
# states:
#
#   docs/captures/t288-wifi-{connected,empty,absent}-<scheme>.png
#   docs/captures/t288-bluetooth-{connected,empty,absent}-<scheme>.png
#   docs/captures/t288-network-{connected,no-firewall,absent}-<scheme>.png
#   docs/captures/t288-vpn-{configured,empty,absent}-<scheme>.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no host bridge. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_network"
INPUT="apps/settings/tests/tst_settings_t288_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t288-settings-connectivity: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_WIFI_FIXTURE=1 \
    DF_BLUETOOTH_FIXTURE=1 \
    DF_NETWORK_FIXTURE=1 \
    DF_VPN_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for scheme in light dark; do
    for name in \
        wifi-connected wifi-empty wifi-absent \
        bluetooth-connected bluetooth-empty bluetooth-absent \
        network-connected network-no-firewall network-absent \
        vpn-configured vpn-empty vpn-absent; do
        image="docs/captures/t288-$name-$scheme.png"
        if [ ! -s "$image" ]; then
            echo "capture-t288-settings-connectivity: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
