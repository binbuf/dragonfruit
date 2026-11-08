#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T228 headless Settings visual check.
#
# Renders the real Settings shell offscreen with the deterministic fixtures and
# writes:
#
#   docs/captures/t228-privacy-list.png          macOS-mapped privacy rows
#   docs/captures/t228-privacy-detail.png        one per-app permission detail
#   docs/captures/t228-security.png              Security group
#   docs/captures/t228-security-sources.png      software-sources sheet
#   docs/captures/t228-biometrics.png            Biometrics & Password
#   docs/captures/t228-online-accounts.png       Online Accounts
#   docs/captures/t228-privacy-absence.png       no portal/bridge host
#   docs/captures/t228-biometrics-absence.png    no fprintd/AccountsService
#   docs/captures/t228-online-accounts-absence.png  GOA absent: pane hidden
#
# Host: Linux. Build: offscreen QPA + software scene graph, private session
# bus, no real portal/fprintd/GOA/LUKS/polkit writes. This is the repeatable
# half of the live check; a host Wayland session can open the real app with the
# same fixtures.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
BIN="$BUILD/apps/settings/tests/tst_settings_panes_capture"
INPUT="apps/settings/tests/tst_settings_security_capture.qml"
ABSENCE_INPUT="apps/settings/tests/tst_settings_security_absence_capture.qml"

if [ ! -x "$BIN" ]; then
    echo "capture-t228-privacy-accounts: $BIN not built — run make build" >&2
    exit 1
fi

mkdir -p docs/captures

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    DF_PRIVACY_FIXTURE=1 \
    DF_ACCOUNTS_FIXTURE=1 \
    DF_BIOMETRICS_FIXTURE=1 \
    DF_ONLINE_ACCOUNTS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$INPUT"

# The absence pass has no settingsd, no portal, and no bridge host.
dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$BIN" -input "$ABSENCE_INPUT"

for image in docs/captures/t228-privacy-list.png \
             docs/captures/t228-privacy-detail.png \
             docs/captures/t228-security.png \
             docs/captures/t228-security-sources.png \
             docs/captures/t228-biometrics.png \
             docs/captures/t228-online-accounts.png \
             docs/captures/t228-privacy-absence.png \
             docs/captures/t228-biometrics-absence.png \
             docs/captures/t228-online-accounts-absence.png; do
    if [ ! -s "$image" ]; then
        echo "capture-t228-privacy-accounts: $image was not written" >&2
        exit 1
    fi
done

echo "capture-t228-privacy-accounts: wrote t228-*.png"
