// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Biometrics & Password pane tests (T228). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_BIOMETRICS_FIXTURE`, so the deterministic
// in-process account client serves a reader, PAM, and a password capability
// with no fprintd, AccountsService, or session bus.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsBiometrics"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.resetBiometricsFixture();
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("biometrics");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready);
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_live_fixture() {
            var shell = make();
            compare(shell.currentPaneId, "biometrics");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.fprintdAvailable, true);
            compare(pane.fingers.length, 1);
            compare(pane.pamAvailable, true);
            compare(pane.pam.unlock, true);
            compare(pane.pam.auth, true);
            compare(pane.passwordAvailable, true);
            compare(pane.pamUnlockToggle.checked, true);
            compare(pane.pamUnlockToggle.enabled, false);
            compare(pane.pamAuthToggle.checked, true);
            compare(pane.absenceNote.visible, false);
            compare(pane.pamAbsenceNote.visible, false);
        }

        function test_add_and_delete_a_fingerprint() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addFingerButton.clicked();
            compare(pane.addFingerDialog.open, true);
            pane.fingerSelect.activateIndex(4); // right thumb
            compare(pane.selectedFinger, "right-thumb");
            pane.startEnrollment();
            tryVerify(function() { return pane.fingers.length === 2; });
            compare(pane.fingers[1].id, "right-thumb");

            // Delete the newly enrolled finger through the confirmation.
            var row = pane.fingerRepeater.itemAt(1);
            compare(row !== null, true);
            var button = findChild(row, "biometricsDeleteFingerButton");
            verify(button);
            button.clicked();
            compare(pane.deleteFingerDialog.open, true);
            var confirm = findChild(pane.deleteFingerDialog, "biometricsDeleteFingerConfirm");
            verify(confirm);
            confirm.clicked();
            tryVerify(function() { return pane.fingers.length === 1; });
            compare(pane.fingers[0].id, "right-index-finger");
        }

        function test_password_change_validates_then_writes() {
            var shell = make();
            var pane = paneOf(shell);
            pane.passwordButton.clicked();
            compare(pane.changePasswordDialog.open, true);
            pane.newPasswordField.text = "one";
            pane.confirmPasswordField.text = "two";
            pane.submitPassword();
            compare(pane.passwordError.length > 0, true);
            compare(pane.changePasswordDialog.open, true);

            pane.newPasswordField.text = "correct horse";
            pane.confirmPasswordField.text = "correct horse";
            pane.submitPassword();
            compare(pane.passwordError, "");
            compare(pane.changePasswordDialog.open, false);
        }

        function test_pam_rows_are_read_only() {
            var shell = make();
            var pane = paneOf(shell);
            // The toggles reflect PAM; they are not writable desktop settings.
            pane.pamUnlockToggle.checked = false;
            compare(pane.pamUnlockToggle.enabled, false);
            compare(pane.pam.unlock, true);
        }
    }
}
