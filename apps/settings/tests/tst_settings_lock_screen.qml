// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Lock Screen-pane tests (T-15.8b). Headless with `DF_SETTINGS_FIXTURE` (see
// CMakeLists.txt), so the `Settings` singleton serves deterministic in-process
// settings with no bus. The cases cover the two session-idle timing rows and
// the four `lock.*` display rows round-tripping live, the `Set...` message
// editor, the energy warning gate, an external change converging into the
// controls, and the absence note.
Item {
    id: stage
    width: 900
    height: 1400

    TestCase {
        id: testCase
        name: "SettingsLockScreen"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; reset the keys before every
        // case so the pane always opens on a known state.
        function init() {
            Settings.set("idle.blank", 300);
            Settings.set("idle.lock", 600);
            Settings.set("idle.dim", 150);
            Settings.set("idle.suspend", 0);
            Settings.set("lock.requirePasswordAfter", 300);
            Settings.set("lock.showUserNameAndPhoto", true);
            Settings.set("lock.showPasswordHints", false);
            Settings.set("lock.showMessageWhenLocked", false);
            Settings.set("lock.message", "");
            Settings.set("lock.showPowerButtons", true);
            Settings.set("lock.passwordHint", "");
            Settings.set("login.windowStyle", "list");
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("lock-screen");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Lock Screen pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_schema_defaults() {
            var shell = make();
            compare(shell.currentPaneId, "lock-screen");
            var pane = paneOf(shell);
            compare(pane.displayOffSeconds, 300);
            compare(pane.displayOffSelect.currentIndex, 2); // For 5 minutes
            compare(pane.requireAfter, 300);
            compare(pane.requirePasswordSelect.currentIndex, 4); // After 5 minutes
            compare(pane.requirePasswordSeconds, 600);
            compare(pane.userNameToggle.checked, true);
            compare(pane.passwordHintsToggle.checked, false);
            compare(pane.messageToggle.checked, false);
            compare(pane.powerButtonsToggle.checked, true);
            compare(pane.energyWarning.visible, false);
        }

        // T281: dim and suspend are separate idle stages with their own keys;
        // neither changes the derived lock stage.
        function test_dim_and_suspend_stages_round_trip_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.dimSeconds, 150);
            compare(pane.dimSelect.currentIndex, 1); // After 2.5 minutes
            compare(pane.suspendSeconds, 0);
            compare(pane.suspendSelect.currentIndex, 4); // Never
            var lockBefore = Settings.values["idle.lock"];

            pane.dimSelect.activateIndex(2); // After 5 minutes
            compare(Settings.values["idle.dim"], 300);
            pane.suspendSelect.activateIndex(0); // After 5 minutes
            compare(Settings.values["idle.suspend"], 300);
            compare(Settings.values["idle.lock"], lockBefore,
                    "dim and suspend never rewrite the lock stage");

            Settings.set("idle.dim", 600);
            Settings.set("idle.suspend", 0);
            tryVerify(function() { return pane.dimSelect.currentIndex === 3; });
            tryVerify(function() { return pane.suspendSelect.currentIndex === 4; });
        }

        function test_timing_rows_round_trip() {            var shell = make();
            var pane = paneOf(shell);

            pane.displayOffSelect.activateIndex(3); // For 10 minutes
            compare(Settings.values["idle.blank"], 600);
            // Require-after is derived from the display-off stage.
            compare(Settings.values["idle.lock"], 900);

            pane.requirePasswordSelect.activateIndex(1); // After 5 seconds
            compare(Settings.values["lock.requirePasswordAfter"], 5);
            compare(Settings.values["idle.lock"], 605);

            // `Never` disables the lock stage with the engine's 0 spelling.
            pane.requirePasswordSelect.activateIndex(6);
            compare(Settings.values["lock.requirePasswordAfter"], -1);
            compare(Settings.values["idle.lock"], 0);

            // The dim and display-off `Never` options are the same 0 spelling
            // and must reach the same disabled stage (the live daemon used to
            // reject the integer write, so the menu snapped back).
            pane.dimSelect.activateIndex(5); // Never
            compare(Settings.values["idle.dim"], 0);
            pane.displayOffSelect.activateIndex(8); // Never
            compare(Settings.values["idle.blank"], 0);
            compare(Settings.values["idle.lock"], 0);
        }

        function test_require_password_means_after_display_off() {
            var shell = make();
            var pane = paneOf(shell);
            // The default display-off stage is 5 minutes and the default
            // after-delay is 5 minutes, so the lock is at 10 minutes of
            // inactivity, not at 10 minutes from the last keypress.
            verify(pane.lockSummaryText.indexOf("after the display turns off") >= 0);
            pane.requirePasswordSelect.activateIndex(0); // Immediately
            compare(Settings.values["idle.lock"], 300);
            pane.requirePasswordSelect.activateIndex(2); // After 30 seconds
            compare(Settings.values["idle.lock"], 330);
        }

        function test_custom_idle_values_show_their_real_duration() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("idle.blank", 420);
            compare(pane.displayOffSelect.currentLabel, "For 7 minutes");
            Settings.set("lock.requirePasswordAfter", 45);
            compare(pane.requirePasswordSelect.currentLabel, "After 45 seconds");
            verify(pane.displayOffSelect.currentLabel !== "Never");
        }

        function test_power_stage_note_documents_one_stage() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.powerStageNote.visible);
            verify(pane.powerStageNote.text.indexOf("does not vary") >= 0);
        }

        function test_display_toggles_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.userNameToggle.toggle();
            compare(Settings.values["lock.showUserNameAndPhoto"], false);
            compare(pane.previewName.visible, false,
                    "the preview follows the user-name choice");
            pane.passwordHintsToggle.toggle();
            compare(Settings.values["lock.showPasswordHints"], true);
            pane.powerButtonsToggle.toggle();
            compare(Settings.values["lock.showPowerButtons"], false);
            compare(pane.previewPowerButtons.visible, false,
                    "the preview follows the power-button choice");
        }

        function test_preview_shows_the_message_and_hint() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.previewMessage.visible, false);
            compare(pane.previewHint.visible, false);

            pane.messageToggle.toggle();
            Settings.set("lock.message", "Back at 3.");
            tryVerify(function() { return pane.previewMessage.visible; });
            compare(pane.previewMessage.text, "Back at 3.");

            pane.passwordHintsToggle.toggle();
            Settings.set("lock.passwordHint", "The one with the cat");
            tryVerify(function() { return pane.previewHint.visible; });
            compare(pane.previewHint.text, "The one with the cat");
        }

        function test_cancel_resets_the_editor_drafts() {
            var shell = make();
            var pane = paneOf(shell);
            pane.messageToggle.toggle();
            pane.messageDialog.show();
            waitForRendering(stage);
            pane.messageDraft = "Unsaved";
            pane.messageDialog.reject();
            compare(Settings.values["lock.message"], "",
                    "Cancel never writes the draft");
            pane.messageDialog.show();
            waitForRendering(stage);
            compare(pane.messageInput.text, "",
                    "reopening seeds the stored value");
            pane.messageDialog.reject();
        }

        function test_energy_warning_tracks_a_long_display_off_delay() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.energyWarning.visible, false);

            pane.displayOffSelect.activateIndex(5); // For 20 minutes
            compare(Settings.values["idle.blank"], 1200);
            compare(pane.energyWarning.visible, true);

            pane.displayOffSelect.activateIndex(2); // For 5 minutes
            compare(pane.energyWarning.visible, false);
        }

        function test_set_message_button_gates_on_the_toggle() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.setMessageButton.enabled, false);

            pane.messageToggle.toggle();
            compare(Settings.values["lock.showMessageWhenLocked"], true);
            compare(pane.setMessageButton.enabled, true);
        }

        function test_message_editor_writes_the_lock_message() {
            var shell = make();
            var pane = paneOf(shell);

            pane.messageToggle.toggle(); // turn it on so Set... is enabled
            pane.messageDialog.show();
            waitForRendering(stage);
            compare(pane.messageDialog.open, true);
            pane.messageDraft = "Back at 3.";
            pane.messageDialog.accept();
            compare(Settings.values["lock.message"], "Back at 3.");

            // Reopening seeds the editor with the stored message.
            pane.messageDialog.show();
            waitForRendering(stage);
            compare(pane.messageInput.text, "Back at 3.");
            pane.messageDialog.reject();
            compare(Settings.values["lock.message"], "Back at 3.");
        }

        function test_external_change_converges_into_the_pane() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.set("lock.requirePasswordAfter", 300);
            compare(pane.requireAfter, 300);
            compare(pane.requirePasswordSelect.currentIndex, 4);

            Settings.set("idle.blank", 0);
            compare(pane.displayOffSelect.currentIndex, 8); // Never
            compare(pane.energyWarning.visible, true);

            Settings.set("lock.showPasswordHints", true);
            compare(pane.passwordHintsToggle.checked, true);
            Settings.set("lock.showPowerButtons", false);
            compare(pane.powerButtonsToggle.checked, false);
        }

        function test_absence_note_hides_when_the_daemon_is_present() {
            // In fixture mode the settings client is available, so the pane
            // shows its controls, not the absence note. The no-daemon path is
            // covered by tst_settings_absence.qml.
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.available, true);
            compare(pane.absenceNote.visible, false);
        }

        // -- T229a: hints, login window, accessibility ------------------------

        function test_password_hint_editor_round_trips_and_gates_on_the_toggle() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.setHintButton.enabled, false);
            pane.passwordHintsToggle.toggle();
            compare(Settings.values["lock.showPasswordHints"], true);
            compare(pane.setHintButton.enabled, true);

            pane.hintDialog.show();
            waitForRendering(stage);
            compare(pane.hintDialog.open, true);
            pane.hintDraft = "The one with the cat";
            pane.hintDialog.accept();
            compare(Settings.values["lock.passwordHint"], "The one with the cat");

            pane.hintDialog.show();
            waitForRendering(stage);
            compare(pane.hintInput.text, "The one with the cat");
            pane.hintDialog.reject();
        }

        function test_login_window_row_is_disabled_with_a_reason() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.loginWindowSelect.enabled, false);
            compare(pane.loginWindowSelect.currentIndex, 0);
            verify(pane.loginWindowSelect.Accessible.name.length > 0);
        }

        function test_accessibility_button_opens_the_accessibility_pane() {
            var shell = make();
            var pane = paneOf(shell);
            pane.accessibilityButton.clicked();
            tryCompare(shell, "currentPaneId", "accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
        }
    }
}