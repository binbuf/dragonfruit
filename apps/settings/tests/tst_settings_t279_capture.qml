// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T279 account/security headless visual check (not part of ctest): the
// Privacy & Security, Biometrics & Password, Users & Groups, and Online
// Accounts panes at the measured 740×820 window in light and dark, including
// the category, enrollment, add-user, and account-details surfaces. Run
// through `scripts/capture-t279-accounts-security.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT279AccountsSecurityCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, paneId) {
            shell.selectPane(paneId);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            wait(200);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        function prepare() {
            Settings.resetPrivacyFixture();
            Settings.setPrivacyAvailableFixture(true);
            Settings.setAccessoriesAvailableFixture(true);
            Settings.set("privacy.locationServices", true);
            Settings.set("privacy.analytics", false);
            Settings.resetBiometricsFixture();
            Settings.setBiometricsReaderFixture(true);
            Settings.setBiometricsDeniedFixture(false);
            Settings.setPasswordOutcomeFixture("applied", "");
            Settings.resetOnlineAccountsFixture();
            Settings.setOnlineAttentionFixture(false);
            Settings.resetAccountsFixture();
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            prepare();
            var shell = make();

            var privacy = showPane(shell, "privacy");
            save(shell, "t279-privacy-" + scheme);
            privacy.openCategory("camera");
            waitForRendering(stage);
            wait(150);
            save(shell, "t279-privacy-category-" + scheme);
            privacy.permissionDialog.accept();
            waitForRendering(stage);
            wait(150);

            var biometrics = showPane(shell, "biometrics");
            save(shell, "t279-biometrics-" + scheme);
            biometrics.addFingerButton.clicked();
            biometrics.fingerSelect.activateIndex(4);
            biometrics.startEnrollment();
            waitForRendering(stage);
            wait(150);
            save(shell, "t279-biometrics-enroll-" + scheme);
            Settings.cancelFingerprintEnrollment();
            biometrics.addFingerDialog.reject();
            waitForRendering(stage);
            wait(150);

            var users = showPane(shell, "users-groups");
            save(shell, "t279-users-" + scheme);
            users.addUserButton.clicked();
            waitForRendering(stage);
            wait(150);
            save(shell, "t279-users-add-" + scheme);
            users.addUserDialog.reject();
            waitForRendering(stage);
            wait(150);

            var online = showPane(shell, "online-accounts");
            save(shell, "t279-online-accounts-" + scheme);
            online.detailsAccountId = "account-1";
            online.detailsDialog.show();
            waitForRendering(stage);
            wait(150);
            save(shell, "t279-online-accounts-details-" + scheme);
            online.detailsDialog.accept();
            waitForRendering(stage);
            wait(150);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
