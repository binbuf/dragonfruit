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
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsBattery"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture client is process-global; reset the active profile before
        // every case so the pane always opens on a known state.
        function init() {
            Settings.refreshBattery();
            Settings.setBatteryFixture("present");
            Settings.setPowerProfile("balanced");
            Settings.set("power.showPercentage", false);
            Settings.set("power.historyRange", 0);
            Settings.set("power.dimOnBattery", false);
            Settings.set("power.preventSleep", false);
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

        function test_usage_history_chart_uses_the_real_sampler() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.historyRange, 0);
            compare(pane.historyCount, 8);
            verify(pane.historyHasData);
            verify(pane.batteryChart.visible);
            compare(pane.historyEmpty.visible, false);
            verify(pane.batteryChart.caption.indexOf("last 24 hours") >= 0);

            pane.rangeControl.activateIndex(1);
            compare(pane.historyRange, 1);
            verify(pane.batteryChart.caption.indexOf("last 10 days") >= 0);
            verify(pane.screenOnRow.visible);
            verify(pane.screenOnRow.description.indexOf("does not report") >= 0);
        }

        function test_an_empty_sampler_is_an_honest_empty_state_with_retry() {
            Settings.setBatteryFixture("empty");
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.historyCount, 0);
            compare(pane.historyHasData, false);
            compare(pane.batteryChart.visible, false);
            verify(pane.historyEmpty.visible);
            verify(pane.historyEmpty.height > 0);
            verify(pane.historyEmptyReason.indexOf("No battery history") >= 0);
            verify(pane.historyRetryButton.visible);
            pane.historyRetryButton.clicked();
            Settings.setBatteryFixture("present");
            shell.destroy();
        }

        function test_the_options_and_power_adapter_sections() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.optionsGroup.visible);
            compare(pane.dimOnBatteryToggle.checked, false);
            pane.dimOnBatteryToggle.toggle();
            compare(Settings.values["power.dimOnBattery"], true);
            compare(pane.preventSleepToggle.checked, false);
            pane.preventSleepToggle.toggle();
            compare(Settings.values["power.preventSleep"], true);
            compare(pane.wakeForNetworkToggle.enabled, false);
            compare(pane.optimizeVideoToggle.enabled, false);
            verify(pane.adapterGroup.visible);
            compare(pane.adapterStateText.text, "On battery");
        }

        function test_absence_keeps_the_structure_and_options_live() {
            Settings.setBatteryFixture("absent");
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.batteryAvailable, false);
            verify(pane.absenceNote.visible);
            verify(pane.batteryAbsenceNote.visible);
            verify(pane.optionsGroup.visible);
            verify(pane.powerSettingsGroup.visible);
            compare(pane.batteryChart.visible, false);
            Settings.setBatteryFixture("present");
            shell.destroy();
        }

        function test_battery_options_and_power_settings_link() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.optionsGroup.visible);
            compare(pane.showPercentageToggle.checked, false);
            pane.showPercentageToggle.toggle();
            compare(Settings.values["power.showPercentage"], true);

            compare(pane.historyRange, 0);
            pane.rangeControl.activateIndex(1);
            compare(Settings.values["power.historyRange"], 1);

            verify(pane.powerSettingsGroup.visible);
            pane.openLockScreenButton.clicked();
            tryVerify(function() { return shell.currentPaneId === "lock-screen"; });
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