// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T226 headless visual check (not part of ctest): the General list, the About
// and Software Update dialogs, the Date & Time and Language & Region
// sub-panes, the Sharing service off state, Backup, Login Items, Startup Disk,
// Reset confirmation, and the host-absent state. Run through
// `scripts/capture-t226-general-panes.sh`.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsGeneralCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }
        Component { id: panelComponent; AboutPanel { } }

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

        function openSubPane(pane, id) {
            pane.openSubPane(id);
            tryVerify(function() { return pane.subPaneLoader.status === Loader.Ready; });
            waitForRendering(stage);
            verify(pane.subPaneLoader.item);
            return pane.subPaneLoader.item;
        }

        function init() {
            Settings.resetUpdatesFixture();
            Settings.resetGeneralFixture();
            Settings.setGeneralAvailableFixture(true);
        }

        function test_capture_general_group() {
            var shell = make();
            var pane = showPane(shell, "general");
            compare(pane.generalAvailable, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-list.png");

            // About + More Info.
            var panel = createTemporaryObject(panelComponent, stage);
            waitForRendering(stage);
            grabImage(panel).save("docs/captures/t226-general-about.png");
            panel.destroy();
            pane.openSubPane("about");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-more-info.png");
            pane.closeSubPane();
            wait(250);

            // Software Update: check, then install and the restart state.
            pane.updateDialog.show();
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-updates.png");
            pane.checkButton.clicked();
            tryVerify(function() { return pane.updateStatusText.text === "1 Update Available"; });
            pane.installButton.clicked();
            tryVerify(function() { return pane.rebootRequired === true; });
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-updates-restart.png");
            pane.restartButton.clicked();
            tryVerify(function() { return pane.updateStatusText.text === "Up to Date"; });
            pane.updateDialog.accept();
            wait(250);

            // Date & Time: the default locale format, then 24-hour.
            var dateTime = openSubPane(pane, "date-time");
            compare(dateTime.available, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-date-time.png");
            dateTime.hour24Toggle.toggle();
            compare(Settings.values["clock.use24Hour"], true);
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-date-time-24h.png");

            // Language & Region.
            pane.closeSubPane();
            wait(150);
            var language = openSubPane(pane, "language");
            compare(language.available, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-language.png");

            // Sharing: stop remote login and compare the state.
            pane.closeSubPane();
            wait(150);
            var sharing = openSubPane(pane, "sharing");
            compare(sharing.available, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-sharing-on.png");
            var remote = sharing.servicesRepeater.itemAt(1);
            verify(remote);
            remote.control.children[0].toggle();
            tryVerify(function() { return Settings.general.sharing.services[1].active === false; });
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-sharing-off.png");

            // Backup.
            pane.closeSubPane();
            wait(150);
            var backup = openSubPane(pane, "backup");
            compare(backup.available, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-backup.png");

            // Login Items.
            pane.closeSubPane();
            wait(150);
            var login = openSubPane(pane, "login");
            compare(login.available, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-login-items.png");

            // Startup Disk.
            pane.closeSubPane();
            wait(150);
            var startup = openSubPane(pane, "startup");
            compare(startup.available, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-startup-disk.png");

            // Reset confirmation.
            pane.closeSubPane();
            wait(150);
            var reset = openSubPane(pane, "reset");
            reset.resetButton.clicked();
            compare(reset.confirmDialog.open, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-reset.png");
            reset.confirmDialog.reject();
            wait(250);

            // The host-absent state: the General list keeps the About/Update
            // rows and the sub-pane rows disappear; each sub-pane shows its
            // own absence note.
            Settings.setGeneralAvailableFixture(false);
            pane.closeSubPane();
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t226-general-absence.png");
        }
    }
}
