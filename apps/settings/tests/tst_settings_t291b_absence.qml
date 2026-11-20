// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291b Physical and Motor absence (part of ctest): with no AT-SPI host and no
// on-screen-keyboard program, the delegate row keeps its anatomy and disables
// only the action with a reason, while the settingsd rows stay live. Run with
// `DF_ACCESSIBILITY_FIXTURE=absent` and
// `DF_ACCESSIBILITY_DELEGATE_FIXTURE=absent`.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsAccessibilityMotorAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.set("input.keyboardNavigation", false);
            Settings.set("input.doubleClickSpeed", 400);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function openMotor(shell) {
            var pane = shell.paneBody.item;
            verify(pane);
            pane.openSubPane("motor");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var motor = pane.subPaneLoader.item;
            verify(motor);
            return motor;
        }

        function test_the_absent_delegate_is_a_disabled_row_with_a_reason() {
            var shell = make();
            var motor = openMotor(shell);

            compare(Settings.accessibilityOnScreenKeyboardAvailable, false);
            compare(motor.onScreenKeyboardAvailable, false);
            compare(motor.onScreenKeyboardRow.visible, true,
                    "an absent delegate stays a row, never an omission");
            compare(motor.onScreenKeyboardRow.trailingActionEnabled, false);
            compare(motor.onScreenKeyboardRow.trailingAction, "Not installed");
            verify(motor.onScreenKeyboardRow.description.indexOf("Not available") >= 0,
                   "the row explains why the action is disabled");

            // The delegate cannot launch.
            motor.onScreenKeyboardRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "");
        }

        function test_the_durable_rows_stay_live_without_the_delegate() {
            var shell = make();
            var motor = openMotor(shell);

            motor.keyboardNavigationToggle.toggle();
            compare(Settings.values["input.keyboardNavigation"], true);

            motor.doubleClickSpeedSlider.setValue(250);
            motor.doubleClickSpeedSlider.commit();
            compare(Settings.values["input.doubleClickSpeed"], 250);

            // The omitted rows stay omitted in this state too.
            compare(SettingsMap.rowFor("accessibility",
                                       "motor.switchControl").state, "omitted");
        }
    }
}
