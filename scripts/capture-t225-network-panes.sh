#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T225 headless Settings Network/Wi-Fi/VPN/firewall visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# Wi-Fi, Network, and VPN fixtures, on a private session bus with no
# system-status bridge host, and writes:
#
#   docs/captures/t225-wifi-pane.png              live Wi-Fi pane
#   docs/captures/t225-wifi-join.png              joined guest network
#   docs/captures/t225-wifi-forget.png            forgotten saved network
#   docs/captures/t225-network-firewall-on.png    firewall toggled on
#   docs/captures/t225-network-firewall-off.png   firewall toggled off
#   docs/captures/t225-vpn-connected.png          connected VPN
#   docs/captures/t225-wifi-quick-sheet.png       shared Wi-Fi quick sheet
#   docs/captures/t225-wifi-absence.png           NetworkManager absent
#   docs/captures/t225-network-absence.png        NetworkManager absent
#   docs/captures/t225-firewall-unavailable.png   nftables status unavailable
#   docs/captures/t225-vpn-plugin-absence.png     no VPN plugin installed
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). This is the repeatable half of the live
# check; a host Wayland session can open the real app with the same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_wifi"
INPUT="apps/settings/tests/tst_settings_network_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t225-network-panes: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_WIFI_FIXTURE=1 \
    DF_NETWORK_FIXTURE=1 \
    DF_VPN_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t225-wifi-pane.png \
             docs/captures/t225-wifi-join.png \
             docs/captures/t225-wifi-forget.png \
             docs/captures/t225-network-firewall-on.png \
             docs/captures/t225-network-firewall-off.png \
             docs/captures/t225-vpn-connected.png \
             docs/captures/t225-wifi-quick-sheet.png \
             docs/captures/t225-wifi-absence.png \
             docs/captures/t225-network-absence.png \
             docs/captures/t225-firewall-unavailable.png \
             docs/captures/t225-vpn-plugin-absence.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t225-network-panes: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t225-network-panes: wrote t225-*.png"
