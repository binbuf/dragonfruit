// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Biometrics & Password absence (T228): no bridge host, no fprintd, no
// AccountsService. The pane still mounts and states the absence; it never
// draws a fake fingerprint row or a dead password button.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsBiometricsAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function test_pane_degrades_with_every_owner_absent() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            compare(Settings.biometricsAvailable, false);
            shell.selectPane("biometrics");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane);
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true);
            compare(pane.fingers.length, 0);
            compare(pane.passwordButton.enabled, false);
            compare(pane.addFingerButton.enabled, false);
        }
    }
}
