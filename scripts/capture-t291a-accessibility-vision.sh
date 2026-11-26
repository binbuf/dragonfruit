#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T291a Accessibility list + Vision visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings and
# accessibility fixtures and writes the mockup intro/big list, the Vision
# sub-pane, the reduce-motion and increase-contrast states, and the AT-SPI
# absence state in light and dark at the measured 740×820 window:
#
#   docs/captures/t291a-accessibility-list-<scheme>.png
#   docs/captures/t291a-accessibility-vision-<scheme>.png
#   docs/captures/t291a-accessibility-vision-reduce-motion-<scheme>.png
#   docs/captures/t291a-accessibility-vision-increase-contrast-<scheme>.png
#   docs/captures/t291a-accessibility-absence-list-<scheme>.png
#   docs/captures/t291a-accessibility-absence-vision-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_accessibility"

if [ ! -x "$BIN" ]; then
    echo "capture-t291a-accessibility-vision: $BIN not built — run make build" >&2
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
        "$BIN" -input "$input"
}

run_capture 1 apps/settings/tests/tst_settings_t291a_capture.qml
run_capture absent apps/settings/tests/tst_settings_t291a_absence_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t291a-accessibility-list-$scheme.png \
        docs/captures/t291a-accessibility-vision-$scheme.png \
        docs/captures/t291a-accessibility-vision-reduce-motion-$scheme.png \
        docs/captures/t291a-accessibility-vision-increase-contrast-$scheme.png \
        docs/captures/t291a-accessibility-absence-list-$scheme.png \
        docs/captures/t291a-accessibility-absence-vision-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t291a-accessibility-vision: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
