// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T270 headless visual check (not part of ctest): every General sub-pane this
// task owns, the searchable pickers, the dialogs, and the host-absent state in
// light and dark. Run through `scripts/capture-t270-general-system.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT270Capture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
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

        function reset() {
            Settings.resetUpdatesFixture();
            Settings.resetGeneralFixture();
            Settings.setGeneralAvailableFixture(true);
            Settings.setUpdatesAbsentFixture(false);
            Settings.setUpdatesProviderFixture(true);
            Settings.setList("locale.languages", []);
            Settings.set("locale.calendar", "gregorian");
            Settings.set("locale.temperature", "celsius");
            Settings.set("locale.dateFormat", "medium");
            Settings.set("locale.numberFormat", "locale");
            Settings.set("backup.destination", "");
            Settings.set("clock.use24Hour", false);
        }

        function captureSubPanes(shell, pane, scheme, suffix) {
            openSubPane(pane, "date-time");
            grabImage(shell).save("docs/captures/t270-general-date-time-" + suffix + "-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "language");
            grabImage(shell).save("docs/captures/t270-general-language-" + suffix + "-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "login");
            grabImage(shell).save("docs/captures/t270-general-login-" + suffix + "-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "sharing");
            grabImage(shell).save("docs/captures/t270-general-sharing-" + suffix + "-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "backup");
            grabImage(shell).save("docs/captures/t270-general-backup-" + suffix + "-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "startup");
            grabImage(shell).save("docs/captures/t270-general-startup-" + suffix + "-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "reset");
            grabImage(shell).save("docs/captures/t270-general-reset-" + suffix + "-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);
        }

        function capture(scheme) {
            reset();
            Settings.set("appearance.colorScheme", scheme);
            var shell = make();
            var pane = showPane(shell, "general");
            grabImage(shell).save("docs/captures/t270-general-list-" + scheme + ".png");

            // Date & Time with the searchable time-zone picker open.
            var dateTime = openSubPane(pane, "date-time");
            grabImage(shell).save("docs/captures/t270-general-date-time-" + scheme + ".png");
            dateTime.timezonePicker.openPicker();
            waitForRendering(stage);
            dateTime.timezonePicker.field.text = "berlin";
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t270-general-timezone-search-" + scheme + ".png");
            dateTime.timezonePicker.closePicker();
            pane.closeSubPane();
            waitForRendering(stage);

            // Language & Region with the add-language picker open.
            var language = openSubPane(pane, "language");
            grabImage(shell).save("docs/captures/t270-general-language-" + scheme + ".png");
            language.addLanguagePicker.openPicker();
            waitForRendering(stage);
            language.addLanguagePicker.field.text = "deutsch";
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t270-general-language-search-" + scheme + ".png");
            language.addLanguagePicker.closePicker();
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "login");
            grabImage(shell).save("docs/captures/t270-general-login-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            // Sharing with a service's Options sheet open.
            var sharing = openSubPane(pane, "sharing");
            grabImage(shell).save("docs/captures/t270-general-sharing-" + scheme + ".png");
            sharing.showOptions(sharing.services[1]);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t270-general-sharing-options-" + scheme + ".png");
            sharing.optionsDialog.accept();
            pane.closeSubPane();
            waitForRendering(stage);

            openSubPane(pane, "backup");
            grabImage(shell).save("docs/captures/t270-general-backup-" + scheme + ".png");
            pane.closeSubPane();
            waitForRendering(stage);

            // Startup Disk with the restart confirmation open.
            var startup = openSubPane(pane, "startup");
            grabImage(shell).save("docs/captures/t270-general-startup-" + scheme + ".png");
            startup.restartButton.clicked();
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t270-general-startup-restart-" + scheme + ".png");
            startup.restartDialog.reject();
            pane.closeSubPane();
            waitForRendering(stage);

            // Transfer or Reset with the typed confirmation open.
            var resetPane = openSubPane(pane, "reset");
            grabImage(shell).save("docs/captures/t270-general-reset-" + scheme + ".png");
            resetPane.resetButton.clicked();
            resetPane.confirmField.text = "RESET";
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t270-general-reset-confirm-" + scheme + ".png");
            resetPane.confirmDialog.reject();
            pane.closeSubPane();
            waitForRendering(stage);

            // Every sub-pane with the host adapter stopped.
            Settings.setGeneralAvailableFixture(false);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t270-general-absence-list-" + scheme + ".png");
            captureSubPanes(shell, pane, scheme, "absence");
            Settings.setGeneralAvailableFixture(true);
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
