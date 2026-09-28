// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// General (About + Software Update) pane tests (T-15.10b). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_UPDATES_FIXTURE` (see CMakeLists.txt), so the
// `Settings` singleton and the host-stack client are deterministic and
// in-process with no bus (and no package manager). The cases cover the About
// dialog's identity rows, the Software Update dialog's check/install/restart
// round-trip, state convergence into the pane row, and the absence note.
Item {
    id: stage
    width: 1000
    height: 1400

    TestCase {
        id: testCase
        name: "SettingsGeneral"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The updates fixture is process-global; reset it before every case so
        // the pane always opens with one security update on offer.
        function init() {
            Settings.resetUpdatesFixture();
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the General pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_disclosure_rows() {
            var shell = make();
            compare(shell.currentPaneId, "general");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            verify(pane.aboutRow.visible, "the About row is visible");
            verify(pane.updateRow.visible, "the Software Update row is visible");
            compare(pane.updateRow.description, "1 Update Available");
            compare(pane.updateSummary, "1 Update Available");
        }

        function test_about_dialog_shows_the_host_identity() {
            var shell = make();
            var pane = paneOf(shell);
            pane.aboutDialog.show();
            compare(pane.aboutDialog.open, true);
            compare(findChild(pane, "generalAboutHeading").text, "Dragonfruit Book");
            var os = findChild(pane, "generalAboutOsValue");
            verify(os, "the OS value text exists");
            compare(os.text, "Dragonfruit Linux 44");
            compare(findChild(pane, "generalAboutMemoryValue").text, "16 GB");
            compare(findChild(pane, "generalAboutChipValue").text, "Example CPU");
            verify(pane.aboutSerialRow.visible, "the serial row shows when present");
            compare(findChild(pane, "generalAboutSerialValue").text, "SERIAL-1");
            pane.aboutDialog.accept();
            compare(pane.aboutDialog.open, false);
        }

        function test_update_dialog_check_install_restart_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            pane.updateDialog.show();
            compare(pane.updateDialog.open, true);
            compare(pane.updateStatusText.text, "1 Update Available");
            compare(pane.updatesRepeater.count, 1);
            compare(pane.checkButton.enabled, true);
            compare(pane.installButton.enabled, true);

            // Check keeps the offered update (the fixture has one).
            pane.checkButton.clicked();
            tryVerify(function() { return pane.updateStatusText.text === "1 Update Available"; });

            // Install clears the list and requires a restart.
            pane.installButton.clicked();
            tryVerify(function() { return pane.rebootRequired === true; });
            compare(pane.updateStatusText.text, "Restart Required");
            compare(pane.restartButton.visible, true);
            compare(pane.installButton.enabled, false);

            // Restart finishes the flow.
            pane.restartButton.clicked();
            tryVerify(function() { return pane.updateStatusText.text === "Up to Date"; });
            compare(pane.rebootRequired, false);
        }

        function test_update_state_converges_into_the_pane_row() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.updateSummary, "1 Update Available");
            // Install through the seam; the pane row converges with the view.
            Settings.installUpdates();
            tryVerify(function() { return pane.updateSummary === "Restart Required"; });
            compare(pane.updateRow.description, "Restart Required");
            Settings.rebootUpdates();
            tryVerify(function() { return pane.updateSummary === "Up to Date"; });
        }

        function test_absence_note_replaces_the_rows_without_a_host() {
            // The absence case itself lives in tst_settings_absence.qml (no
            // fixture, no bus); this asserts the gate is wired so a non-ready
            // view never shows the rows.
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.absenceNote.visible, false);
        }
    }
}