// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Displays-pane tests (T-09.5, completed T273). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_DISPLAY_FIXTURE`, so the `Settings` singleton
// serves the schema defaults and a deterministic real-output list: no bus, no
// compositor, no daemon. Every control is asserted to apply live to its
// `display.*` key on the same event-loop turn, and an external settingsd
// change flows back into the control (the T-09.1b converge rule).
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsDisplays"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global, so reset every key this pane
        // touches to its schema default before each case.
        function init() {
            Settings.setDisplayFixture("single");
            Settings.set("display.scale", 1.0);
            Settings.set("display.rotation", "normal");
            Settings.set("display.brightness", 1.0);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("displays");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the displays body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function capability(pane, id) {
            for (var i = 0; i < pane.capabilityRows.length; ++i) {
                if (String(pane.capabilityRows[i].id) === id)
                    return pane.capabilityRows[i];
            }
            return null;
        }

        function test_pane_opens_with_the_real_output_and_controls() {
            var shell = make();
            compare(shell.currentPaneId, "displays");
            var pane = paneOf(shell);

            compare(pane.outputs.length, 1);
            compare(pane.outputName, "Built-in Display");
            compare(pane.displayGroup.title, "Built-in Display");
            compare(pane.previewLabel.text, "Built-in Display");
            compare(pane.refreshText.text, "120 Hz");
            // The preview is proportional to the real output, not a fixed
            // 16:10 frame.
            var expected = 1512 / 982;
            verify(Math.abs(pane.previewScreen.width / pane.previewScreen.height
                            - expected) < 0.02,
                   "preview aspect " + (pane.previewScreen.width
                                        / pane.previewScreen.height)
                   + " must follow " + expected);
            compare(pane.tileItems.length, 5);
            compare(pane.tileItems[0].modelData.label, "Larger Text");
            compare(pane.tileItems[4].modelData.label, "Most Space");
            verify(pane.tileItems[2].selected);
            compare(pane.rotationSelect.currentIndex, 0);
            verify(pane.footer.text.indexOf("scaled resolution") >= 0);
            compare(pane.absenceNote.visible, false);
        }

        function test_resolution_tiles_apply_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.tileItems[0].choose(); // Larger Text -> 1.5
            verify(Math.abs(Settings.values["display.scale"] - 1.5) < 0.001);
            verify(pane.tileItems[0].selected);
            verify(!pane.tileItems[2].selected);

            pane.tileItems[4].choose(); // Most Space -> 0.75
            verify(Math.abs(Settings.values["display.scale"] - 0.75) < 0.001);

            Settings.set("display.scale", 1.25);
            verify(pane.tileItems[1].selected);
            verify(!pane.tileItems[4].selected);
        }

        function test_rotation_select_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.rotationSelect.activateIndex(3); // 270°
            compare(Settings.values["display.rotation"], "270");
            compare(pane.rotationSelect.currentIndex, 3);

            pane.rotationSelect.activateIndex(1); // 90°
            compare(Settings.values["display.rotation"], "90");

            Settings.set("display.rotation", "normal");
            compare(pane.rotationSelect.currentIndex, 0);
        }

        function test_rotation_row_control_is_right_aligned() {
            var shell = make();
            var pane = paneOf(shell);
            var row = pane.rotationGroup.rows.children[0];
            verify(row);
            verify(row.control.x > row.width * 0.55,
                   "rotation control must sit at the right, x=" + row.control.x
                   + " row.width=" + row.width);
        }

        function test_brightness_applies_live_and_converges() {
            var shell = make();
            var pane = paneOf(shell);
            fuzzyCompare(pane.brightnessSlider.value, 1.0, 0.0001);

            pane.brightnessSlider.setValue(0.4);
            pane.brightnessSlider.commit();
            fuzzyCompare(Settings.values["display.brightness"], 0.4, 0.0001);

            Settings.set("display.brightness", 0.75);
            tryVerify(function() {
                return Math.abs(pane.brightnessSlider.value - 0.75) < 0.0001;
            });
        }

        function test_the_capability_table_is_the_one_source_for_disabled_rows() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.capabilityRows.length, 16);
            compare(pane.advancedRows.length, 4);

            var supported = ["brightness", "rotation"];
            for (var i = 0; i < pane.capabilityRows.length; ++i) {
                var row = pane.capabilityRows[i];
                var live = supported.indexOf(String(row.id)) >= 0;
                compare(row.supported, live, String(row.id));
                if (!live) {
                    verify(String(row.reason).length > 0,
                           String(row.id) + " must carry a reason");
                    verify(String(row.reason).indexOf("OPEN") < 0,
                           String(row.id) + " must not leak an internal marker");
                }
            }

            // The main pane's disabled controls carry a user-facing reason.
            compare(pane.useAsSelect.enabled, false);
            verify(pane.useAsRow.description.length > 0);
            verify(pane.useAsRow.description.indexOf("OPEN") < 0);
            compare(pane.autoBrightnessToggle.enabled, false);
            compare(pane.nightLightToggle.enabled, false);
            compare(pane.nightLightScheduleSelect.enabled, false);
            compare(pane.nightLightTemperatureSlider.enabled, false);
            compare(pane.colorProfileSelect.enabled, false);
            compare(pane.tvModeSelect.enabled, false);
            compare(pane.arrangementButton.enabled, false);
            verify(capability(pane, "hdr").supported === false);
            verify(capability(pane, "vrr").supported === false);
            verify(capability(pane, "refreshRate").reason.length > 0);
            verify(capability(pane, "refreshRate").reason.indexOf("OPEN") < 0);
        }

        function test_advanced_sheet_lists_only_advanced_rows() {
            var shell = make();
            var pane = paneOf(shell);

            pane.advancedButton.clicked();
            tryCompare(pane.advancedPopup, "open", true);
            compare(pane.advancedRows.length, 4);
            for (var i = 0; i < pane.advancedRows.length; ++i) {
                verify(String(pane.advancedRows[i].id) !== "brightness");
                verify(String(pane.advancedRows[i].id) !== "rotation");
            }
            pane.advancedPopup.hide();
        }

        function test_per_output_selector_scopes_the_preview_and_refresh() {
            Settings.setDisplayFixture("dual");
            var shell = make();
            var pane = paneOf(shell);
            tryVerify(function() { return pane.outputs.length === 2; });
            compare(pane.outputRepeater.count, 2);
            verify(pane.displayListGroup.visible);
            compare(pane.outputName, "Built-in Display");
            compare(pane.refreshText.text, "120 Hz");

            pane.chooseOutput("external");
            compare(pane.outputName, "External Display");
            compare(pane.previewLabel.text, "External Display");
            compare(pane.refreshText.text, "60 Hz");
            var expected = 2560 / 1440;
            verify(Math.abs(pane.previewScreen.width / pane.previewScreen.height
                            - expected) < 0.02);

            // The arrangement preview is a second, read-only selector.
            compare(pane.arrangementGroup.visible, true);
            Settings.setDisplayFixture("single");
        }

        function test_absence_keeps_settingsd_controls_live() {
            Settings.setDisplayFixture("absent");
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.outputs.length, 0);
            compare(pane.outputName, "No display");
            compare(pane.previewLabel.text, "No display");
            verify(pane.absenceNote.visible);
            compare(pane.displayGroup.title, "No display");
            // The settingsd controls are not host reads and stay live.
            compare(pane.brightnessSlider.enabled, true);
            compare(pane.rotationSelect.enabled, true);
            pane.brightnessSlider.setValue(0.3);
            pane.brightnessSlider.commit();
            fuzzyCompare(Settings.values["display.brightness"], 0.3, 0.0001);
            Settings.setDisplayFixture("single");
        }
    }
}
