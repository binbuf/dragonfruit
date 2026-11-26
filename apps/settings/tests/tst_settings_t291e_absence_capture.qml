// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291e Accessibility absence capture (not part of ctest): with no AT-SPI
// bridge host and no speech delegate, the complete five-row list keeps its
// structure, the Vision status rows say Unavailable, and the General shortcut
// row stays live because it is a settingsd preference. Run through
// `scripts/capture-t291e-accessibility-general.sh` with
// `DF_ACCESSIBILITY_FIXTURE=absent` and
// `DF_ACCESSIBILITY_DELEGATE_FIXTURE=absent`. Two stills (light/dark).
Item {
    id: stage
    width: 740
    height: 820
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT291eAccessibilityGeneralAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function run(scheme) {
            stage.currentScheme = scheme;
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("accessibility.shortcut", "");

            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane);
            compare(Settings.accessibilityAvailable, false);
            compare(Settings.accessibilitySpeechAvailable, false);
            compare(pane.bigList.rows.children.length, 5);
            grabImage(shell).save("docs/captures/t291e-accessibility-absence-list-"
                                  + stage.currentScheme + ".png");

            // The Vision body is the one that changes with the host gone.
            pane.openSubPane("vision");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var vision = pane.subPaneLoader.item;
            verify(vision);
            compare(vision.ready, false);
            compare(vision.absenceNote.visible, true);
            grabImage(shell).save("docs/captures/t291e-accessibility-absence-vision-"
                                  + stage.currentScheme + ".png");
            shell.destroy();
        }

        function test_absence_light() { run("light"); }
        function test_absence_dark() { run("dark"); }
    }
}
