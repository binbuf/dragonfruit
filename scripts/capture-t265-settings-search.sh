#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T265 headless System Settings search visual check.
#
# Renders the real shell with the deterministic settings fixture and writes:
#
#   docs/captures/t265-settings-search-grouped.png    grouped, capped results
#   docs/captures/t265-settings-search-no-match.png   no-match state
#   docs/captures/t265-settings-search-long-string.png long localized query
#   docs/captures/t265-settings-search-keyboard.png   Down-from-field focus
#   docs/captures/t265-settings-search-compact.png    720x480 grouped results
#
# Host: Linux. Build: offscreen QPA + software scene graph, no settingsd and
# no bridge host. This is the repeatable half of the live check; a host Wayland
# session can open the real app with the same fixture.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_t265_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t265-settings-search: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

QT_QPA_PLATFORM=offscreen \
QT_QUICK_BACKEND=software \
DF_SETTINGS_FIXTURE=1 \
QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t265-settings-search-grouped.png \
             docs/captures/t265-settings-search-no-match.png \
             docs/captures/t265-settings-search-long-string.png \
             docs/captures/t265-settings-search-keyboard.png \
             docs/captures/t265-settings-search-compact.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t265-settings-search: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t265-settings-search: wrote t265-settings-search-*.png"
