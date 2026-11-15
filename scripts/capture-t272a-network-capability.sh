#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T272a headless Network/VPN capability visual check.
#
# Renders the real Settings shell offscreen with the deterministic Network and
# VPN fixtures and writes light/dark stills for the firewall backend states and
# the VPN Add-path capability states:
#
#   docs/captures/t272a-firewall-ufw-normal-user-<scheme>.png
#   docs/captures/t272a-firewall-iptables-only-<scheme>.png
#   docs/captures/t272a-firewall-nftables-normal-user-<scheme>.png
#   docs/captures/t272a-firewall-no-manager-<scheme>.png
#   docs/captures/t272a-vpn-add-plugin-<scheme>.png
#   docs/captures/t272a-vpn-wireguard-form-<scheme>.png
#   docs/captures/t272a-vpn-no-editor-<scheme>.png
#   docs/captures/t272a-vpn-no-add-<scheme>.png
#   docs/captures/t272a-vpn-needs-setup-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_network"
INPUT="apps/settings/tests/tst_settings_t272a_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t272a-network-capability: $BIN not built — run make build" >&2
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
        docs/captures/t272a-firewall-ufw-normal-user-$scheme.png \
        docs/captures/t272a-firewall-iptables-only-$scheme.png \
        docs/captures/t272a-firewall-nftables-normal-user-$scheme.png \
        docs/captures/t272a-firewall-no-manager-$scheme.png \
        docs/captures/t272a-vpn-add-plugin-$scheme.png \
        docs/captures/t272a-vpn-wireguard-form-$scheme.png \
        docs/captures/t272a-vpn-no-editor-$scheme.png \
        docs/captures/t272a-vpn-no-add-$scheme.png \
        docs/captures/t272a-vpn-needs-setup-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t272a-network-capability: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t272a-network-capability: wrote t272a-*.png"
