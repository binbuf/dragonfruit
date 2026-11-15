// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T264 headless visual check (not part of ctest): the General -> About journey
// (open, Back to the list) and a row deep link into Language -> Region, in
// light and dark. Run through `scripts/capture-t264-settings-navigation.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsT264Capture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820,
                                                highlightHoldMs: 60000 });
            waitForRendering(stage);
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function setScheme(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            waitForRendering(stage);
        }

        function capture(shell, path) {
            waitForRendering(stage);
            grabImage(shell).save(path);
        }

        function journey(scheme) {
            setScheme(scheme);
            var shell = make();
            capture(shell, "docs/captures/t264-settings-general-" + scheme + ".png");

            // General -> About is an in-place sub-pane.
            shell.openSubPane("about");
            tryVerify(function() {
                return shell.currentSubPaneId === "about"
                    && shell.paneBody.item.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            capture(shell, "docs/captures/t264-settings-about-" + scheme + ".png");

            // Back closes the sub-pane before it changes pane.
            shell.back();
            tryVerify(function() { return shell.currentSubPaneId === ""; });
            compare(shell.currentPaneId, "general");
            capture(shell, "docs/captures/t264-settings-back-" + scheme + ".png");

            // A row deep link opens the sub-pane and lands on the row.
            verify(shell.deepLink(
                "settings://pane/general/row/language.region"));
            tryVerify(function() {
                return shell.currentSubPaneId === "language"
                    && shell.highlightTarget !== null
                    && shell.highlightTarget.settingsId === "language.region";
            });
            wait(60);
            waitForRendering(stage);
            capture(shell, "docs/captures/t264-settings-deep-link-" + scheme + ".png");
        }

        function test_capture_light() {
            journey("light");
        }

        function test_capture_dark() {
            journey("dark");
        }
    }
}
