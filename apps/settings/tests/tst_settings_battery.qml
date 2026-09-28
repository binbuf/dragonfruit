// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Battery-pane tests (T-15.6b). Headless with `DF_SETTINGS_FIXTURE` and
// `DF_BATTERY_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton serves
// a deterministic in-process battery adapter with no bus. The cases cover the
// Low Power Mode profile picker round-tripping through the adapter, the
// Battery Health/Charging info rows, the Usage History range switch, and the
// absence note.
Item {
    id: stage
    width: 900
    height: 1400

    TestCase {
        id: testCase
        name: "SettingsBattery"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture client is process-global; reset the active profile before
        // every case so the pane always opens on a known state.
        function init() {
            Settings.setPowerProfile("balanced");
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("battery");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Battery body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_battery_and_profiles() {
            var shell = make();
            compare(shell.currentPaneId, "battery");
            var pane = paneOf(shell);
            verify(pane.present, "the battery view must be present");
            verify(pane.profilesAvailable, "the profiles daemon must be present");
            compare(pane.percent, 71);
            compare(pane.healthLabel, "Normal");
            compare(pane.capacity, 96);
            compare(pane.chargeCycles, 112);
            compare(pane.chargeLabel, "On Battery");
            compare(pane.profiles.length, 3);
            compare(pane.activeProfile, "balanced");
            compare(pane.profileSelect.currentIndex, 1);
            compare(pane.profileSelect.currentLabel, "Balanced");
            compare(pane.absenceNote.visible, false);
        }

        function test_profile_picker_round_trips_through_the_adapter() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.battery.activeProfile, "balanced");

            pane.profileSelect.activateIndex(0); // Power Saver
            compare(Settings.battery.activeProfile, "power-saver");
            compare(pane.activeProfile, "power-saver");
            compare(pane.profileSelect.currentIndex, 0);
            compare(pane.profileSelect.currentLabel, "Power Saver");

            pane.profileSelect.activateIndex(2); // Performance
            compare(Settings.battery.activeProfile, "performance");
            compare(pane.profileLabel, "Performance");
        }

        function test_external_profile_change_converges_into_the_pane() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setPowerProfile("power-saver");
            compare(pane.activeProfile, "power-saver");
            compare(pane.profileSelect.currentIndex, 0);
        }

        function test_info_rows_expand_their_details() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.healthDetail.visible, false);
            compare(pane.healthDetail.text, "Capacity 96% of design \u00b7 112 cycles");
            pane.healthInfoButton.activate();
            compare(pane.healthInfoOpen, true);
            compare(pane.healthDetail.visible, true);

            compare(pane.chargingDetail.visible, false);
            verify(pane.chargingDetail.text.indexOf("2 h") >= 0,
                   "the discharging estimate uses time-to-empty");
            pane.chargingInfoButton.activate();
            compare(pane.chargingDetail.visible, true);
        }

        function test_usage_history_range_switch_rewords_the_empty_state() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.historyRange, 0);
            verify(pane.batteryChart.caption.indexOf("last 24 hours") >= 0);
            verify(pane.screenChart.caption.indexOf("last 24 hours") >= 0);

            pane.rangeControl.activateIndex(1);
            compare(pane.historyRange, 1);
            verify(pane.batteryChart.caption.indexOf("last 10 days") >= 0);
            verify(pane.screenChart.caption.indexOf("last 10 days") >= 0);
        }

        function test_absence_note_hides_when_the_service_is_present() {
            // In fixture mode the bridge client is available, so the pane shows
            // its controls, not the absence note. The no-host path is covered by
            // tst_settings_absence.qml.
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.batteryAvailable, true);
            compare(pane.absenceNote.visible, false);
        }
    }
}