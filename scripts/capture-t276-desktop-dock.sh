#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T276 Desktop & Dock / Overview headless visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture and writes the Dock, Desktop & Stage, Widgets, Windows,
# default-browser, Overview, and Hot Corners groups in light and dark at the
# measured 740×820 window. A second pass with no settings daemon writes the
# absence stills:
#
#   docs/captures/t276-desktop-dock-dock-<scheme>.png
#   docs/captures/t276-desktop-dock-desktop-stage-<scheme>.png
#   docs/captures/t276-desktop-dock-widgets-<scheme>.png
#   docs/captures/t276-desktop-dock-windows-<scheme>.png
#   docs/captures/t276-desktop-dock-default-browser-<scheme>.png
#   docs/captures/t276-desktop-dock-overview-<scheme>.png
#   docs/captures/t276-desktop-dock-hot-corners-<scheme>.png
#   docs/captures/t276-desktop-dock-absence-<scheme>.png
#   docs/captures/t276-desktop-dock-overview-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_desktop_dock" ] || [ ! -x "$TESTS/tst_settings_absence" ]; then
    echo "capture-t276-desktop-dock: pane tests not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

run_capture() {
    local fixture="$1"
    local input="$2"
    if [ "$fixture" = "present" ]; then
        dbus-run-session -- env \
            QT_QPA_PLATFORM=offscreen \
            QT_QUICK_BACKEND=software \
            DF_SETTINGS_FIXTURE=1 \
            QML2_IMPORT_PATH="$BUILD/qml" \
            "$TESTS/tst_settings_desktop_dock" \
            -input "$input"
    else
        dbus-run-session -- env \
            QT_QPA_PLATFORM=offscreen \
            QT_QUICK_BACKEND=software \
            QML2_IMPORT_PATH="$BUILD/qml" \
            "$TESTS/tst_settings_absence" \
            -input "$input"
    fi
}

run_capture present apps/settings/tests/tst_settings_t276_capture.qml
run_capture absent apps/settings/tests/tst_settings_t276_absence_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t276-desktop-dock-dock-$scheme.png \
        docs/captures/t276-desktop-dock-desktop-stage-$scheme.png \
        docs/captures/t276-desktop-dock-widgets-$scheme.png \
        docs/captures/t276-desktop-dock-windows-$scheme.png \
        docs/captures/t276-desktop-dock-default-browser-$scheme.png \
        docs/captures/t276-desktop-dock-overview-$scheme.png \
        docs/captures/t276-desktop-dock-hot-corners-$scheme.png \
        docs/captures/t276-desktop-dock-absence-$scheme.png \
        docs/captures/t276-desktop-dock-overview-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t276-desktop-dock: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t276-desktop-dock: wrote t276-desktop-dock-*.png"
