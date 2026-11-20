#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T277 Quick Settings and Menu Bar headless visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture and writes the categorized modules, pages, live preview, Add Controls
# sheet, Reset, menu-bar controls, per-control options, third-party list, and
# Reset stills in light and dark at the measured 740×820 window:
#
#   docs/captures/t277-quick-settings-modules-<scheme>.png
#   docs/captures/t277-quick-settings-pages-<scheme>.png
#   docs/captures/t277-quick-settings-preview-<scheme>.png
#   docs/captures/t277-quick-settings-add-controls-<scheme>.png
#   docs/captures/t277-quick-settings-reset-<scheme>.png
#   docs/captures/t277-quick-settings-reset-restored-<scheme>.png
#   docs/captures/t277-menu-bar-controls-<scheme>.png
#   docs/captures/t277-menu-bar-options-<scheme>.png
#   docs/captures/t277-menu-bar-third-party-<scheme>.png
#   docs/captures/t277-menu-bar-reset-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_menu_bar_controls" ]; then
    echo "capture-t277-quick-settings-menu-bar: pane test not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_menu_bar_controls" \
    -input apps/settings/tests/tst_settings_t277_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t277-quick-settings-modules-$scheme.png \
        docs/captures/t277-quick-settings-pages-$scheme.png \
        docs/captures/t277-quick-settings-preview-$scheme.png \
        docs/captures/t277-quick-settings-add-controls-$scheme.png \
        docs/captures/t277-quick-settings-reset-$scheme.png \
        docs/captures/t277-quick-settings-reset-restored-$scheme.png \
        docs/captures/t277-menu-bar-controls-$scheme.png \
        docs/captures/t277-menu-bar-options-$scheme.png \
        docs/captures/t277-menu-bar-third-party-$scheme.png \
        docs/captures/t277-menu-bar-reset-$scheme.png \
        docs/captures/t277-menu-bar-reset-restored-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t277-quick-settings-menu-bar: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t277-quick-settings-menu-bar: wrote t277-quick-settings-*.png and t277-menu-bar-*.png"
