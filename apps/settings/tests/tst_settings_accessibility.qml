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
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Glass.rollout = "legacy";
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
            compare(pane.contrastGroup.visible, true);
            // The absence note is for the host-absent case (tst_settings_absence).
            compare(pane.absenceNote.visible, false);
        }

        function test_reduce_transparency_round_trips_and_applies_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.reduceTransparency, false);
            compare(pane.reduceTransparencyToggle.checked, false);

            pane.reduceTransparencyToggle.toggle();
            compare(Settings.values["accessibility.reduceTransparency"], true);
            compare(pane.reduceTransparency, true);
            compare(Glass.reduceTransparency, true);
            // The override wins in the legacy look too, independent of the
            // appearance rollout.
            compare(Glass.rollout, "legacy");
            compare(pane.previewResolved.renderMode, "solid");
            compare(pane.previewResolved.sliderApplied, false);
        }

        function test_increase_contrast_round_trips_and_combines_with_reduce_transparency() {
            var shell = make();
            var pane = paneOf(shell);
            var plainEdge = pane.previewResolved.edge;

            pane.reduceTransparencyToggle.toggle();
            pane.increaseContrastToggle.toggle();
            compare(Settings.values["accessibility.reduceTransparency"], true);
            compare(Settings.values["accessibility.increaseContrast"], true);
            compare(Glass.increaseContrast, true);

            var combined = pane.previewResolved;
            compare(combined.renderMode, "solid");
            verify(combined.edge > plainEdge,
                   "Increase Contrast must strengthen the opaque plate edge");
            compare(combined.sliderApplied, false);
        }

        function test_the_overrides_follow_external_changes() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("accessibility.reduceTransparency", true);
            Settings.set("accessibility.increaseContrast", true);
            tryVerify(function() {
                return pane.reduceTransparencyToggle.checked
                        && pane.increaseContrastToggle.checked;
            });
            compare(Glass.reduceTransparency, true);
            compare(Glass.increaseContrast, true);
        }

        function test_the_overrides_are_keyboard_operable() {
            var shell = make();
            var pane = paneOf(shell);
            pane.reduceTransparencyToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["accessibility.reduceTransparency"], true);

            pane.increaseContrastToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["accessibility.increaseContrast"], true);
        }

        function test_reduce_motion_stays_independent_of_the_glass_overrides() {
            var shell = make();
            var pane = paneOf(shell);
            pane.reduceMotionToggle.toggle();
            compare(Settings.values["accessibility.reduceMotion"], true);
            compare(Settings.values["accessibility.reduceTransparency"], false);
            compare(Settings.values["accessibility.increaseContrast"], false);
            compare(Theme.reducedMotion, true);
        }
    }
}