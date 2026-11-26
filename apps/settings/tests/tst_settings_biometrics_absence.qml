// SPDX-License-Identifier: GPL-3.0-or-later
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

        // T285 removed the top-level Biometrics pane; the component stays for
        // T299, so this absence case mounts it directly.
        Component { id: biometricsComponent; BiometricsPane { } }

        function test_pane_degrades_with_every_owner_absent() {
            var pane = createTemporaryObject(biometricsComponent, stage,
                                             { width: 480, height: stage.height });
            waitForRendering(stage);
            compare(Settings.biometricsAvailable, false);
            verify(pane);
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true);
            compare(pane.fingers.length, 0);
            compare(pane.passwordButton.enabled, false);
            compare(pane.addFingerButton.enabled, false);
        }
    }
}
