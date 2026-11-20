// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291b Accessibility list + Physical and Motor headless visual check (not part
// of ctest): the two-row big list, the Physical and Motor sub-pane, and the
// reduce-motion/increase-contrast accommodation state at the measured 740×820
// window, in light and dark. Run through
// `scripts/capture-t291b-accessibility-physical-motor.sh`.
Item {
    id: stage
    width: 740
    height: 820
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT291bAccessibilityMotorCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global. Reset every key this pane
        // touches so a still never inherits the previous scheme's state.
        function resetState(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("input.keyboardNavigation", false);
            Settings.set("input.doubleClickSpeed", 400);
        }

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

        function openMotor(shell) {
            var pane = shell.paneBody.item;
            pane.openSubPane("motor");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            verify(pane.subPaneLoader.item);
            return pane.subPaneLoader.item;
        }

        function scrollTo(shell, item) {
            if (!item) {
                shell.detailScroll.flickable.contentY = 0;
                return;
            }
            var y = item.mapToItem(shell.detailContent, 0, 0).y;
            shell.detailScroll.flickable.contentY = Math.max(0, y - 12);
            waitForRendering(stage);
        }

        function capture(shell, name, item) {
            scrollTo(shell, item);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t291b-accessibility-"
                                  + name + "-" + stage.currentScheme + ".png");
        }

        function run(scheme) {
            stage.currentScheme = scheme;
            resetState(scheme);
            var shell = make();
            showPane(shell);

            // The mockup intro and the Vision + Physical and Motor big list.
            capture(shell, "list", null);

            var motor = openMotor(shell);
            capture(shell, "motor", null);

            // The accommodation state: one still only (dark), so the six-still
            // budget covers the light/dark pair, absence, and this state.
            if (scheme === "dark") {
                Settings.set("accessibility.reduceMotion", true);
                Settings.set("accessibility.increaseContrast", true);
                waitForRendering(stage);
                capture(shell, "motor-contrast", motor.keyboardGroup);
                Settings.set("accessibility.reduceMotion", false);
                Settings.set("accessibility.increaseContrast", false);
            }

            shell.paneBody.item.closeSubPane();
            waitForRendering(stage);
            shell.destroy();
        }

        function test_capture_light() { run("light"); }
        function test_capture_dark() { run("dark"); }
    }
}
