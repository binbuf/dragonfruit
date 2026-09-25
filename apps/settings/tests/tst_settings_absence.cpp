// SPDX-License-Identifier: MIT
// Settings absent-provider matrix test runner (T-09.6b): `QUICK_TEST_MAIN`
// finds the QML TestCase in this directory. Run headless on the offscreen
// platform with the software scene graph, under a private `dbus-run-session`
// with no settingsd and no xdg-desktop-portal (see CMakeLists.txt).
#include <QtQuickTest>

QUICK_TEST_MAIN(tst_settings_absence)