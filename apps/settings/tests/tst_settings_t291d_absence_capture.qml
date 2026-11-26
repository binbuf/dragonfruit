// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291d Accessibility Speech absence capture (not part of ctest): with no
// AT-SPI bridge host and no speech engine the Live Speech row stays visible
// and disables only its sample action with the missing-program reason. Run
// through `scripts/capture-t291d-accessibility-speech.sh` with
// `DF_ACCESSIBILITY_FIXTURE=absent` and
// `DF_ACCESSIBILITY_DELEGATE_FIXTURE=absent`. One still (light) keeps the
// T291d capture at five images.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT291dAccessibilitySpeechAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function capture() {
            Settings.set("appearance.colorScheme", "light");
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

            pane.openSubPane("speech");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var speech = pane.subPaneLoader.item;
            verify(speech);
            compare(speech.speakSampleRow.trailingActionEnabled, false);
            grabImage(shell).save(
                "docs/captures/t291d-accessibility-speech-absence-light.png");
            shell.destroy();
        }

        function test_absence() { capture(); }
    }
}
