// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291d Accessibility list + Speech headless visual check (not part of ctest):
// the four-row big list and the Speech sub-pane at the measured 740×820
// window, in light and dark. Run through
// `scripts/capture-t291d-accessibility-speech.sh`.
Item {
    id: stage
    width: 740
    height: 820
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT291dAccessibilitySpeechCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function resetState(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
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

        function openSpeech(shell) {
            var pane = shell.paneBody.item;
            pane.openSubPane("speech");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            verify(pane.subPaneLoader.item);
            return pane.subPaneLoader.item;
        }

        function capture(shell, name) {
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t291d-accessibility-"
                                  + name + "-" + stage.currentScheme + ".png");
        }

        function run(scheme) {
            stage.currentScheme = scheme;
            resetState(scheme);
            var shell = make();
            showPane(shell);
            capture(shell, "list");
            openSpeech(shell);
            capture(shell, "speech");
            shell.paneBody.item.closeSubPane();
            waitForRendering(stage);
            shell.destroy();
        }

        function test_capture_light() { run("light"); }
        function test_capture_dark() { run("dark"); }
    }
}
