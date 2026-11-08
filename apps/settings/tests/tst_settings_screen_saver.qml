// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Screen Saver pane tests (T227). The style/clock options feed the shell's
// lock scene and the start-after value is the session idle engine's
// `idle.blank` key.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsScreenSaver"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.set("screenSaver.style", "clock");
            Settings.set("screenSaver.clock", true);
            Settings.set("screenSaver.showDate", true);
            Settings.set("screenSaver.showSeconds", false);
            Settings.set("idle.blank", 300);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("screen-saver");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready);
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_schema_defaults() {
            var shell = make();
            compare(shell.currentPaneId, "screen-saver");
            var pane = paneOf(shell);
            compare(pane.styleControl.currentIndex, 0); // clock
            compare(pane.startAfterSelect.currentIndex, 2); // 5 minutes
            verify(pane.clockToggle.checked);
            verify(pane.showDateToggle.checked);
            verify(!pane.showSecondsToggle.checked);
        }

        function test_style_and_idle_time_apply_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.styleControl.activateIndex(1); // none
            compare(Settings.values["screenSaver.style"], "none");
            pane.styleControl.activateIndex(0);
            compare(Settings.values["screenSaver.style"], "clock");

            pane.startAfterSelect.activateIndex(1); // 1 minute
            compare(Settings.values["idle.blank"], 60);
        }

        function test_clock_options_apply_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.clockToggle.toggled(false);
            compare(Settings.values["screenSaver.clock"], false);
            pane.showDateToggle.toggled(false);
            compare(Settings.values["screenSaver.showDate"], false);
            pane.showSecondsToggle.toggled(true);
            compare(Settings.values["screenSaver.showSeconds"], true);
        }

        function test_external_change_converges() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("screenSaver.style", "none");
            tryVerify(function() { return pane.styleControl.currentIndex === 1; });
            Settings.set("idle.blank", 600);
            tryVerify(function() { return pane.startAfterSelect.currentIndex === 3; });
        }

        function test_hot_corners_link_opens_overview() {
            var shell = make();
            var pane = paneOf(shell);
            pane.hotCornersButton.clicked();
            compare(shell.currentPaneId, "mission-control");
        }
    }
}
