// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291a Accessibility list + Vision headless visual check (not part of ctest):
// the mockup intro and big list, the Vision sub-pane, and the reduce-motion
// and increase-contrast states at the measured 740×820 window, in light and
// dark. Run through `scripts/capture-t291a-accessibility-vision.sh`.
Item {
    id: stage
    width: 740
    height: 820
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT291aAccessibilityVisionCapture"
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

        function openVision(shell) {
            var pane = shell.paneBody.item;
            pane.openSubPane("vision");
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
            grabImage(shell).save("docs/captures/t291a-accessibility-"
                                  + name + "-" + stage.currentScheme + ".png");
        }

        function run(scheme) {
            stage.currentScheme = scheme;
            resetState(scheme);
            var shell = make();
            var pane = showPane(shell);

            // The mockup intro and the one-row big list.
            capture(shell, "list", null);

            var vision = openVision(shell);
            capture(shell, "vision", null);

            // Reduce Motion is one live settingsd key.
            Settings.set("accessibility.reduceMotion", true);
            waitForRendering(stage);
            capture(shell, "vision-reduce-motion", vision.motionGroup);
            Settings.set("accessibility.reduceMotion", false);

            // Increase Contrast is the Glass resolver's other input.
            Settings.set("accessibility.increaseContrast", true);
            waitForRendering(stage);
            capture(shell, "vision-increase-contrast", vision.displayGroup);
            Settings.set("accessibility.increaseContrast", false);

            pane.closeSubPane();
            waitForRendering(stage);
            shell.destroy();
        }

        function test_capture_light() { run("light"); }
        function test_capture_dark() { run("dark"); }
    }
}
