// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291c Accessibility Hearing absence capture (not part of ctest): the sub-pane
// with no AT-SPI bridge host, so the Audio group keeps the stored-only balance
// row live while the ownerless sections stay map omissions. Run through
// `scripts/capture-t291c-accessibility-hearing.sh` with
// `DF_ACCESSIBILITY_FIXTURE=absent`. One still (light) keeps the T291c capture
// at six images.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT291cAccessibilityHearingAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function capture() {
            Settings.set("appearance.colorScheme", "light");
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("sound.balance", 0.5);
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane);
            compare(Settings.accessibilityAvailable, false);

            pane.openSubPane("hearing");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var hearing = pane.subPaneLoader.item;
            verify(hearing);
            compare(hearing.audioGroup.visible, true);
            grabImage(shell).save(
                "docs/captures/t291c-accessibility-hearing-absence-light.png");
            shell.destroy();
        }

        function test_absence() { capture(); }
    }
}
