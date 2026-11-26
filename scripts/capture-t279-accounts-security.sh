#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# T279 Privacy & Security / Biometrics & Password / Users & Groups / Online
# Accounts headless visual check.
#
# Renders the real Settings shell offscreen with the deterministic privacy,
# account, biometrics, and online-accounts fixtures and writes every pane plus
# the category, enrollment, add-user, and account-details surfaces in light and
# dark at the measured 740×820 window. A second pass with no host fixtures
# writes the absence stills:
#
#   docs/captures/t279-privacy-<scheme>.png
#   docs/captures/t279-privacy-category-<scheme>.png
#   docs/captures/t279-biometrics-<scheme>.png
#   docs/captures/t279-biometrics-enroll-<scheme>.png
#   docs/captures/t279-users-<scheme>.png
#   docs/captures/t279-users-add-<scheme>.png
#   docs/captures/t279-online-accounts-<scheme>.png
#   docs/captures/t279-online-accounts-details-<scheme>.png
#   docs/captures/t279-<pane>-absence-<scheme>.png
#
# The host is Linux; the build is the offscreen QPA platform with the software
# scene graph (no GPU, no host bus). A host Wayland session can open the real
# app with the same fixtures for the live visual check.
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD="${BUILD_DIR:-build}"
TESTS="$BUILD/apps/settings/tests"

if [ ! -x "$TESTS/tst_settings_privacy" ]; then
    echo "capture-t279-accounts-security: pane tests not built — run make build" >&2
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
    "$TESTS/tst_settings_privacy" \
    -input apps/settings/tests/tst_settings_t279_capture.qml

dbus-run-session -- env \
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    DF_SETTINGS_FIXTURE=1 \
    QML2_IMPORT_PATH="$BUILD/qml" \
    "$TESTS/tst_settings_privacy" \
    -input apps/settings/tests/tst_settings_t279_absence_capture.qml

for scheme in light dark; do
    for image in \
        docs/captures/t279-privacy-$scheme.png \
        docs/captures/t279-privacy-category-$scheme.png \
        docs/captures/t279-biometrics-$scheme.png \
        docs/captures/t279-biometrics-enroll-$scheme.png \
        docs/captures/t279-users-$scheme.png \
        docs/captures/t279-users-add-$scheme.png \
        docs/captures/t279-online-accounts-$scheme.png \
        docs/captures/t279-online-accounts-details-$scheme.png \
        docs/captures/t279-privacy-absence-$scheme.png \
        docs/captures/t279-biometrics-absence-$scheme.png \
        docs/captures/t279-users-absence-$scheme.png \
        docs/captures/t279-online-accounts-absence-$scheme.png; do
        if [ ! -s "$image" ]; then
            echo "capture-t279-accounts-security: $image was not written" >&2
            exit 1
        fi
    done
done

echo "capture-t279-accounts-security: wrote t279-*.png"
