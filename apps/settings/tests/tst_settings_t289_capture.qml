// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T289 Battery/Displays headless visual check (not part of ctest): both panes
// in light and dark, with the battery present/absent and the display
// present/projector/absent, plus the Options… and Advanced… sheets. Run
// through `scripts/capture-t289-settings-battery-displays.sh`.
Item {
    id: stage
    width: 740
    height: 820
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT289Capture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function setScheme(scheme) {
            stage.currentScheme = scheme;
            Settings.set("appearance.colorScheme", scheme);
            waitForRendering(stage);
        }

        function resetFixtures() {
            Settings.resetBatteryFixture();
            Settings.setBatteryFixture("present");
            Settings.setPowerProfile("balanced");
            Settings.setDisplayFixture("single");
            Settings.set("display.scale", 1.0);
            Settings.set("power.historyRange", 0);
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function capture(shell, name, item) {
            if (item) {
                var y = item.mapToItem(shell.detailContent, 0, 0).y;
                shell.detailScroll.flickable.contentY = Math.max(0, y - 12);
            } else {
                shell.detailScroll.flickable.contentY = 0;
            }
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t289-" + name + "-"
                                  + stage.currentScheme + ".png");
        }

        // Let a sheet's close animation finish so the next still is not a
        // ghost of the previous surface.
        function closeSheet(dialog) {
            dialog.hide();
            tryCompare(dialog, "open", false);
            wait(400);
        }

        function run(scheme) {
            resetFixtures();
            setScheme(scheme);
            var shell = make();

            // Battery: a present battery with sampler points, the Options…
            // sheet, and the absent service.
            Settings.setBatteryFixture("present");
            var battery = showPane(shell, "battery");
            capture(shell, "battery-present");
            battery.optionsDialog.show();
            tryCompare(battery.optionsDialog, "open", true);
            waitForRendering(stage);
            capture(shell, "battery-options");
            closeSheet(battery.optionsDialog);
            Settings.setBatteryFixture("absent");
            showPane(shell, "battery");
            capture(shell, "battery-absent");
            Settings.setBatteryFixture("present");

            // Displays: one real output, the Advanced… sheet, a projector
            // output, and no output.
            Settings.setDisplayFixture("single");
            var displays = showPane(shell, "displays");
            capture(shell, "displays-present");
            displays.advancedPopup.show();
            tryCompare(displays.advancedPopup, "open", true);
            waitForRendering(stage);
            capture(shell, "displays-advanced");
            closeSheet(displays.advancedPopup);
            Settings.setDisplayFixture("projector");
            tryVerify(function() { return displays.projectorPresent === true; });
            capture(shell, "displays-projector", displays.projectorGroup);
            Settings.setDisplayFixture("absent");
            tryVerify(function() { return displays.outputs.length === 0; });
            capture(shell, "displays-absent");
            Settings.setDisplayFixture("single");

            shell.destroy();
        }

        function test_capture_light() {
            run("light");
        }

        function test_capture_dark() {
            run("dark");
        }
    }
}
