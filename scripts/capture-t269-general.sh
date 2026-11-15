#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T269 headless General-page visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings,
# updates, General, and storage fixtures, and writes light/dark stills:
#
#   docs/captures/t269-general-list-<scheme>.png
#   docs/captures/t269-general-about-<scheme>.png
#   docs/captures/t269-general-system-report-<scheme>.png
#   docs/captures/t269-general-software-update-<scheme>.png
#   docs/captures/t269-general-storage-<scheme>.png
#   docs/captures/t269-general-autofill-<scheme>.png
#   docs/captures/t269-general-absence-updates-only-<scheme>.png
#   docs/captures/t269-general-absence-general-only-<scheme>.png
#   docs/captures/t269-general-absence-both-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_general"
INPUT="apps/settings/tests/tst_settings_t269_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t269-general: $BIN not built — run make build" >&2
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
        docs/captures/t269-general-list-$scheme.png \
        docs/captures/t269-general-about-$scheme.png \
        docs/captures/t269-general-system-report-$scheme.png \
        docs/captures/t269-general-software-update-$scheme.png \
        docs/captures/t269-general-storage-$scheme.png \
        docs/captures/t269-general-autofill-$scheme.png \
        docs/captures/t269-general-absence-updates-only-$scheme.png \
        docs/captures/t269-general-absence-general-only-$scheme.png \
        docs/captures/t269-general-absence-both-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t269-general: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t269-general: wrote t269-general-*.png"
