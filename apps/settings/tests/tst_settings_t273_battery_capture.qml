// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T273 Battery headless visual check (not part of ctest): the live sampler
// chart, the honest empty state, and UPower/service absence, in light and
// dark at the measured 740×820 window. Run through
// `scripts/capture-t273-data-panes.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT273BatteryCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function scrollTo(shell, item) {
            var y = item.mapToItem(shell.detailContent, 0, 0).y;
            shell.detailScroll.flickable.contentY = Math.max(0, y - 12);
            waitForRendering(stage);
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.refreshBattery();

            // Live: the sampler has real points, so the chart draws.
            Settings.setBatteryFixture("present");
            var shell = make();
            var pane = showPane(shell, "battery");
            scrollTo(shell, pane.batteryChart);
            grabImage(shell).save("docs/captures/t273-battery-live-" + scheme + ".png");

            // Empty: a present battery before the sampler has two points.
            Settings.setBatteryFixture("empty");
            scrollTo(shell, pane.historyEmpty);
            grabImage(shell).save("docs/captures/t273-battery-empty-" + scheme + ".png");

            // Absence: the bridge host is not running; settingsd options stay.
            Settings.setBatteryFixture("absent");
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t273-battery-absence-" + scheme + ".png");

            Settings.setBatteryFixture("present");
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
