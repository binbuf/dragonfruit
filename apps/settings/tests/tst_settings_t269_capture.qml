// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T269 headless visual check (not part of ctest): the General list, About,
// System Report, Software Update, Storage, Autofill & Passwords, and the three
// partial-absence states in light and dark. Run through
// `scripts/capture-t269-general.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT269Capture"
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

        function openSubPane(pane, id) {
            pane.openSubPane(id);
            tryVerify(function() { return pane.subPaneLoader.status === Loader.Ready; });
            waitForRendering(stage);
            verify(pane.subPaneLoader.item);
            return pane.subPaneLoader.item;
        }

        function reset() {
            Settings.resetUpdatesFixture();
            Settings.resetGeneralFixture();
            Settings.setGeneralAvailableFixture(true);
            Settings.setUpdatesAbsentFixture(false);
            Settings.setUpdatesProviderFixture(true);
            Settings.set("defaults.passwordManager", "");
        }

        function capture(scheme) {
            reset();
            Settings.set("appearance.colorScheme", scheme);
            var shell = make();
            var pane = showPane(shell, "general");
            grabImage(shell).save("docs/captures/t269-general-list-" + scheme + ".png");

            openSubPane(pane, "about");
            grabImage(shell).save("docs/captures/t269-general-about-" + scheme + ".png");
            pane.subPaneLoader.item.openReport();
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t269-general-system-report-" + scheme + ".png");
            pane.subPaneLoader.item.reportDialog.accept();
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "software-update");
            grabImage(shell).save("docs/captures/t269-general-software-update-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "storage");
            grabImage(shell).save("docs/captures/t269-general-storage-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "passwords");
            grabImage(shell).save("docs/captures/t269-general-autofill-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            // Only the update host is present.
            Settings.setGeneralAvailableFixture(false);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t269-general-absence-updates-only-" + scheme + ".png");

            // Only the General host is present.
            Settings.setGeneralAvailableFixture(true);
            Settings.setUpdatesAbsentFixture(true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t269-general-absence-general-only-" + scheme + ".png");

            // Neither half is present.
            Settings.setGeneralAvailableFixture(false);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t269-general-absence-both-" + scheme + ".png");

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
