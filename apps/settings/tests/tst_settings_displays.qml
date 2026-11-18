// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Displays-pane tests (T-09.5, completed T273; mockup order T289). Headless
// with `DF_SETTINGS_FIXTURE` and `DF_DISPLAY_FIXTURE`, so the `Settings`
// singleton serves the schema defaults and a deterministic real-output list:
// no bus, no compositor, no daemon. Every control is asserted to apply live to
// its `display.*` key on the same event-loop turn, and an external settingsd
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

        function collectRows(item, out) {
            if (!item || item.children === undefined)
                return out;
            if (item.settingsId !== undefined && item.label !== undefined
                    && String(item.settingsId).length > 0)
                out.push(item);
            var children = item.children;
            for (var i = 0; i < children.length; ++i)
                collectRows(children[i], out);
            return out;
        }

        function test_pane_opens_with_the_display_row_and_resolution_radios() {
            var shell = make();
            compare(shell.currentPaneId, "displays");
            var pane = paneOf(shell);

            compare(pane.outputs.length, 1);
            compare(pane.outputName, "Built-in Display");
            compare(pane.displayRow.label, "Built-in Display");
            compare(pane.displayRow.description, "1512 × 982 (Default) · 120 Hz");
            // Three of the five scale steps are shown by default; the selected
            // one is always visible.
            compare(pane.resolutionRadios.count, 3);
            compare(pane.resolutionRadios.currentIndex, 1);
            compare(pane.resolutionVisibleOptions[1].label,
                    "1512 × 982 (Default)");
            verify(pane.scalingNote.text.indexOf("scaled resolution") >= 0);
            compare(pane.absenceNote.visible, false);
            compare(pane.advancedButton.text, "Advanced…");
            compare(pane.nightLightButton.text, "Night Light…");
            compare(pane.refreshRow.description.indexOf("Currently 120 Hz") >= 0, true);
        }

        function test_show_all_resolutions_reveals_the_full_scale_list() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.showAllResolutions, false);
            compare(pane.resolutionRadios.count, 3);

            pane.showAllResolutionsToggle.toggle();
            compare(pane.showAllResolutions, true);
            compare(pane.resolutionRadios.count, 5);
            compare(pane.resolutionRadios.currentIndex, 2);
            verify(pane.resolutionVisibleOptions[0].label.indexOf("Larger Text") >= 0);
            verify(pane.resolutionVisibleOptions[4].label.indexOf("Most Space") >= 0);

            pane.resolutionRadios.select(0); // Larger Text -> 1.5
            verify(Math.abs(Settings.values["display.scale"] - 1.5) < 0.001);
            compare(pane.resolutionRadios.currentIndex, 0);
        }

        function test_resolution_radios_apply_live_and_converge() {
            var shell = make();
            var pane = paneOf(shell);

            pane.resolutionRadios.select(0); // Large -> 1.25
            verify(Math.abs(Settings.values["display.scale"] - 1.25) < 0.001);
            compare(pane.resolutionRadios.currentIndex, 0);
            verify(pane.displayRow.description.indexOf("(Large)") >= 0);

            Settings.set("display.scale", 0.875);
            compare(pane.resolutionRadios.currentIndex, 2);
            verify(pane.displayRow.description.indexOf("(More Space)") >= 0);

            // An externally selected hidden step is added to the visible list
            // rather than leaving the group with no selection.
            Settings.set("display.scale", 1.5);
            compare(pane.resolutionRadios.count, 4);
            // The selected hidden step is added to the list (first, because it
            // is the highest scale); the group always has one selection.
            compare(pane.resolutionRadios.currentIndex, 0);
            compare(pane.resolutionVisibleOptions[0].value, 1.5);
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
            compare(pane.ambientColorToggle.enabled, false);
            verify(pane.ambientColorRow.description.length > 0);
            compare(pane.colorProfileSelect.enabled, false);
            verify(pane.colorPresetRow.description.length > 0);
            compare(pane.tvModeSelect.enabled, false);
            compare(pane.arrangementButton.enabled, false);
            verify(capability(pane, "hdr").supported === false);
            verify(capability(pane, "vrr").supported === false);
            verify(capability(pane, "refreshRate").reason.length > 0);
            verify(capability(pane, "refreshRate").reason.indexOf("OPEN") < 0);
        }

        function test_display_details_sheet_lists_the_real_outputs() {
            var shell = make();
            var pane = paneOf(shell);
            pane.displayRow.disclosureClicked();
            tryCompare(pane.displayDetailsDialog, "open", true);
            compare(pane.outputRepeater.count, 1);
            pane.displayDetailsDialog.hide();

            Settings.setDisplayFixture("dual");
            tryVerify(function() { return pane.outputs.length === 2; });
            pane.displayDetailsDialog.show();
            compare(pane.outputRepeater.count, 2);
            pane.chooseOutput("external");
            compare(pane.outputName, "External Display");
            compare(pane.displayRow.label, "External Display");
            compare(pane.displayRow.description, "2560 × 1440 (Default) · 60 Hz");
            verify(pane.refreshRow.description.indexOf("Currently 60 Hz") >= 0);
            pane.displayDetailsDialog.hide();
            Settings.setDisplayFixture("single");
        }

        function test_projector_row_only_when_the_host_reports_one() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.projectorGroup.visible);
            compare(pane.projectorPresent, false);
            compare(pane.projectorModeRow.visible, false);
            verify(pane.projectorAbsenceRow.visible);
            verify(pane.projectorAbsenceRow.description.indexOf("No TV or projector") >= 0);

            Settings.setDisplayFixture("projector");
            tryVerify(function() { return pane.projectorPresent === true; });
            compare(pane.projectorModeRow.visible, true);
            compare(pane.projectorAbsenceRow.visible, false);
            compare(pane.tvModeSelect.enabled, false);
            verify(pane.projectorModeRow.description.indexOf("not available") >= 0);
            Settings.setDisplayFixture("single");
        }

        function test_advanced_sheet_holds_arrangement_rotation_and_capabilities() {
            var shell = make();
            var pane = paneOf(shell);

            pane.advancedButton.clicked();
            tryCompare(pane.advancedPopup, "open", true);
            compare(pane.advancedRepeater.count, pane.advancedRows.length);
            for (var i = 0; i < pane.advancedRows.length; ++i) {
                verify(String(pane.advancedRows[i].id) !== "brightness");
                verify(String(pane.advancedRows[i].id) !== "rotation");
            }
            verify(pane.arrangementGroup.visible);
            verify(pane.rotationGroup.visible);
            verify(pane.useAsGroup.visible);
            compare(pane.rotationSelect.enabled, true);

            pane.rotationSelect.activateIndex(3); // 270°
            compare(Settings.values["display.rotation"], "270");
            compare(pane.rotationSelect.currentIndex, 3);
            Settings.set("display.rotation", "normal");
            compare(pane.rotationSelect.currentIndex, 0);
            pane.advancedPopup.hide();
        }

        function test_night_light_sheet_holds_the_real_rows() {
            var shell = make();
            var pane = paneOf(shell);
            pane.nightLightButton.clicked();
            tryCompare(pane.nightLightDialog, "open", true);
            verify(pane.nightLightGroup.visible);
            compare(pane.nightLightToggle.enabled, false);
            compare(pane.nightLightScheduleSelect.enabled, false);
            compare(pane.nightLightTemperatureSlider.enabled, false);
            verify(pane.nightLightRow.description.length > 0);
            verify(pane.nightLightRow.description.indexOf("OPEN") < 0);
            pane.nightLightDialog.hide();
        }

        function test_absence_keeps_settingsd_controls_live() {
            Settings.setDisplayFixture("absent");
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.outputs.length, 0);
            compare(pane.outputName, "No display");
            compare(pane.displayRow.label, "No display");
            compare(pane.displayRow.description, "No display");
            verify(pane.absenceNote.visible);
            // The settingsd controls are not host reads and stay live.
            compare(pane.brightnessSlider.enabled, true);
            compare(pane.rotationSelect.enabled, true);
            compare(pane.resolutionRadios.count, 3);
            verify(pane.resolutionVisibleOptions[1].label.indexOf("Default") >= 0);
            verify(pane.resolutionVisibleOptions[1].label.indexOf("×") < 0,
                   "no fabricated resolution without an output");
            pane.resolutionRadios.select(0); // Large -> 1.25
            verify(Math.abs(Settings.values["display.scale"] - 1.25) < 0.001);
            pane.brightnessSlider.setValue(0.3);
            pane.brightnessSlider.commit();
            fuzzyCompare(Settings.values["display.brightness"], 0.3, 0.0001);
            Settings.setDisplayFixture("single");
        }

        function test_every_rendered_row_has_a_shipped_map_row() {
            var shell = make();
            var pane = paneOf(shell);
            var rows = collectRows(pane, []);
            collectRows(pane.displayDetailsDialog, rows);
            collectRows(pane.advancedPopup, rows);
            collectRows(pane.nightLightDialog, rows);
            verify(rows.length >= 12, "walked " + rows.length + " rows");
            for (var i = 0; i < rows.length; ++i) {
                var id = String(rows[i].settingsId);
                var mapRow = SettingsMap.rowFor("displays", id);
                verify(mapRow !== null,
                       "rendered row '" + id + "' must have a map row");
                compare(mapRow.state, "shipped", id);
            }
        }
    }
}
