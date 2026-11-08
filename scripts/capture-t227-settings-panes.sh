#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T227 headless Settings visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture, plus the live Control Center panel with a customized module list,
# and writes:
#
#   docs/captures/t227-appearance.png               Appearance completion rows
#   docs/captures/t227-quick-settings.png           Quick Settings pane
#   docs/captures/t227-quick-settings-live.png      the live panel, customized
#   docs/captures/t227-search.png                   Search pane
#   docs/captures/t227-screen-saver.png             Screen Saver pane
#   docs/captures/t227-desktop-stage.png            Desktop & Stage group
#   docs/captures/t227-displays-advanced.png        Displays advanced sheet
#   docs/captures/t227-search-absence.png           no settingsd: Search
#   docs/captures/t227-displays-absence.png         no settingsd: OPEN rows
#   docs/captures/t227-quick-settings-absence.png   no settingsd: Quick Settings
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). This is the repeatable half of the live
# check; a host Wayland session can open the real app with the same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_panes_capture.qml"
ABSENCE_INPUT="apps/settings/tests/tst_settings_panes_absence_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t227-settings-panes: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

# The absence pass has no settingsd on the private bus.
dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$ABSENCE_INPUT"

for image in docs/captures/t227-appearance.png \
             docs/captures/t227-quick-settings.png \
             docs/captures/t227-quick-settings-live.png \
             docs/captures/t227-search.png \
             docs/captures/t227-screen-saver.png \
             docs/captures/t227-desktop-stage.png \
             docs/captures/t227-displays-advanced.png \
             docs/captures/t227-search-absence.png \
             docs/captures/t227-displays-absence.png \
             docs/captures/t227-quick-settings-absence.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t227-settings-panes: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t227-settings-panes: wrote t227-*.png"
