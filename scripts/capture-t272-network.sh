#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T272 headless Network/VPN visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# NetworkManager, and VPN fixtures, and writes light/dark stills for the
# service list, per-service details, firewall options, the optional VPN pane,
# the Add Service sheet, and the NetworkManager-absent state (with the firewall
# still usable):
#
#   docs/captures/t272-network-services-<scheme>.png
#   docs/captures/t272-network-details-<scheme>.png
#   docs/captures/t272-network-firewall-<scheme>.png
#   docs/captures/t272-network-add-service-<scheme>.png
#   docs/captures/t272-network-vpn-<scheme>.png
#   docs/captures/t272-network-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_network"
INPUT="apps/settings/tests/tst_settings_t272_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t272-network: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_NETWORK_FIXTURE=1 \
    DF_VPN_FIXTURE=1 \
    DF_WIFI_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for scheme in light dark; do
    for image in \
        docs/captures/t272-network-services-$scheme.png \
        docs/captures/t272-network-details-$scheme.png \
        docs/captures/t272-network-firewall-$scheme.png \
        docs/captures/t272-network-add-service-$scheme.png \
        docs/captures/t272-network-vpn-$scheme.png \
        docs/captures/t272-network-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t272-network: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t272-network: wrote t272-network-*.png"
