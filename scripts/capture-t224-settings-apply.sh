#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T224 headless Settings apply/validation/auth/absence visual check.
#
# Renders the real Settings shell offscreen with the deterministic settings and
# accounts fixtures, on a private session bus with no system-status bridge host,
# and writes:
#
#   docs/captures/t224-settings-live.png            live apply (Dock autohide)
#   docs/captures/t224-settings-write-failure.png   forced failure + inline revert
#   docs/captures/t224-settings-auth-cancel.png     cancelled admin prompt
#   docs/captures/t224-settings-service-absence.png stopped-service absence note
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). This is the repeatable half of the live
# check; a host Wayland session can open the real app with the same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_apply_ui"
INPUT="apps/settings/tests/tst_settings_apply_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t224-settings-apply: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_ACCOUNTS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

for image in docs/captures/t224-settings-live.png \
             docs/captures/t224-settings-write-failure.png \
             docs/captures/t224-settings-auth-cancel.png \
             docs/captures/t224-settings-service-absence.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t224-settings-apply: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t224-settings-apply: wrote t224-settings-{live,write-failure,auth-cancel,service-absence}.png"
