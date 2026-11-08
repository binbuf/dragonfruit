// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T228 headless visual check (not part of ctest): the Privacy & Security
// macOS-mapped list, one per-app permission detail, the Security group and its
// software-sources sheet, the Biometrics & Password pane, and the Online
// Accounts pane. Run through `scripts/capture-t228-privacy-accounts.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsSecurityCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width,
                                                height: stage.height,
                                                highlightHoldMs: 50 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function test_capture_privacy_security_biometrics_accounts() {
            var shell = make();

            // Privacy: the macOS-mapped rows with their Linux differences.
            var privacy = showPane(shell, "privacy");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t228-privacy-list.png");

            // One per-app permission detail: Camera's two fixture apps.
            privacy.openCategory("camera");
            wait(250);
            waitForRendering(stage);
            verify(privacy.permissionDialog.open);
            grabImage(shell).save("docs/captures/t228-privacy-detail.png");
            privacy.permissionDialog.accept();
            wait(250);

            // Security: scroll the group into view and capture it.
            var y = privacy.securityGroup.mapToItem(shell.detailContent, 0, 0).y;
            shell.detailScroll.flickable.contentY = Math.max(0, y - 12);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t228-security.png");

            // The software-sources sheet.
            privacy.sourcesButton.clicked();
            wait(250);
            waitForRendering(stage);
            verify(privacy.sourcesDialog.open);
            grabImage(shell).save("docs/captures/t228-security-sources.png");
            privacy.sourcesDialog.accept();
            wait(250);

            // Biometrics & Password.
            showPane(shell, "biometrics");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t228-biometrics.png");

            // Online Accounts.
            showPane(shell, "online-accounts");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t228-online-accounts.png");
        }
    }
}
