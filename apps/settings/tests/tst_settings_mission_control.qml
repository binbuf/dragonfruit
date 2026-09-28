// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Mission Control & Hot Corners-pane tests (T-15.5b). Headless with
// `DF_SETTINGS_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton
// serves deterministic in-process settings with no bus. The cases cover the
// four hot-corner assignments and the gesture rows round-tripping live, an
// external change converging into the controls, and the absence note.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsMissionControl"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; reset the keys before every
        // case so the pane always opens on a known state.
        function init() {
            Settings.set("overview.hotCornerTopLeft", "mission-control");
            Settings.set("overview.hotCornerTopRight", "notification-center");
            Settings.set("overview.hotCornerBottomLeft", "desktop-reveal");
            Settings.set("overview.hotCornerBottomRight", "lock-screen");
            Settings.set("gestures.enabled", true);
            Settings.set("gestures.spaceSwitch", true);
            Settings.set("gestures.missionControl", true);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("mission-control");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Mission Control pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_compositor_default_map() {
            var shell = make();
            compare(shell.currentPaneId, "mission-control");
            var pane = paneOf(shell);
            compare(pane.topLeft, "mission-control");
            compare(pane.topRight, "notification-center");
            compare(pane.bottomLeft, "desktop-reveal");
            compare(pane.bottomRight, "lock-screen");
            compare(pane.topLeftSelect.currentIndex, 1);
            compare(pane.bottomRightSelect.currentIndex, 4);
            compare(pane.gestureMissionToggle.checked, true);
            compare(pane.gestureSpaceToggle.checked, true);
        }

        function test_hot_corner_rows_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.topLeftSelect.activateIndex(0); // none
            compare(Settings.values["overview.hotCornerTopLeft"], "none");

            pane.topRightSelect.activateIndex(2); // Notification Center
            compare(Settings.values["overview.hotCornerTopRight"], "notification-center");

            pane.bottomLeftSelect.activateIndex(1); // Mission Control
            compare(Settings.values["overview.hotCornerBottomLeft"], "mission-control");

            pane.bottomRightSelect.activateIndex(3); // Desktop
            compare(Settings.values["overview.hotCornerBottomRight"], "desktop-reveal");
        }

        function test_gesture_rows_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.gestureMissionToggle.toggle();
            compare(Settings.values["gestures.missionControl"], false);
            pane.gestureMissionToggle.toggle();
            compare(Settings.values["gestures.missionControl"], true);

            pane.gestureSpaceToggle.toggle();
            compare(Settings.values["gestures.spaceSwitch"], false);
        }

        function test_external_change_converges_into_the_pane() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.set("overview.hotCornerTopLeft", "lock-screen");
            compare(pane.topLeft, "lock-screen");
            compare(pane.topLeftSelect.currentIndex, 4);

            Settings.set("gestures.missionControl", false);
            compare(pane.gestureMissionToggle.checked, false);
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