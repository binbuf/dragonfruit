#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T287 General big-list and sub-pane headless visual check.
#
# Renders the real Settings shell with the deterministic settings, updates,
# General, and storage fixtures and writes:
#
#   docs/captures/t287-general-list-<scheme>.png
#   docs/captures/t287-general-<sub-pane>-<scheme>.png   (11 sub-panes)
#   docs/captures/t287-general-software-update-no-provider-<scheme>.png
#   docs/captures/t287-general-absence-<scheme>.png
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no host bridge. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_general"
INPUT="apps/settings/tests/tst_settings_t287_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t287-settings-general: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_UPDATES_FIXTURE=1 \
    DF_GENERAL_FIXTURE=1 \
    DF_STORAGE_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for scheme in light dark; do
    for image in \
        docs/captures/t287-general-list-$scheme.png \
        docs/captures/t287-general-about-$scheme.png \
        docs/captures/t287-general-software-update-$scheme.png \
        docs/captures/t287-general-storage-$scheme.png \
        docs/captures/t287-general-passwords-$scheme.png \
        docs/captures/t287-general-date-time-$scheme.png \
        docs/captures/t287-general-language-$scheme.png \
        docs/captures/t287-general-login-$scheme.png \
        docs/captures/t287-general-sharing-$scheme.png \
        docs/captures/t287-general-startup-$scheme.png \
        docs/captures/t287-general-backup-$scheme.png \
        docs/captures/t287-general-reset-$scheme.png \
        docs/captures/t287-general-software-update-no-provider-$scheme.png \
        docs/captures/t287-general-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t287-settings-general: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
