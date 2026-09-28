// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Accessibility pane tests (T-15.14b). Headless with `DF_SETTINGS_FIXTURE` and
// `DF_ACCESSIBILITY_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton
// and the AT-SPI host-stack client are deterministic and in-process with no
// bus. The cases cover the live read-only status rows and the one settingsd
// preference round-tripping live.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsAccessibility"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture is process-global; reset it before every case so each
        // starts on the known bridge-and-screen-reader-on bus, and reset the
        // one durable preference.
        function init() {
            Settings.resetAccessibilityFixture();
            Settings.set("accessibility.reduceMotion", false);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Accessibility pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_live_status() {
            var shell = make();
            compare(shell.currentPaneId, "accessibility");
            compare(shell.currentPane.title, "Accessibility");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.available, true);
            compare(pane.screenReaderOn, true);
            compare(pane.enabledOn, true);
            compare(pane.bridgeLabel, "On");
            compare(pane.screenReaderLabel, "On");
            compare(findChild(pane, "accessibilityScreenReaderValue").text, "On");
            compare(findChild(pane, "accessibilityBridgeValue").text, "On");
        }

        function test_reduce_motion_round_trips_settingsd() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.reduceMotion, false);
            compare(pane.reduceMotionToggle.checked, false);

            pane.reduceMotionToggle.toggle();
            compare(Settings.values["accessibility.reduceMotion"], true);
            compare(pane.reduceMotion, true);

            pane.reduceMotionToggle.toggle();
            compare(Settings.values["accessibility.reduceMotion"], false);
        }

        function test_the_toggle_follows_an_external_change() {
            var shell = make();
            var pane = paneOf(shell);
            // A change from elsewhere (settingsd / another control) reaches the
            // toggle through the binding.
            Settings.set("accessibility.reduceMotion", true);
            tryVerify(function() { return pane.reduceMotionToggle.checked === true; });
        }

        function test_the_status_group_is_visible_only_when_the_bus_answers() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.statusGroup.visible, true);
            compare(pane.motionGroup.visible, true);
            // The absence note is for the host-absent case (tst_settings_absence).
            compare(pane.absenceNote.visible, false);
        }
    }
}