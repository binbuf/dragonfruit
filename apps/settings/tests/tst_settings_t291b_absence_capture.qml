// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291b Accessibility Physical and Motor absence capture (not part of ctest):
// the sub-pane with no on-screen-keyboard delegate, so the delegate row keeps
// its anatomy and disables only the action. Run through
// `scripts/capture-t291b-accessibility-physical-motor.sh` with
// `DF_ACCESSIBILITY_DELEGATE_FIXTURE=absent`. One still (light) keeps the
// T291b capture at six images.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT291bAccessibilityMotorAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function capture() {
            Settings.set("appearance.colorScheme", "light");
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("input.keyboardNavigation", false);
            Settings.set("input.doubleClickSpeed", 400);
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane);

            pane.openSubPane("motor");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var motor = pane.subPaneLoader.item;
            verify(motor);
            compare(motor.onScreenKeyboardAvailable, false);
            compare(motor.onScreenKeyboardRow.trailingActionEnabled, false);
            grabImage(shell).save(
                "docs/captures/t291b-accessibility-motor-absence-light.png");
            shell.destroy();
        }

        function test_absence() { capture(); }
    }
}
