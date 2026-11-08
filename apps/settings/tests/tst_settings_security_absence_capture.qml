// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T228 absence stills: no settingsd, no portal, no bridge host, no fprintd,
// no GOA, no LUKS/polkit read. The panes state the absence and the optional
// Online Accounts pane is absent from the sidebar. Run through
// `scripts/capture-t228-privacy-accounts.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsSecurityAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function test_capture_absence() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width,
                                                height: stage.height,
                                                highlightHoldMs: 50 });
            waitForRendering(stage);
            compare(Settings.privacyAvailable, false);
            compare(Settings.biometricsAvailable, false);
            compare(Settings.onlineAccountsAvailable, false);

            showPane(shell, "privacy");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t228-privacy-absence.png");

            showPane(shell, "biometrics");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t228-biometrics-absence.png");

            // Online Accounts is hidden, not a dead entry: a search for it
            // returns no pane.
            showPane(shell, "appearance");
            shell.searchText = "online accounts";
            waitForRendering(stage);
            compare(shell.visiblePanes.length, 0);
            grabImage(shell).save("docs/captures/t228-online-accounts-absence.png");
        }
    }
}
