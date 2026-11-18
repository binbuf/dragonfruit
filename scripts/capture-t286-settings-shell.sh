#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T286 Settings shell geometry headless visual check.
#
# Renders the real shell with the deterministic settings fixture and writes:
#
#   docs/captures/t286-shell-740x820-<scheme>.png
#   docs/captures/t286-shell-720x480-<scheme>.png
#   docs/captures/t286-sidebar-selection-<scheme>.png
#   docs/captures/t286-sidebar-spacers-<scheme>.png
#   docs/captures/t286-search-results-<scheme>.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no bridge host. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixture.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t286_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t286-settings-shell: $BIN not built — run make build" >&2
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
        docs/captures/t286-shell-740x820-$scheme.png \
        docs/captures/t286-shell-720x480-$scheme.png \
        docs/captures/t286-sidebar-selection-$scheme.png \
        docs/captures/t286-sidebar-spacers-$scheme.png \
        docs/captures/t286-search-results-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t286-settings-shell: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
