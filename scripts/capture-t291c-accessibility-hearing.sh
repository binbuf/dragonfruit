#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T291c Accessibility Hearing visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings and
# accessibility fixtures and writes the three-row big list, the Hearing
# sub-pane, the reduce-motion/increase-contrast accommodation state, and the
# absent-bridge state at the measured 740×820 window. Six stills, the T291c
# budget:
#
#   docs/captures/t291c-accessibility-list-<scheme>.png
#   docs/captures/t291c-accessibility-hearing-<scheme>.png
#   docs/captures/t291c-accessibility-hearing-contrast-dark.png
#   docs/captures/t291c-accessibility-hearing-absence-light.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_accessibility"

if [ ! -x "$BIN" ]; then
    echo "capture-t291c-accessibility-hearing: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

run_capture() {
    local fixture="$1"
    local delegates="$2"
    local input="$3"
    dbus-run-session -- env \
        QT_QPA_PLATFORM=offscreen \
        QT_QUICK_BACKEND=software \
        DF_SETTINGS_FIXTURE=1 \
        DF_ACCESSIBILITY_FIXTURE="$fixture" \
        DF_ACCESSIBILITY_DELEGATE_FIXTURE="$delegates" \
        QML2_IMPORT_PATH="$BUILD/qml" \
        "$BIN" -input "$input"
}

run_capture 1 present apps/settings/tests/tst_settings_t291c_capture.qml
run_capture absent absent apps/settings/tests/tst_settings_t291c_absence_capture.qml

for image in \
    docs/captures/t291c-accessibility-list-light.png \
    docs/captures/t291c-accessibility-list-dark.png \
    docs/captures/t291c-accessibility-hearing-light.png \
    docs/captures/t291c-accessibility-hearing-dark.png \
    docs/captures/t291c-accessibility-hearing-contrast-dark.png \
    docs/captures/t291c-accessibility-hearing-absence-light.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t291c-accessibility-hearing: missing $image" >&2
        exit 1
    fi
    echo "wrote $image"
done
