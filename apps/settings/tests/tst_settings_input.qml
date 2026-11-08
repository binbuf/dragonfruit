// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Keyboard/Mouse/Trackpad-pane tests (T-15.4b). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_INPUT_FIXTURE` (see CMakeLists.txt), so the
// `Settings` singleton serves deterministic in-process settings and input
// inventories with no bus. The cases cover the settingsd-backed rows
// round-tripping live, the read-only inventory rendering, and the trackpad
// tabs.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsInput"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; reset the input keys before
        // every case so a pane always opens on a known state.
        function init() {
            Settings.set("input.repeatRate", 25);
            Settings.set("input.repeatDelay", 200);
            Settings.set("input.keyboardBrightness", 0.5);
            Settings.set("input.adjustBrightnessLowLight", true);
            Settings.set("input.backlightOffAfter", 0);
            Settings.set("input.keyboardNavigation", false);
            Settings.set("input.emojiKeyAction", "emoji");
            Settings.set("input.secondaryClick", "right");
            Settings.set("input.scrollSpeed", 0.0);
            Settings.set("input.doubleClickSpeed", 400);
            Settings.set("input.haptics", true);
            Settings.set("input.lookUp", "force-click");
            Settings.setList("input.sources", []);
            Settings.setList("input.replacements", []);
            Settings.set("input.dictation", false);
            Settings.set("input.pointerSpeed", 0.0);
            Settings.set("input.naturalScroll", true);
            Settings.set("input.tapToClick", true);
            Settings.set("input.leftHanded", false);
            Settings.set("input.scrollMethod", "two-finger");
            Settings.set("gestures.enabled", true);
            Settings.set("gestures.spaceSwitch", true);
            Settings.set("gestures.missionControl", true);
        }

        function make(paneId) {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane(paneId);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the input pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        // -- Keyboard ---------------------------------------------------------

        function test_keyboard_pane_opens_with_the_inventory_and_rows() {
            var shell = make("keyboard");
            compare(shell.currentPaneId, "keyboard");
            var pane = paneOf(shell);
            verify(pane.keyboard, "the body is in its keyboard section");
            verify(pane.ready, "the input inventory must be present");
            verify(pane.inventoryGroup.visible);
            compare(pane.deviceRepeater.count, 3);
            compare(pane.devices[0].kind, "keyboard");
            compare(pane.devices[0].name, "AT Translated Set 2 keyboard");
            compare(pane.devices[2].kind, "touchpad");
        }

        function test_keyboard_repeat_rows_round_trip() {
            var shell = make("keyboard");
            var pane = paneOf(shell);

            compare(pane.repeatRate, 25);
            // The slider is a 0..1 fraction of the 0..200 Hz range.
            pane.repeatRateSlider.setValue(0.5);
            pane.repeatRateSlider.commit();
            compare(Settings.values["input.repeatRate"], 100);

            // The delay slider is inverted (left Long, right Short).
            pane.repeatDelaySlider.setValue(0.5);
            pane.repeatDelaySlider.commit();
            compare(Settings.values["input.repeatDelay"], 2500);
        }

        function test_keyboard_brightness_and_navigation_rows_round_trip() {
            var shell = make("keyboard");
            var pane = paneOf(shell);

            pane.keyboardBrightnessSlider.setValue(0.2);
            pane.keyboardBrightnessSlider.commit();
            verify(Math.abs(Settings.values["input.keyboardBrightness"] - 0.2) < 0.001);

            pane.adjustBrightnessToggle.toggle();
            compare(Settings.values["input.adjustBrightnessLowLight"], false);

            pane.backlightOffSelect.activateIndex(2); // After 30 seconds
            compare(Settings.values["input.backlightOffAfter"], 30);

            pane.keyboardNavigationToggle.toggle();
            compare(Settings.values["input.keyboardNavigation"], true);

            pane.emojiKeySelect.activateIndex(1); // Do Nothing
            compare(Settings.values["input.emojiKeyAction"], "none");
        }

        function test_keyboard_input_sources_replacements_and_dictation() {
            var shell = make("keyboard");
            var pane = paneOf(shell);

            pane.sourcesDialog.show();
            waitForRendering(stage);
            verify(pane.sourcesDialog.open);
            pane.newSourceInput.text = "de";
            pane.addSource();
            compare(Settings.values["input.sources"].length, 1);
            compare(Settings.values["input.sources"][0], "de");
            pane.removeSource("de");
            compare(Settings.values["input.sources"].length, 0);
            pane.sourcesDialog.accept();
            wait(250);

            pane.newReplacementFrom = "omw";
            pane.newReplacementTo = "On my way";
            pane.addReplacement();
            compare(Settings.values["input.replacements"].length, 1);
            compare(Settings.values["input.replacements"][0], "omw\tOn my way");
            pane.removeReplacement("omw");
            compare(Settings.values["input.replacements"].length, 0);

            // The fixture has no speech-to-text command: the row is present
            // and disabled with its own reason, never a fake toggle.
            verify(!pane.dictationToggle.enabled);
            verify(pane.dictationAbsenceNote.visible);
        }

        // -- Mouse ------------------------------------------------------------

        function test_mouse_pane_round_trips_the_pointer_rows() {
            var shell = make("mouse");
            var pane = paneOf(shell);
            verify(pane.mouse, "the body is in its mouse section");

            pane.pointerSpeedSlider.setValue(0.75);
            pane.pointerSpeedSlider.commit();
            verify(Math.abs(Settings.values["input.pointerSpeed"] - 0.75) < 0.001);

            pane.naturalScrollToggle.toggle();
            compare(Settings.values["input.naturalScroll"], false);

            pane.leftHandedToggle.toggle();
            compare(Settings.values["input.leftHanded"], true);

            pane.scrollMethodSelect.activateIndex(1); // Edge Scrolling
            compare(Settings.values["input.scrollMethod"], "edge");

            pane.secondaryClickSelect.activateIndex(1); // Left
            compare(Settings.values["input.secondaryClick"], "left");

            pane.scrollSpeedSlider.setValue(-0.3);
            pane.scrollSpeedSlider.commit();
            verify(Math.abs(Settings.values["input.scrollSpeed"] - (-0.3)) < 0.001);

            pane.doubleClickSlider.setValue(1.0);
            pane.doubleClickSlider.commit();
            compare(Settings.values["input.doubleClickSpeed"], 900);
        }

        // -- Trackpad ---------------------------------------------------------

        function test_trackpad_tabs_show_the_matching_rows() {
            var shell = make("trackpad");
            var pane = paneOf(shell);
            verify(pane.trackpad, "the body is in its trackpad section");
            compare(pane.tabIndex, 0);
            verify(pane.pointClick);

            pane.tabIndex = 1;
            waitForRendering(stage);
            verify(pane.scrollZoom);
            pane.tabIndex = 2;
            waitForRendering(stage);
            verify(pane.moreGestures);
            verify(pane.pairTrackpadRow.visible);
            compare(pane.pairTrackpadRow.enabled, true);
        }

        function test_trackpad_point_click_and_gestures_round_trip() {
            var shell = make("trackpad");
            var pane = paneOf(shell);

            pane.trackpadSpeed.setValue(-0.5);
            pane.trackpadSpeed.commit();
            verify(Math.abs(Settings.values["input.pointerSpeed"] - (-0.5)) < 0.001);

            pane.tapToClickToggle.toggle();
            compare(Settings.values["input.tapToClick"], false);

            pane.gesturesEnabledToggle.toggle();
            compare(Settings.values["gestures.enabled"], false);
            pane.gestureSpaceToggle.toggle();
            compare(Settings.values["gestures.spaceSwitch"], false);
            pane.gestureMissionToggle.toggle();
            compare(Settings.values["gestures.missionControl"], false);

            pane.trackpadSecondarySelect.activateIndex(1);
            compare(Settings.values["input.secondaryClick"], "left");
            pane.lookUpSelect.activateIndex(2); // Off
            compare(Settings.values["input.lookUp"], "off");
            pane.hapticsToggle.toggle();
            compare(Settings.values["input.haptics"], false);
        }

        function test_external_change_converges_into_the_pane() {
            var shell = make("mouse");
            var pane = paneOf(shell);
            Settings.set("input.pointerSpeed", 0.25);
            verify(Math.abs(pane.pointerSpeed - 0.25) < 0.001);
            Settings.set("input.naturalScroll", false);
            compare(pane.naturalScrollToggle.checked, false);
            Settings.set("input.scrollMethod", "button");
            compare(pane.scrollMethodSelect.currentIndex, 2);
        }
    }
}