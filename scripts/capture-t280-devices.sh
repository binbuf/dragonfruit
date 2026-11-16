#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T280 Sound / Keyboard / Mouse / Trackpad / Printers & Scanners / Game
# Controllers headless visual check.
#
# Renders the real Settings shell offscreen with the deterministic audio,
# input, and printer fixtures and writes every pane plus the Sound input tab,
# the keyboard shortcuts/dictation sheets, the three trackpad tabs, the
# add-printer dialog, and the controller test dialog in light and dark at the
# measured 740×820 window. A second pass with no adapter fixtures writes the
# absence stills:
#
#   docs/captures/t280-sound-<scheme>.png
#   docs/captures/t280-sound-input-<scheme>.png
#   docs/captures/t280-keyboard-<scheme>.png
#   docs/captures/t280-keyboard-dictation-<scheme>.png
#   docs/captures/t280-keyboard-shortcuts-<scheme>.png
#   docs/captures/t280-mouse-<scheme>.png
#   docs/captures/t280-trackpad-{point,scroll-zoom,gestures}-<scheme>.png
#   docs/captures/t280-printers-<scheme>.png
#   docs/captures/t280-printers-add-<scheme>.png
#   docs/captures/t280-controllers-<scheme>.png
#   docs/captures/t280-controllers-test-<scheme>.png
#   docs/captures/t280-<pane>-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_sound" ]; then
    echo "capture-t280-devices: pane tests not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_SOUND_FIXTURE=1 \
    DF_INPUT_FIXTURE=1 \
    DF_PRINTERS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_sound" \
    -input apps/settings/tests/tst_settings_t280_capture.qml

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_sound" \
    -input apps/settings/tests/tst_settings_t280_absence_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t280-sound-$scheme.png \
        docs/captures/t280-sound-input-$scheme.png \
        docs/captures/t280-keyboard-$scheme.png \
        docs/captures/t280-keyboard-dictation-$scheme.png \
        docs/captures/t280-keyboard-shortcuts-$scheme.png \
        docs/captures/t280-mouse-$scheme.png \
        docs/captures/t280-trackpad-point-$scheme.png \
        docs/captures/t280-trackpad-scroll-zoom-$scheme.png \
        docs/captures/t280-trackpad-gestures-$scheme.png \
        docs/captures/t280-printers-$scheme.png \
        docs/captures/t280-printers-add-$scheme.png \
        docs/captures/t280-controllers-$scheme.png \
        docs/captures/t280-controllers-test-$scheme.png \
        docs/captures/t280-sound-absence-$scheme.png \
        docs/captures/t280-keyboard-absence-$scheme.png \
        docs/captures/t280-mouse-absence-$scheme.png \
        docs/captures/t280-trackpad-absence-$scheme.png \
        docs/captures/t280-printers-absence-$scheme.png \
        docs/captures/t280-controllers-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t280-devices: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t280-devices: wrote t280-*.png"
