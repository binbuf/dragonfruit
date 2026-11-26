#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# T293b Menu Bar Controls visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings
# fixture and writes the in-place Menu Bar Controls sub-pane at the measured
# 740×820 window in light and dark:
#
#   docs/captures/t293b-menu-bar-controls-entry-<scheme>.png    (the Menu Bar
#                                                               list footer)
#   docs/captures/t293b-menu-bar-controls-pages-<scheme>.png    (Pages)
#   docs/captures/t293b-menu-bar-controls-modules-<scheme>.png  (Modules and
#                                                               control sizes)
#   docs/captures/t293b-menu-bar-controls-preview-<scheme>.png  (live preview
#                                                               and Menu Bar
#                                                               Only)
#   docs/captures/t293b-menu-bar-controls-add-<scheme>.png      (Add Controls)
#   docs/captures/t293b-menu-bar-controls-reset-<scheme>.png    (Reset result)
#
# Six interactive surfaces per scheme. The host is Linux; the build is the
# offscreen QPA platform with the software scene graph (no GPU, no host bus).
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_menu_bar" ]; then
    echo "capture-t293b-settings-menu-bar-controls: test not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_menu_bar" \
    -input apps/settings/tests/tst_settings_t293b_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t293b-menu-bar-controls-entry-$scheme.png \
        docs/captures/t293b-menu-bar-controls-pages-$scheme.png \
        docs/captures/t293b-menu-bar-controls-modules-$scheme.png \
        docs/captures/t293b-menu-bar-controls-preview-$scheme.png \
        docs/captures/t293b-menu-bar-controls-add-$scheme.png \
        docs/captures/t293b-menu-bar-controls-reset-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t293b-settings-menu-bar-controls: missing $image" >&2
            exit 1
        fi
        echo "wrote $image"
    done
done
