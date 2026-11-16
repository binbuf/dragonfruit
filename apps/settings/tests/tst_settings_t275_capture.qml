// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T275 Accessibility headless visual check (not part of ctest): every section,
// the delegate rows, and the absent-AT-SPI structure at the measured 740×820
// window. Run through `scripts/capture-t275-accessibility.sh`.
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
            save(shell, "t275-accessibility-vision-" + scheme);

            scrollTo(shell, pane.displayGroup);
            save(shell, "t275-accessibility-display-" + scheme);

            scrollTo(shell, pane.magnifierGroup);
            save(shell, "t275-accessibility-zoom-" + scheme);

            scrollTo(shell, pane.motionGroup);
            save(shell, "t275-accessibility-motion-" + scheme);

            scrollTo(shell, pane.readSpeakGroup);
            save(shell, "t275-accessibility-read-speak-" + scheme);

            scrollTo(shell, pane.hearingGroup);
            save(shell, "t275-accessibility-hearing-" + scheme);

            scrollTo(shell, pane.motorGroup);
            save(shell, "t275-accessibility-motor-" + scheme);

            scrollTo(shell, pane.speechGroup);
            save(shell, "t275-accessibility-speech-" + scheme);

            scrollTo(shell, pane.shortcutGroup);
            save(shell, "t275-accessibility-shortcut-" + scheme);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
