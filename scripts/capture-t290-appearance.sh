#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T290g Appearance visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture and the icon/cursor theme provider fixtures, and writes the landed
# Appearance IA in light and dark at the measured 740×820 window:
#
#   docs/captures/t290-appearance-pane-<scheme>.png
#   docs/captures/t290-appearance-modes-<scheme>.png
#   docs/captures/t290-appearance-glass-<scheme>.png
#   docs/captures/t290-appearance-accent-popup-<scheme>.png
#   docs/captures/t290-appearance-highlight-popup-<scheme>.png
#   docs/captures/t290-appearance-advanced-<scheme>.png
#   docs/captures/t290-appearance-reduce-transparency-<scheme>.png
#   docs/captures/t290-appearance-increase-contrast-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_appearance"
INPUT="apps/settings/tests/tst_settings_t290_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t290-appearance: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_ICON_THEME_FIXTURE=present \
    DF_CURSOR_THEME_FIXTURE=present \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for scheme in light dark; do
    for name in \
        pane modes glass accent-popup highlight-popup advanced \
        reduce-transparency increase-contrast; do
        image="docs/captures/t290-appearance-$name-$scheme.png"
        if [ ! -s "$image" ]; then
            echo "capture-t290-appearance: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
