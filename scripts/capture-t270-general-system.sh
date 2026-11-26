#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T270 headless General sub-pane visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# updates, and General fixtures, and writes light/dark stills for Date & Time,
# Language & Region, Login Items, Sharing, Backup, Startup Disk, Transfer or
# Reset, the searchable pickers, the dialogs, and every sub-pane with the host
# adapter stopped:
#
#   docs/captures/t270-general-list-<scheme>.png
#   docs/captures/t270-general-date-time-<scheme>.png
#   docs/captures/t270-general-timezone-search-<scheme>.png
#   docs/captures/t270-general-language-<scheme>.png
#   docs/captures/t270-general-language-search-<scheme>.png
#   docs/captures/t270-general-login-<scheme>.png
#   docs/captures/t270-general-sharing-<scheme>.png
#   docs/captures/t270-general-sharing-options-<scheme>.png
#   docs/captures/t270-general-backup-<scheme>.png
#   docs/captures/t270-general-startup-<scheme>.png
#   docs/captures/t270-general-startup-restart-<scheme>.png
#   docs/captures/t270-general-reset-<scheme>.png
#   docs/captures/t270-general-reset-confirm-<scheme>.png
#   docs/captures/t270-general-absence-list-<scheme>.png
#   docs/captures/t270-general-<sub-pane>-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_general"
INPUT="apps/settings/tests/tst_settings_t270_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t270-general-system: $BIN not built — run make build" >&2
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

sub_panes="date-time language login sharing backup startup reset"

for scheme in light dark; do
    for image in \
        docs/captures/t270-general-list-$scheme.png \
        docs/captures/t270-general-date-time-$scheme.png \
        docs/captures/t270-general-timezone-search-$scheme.png \
        docs/captures/t270-general-language-$scheme.png \
        docs/captures/t270-general-language-search-$scheme.png \
        docs/captures/t270-general-login-$scheme.png \
        docs/captures/t270-general-sharing-$scheme.png \
        docs/captures/t270-general-sharing-options-$scheme.png \
        docs/captures/t270-general-backup-$scheme.png \
        docs/captures/t270-general-startup-$scheme.png \
        docs/captures/t270-general-startup-restart-$scheme.png \
        docs/captures/t270-general-reset-$scheme.png \
        docs/captures/t270-general-reset-confirm-$scheme.png \
        docs/captures/t270-general-absence-list-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t270-general-system: $image was not written" >&2
            exit 1
        fi
    done
    for sub in $sub_panes; do
        image="docs/captures/t270-general-$sub-absence-$scheme.png"
        if [ ! -s "$image" ]; then
            echo "capture-t270-general-system: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t270-general-system: wrote t270-general-*.png"
