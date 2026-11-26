// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T273 Displays headless visual check (not part of ctest): one real output,
// two outputs with the per-output selector, and no output, in light and dark
// at 740×820. Run through `scripts/capture-t273-data-panes.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT273DisplaysCapture"
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

            // One output: the real platform name and a proportional preview.
            Settings.setDisplayFixture("single");
            var shell = make();
            var pane = showPane(shell, "displays");
            grabImage(shell).save("docs/captures/t273-displays-single-" + scheme + ".png");

            // Two outputs: the per-output selector and the second output's
            // real refresh/aspect.
            Settings.setDisplayFixture("dual");
            tryVerify(function() { return pane.outputs.length === 2; });
            pane.chooseOutput("external");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t273-displays-dual-" + scheme + ".png");

            // No output: the preview says so and settingsd controls stay live.
            Settings.setDisplayFixture("absent");
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t273-displays-absence-" + scheme + ".png");

            Settings.setDisplayFixture("single");
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
