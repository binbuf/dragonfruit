#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T293a Menu Bar visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture and writes the restyled Menu Bar pane at the measured 740×820 window
# in light and dark:
#
#   docs/captures/t293a-menu-bar-main-<scheme>.png       (Behaviour, OS glyph,
#                                                         first controls)
#   docs/captures/t293a-menu-bar-os-glyph-<scheme>.png   (the radio group with
#                                                         both actual-size previews)
#   docs/captures/t293a-menu-bar-controls-<scheme>.png   (the controls list)
#   docs/captures/t293a-menu-bar-footer-<scheme>.png     (the third-party list
#                                                         and the four actions)
#   docs/captures/t293a-menu-bar-reset-<scheme>.png      (Reset restored the
#                                                         Dragonfruit default)
#   docs/captures/t293a-menu-bar-advanced-<scheme>.png   (the Advanced Menu Bar
#                                                         sheet)
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_menu_bar" ]; then
    echo "capture-t293a-settings-menu-bar: pane test not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_menu_bar" \
    -input apps/settings/tests/tst_settings_t293a_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t293a-menu-bar-main-$scheme.png \
        docs/captures/t293a-menu-bar-os-glyph-$scheme.png \
        docs/captures/t293a-menu-bar-controls-$scheme.png \
        docs/captures/t293a-menu-bar-footer-$scheme.png \
        docs/captures/t293a-menu-bar-reset-$scheme.png \
        docs/captures/t293a-menu-bar-advanced-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t293a-settings-menu-bar: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
