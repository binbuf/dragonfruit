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

        Component { id: missionControlComponent; MissionControlPane { } }

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
            Settings.set("overview.hotCornerTopLeftModifier", "none");
            Settings.set("overview.hotCornerTopRightModifier", "none");
            Settings.set("overview.hotCornerBottomLeftModifier", "none");
            Settings.set("overview.hotCornerBottomRightModifier", "none");
            Settings.set("overview.switchToAppSpace", true);
            Settings.set("workspaces.count", 3);
        }

        // T285 removed the top-level Overview pane. The component stays for
        // T292 to fold into Desktop & Dock -> Workspace Overview, so these
        // cases exercise the pane directly instead of through the shell.
        function make(width, height) {
            var pane = createTemporaryObject(missionControlComponent, stage, {
                width: width !== undefined ? width : 480,
                height: height !== undefined ? height : stage.height
            });
            waitForRendering(stage);
            verify(pane);
            return pane;
        }

        function paneOf(pane) {
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_compositor_default_map() {
            var pane = make();
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

        // -- T276: modifiers, workspace count, and the full action list ------

        function test_every_corner_has_a_modifier_that_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.topLeftModifierSelect.currentIndex, 0);

            pane.topLeftModifierSelect.activateIndex(1);   // Command
            compare(Settings.values["overview.hotCornerTopLeftModifier"], "command");
            pane.topRightModifierSelect.activateIndex(2);  // Shift
            compare(Settings.values["overview.hotCornerTopRightModifier"], "shift");
            pane.bottomLeftModifierSelect.activateIndex(3); // Option
            compare(Settings.values["overview.hotCornerBottomLeftModifier"], "option");
            pane.bottomRightModifierSelect.activateIndex(4); // Control
            compare(Settings.values["overview.hotCornerBottomRightModifier"], "control");

            // External changes converge into the selectors.
            Settings.set("overview.hotCornerTopLeftModifier", "control");
            tryVerify(function() {
                return pane.topLeftModifierSelect.currentIndex === 4;
            });
        }

        function test_the_full_hot_corner_action_list_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            var expected = [
                "none", "mission-control", "notification-center",
                "desktop-reveal", "lock-screen", "launchpad",
                "control-center", "show-dock", "menu-bar", "focus-dock"
            ];
            compare(pane.actionOptions.length, expected.length);
            for (var i = 0; i < expected.length; ++i)
                compare(pane.actionOptions[i].value, expected[i]);

            // The additions map to the shell's existing input actions.
            pane.topLeftSelect.activateIndex(5); // Applications
            compare(Settings.values["overview.hotCornerTopLeft"], "launchpad");
            pane.topLeftSelect.activateIndex(6); // Quick Settings
            compare(Settings.values["overview.hotCornerTopLeft"], "control-center");
        }

        function test_switch_to_app_space_applies_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.switchToAppSpaceToggle.checked, true);
            pane.switchToAppSpaceToggle.toggle();
            compare(Settings.values["overview.switchToAppSpace"], false);
            Settings.set("overview.switchToAppSpace", true);
            compare(pane.switchToAppSpaceToggle.checked, true);
        }

        function test_workspace_count_applies_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.workspaceCountSelect.currentIndex, 2); // three spaces
            pane.workspaceCountSelect.activateIndex(4);         // five spaces
            compare(Settings.values["workspaces.count"], 5);
            compare(pane.workspaceCountSelect.currentIndex, 4);
            Settings.set("workspaces.count", 2);
            tryVerify(function() {
                return pane.workspaceCountSelect.currentIndex === 1;
            });
        }

        function test_unimplemented_rows_are_disabled_with_honest_reasons() {
            var shell = make();
            var pane = paneOf(shell);
            var rows = [pane.rearrangeRow, pane.groupByAppRow, pane.dragToOpenRow,
                        pane.shortcutsRow];
            for (var i = 0; i < rows.length; ++i) {
                compare(rows[i].enabled, false, "row " + i + " must be disabled");
                verify(rows[i].description.length > 0);
                verify(rows[i].description.indexOf("OPEN") < 0);
                verify(rows[i].description.indexOf("T2") < 0);
            }
            // Per-display workspaces are read-only: the compositor always
            // keeps one list per output.
            compare(pane.perDisplayRow.enabled, true);
        }

        function collectRowIds(item, out) {
            if (item.settingsId !== undefined
                    && String(item.settingsId).length > 0)
                out.push(item.settingsId);
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i)
                collectRowIds(kids[i], out);
            return out;
        }

        function test_rows_match_the_executable_map() {
            var shell = make();
            var pane = paneOf(shell);
            var rendered = collectRowIds(pane, []);
            var mapRows = SettingsMap.rowsFor("mission-control");
            verify(mapRows.length > 0);
            for (var i = 0; i < mapRows.length; ++i) {
                verify(rendered.indexOf(mapRows[i].id) >= 0,
                       "map row " + mapRows[i].id + " has no rendered row");
            }
        }

        function test_measured_window_740x820_keeps_every_row_reachable() {
            var pane = make(480, 820);
            var groups = [pane.missionControlGroup, pane.workspacesGroup,
                          pane.hotCornersGroup];
            for (var g = 0; g < groups.length; ++g) {
                var rows = groups[g].rows.children;
                for (var i = 0; i < rows.length; ++i) {
                    var row = rows[i];
                    if (row.control === undefined)
                        continue; // a group description, not a SettingsRow
                    verify(row.width > 0, "row " + i + " in group " + g
                           + " must have width");
                    if (row.control.width === 0)
                        continue; // a text-only status row has no control
                    verify(row.control.x + row.control.width <= row.width + 1,
                           "row " + i + " in group " + g
                           + " control must stay inside the row");
                }
            }
        }
    }
}