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
            Settings.set("lock.showUserNameAndPhoto", true);
            Settings.set("lock.showPasswordHints", false);
            Settings.set("lock.showMessageWhenLocked", false);
            Settings.set("lock.message", "");
            Settings.set("lock.showPowerButtons", true);
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
            compare(pane.requirePasswordSeconds, 600);
            compare(pane.requirePasswordSelect.currentIndex, 4); // After 10 minutes
            compare(pane.userNameToggle.checked, true);
            compare(pane.passwordHintsToggle.checked, false);
            compare(pane.messageToggle.checked, false);
            compare(pane.powerButtonsToggle.checked, true);
            compare(pane.energyWarning.visible, false);
        }

        function test_timing_rows_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.displayOffSelect.activateIndex(3); // For 10 minutes
            compare(Settings.values["idle.blank"], 600);

            pane.requirePasswordSelect.activateIndex(0); // After 5 seconds
            compare(Settings.values["idle.lock"], 5);

            // `Never` disables each stage with the engine's 0 spelling.
            pane.requirePasswordSelect.activateIndex(6);
            compare(Settings.values["idle.lock"], 0);
        }

        function test_display_toggles_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.userNameToggle.toggle();
            compare(Settings.values["lock.showUserNameAndPhoto"], false);
            pane.passwordHintsToggle.toggle();
            compare(Settings.values["lock.showPasswordHints"], true);
            pane.powerButtonsToggle.toggle();
            compare(Settings.values["lock.showPowerButtons"], false);
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
            pane.messageInput.text = "Back at 3.";
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

            Settings.set("idle.lock", 300);
            compare(pane.requirePasswordSeconds, 300);
            compare(pane.requirePasswordSelect.currentIndex, 3);

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
    }
}