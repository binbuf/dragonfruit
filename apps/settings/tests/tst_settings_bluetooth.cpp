// SPDX-License-Identifier: MIT
// Bluetooth-pane test runner (T-15.1b): `QUICK_TEST_MAIN` finds the QML
// TestCase in this directory. Run headless with the offscreen platform, the
// software scene graph, `DF_SETTINGS_FIXTURE`, and `DF_BLUETOOTH_FIXTURE`
// (see CMakeLists.txt), so the controls round-trip through a deterministic
// in-process adapter with no bus.
#include <QtQuickTest>

QUICK_TEST_MAIN(tst_settings_bluetooth)