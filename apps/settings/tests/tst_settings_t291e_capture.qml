// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291e Accessibility close-out headless visual check (not part of ctest): the
// complete five-row big list, every shipped sub-pane (Vision, Physical and
// Motor, Hearing, Speech, General), and the reduce-motion and
// increase-contrast accommodation states at the measured 740×820 window, in
// light and dark. Run through `scripts/capture-t291e-accessibility-general.sh`.
Item {
    id: stage
    width: 740
    height: 820
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT291eAccessibilityGeneralCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global. Reset every key this pane
        // touches so a still never inherits the previous scheme's state.
        function resetState(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("accessibility.differentiateWithoutColor", false);
            Settings.set("accessibility.textSize", "default");
            Settings.set("accessibility.dimFlashingLights", false);
            Settings.set("accessibility.magnifier", false);
            Settings.set("accessibility.magnifierZoom", 2.0);
            Settings.set("accessibility.magnifierMode", "follow-focus");
            Settings.set("accessibility.shortcut", "");
            Settings.set("sound.balance", 0.5);
            Settings.set("input.doubleClickSpeed", 400);
            Settings.set("input.keyboardNavigation", false);
            Settings.set("appearance.cursorSize", 24);
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

        function openSubPane(shell, id) {
            var pane = shell.paneBody.item;
            pane.openSubPane(id);
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            verify(pane.subPaneLoader.item, id + " must render a body");
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
            grabImage(shell).save("docs/captures/t291e-accessibility-"
                                  + name + "-" + stage.currentScheme + ".png");
        }

        function run(scheme) {
            stage.currentScheme = scheme;
            resetState(scheme);
            var shell = make();
            var pane = showPane(shell);

            // The complete mockup list: five sections after the intro.
            compare(pane.bigList.rows.children.length, 5);
            capture(shell, "list", null);

            // Every shipped sub-pane, in mockup order.
            var vision = openSubPane(shell, "vision");
            capture(shell, "vision", null);

            // Reduce Motion is one live settingsd key (screen-reader fans see
            // the accommodation state on the Motion group).
            Settings.set("accessibility.reduceMotion", true);
            waitForRendering(stage);
            capture(shell, "reduce-motion", vision.motionGroup);
            Settings.set("accessibility.reduceMotion", false);
            pane.closeSubPane();
            waitForRendering(stage);

            var motor = openSubPane(shell, "motor");
            capture(shell, "motor", null);
            pane.closeSubPane();
            waitForRendering(stage);

            var hearing = openSubPane(shell, "hearing");
            capture(shell, "hearing", null);
            pane.closeSubPane();
            waitForRendering(stage);

            var speech = openSubPane(shell, "speech");
            capture(shell, "speech", null);
            pane.closeSubPane();
            waitForRendering(stage);

            // General: the Accessibility Shortcut recorder, empty and recorded.
            var general = openSubPane(shell, "general");
            capture(shell, "general", null);
            general.recorder.startRecording();
            keyClick(Qt.Key_K, Qt.MetaModifier | Qt.ShiftModifier);
            compare(Settings.values["accessibility.shortcut"],
                    "Cmd+Shift+K");
            waitForRendering(stage);
            capture(shell, "general-recorded", general.shortcutGroup);
            Settings.set("accessibility.shortcut", "");
            pane.closeSubPane();
            waitForRendering(stage);

            // Increase Contrast is the Glass resolver's other input; show it
            // on the list so the whole pane's chrome is covered.
            Settings.set("accessibility.increaseContrast", true);
            waitForRendering(stage);
            capture(shell, "increase-contrast", null);
            Settings.set("accessibility.increaseContrast", false);

            shell.destroy();
        }

        function test_capture_light() { run("light"); }
        function test_capture_dark() { run("dark"); }
    }
}
