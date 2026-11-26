#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T275 Accessibility headless visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture and the accessibility delegate fixture, and writes every section, the
# delegate rows, and the absent-AT-SPI structure in light and dark at the
# measured 740×820 window:
#
#   docs/captures/t275-accessibility-vision-<scheme>.png
#   docs/captures/t275-accessibility-display-<scheme>.png
#   docs/captures/t275-accessibility-zoom-<scheme>.png
#   docs/captures/t275-accessibility-motion-<scheme>.png
#   docs/captures/t275-accessibility-read-speak-<scheme>.png
#   docs/captures/t275-accessibility-hearing-<scheme>.png
#   docs/captures/t275-accessibility-motor-<scheme>.png
#   docs/captures/t275-accessibility-speech-<scheme>.png
#   docs/captures/t275-accessibility-shortcut-<scheme>.png
#   docs/captures/t275-accessibility-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_accessibility" ]; then
    echo "capture-t275-accessibility: pane tests not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

run_capture() {
    local fixture="$1"
    local input="$2"
    dbus-run-session -- env \
        QT_QPA_PLATFORM=offscreen \
        QT_QUICK_BACKEND=software \
        DF_SETTINGS_FIXTURE=1 \
        DF_ACCESSIBILITY_FIXTURE="$fixture" \
        DF_ACCESSIBILITY_DELEGATE_FIXTURE=present \
        QML2_IMPORT_PATH="$BUILD/qml" \
        "$TESTS/tst_settings_accessibility" \
        -input "$input"
}

run_capture 1 apps/settings/tests/tst_settings_t275_capture.qml
run_capture absent apps/settings/tests/tst_settings_t275_absence_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t275-accessibility-list-$scheme.png \
        docs/captures/t275-accessibility-vision-$scheme.png \
        docs/captures/t275-accessibility-display-$scheme.png \
        docs/captures/t275-accessibility-zoom-$scheme.png \
        docs/captures/t275-accessibility-motion-$scheme.png \
        docs/captures/t275-accessibility-read-speak-$scheme.png \
        docs/captures/t275-accessibility-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t275-accessibility: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t275-accessibility: wrote t275-accessibility-*.png"
