// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T275 Accessibility headless visual check (not part of ctest).
//
// T291a replaced the long form with the mockup big list and in-place
// sub-panes. This legacy target now captures the top-level list and the
// Vision sub-pane it ships; T291b-T291e extend the list and their own
// captures. Run through `scripts/capture-t275-accessibility.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT275AccessibilityCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell) {
            shell.selectPane("accessibility");
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

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            var shell = make();
            var pane = showPane(shell);

            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            save(shell, "t275-accessibility-list-" + scheme);

            pane.openSubPane("vision");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var vision = pane.subPaneLoader.item;
            verify(vision);

            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            save(shell, "t275-accessibility-vision-" + scheme);

            scrollTo(shell, vision.displayGroup);
            save(shell, "t275-accessibility-display-" + scheme);

            scrollTo(shell, vision.zoomGroup);
            save(shell, "t275-accessibility-zoom-" + scheme);

            scrollTo(shell, vision.motionGroup);
            save(shell, "t275-accessibility-motion-" + scheme);

            scrollTo(shell, vision.readSpeakGroup);
            save(shell, "t275-accessibility-read-speak-" + scheme);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
