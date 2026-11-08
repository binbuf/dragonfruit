#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T226 headless Settings General-group visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# updates, and General fixtures, on a private session bus with no
# system-status bridge host, and writes:
#
#   docs/captures/t226-general-list.png              the General list
#   docs/captures/t226-general-about.png             About This System
#   docs/captures/t226-general-more-info.png         More Info (GPU/disk/kernel)
#   docs/captures/t226-general-updates.png           Software Update check
#   docs/captures/t226-general-updates-restart.png   install/restart state
#   docs/captures/t226-general-date-time.png         Date & Time
#   docs/captures/t226-general-date-time-24h.png     the 24-hour clock format
#   docs/captures/t226-general-language.png          Language & Region
#   docs/captures/t226-general-sharing-on.png        Sharing before stopping
#   docs/captures/t226-general-sharing-off.png       Sharing after stopping
#   docs/captures/t226-general-backup.png            Backup
#   docs/captures/t226-general-login-items.png       Login Items
#   docs/captures/t226-general-startup-disk.png      Startup Disk
#   docs/captures/t226-general-reset.png             Reset confirmation
#   docs/captures/t226-general-absence.png           host absent
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). This is the repeatable half of the live
# check; a host Wayland session can open the real app with the same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_general"
INPUT="apps/settings/tests/tst_settings_general_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t226-general-panes: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_UPDATES_FIXTURE=1 \
    DF_GENERAL_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t226-general-list.png \
             docs/captures/t226-general-about.png \
             docs/captures/t226-general-more-info.png \
             docs/captures/t226-general-updates.png \
             docs/captures/t226-general-updates-restart.png \
             docs/captures/t226-general-date-time.png \
             docs/captures/t226-general-date-time-24h.png \
             docs/captures/t226-general-language.png \
             docs/captures/t226-general-sharing-on.png \
             docs/captures/t226-general-sharing-off.png \
             docs/captures/t226-general-backup.png \
             docs/captures/t226-general-login-items.png \
             docs/captures/t226-general-startup-disk.png \
             docs/captures/t226-general-reset.png \
             docs/captures/t226-general-absence.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t226-general-panes: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t226-general-panes: wrote t226-*.png"
