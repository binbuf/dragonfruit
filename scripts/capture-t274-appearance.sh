#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T274 Appearance headless visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture and the icon/cursor theme provider fixture, and writes the
# consolidated groups, the long-label layout, the Reduce Transparency
# override, and the live preview in light and dark at the measured 740×820
# window:
#
#   docs/captures/t274-appearance-<scheme>.png
#   docs/captures/t274-appearance-preview-<scheme>.png
#   docs/captures/t274-appearance-long-string-<scheme>.png
#   docs/captures/t274-appearance-reduce-transparency-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_appearance" ]; then
    echo "capture-t274-appearance: pane tests not built — run make build" >&2
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
    "$TESTS/tst_settings_appearance" \
    -input apps/settings/tests/tst_settings_t274_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t274-appearance-$scheme.png \
        docs/captures/t274-appearance-preview-$scheme.png \
        docs/captures/t274-appearance-long-string-$scheme.png \
        docs/captures/t274-appearance-reduce-transparency-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t274-appearance: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t274-appearance: wrote t274-appearance-*.png"
