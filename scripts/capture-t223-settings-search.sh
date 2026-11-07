#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T223 headless Settings search/deep-link/row-anatomy visual check.
#
# Renders the real Settings shell with the deterministic settings fixture and
# writes:
#
#   docs/captures/t223-settings-search.png        ranked search results
#   docs/captures/t223-settings-deep-link.png     deep-linked row + highlight
#   docs/captures/t223-settings-long-strings.png  long localized pane strings
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no session bus). This is the repeatable half of the live
# check; a host Wayland session can open the real app with the same fixture.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_search"
INPUT="apps/settings/tests/tst_settings_search_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t223-settings-search: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

QT_QPA_PLATFORM=offscreen \
QT_QUICK_BACKEND=software \
DF_SETTINGS_FIXTURE=1 \
QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t223-settings-search.png \
             docs/captures/t223-settings-deep-link.png \
             docs/captures/t223-settings-long-strings.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t223-settings-search: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t223-settings-search: wrote t223-settings-{search,deep-link,long-strings}.png"
