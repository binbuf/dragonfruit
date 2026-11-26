// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Biometrics & Password pane tests (T228; T279 completion). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_BIOMETRICS_FIXTURE`, so the deterministic
// in-process account client serves a reader, PAM, and a password capability
// with no fprintd, AccountsService, or session bus. The cases cover the staged
// enrollment wizard (fprintd's own stage text, never a percentage), a denied
// write at the row, an unsupported reader, and a password dialog that waits for
// the host's answer.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsBiometrics"
        when: windowShown

        // T285 removed the top-level Biometrics pane; T299 owns folding its
        // remaining settings into Users & Groups. The component stays, so
        // these cases exercise it directly.
        Component { id: biometricsComponent; BiometricsPane { } }

        function init() {
            Settings.resetBiometricsFixture();
            Settings.setBiometricsReaderFixture(true);
            Settings.setBiometricsDeniedFixture(false);
            Settings.setPasswordOutcomeFixture("applied", "");
        }

        function make(width, height) {
            var pane = createTemporaryObject(biometricsComponent, stage,
                                             { width: width || 480,
                                               height: height || stage.height });
            waitForRendering(stage);
            verify(pane);
            return pane;
        }

        function paneOf(pane) {
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_live_fixture() {
            var pane = make();
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

        function test_enrollment_wizard_reports_fprintd_stages() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addFingerButton.clicked();
            compare(pane.addFingerDialog.open, true);
            compare(pane.addFingerDialog.started, false);
            pane.fingerSelect.activateIndex(4); // right thumb
            compare(pane.selectedFinger, "right-thumb");

            pane.startEnrollment();
            compare(pane.addFingerDialog.started, true);
            compare(pane.enrolling !== null, true);
            // fprintd's own stage id and its human line, never a percentage.
            compare(pane.enrolling.stage, "enroll-stage-passed");
            compare(pane.enrollStageText.text,
                    "Fingerprint captured; place the same finger again");
            verify(pane.enrollRawStageText.text.indexOf("enroll-stage-passed") >= 0);
            verify(pane.enrollStageText.text.indexOf("%") < 0);

            Settings.advanceFingerprintEnrollmentFixture();
            tryVerify(function() { return pane.fingers.length === 2; });
            compare(pane.fingers[1].id, "right-thumb");
            tryVerify(function() { return pane.addFingerDialog.open === false; });

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

        function test_a_denied_enrollment_shows_at_the_row_and_in_the_wizard() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setBiometricsDeniedFixture(true);
            pane.addFingerButton.clicked();
            pane.startEnrollment();
            tryVerify(function() { return pane.writeState === "denied"; });
            verify(pane.fingerprintRow.errorText.length > 0);
            verify(pane.enrollDialogError.text.length > 0);
            // The view is unchanged: no finger was invented.
            compare(pane.fingers.length, 1);
        }

        function test_an_unsupported_reader_degrades_only_the_fingerprint_half() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setBiometricsReaderFixture(false);
            tryVerify(function() { return pane.device === null; });
            // fprintd is running; the reader is the absent half.
            compare(pane.fprintdAvailable, true);
            compare(pane.addFingerButton.enabled, false);
            compare(pane.fingerprintRow.description,
                    "No fingerprint reader is connected.");
            // The password half stays live.
            compare(pane.passwordAvailable, true);
            compare(pane.passwordButton.enabled, true);
        }

        function test_password_change_validates_then_waits_for_the_host() {
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
            tryVerify(function() { return pane.changePasswordDialog.open === false; });
            compare(pane.passwordError, "");
        }

        function test_a_denied_password_keeps_the_dialog_open() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setPasswordOutcomeFixture("denied", "Authentication was denied.");
            pane.passwordButton.clicked();
            pane.newPasswordField.text = "correct horse";
            pane.confirmPasswordField.text = "correct horse";
            pane.submitPassword();
            tryVerify(function() { return pane.writeState === "denied"; });
            // The host refused; the dialog stays open with its own message.
            compare(pane.changePasswordDialog.open, true);
            verify(pane.passwordError.length > 0);
            compare(pane.passwordErrorNote.visible, true);
        }

        function test_pam_rows_are_read_only() {
            var shell = make();
            var pane = paneOf(shell);
            // The toggles reflect PAM; they are not writable desktop settings.
            pane.pamUnlockToggle.checked = false;
            compare(pane.pamUnlockToggle.enabled, false);
            compare(pane.pam.unlock, true);
        }

        // T279: the measured 740×820 window keeps both halves reachable.
        function test_measured_window_740x820_keeps_both_halves() {
            var pane = make(740, 820);
            verify(pane.width > 0 && pane.width <= 740);
            verify(pane.passwordRow.width > 0);
            verify(pane.fingerprintRow.width > 0);
            verify(pane.passwordRow.width <= 740);
        }
    }
}
