#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T292b Workspace Overview visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture and writes the merged Workspace Overview section (the former Mission
# Control / Workspaces / Hot Corners / Shortcuts surface) and its Advanced
# Workspace Overview sheet (workspace count, gestures, the four corner
# assignments), reached through the legacy `settings://pane/mission-control`
# deep link and the footer's Hot Corners… action. A second pass with no settings
# daemon writes the absence stills. All at the measured 740×820 window, in light
# and dark:
#
#   docs/captures/t292b-workspace-overview-merged-<scheme>.png
#   docs/captures/t292b-workspace-overview-footer-<scheme>.png
#   docs/captures/t292b-workspace-overview-advanced-<scheme>.png
#   docs/captures/t292b-workspace-overview-absence-<scheme>.png
#   docs/captures/t292b-workspace-overview-absence-advanced-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_desktop_dock" ] || [ ! -x "$TESTS/tst_settings_absence" ]; then
    echo "capture-t292b-workspace-overview: pane tests not built — run make build" >&2
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

run_capture present apps/settings/tests/tst_settings_t292b_capture.qml
run_capture absent apps/settings/tests/tst_settings_t292b_absence_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t292b-workspace-overview-merged-$scheme.png \
        docs/captures/t292b-workspace-overview-footer-$scheme.png \
        docs/captures/t292b-workspace-overview-advanced-$scheme.png \
        docs/captures/t292b-workspace-overview-absence-$scheme.png \
        docs/captures/t292b-workspace-overview-absence-advanced-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t292b-workspace-overview: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
