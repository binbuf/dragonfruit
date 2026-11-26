// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T273 Storage headless visual check (not part of ctest): the stacked chart
// with total/used/free and its legend, the volume list with capacity, the
// live Manage delegate, the empty UDisks2 state, and the absent-daemon state,
// in light and dark at 740×820. Run through
// `scripts/capture-t273-data-panes.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT273StorageCapture"
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

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.refreshStorage();

            // Live: one mountable volume, usage chart, legend, Manage.
            Settings.setStorageFixture("live");
            var shell = make();
            var pane = showPane(shell, "storage");
            grabImage(shell).save("docs/captures/t273-storage-live-" + scheme + ".png");

            // Empty: UDisks2 present with no mountable volume.
            Settings.setStorageFixture("empty");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t273-storage-empty-" + scheme + ".png");

            // Absent: UDisks2 gone; df usage and Manage stay live.
            Settings.setStorageFixture("absent");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t273-storage-absence-" + scheme + ".png");

            Settings.setStorageFixture("live");
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
