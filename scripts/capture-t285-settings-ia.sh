#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T285 reduced Settings IA headless visual check.
#
# Renders the real shell with the deterministic settings fixture and writes:
#
#   docs/captures/t285-sidebar-order-<scheme>.png
#   docs/captures/t285-status-row-<scheme>.png
#   docs/captures/t285-alias-workspace-overview-<scheme>.png
#   docs/captures/t285-alias-menu-bar-controls-<scheme>.png
#   docs/captures/t285-alias-internet-accounts-<scheme>.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no bridge host. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixture.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t285_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t285-settings-ia: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for scheme in light dark; do
    for image in \
        docs/captures/t285-sidebar-order-$scheme.png \
        docs/captures/t285-status-row-$scheme.png \
        docs/captures/t285-alias-workspace-overview-$scheme.png \
        docs/captures/t285-alias-menu-bar-controls-$scheme.png \
        docs/captures/t285-alias-internet-accounts-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t285-settings-ia: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
