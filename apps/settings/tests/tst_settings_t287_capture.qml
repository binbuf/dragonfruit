// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T287 headless visual check (not part of ctest): the General big list and
// every General sub-pane, in light and dark, plus the host-absent state. Run
// through `scripts/capture-t287-settings-general.sh`.
Item {
    id: stage
    width: 1000
    height: 900
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT287Capture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function setScheme(scheme) {
            stage.currentScheme = scheme;
            Settings.set("appearance.colorScheme", scheme);
            waitForRendering(stage);
        }

        function resetFixtures() {
            Settings.resetUpdatesFixture();
            Settings.resetGeneralFixture();
            Settings.setGeneralAvailableFixture(true);
            Settings.setUpdatesAbsentFixture(false);
            Settings.setUpdatesProviderFixture(true);
            Settings.set("defaults.passwordManager", "");
        }

        function capture(shell, name) {
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t287-" + name + "-"
                                  + stage.currentScheme + ".png");
        }

        function capturePane(shell, id) {
            var pane = shell.paneBody.item;
            pane.openSubPane(id);
            tryVerify(function() { return pane.subPaneLoader.status === Loader.Ready; });
            waitForRendering(stage);
            capture(shell, "general-" + id);
            pane.closeSubPane();
            waitForRendering(stage);
        }

        function run(scheme) {
            resetFixtures();
            setScheme(scheme);
            var shell = make();
            capture(shell, "general-list");

            var ids = ["about", "software-update", "storage", "passwords",
                       "date-time", "language", "login", "sharing", "startup",
                       "backup", "reset"];
            for (var i = 0; i < ids.length; ++i)
                capturePane(shell, ids[i]);

            // The provider-absent card is an honest state, not an empty group.
            Settings.setUpdatesProviderFixture(false);
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            pane.openSubPane("software-update");
            tryVerify(function() { return pane.subPaneLoader.status === Loader.Ready; });
            waitForRendering(stage);
            capture(shell, "general-software-update-no-provider");
            pane.closeSubPane();

            // The host-absent state keeps every row with its own summary.
            Settings.setGeneralAvailableFixture(false);
            waitForRendering(stage);
            capture(shell, "general-absence");

            Settings.setUpdatesProviderFixture(true);
            Settings.setGeneralAvailableFixture(true);
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
