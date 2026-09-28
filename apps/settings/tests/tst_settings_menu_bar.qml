// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Menu Bar-pane tests (T-15.9b). Headless with `DF_SETTINGS_FIXTURE` (see
// CMakeLists.txt), so the `Settings` singleton serves deterministic in-process
// settings with no bus. The cases cover the behavior rows (auto-hide,
// background, recent items), the `Clock Options...` sheet, the per-control
// visibility toggles, an external change converging into the controls, and the
// absence note. The menu bar is shell-native, so the pane writes only settingsd
// keys and no adapter fixture is needed.
Item {
    id: stage
    width: 900
    height: 1600

    TestCase {
        id: testCase
        name: "SettingsMenuBar"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; reset the keys before every
        // case so the pane always opens on a known state.
        function init() {
            Settings.set("menu.autoHide", "full-screen");
            Settings.set("menu.showBackground", true);
            Settings.set("menu.recentItems", 10);
            Settings.set("menu.clock.showDate", true);
            Settings.set("menu.clock.showSeconds", false);
            Settings.set("menu.control.wifi", true);
            Settings.set("menu.control.bluetooth", true);
            Settings.set("menu.control.battery", true);
            Settings.set("menu.control.focus", true);
            Settings.set("menu.control.volume", true);
            Settings.set("menu.control.accessibility", true);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("menu-bar");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Menu Bar pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function controlToggle(pane, index) {
            var row = pane.controlsRepeater.itemAt(index);
            verify(row, "control row " + index + " must exist");
            return findChild(row, "menuBarControlToggle");
        }

        function test_pane_opens_with_the_schema_defaults() {
            var shell = make();
            compare(shell.currentPaneId, "menu-bar");
            var pane = paneOf(shell);
            compare(pane.autoHide, "full-screen");
            compare(pane.autoHideSelect.currentIndex, 2); // In Full Screen Only
            compare(pane.backgroundToggle.checked, true);
            compare(pane.recentItems, 10);
            compare(pane.recentSelect.currentIndex, 2); // 10
            compare(pane.clockShowDate, true);
            compare(pane.clockShowSeconds, false);
            compare(pane.controlsRepeater.count, 6);
        }

        function test_behavior_rows_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.autoHideSelect.activateIndex(0); // Never
            compare(Settings.values["menu.autoHide"], "never");
            pane.autoHideSelect.activateIndex(1); // Always
            compare(Settings.values["menu.autoHide"], "always");

            pane.backgroundToggle.toggle();
            compare(Settings.values["menu.showBackground"], false);

            pane.recentSelect.activateIndex(0); // None
            compare(Settings.values["menu.recentItems"], 0);
            pane.recentSelect.activateIndex(6); // 50
            compare(Settings.values["menu.recentItems"], 50);
        }

        function test_control_visibility_toggles_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            // The list order is the reference order: Wi-Fi, Bluetooth,
            // Battery, Focus, Sound, Accessibility.
            var keys = ["menu.control.wifi", "menu.control.bluetooth",
                        "menu.control.battery", "menu.control.focus",
                        "menu.control.volume", "menu.control.accessibility"];
            for (var i = 0; i < keys.length; ++i) {
                var toggle = controlToggle(pane, i);
                verify(toggle, "toggle " + i + " must exist");
                compare(toggle.checked, true);
                toggle.toggle();
                compare(Settings.values[keys[i]], false);
            }
        }

        function test_clock_options_sheet_round_trips() {
            var shell = make();
            var pane = paneOf(shell);

            pane.clockDialog.show();
            waitForRendering(stage);
            compare(pane.clockDialog.open, true);

            pane.clockDateToggle.toggle();
            compare(Settings.values["menu.clock.showDate"], false);
            pane.clockSecondsToggle.toggle();
            compare(Settings.values["menu.clock.showSeconds"], true);

            pane.clockDialog.accept();
            compare(pane.clockDialog.open, false);
        }

        function test_external_change_converges_into_the_pane() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.set("menu.autoHide", "always");
            compare(pane.autoHide, "always");
            compare(pane.autoHideSelect.currentIndex, 1);

            Settings.set("menu.recentItems", 30);
            compare(pane.recentSelect.currentIndex, 5);

            Settings.set("menu.clock.showSeconds", true);
            compare(pane.clockSecondsToggle.checked, true);

            Settings.set("menu.control.wifi", false);
            compare(controlToggle(pane, 0).checked, false);
        }

        function test_absence_note_hides_when_the_daemon_is_present() {
            // In fixture mode the settings client is available, so the pane
            // shows its controls, not the absence note. The no-daemon path is
            // covered by tst_settings_absence.qml.
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.available, true);
            compare(pane.absenceNote.visible, false);
        }
    }
}