#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T222 headless Settings map visual check.
#
# Renders the Settings shell offscreen with `DF_SETTINGS_DEBUG=1`, so the
# Pane & Row Owners debug view is open on launch, and writes:
#
#   docs/captures/t222-settings-debug.png    sidebar + owner debug view
#   docs/captures/t222-settings-sidebar.png  the same shell after closing it
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no session bus, deterministic settings fixture). This is
# the repeatable half of the live check; a host Wayland session can open the
# real app with the same env var.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_map"
INPUT="apps/settings/tests/tst_settings_map_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t222-settings-map: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

QT_QPA_PLATFORM=offscreen \
QT_QUICK_BACKEND=software \
DF_SETTINGS_FIXTURE=1 \
DF_SETTINGS_DEBUG=1 \
QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t222-settings-debug.png docs/captures/t222-settings-sidebar.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t222-settings-map: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t222-settings-map: wrote docs/captures/t222-settings-debug.png and docs/captures/t222-settings-sidebar.png"
