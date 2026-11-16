// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Screen Saver pane tests (T227, completed by T278). The style gallery selects
// the first-party lock scene, the clock options feed that scene, and the
// start-after value is the session idle engine's `idle.blank` stage.
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
            compare(pane.style, "clock");
            compare(pane.styleGallery.count, 2);
            compare(pane.styleTiles.length, 2);
            verify(pane.styleTiles[0].selected);
            compare(pane.startAfterSelect.currentIndex, 2); // 5 minutes
            verify(pane.clockToggle.checked);
            verify(pane.showDateToggle.checked);
            verify(!pane.showSecondsToggle.checked);
            verify(pane.saverEnabled);
            verify(!pane.disabledReason.visible);
        }

        function test_gallery_selects_the_style_and_previews_the_clock() {
            var shell = make();
            var pane = paneOf(shell);
            var clockTile = pane.styleTiles[0];
            var noneTile = pane.styleTiles[1];
            verify(clockTile.selected);
            compare(clockTile.timeText, "9:41");
            verify(clockTile.dateText.length > 0);

            noneTile.choose();
            compare(Settings.values["screenSaver.style"], "none");
            verify(noneTile.selected);
            verify(!pane.saverEnabled);
            verify(pane.disabledReason.visible);
            verify(!pane.startAfterSelect.enabled,
                   "None disables Start After instead of blanking the display");
            verify(!pane.clockToggle.enabled);

            clockTile.choose();
            compare(Settings.values["screenSaver.style"], "clock");
            verify(pane.startAfterSelect.enabled);
        }

        function test_start_after_has_two_and_five_hours_and_never() {
            var shell = make();
            var pane = paneOf(shell);
            var labels = [];
            for (var i = 0; i < pane.startAfterModel.length; ++i)
                labels.push(pane.startAfterModel[i].label);
            verify(labels.indexOf("After 2 hours") >= 0, labels.join(","));
            verify(labels.indexOf("After 5 hours") >= 0, labels.join(","));
            verify(labels.indexOf("Never") >= 0, labels.join(","));

            pane.startAfterSelect.activateIndex(7); // 2 hours
            compare(Settings.values["idle.blank"], 7200);
            pane.startAfterSelect.activateIndex(8); // 5 hours
            compare(Settings.values["idle.blank"], 18000);
            pane.startAfterSelect.activateIndex(0); // Never
            compare(Settings.values["idle.blank"], 0);
        }

        function test_clock_options_apply_live_and_disable_without_a_clock() {
            var shell = make();
            var pane = paneOf(shell);

            pane.clockToggle.toggle();
            compare(Settings.values["screenSaver.clock"], false);
            verify(!pane.showDateToggle.enabled,
                   "date and seconds depend on the clock");
            pane.clockToggle.toggle();
            compare(Settings.values["screenSaver.clock"], true);

            pane.showDateToggle.toggle();
            compare(Settings.values["screenSaver.showDate"], false);
            pane.showSecondsToggle.toggle();
            compare(Settings.values["screenSaver.showSeconds"], true);
            compare(pane.styleTiles[0].timeText, "9:41:05");
        }

        function test_external_change_converges() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("screenSaver.style", "none");
            tryVerify(function() { return pane.styleTiles[1].selected; });
            Settings.set("screenSaver.style", "clock");
            Settings.set("idle.blank", 600);
            tryVerify(function() { return pane.startAfterSelect.currentIndex === 3; });
        }

        function test_custom_idle_value_keeps_its_real_duration() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("idle.blank", 420);
            compare(pane.startAfterSelect.currentLabel, "After 7 minutes");
            verify(pane.startAfterSelect.currentLabel !== "Never");
        }

        function test_hot_corners_link_opens_the_actual_controls() {
            var shell = make();
            var pane = paneOf(shell);
            pane.hotCornersButton.clicked();
            compare(shell.currentPaneId, "mission-control");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            verify(paneOf(shell).hotCornersGroup.visible);
        }
    }
}
