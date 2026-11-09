#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Capture the two About surfaces at 1×. Fractional/integer scale behavior is
# exercised by the QML tests; Qt's offscreen grabImage clips at high DPI.
set -euo pipefail
cd "$(dirname "$0")/.."
build_dir="${BUILD_DIR:-build}"
binary="$build_dir/apps/settings/tests/tst_settings_general"
test -x "$binary"
mkdir -p docs/captures
dbus-run-session -- env QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software \
    QT_SCALE_FACTOR=1 DF_SETTINGS_FIXTURE=1 DF_UPDATES_FIXTURE=1 \
    DF_GENERAL_FIXTURE=1 QML2_IMPORT_PATH="$build_dir/qml" \
    "$binary" -input apps/settings/tests/tst_settings_about_capture.qml
for scheme in light dark; do
    for surface in panel page; do
        test -s "docs/captures/t230c-about-$surface-$scheme.png"
    done
done
test -s "docs/captures/t230c-about-panel-long.png"
test -s "docs/captures/t230c-about-page-long.png"
echo "capture-t230c-about: wrote panel/page light/dark plus the long localized surfaces at 1x"
