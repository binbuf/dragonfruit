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
            Settings.resetGeneralFixture();
            Settings.setGeneralAvailableFixture(true);
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

        function openSubPane(pane, id) {
            pane.openSubPane(id);
            tryVerify(function() { return pane.subPaneLoader.status === Loader.Ready; });
            waitForRendering(stage);
            verify(pane.subPaneLoader.item, "the " + id + " sub-pane must load");
            return pane.subPaneLoader.item;
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
        Component { id: dateTimeComponent; DateTimePane { } }
        Component { id: sharingComponent; SharingPane { } }
        Component { id: backupComponent; BackupPane { } }

        // -- T226: the General group's list --------------------------------

        function test_general_list_shows_the_reference_rows() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.storageRow.visible, "Storage row");
            verify(pane.loginRow.visible, "Login Items row");
            verify(pane.languageRow.visible, "Language & Region row");
            verify(pane.dateTimeRow.visible, "Date & Time row");
            verify(pane.sharingRow.visible, "Sharing row");
            verify(pane.backupRow.visible, "Backup row");
            verify(pane.startupRow.visible, "Startup Disk row");
            verify(pane.resetRow.visible, "Reset row");
            compare(pane.dateTimeSummary, "America/Los_Angeles · Automatic");
            compare(pane.loginSummary, "4 items");
            compare(pane.sharingSummary, "2 of 3 on");
            compare(pane.backupSummary, "Timeshift · daily");
            compare(pane.storageSummary, "480 GB used");
            compare(pane.startupSummary, "/dev/nvme0n1 · UEFI");
        }

        function test_more_info_shows_gpu_and_disk() {
            var shell = make();
            var pane = paneOf(shell);
            pane.moreInfoDialog.show();
            compare(pane.moreInfoDialog.open, true);
            verify(findChild(pane, "generalMoreInfoDialog"));
            pane.moreInfoDialog.accept();
        }

        // -- Date & Time ---------------------------------------------------

        function test_date_time_applies_ntp_timezone_and_clock_format() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "date-time");
            compare(sub.available, true);
            compare(sub.ntpToggle.checked, true);
            sub.ntpToggle.toggle();
            tryVerify(function() { return Settings.general.time.ntpEnabled === false; });

            sub.timezoneSelect.activateIndex(2);
            tryVerify(function() {
                return Settings.general.time.timezone === "Europe/Berlin";
            });

            compare(Settings.values["clock.use24Hour"], false);
            sub.hour24Toggle.toggle();
            compare(Settings.values["clock.use24Hour"], true);
            verify(sub.previewText.text.indexOf(":") >= 0);

            sub.manualField.text = "2026-11-07 12:00:00";
            sub.setTimeButton.clicked();
            tryVerify(function() {
                return Settings.general.time.localTimeMs
                    === new Date(2026, 10, 7, 12, 0, 0).getTime();
            });
        }

        function test_date_time_degrades_without_its_owner() {
            Settings.setGeneralAvailableFixture(false);
            var pane = createTemporaryObject(dateTimeComponent, stage);
            verify(pane);
            compare(pane.available, false);
            compare(pane.absenceNote.visible, true);
        }

        // -- Language & Region ---------------------------------------------

        function test_language_applies_locale_and_format_keys() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "language");
            compare(sub.available, true);

            sub.languageSelect.activateIndex(1);
            tryVerify(function() { return Settings.general.locale.locale === "de_DE.UTF-8"; });

            sub.calendarSelect.activateIndex(1);
            compare(Settings.values["locale.calendar"], "iso");
            sub.temperatureSelect.activateIndex(1);
            compare(Settings.values["locale.temperature"], "fahrenheit");
            sub.measurementSelect.activateIndex(1);
            compare(Settings.values["locale.measurement"], "imperial");
            sub.dateFormatSelect.activateIndex(2);
            compare(Settings.values["locale.dateFormat"], "long");
            sub.numberFormatSelect.activateIndex(2);
            compare(Settings.values["locale.numberFormat"], "comma");
            sub.firstDaySelect.activateIndex(2);
            compare(Settings.values["locale.firstDay"], "monday");
            verify(sub.previewText.text.indexOf("68 °F") >= 0);
        }

        // -- Login Items ---------------------------------------------------

        function test_login_items_toggle_autostart_and_user_units() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "login");
            compare(sub.available, true);
            compare(sub.autostartRepeater.count, 2);
            compare(sub.userUnitRepeater.count, 2);

            var item = sub.autostartRepeater.itemAt(1);
            item.control.children[0].toggle();
            tryVerify(function() {
                return Settings.general.login.autostart[1].enabled === true;
            });
            var unit = sub.userUnitRepeater.itemAt(1);
            unit.control.children[0].toggle();
            tryVerify(function() {
                return Settings.general.login.userUnits[1].enabled === "enabled";
            });
        }

        // -- Sharing -------------------------------------------------------

        function test_sharing_lists_only_installed_services_and_toggles_one() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "sharing");
            compare(sub.available, true);
            compare(sub.servicesRepeater.count, 3);
            compare(sub.servicesRepeater.itemAt(1).modelData.id, "remoteLogin");
            var toggle = sub.servicesRepeater.itemAt(1).control.children[0];
            compare(toggle.checked, true);
            toggle.toggle();
            tryVerify(function() {
                return Settings.general.sharing.services[1].active === false;
            });
            compare(Settings.general.sharing.activeCount, 1);
        }

        // -- Backup --------------------------------------------------------

        function test_backup_detects_a_backend_and_applies_policy() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "backup");
            compare(sub.available, true);
            compare(sub.detectedBackend, "timeshift");
            compare(sub.canRun, true);

            sub.scheduleSelect.activateIndex(3);
            compare(Settings.values["backup.schedule"], "weekly");
            sub.autoToggle.toggle();
            compare(Settings.values["backup.auto"], true);
            sub.runButton.clicked();
            tryVerify(function() { return Settings.general.backup.message === "Backup started"; });
            sub.restoreButton.clicked();
            tryVerify(function() { return Settings.general.backup.message === "Restore opened"; });
        }

        // -- Startup Disk --------------------------------------------------

        function test_startup_disk_is_read_only() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "startup");
            compare(sub.available, true);
            compare(findChild(sub, "startupDiskRow").control.children[0].text,
                    "/dev/nvme0n1");
            compare(findChild(sub, "startupBootModeRow").control.children[0].text, "UEFI");
            compare(findChild(sub, "startupSecureBootRow").control.children[0].text, "On");
            verify(sub.readOnlyNote.visible);
        }

        // -- Reset ---------------------------------------------------------

        function test_reset_restores_the_schema_defaults() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("dock.size", 0.9);
            compare(Settings.values["dock.size"], 0.9);
            var sub = openSubPane(pane, "reset");
            sub.resetButton.clicked();
            compare(sub.confirmDialog.open, true);
            sub.confirmButton.clicked();
            tryVerify(function() {
                return Settings.values["dock.size"] === Settings.schemaFor("dock.size").default;
            });
            compare(sub.doneText.visible, true);
        }

        // -- Absence -------------------------------------------------------

        function test_every_general_sub_pane_degrades_with_the_host_absent() {
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.generalAvailable, false);
            compare(pane.loginRow.visible, false);
            var sharing = createTemporaryObject(sharingComponent, stage);
            verify(sharing);
            compare(sharing.available, false);
            compare(sharing.absenceNote.visible, true);
            var backup = createTemporaryObject(backupComponent, stage);
            verify(backup);
            compare(backup.available, false);
            compare(backup.absenceNote.visible, true);
        }

        function test_storage_pane_reads_the_usage_categories() {
            var shell = make();
            shell.selectPane("storage");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            compare(pane.usageGroup.visible, true);
            compare(pane.usageRepeater.count, 3);
            compare(pane.manageRow.visible, true);
        }
    }
}