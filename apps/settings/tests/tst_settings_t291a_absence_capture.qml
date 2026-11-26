// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291a Accessibility absence capture (not part of ctest): the mockup list and
// the Vision sub-pane with no AT-SPI bridge host. Run through
// `scripts/capture-t291a-accessibility-vision.sh` with
// `DF_ACCESSIBILITY_FIXTURE=absent`.
Item {
    id: stage
    width: 740
    height: 820
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT291aAccessibilityVisionAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function capture(scheme) {
            stage.currentScheme = scheme;
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane);

            grabImage(shell).save("docs/captures/t291a-accessibility-absence-list-"
                                  + scheme + ".png");

            pane.openSubPane("vision");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var vision = pane.subPaneLoader.item;
            verify(vision);
            compare(vision.ready, false);
            compare(vision.absenceNote.visible, true);
            grabImage(shell).save("docs/captures/t291a-accessibility-absence-vision-"
                                  + scheme + ".png");
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
