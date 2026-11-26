// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T275 Accessibility absence capture (not part of ctest): the pane with no
// AT-SPI bridge host. Run through `scripts/capture-t275-accessibility.sh`
// with `DF_ACCESSIBILITY_FIXTURE=absent`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT275AccessibilityAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane);
            pane.openSubPane("vision");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var vision = pane.subPaneLoader.item;
            verify(vision);
            compare(vision.ready, false);
            compare(vision.absenceNote.visible, true);
            grabImage(shell).save("docs/captures/t275-accessibility-absence-"
                                  + scheme + ".png");
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
