#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T291e Accessibility General + close-out visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings and
# accessibility fixtures and writes the complete five-row big list, every
# shipped sub-pane (Vision, Physical and Motor, Hearing, Speech, General), the
# General recorder recorded state, the reduce-motion and increase-contrast
# accommodation states, and the absent-host state at the measured 740×820
# window, in light and dark:
#
#   docs/captures/t291e-accessibility-list-<scheme>.png
#   docs/captures/t291e-accessibility-vision-<scheme>.png
#   docs/captures/t291e-accessibility-reduce-motion-<scheme>.png
#   docs/captures/t291e-accessibility-motor-<scheme>.png
#   docs/captures/t291e-accessibility-hearing-<scheme>.png
#   docs/captures/t291e-accessibility-speech-<scheme>.png
#   docs/captures/t291e-accessibility-general-<scheme>.png
#   docs/captures/t291e-accessibility-general-recorded-<scheme>.png
#   docs/captures/t291e-accessibility-increase-contrast-<scheme>.png
#   docs/captures/t291e-accessibility-absence-list-<scheme>.png
#   docs/captures/t291e-accessibility-absence-vision-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_accessibility"

if [ ! -x "$BIN" ]; then
    echo "capture-t291e-accessibility-general: $BIN not built — run make build" >&2
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

run_capture 1 present apps/settings/tests/tst_settings_t291e_capture.qml
run_capture absent absent apps/settings/tests/tst_settings_t291e_absence_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t291e-accessibility-list-$scheme.png \
        docs/captures/t291e-accessibility-vision-$scheme.png \
        docs/captures/t291e-accessibility-reduce-motion-$scheme.png \
        docs/captures/t291e-accessibility-motor-$scheme.png \
        docs/captures/t291e-accessibility-hearing-$scheme.png \
        docs/captures/t291e-accessibility-speech-$scheme.png \
        docs/captures/t291e-accessibility-general-$scheme.png \
        docs/captures/t291e-accessibility-general-recorded-$scheme.png \
        docs/captures/t291e-accessibility-increase-contrast-$scheme.png \
        docs/captures/t291e-accessibility-absence-list-$scheme.png \
        docs/captures/t291e-accessibility-absence-vision-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t291e-accessibility-general: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
