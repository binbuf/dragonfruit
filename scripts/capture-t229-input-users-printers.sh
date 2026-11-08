#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T229 headless Settings visual check.
#
# Renders the real Settings shell offscreen with the deterministic fixtures and
# writes:
#
#   docs/captures/t229-keyboard-input-sources.png   input sources/replacements/dictation
#   docs/captures/t229-keyboard-sources-dialog.png  the input-sources editor
#   docs/captures/t229-mouse-pointer-speed.png      pointer speed + new rows
#   docs/captures/t229-trackpad-point-click.png     secondary click/look up/haptics
#   docs/captures/t229-sound-volume.png             changed volume + Test
#   docs/captures/t229-users-groups-directory.png   directory/admin group + group edit
#   docs/captures/t229-printers-add.png             discovery list
#   docs/captures/t229-printers-added.png           the added queue
#   docs/captures/t229-printers-options.png         per-queue options sheet
#   docs/captures/t229-controllers-list.png         controller list
#   docs/captures/t229-controllers-test.png         diagnostic test read
#   docs/captures/t229-battery-options.png          battery options + power link
#   docs/captures/t229-controllers-absence.png      no controller inventory
#   docs/captures/t229-printers-absence.png         no CUPS/SANE
#   docs/captures/t229-sound-absence.png            no WirePlumber
#
# Host: Linux. Build: offscreen QPA + software scene graph, private session
# bus, no real CUPS/SANE/controllers/audio writes. This is the repeatable half
# of the live check; a host Wayland session can open the real app with the
# same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t229_capture.qml"
ABSENCE_INPUT="apps/settings/tests/tst_settings_t229_absence_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t229-input-users-printers: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_SOUND_FIXTURE=1 \
    DF_INPUT_FIXTURE=1 \
    DF_ACCOUNTS_FIXTURE=1 \
    DF_PRINTERS_FIXTURE=1 \
    DF_BATTERY_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

# The absence pass has no settingsd and no bridge host.
dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$ABSENCE_INPUT"

for image in docs/captures/t229-keyboard-input-sources.png \
             docs/captures/t229-keyboard-sources-dialog.png \
             docs/captures/t229-mouse-pointer-speed.png \
             docs/captures/t229-trackpad-point-click.png \
             docs/captures/t229-sound-volume.png \
             docs/captures/t229-users-groups-directory.png \
             docs/captures/t229-printers-add.png \
             docs/captures/t229-printers-added.png \
             docs/captures/t229-printers-options.png \
             docs/captures/t229-controllers-list.png \
             docs/captures/t229-controllers-test.png \
             docs/captures/t229-battery-options.png \
             docs/captures/t229-controllers-absence.png \
             docs/captures/t229-printers-absence.png \
             docs/captures/t229-sound-absence.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t229-input-users-printers: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t229-input-users-printers: wrote t229-*.png"
