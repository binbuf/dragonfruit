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
        Component { id: panelComponent; AboutPanel { } }
        Component { id: pageComponent; AboutPage { } }
        Component { id: windowComponent; SettingsWindow { startAboutPanel: true } }
        Component { id: mainWindowComponent; SettingsWindow { } }

        // The updates fixture is process-global; reset it before every case so
        // the pane always opens with one security update on offer.
        function init() {
            Settings.resetUpdatesFixture();
            Settings.resetGeneralFixture();
            Settings.setGeneralAvailableFixture(true);
            Settings.setUpdatesAbsentFixture(false);
            Settings.setUpdatesProviderFixture(true);
            Settings.set("defaults.passwordManager", "");
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
            compare(pane.identityReady, true);
            verify(pane.aboutRow.visible, "the About row is visible");
            verify(pane.updateRow.visible, "the Software Update row is visible");
            compare(pane.updateSummary, "1 Update Available");
            compare(pane.updateRow.secondaryText, "");
            verify(pane.passwordsRow.visible, "the Passwords & Keys row is visible");
            compare(pane.passwordsSummary, "2 providers");
        }

        function test_general_big_list_order_heights_and_sub_lines() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.generalIntro.text,
                    "Manage your overall setup and preferences for this computer, "
                    + "such as updates, language, sharing, and startup.");
            compare(pane.bigList.rows.children.length, 11,
                    "device management is omitted from the rendered list");

            var ids = ["aboutRow", "updateRow", "storageRow", "passwordsRow",
                       "dateTimeRow", "languageRow", "loginRow", "sharingRow",
                       "startupRow", "backupRow", "resetRow"];
            var labels = ["About", "Software Update", "Storage", "Passwords & Keys",
                          "Date & Time", "Language & Region",
                          "Login Items & Extensions", "Sharing", "Startup Disk",
                          "Backups", "Transfer or Reset"];
            for (var i = 0; i < ids.length; ++i) {
                var row = pane[ids[i]];
                verify(row.visible, ids[i] + " must be visible");
                compare(row.label, labels[i]);
                verify(row.height === 52, ids[i] + " height=" + row.height);
                verify(row.category !== null, ids[i] + " must carry a category tile");
                verify(String(row.category.source).length > 0);
            }
            compare(pane.aboutRow.secondaryText, "");
            compare(pane.passwordsRow.secondaryText, "Managed by the system keyring");
            compare(pane.backupRow.secondaryText,
                    "Restic / Pika / Timeshift destinations");
            compare(pane.resetRow.secondaryText, "");
        }

        function test_device_management_is_omitted_with_a_map_reason() {
            var row = SettingsMap.rowFor("general", "device.management");
            verify(row !== null, "the omitted device management row is mapped");
            compare(row.state, "omitted");
            compare(row.absence, "omitted");
            compare(row.label, "Managed Devices");
            verify(row.reason.length > 0, "the omission carries a reason");
            verify(row.section.length > 0);
            compare(SettingsMap.subPaneForRow("general", "device.management"), "");
            var shell = make();
            var pane = paneOf(shell);
            for (var i = 0; i < pane.bigList.rows.children.length; ++i) {
                verify(pane.bigList.rows.children[i].label !== "Managed Devices",
                       "the omitted managed-devices row must not be rendered");
            }
        }

        function test_about_page_shows_the_host_identity() {
            var shell = make();
            shell.openAboutPage();
            var pane = paneOf(shell);
            tryVerify(function() { return pane.subPaneLoader.item !== null; });
            compare(pane.subPaneId, "about");
            var page = pane.subPaneLoader.item;
            compare(page.width, pane.width);
            compare(findChild(page, "aboutPageHero").width, 128);
            compare(findChild(page, "aboutPageHero").height, 128);
            compare(findChild(page, "aboutPageName").text, "Dragonfruit Book");
            compare(findChild(page, "aboutPageHostName").text, "Dragonfruit Book");
            compare(findChild(page, "aboutPageProcessor").text, "Example CPU");
            compare(findChild(page, "aboutPageMemory").text, "16 GB");
            compare(findChild(page, "aboutPageDiskEncryption").text, "Not available");
            compare(findChild(page, "aboutPageSecurityUpdates").text,
                    "Included in the available updates");
            compare(findChild(page, "aboutPageOs").text, "Dragonfruit Linux 44");
            compare(findChild(page, "aboutPageKernel").text, "6.12.0 · x86_64");
            compare(findChild(page, "aboutPageSession").text, "Wayland");
            verify(findChild(page, "aboutPageDisplay").text.length > 0);
            compare(findChild(page, "aboutPageVolume").text, "Volume: /");
            verify(findChild(page, "aboutPageStorageRow").description.indexOf("used") >= 0);
            compare(findChild(page, "aboutPageCopyright").text.indexOf("2026") >= 0, true);
            pane.closeSubPane();
            compare(pane.subPaneId, "");
        }

        function test_about_system_report_is_a_real_sheet() {
            var shell = make();
            shell.openAboutPage();
            var pane = paneOf(shell);
            tryVerify(function() { return pane.subPaneLoader.item !== null; });
            var page = pane.subPaneLoader.item;
            compare(page.reportOpen, false);
            findChild(page, "aboutPageSystemReport").clicked();
            compare(page.reportOpen, true);
            compare(page.reportDialog.open, true);
            compare(findChild(page, "aboutReportHostName").control.children[0].text, "dragon");
            compare(findChild(page, "aboutReportProcessor").control.children[0].text, "Example CPU");
            compare(findChild(page, "aboutReportGraphics").control.children[0].text, "Example GPU");
            compare(findChild(page, "aboutReportKernel").control.children[0].text, "6.12.0");
            compare(findChild(page, "aboutReportArchitecture").control.children[0].text, "x86_64");
            verify(findChild(page, "aboutReportDisplay").control.children[0].text.length > 0);
            verify(findChild(page, "aboutReportStorage").control.children[0].text.length > 0);
            page.reportDialog.accept();
            compare(page.reportOpen, false);
        }

        function test_about_never_leaves_the_window_at_the_design_default() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 900, height: 700 });
            waitForRendering(stage);
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            shell.openAboutPage();
            tryVerify(function() { return shell.aboutOpen; });
            compare(shell.width, 900);
            compare(shell.height, 700);
            shell.paneBody.item.closeSubPane();
            tryVerify(function() { return !shell.aboutOpen; });
            compare(shell.width, 900);
            compare(shell.height, 700);
        }

        function test_compact_panel_geometry_rows_and_absence() {
            var panel = createTemporaryObject(panelComponent, stage);
            verify(panel);
            compare(panel.width, 280);
            compare(panel.height, 503);
            compare(panel.chromeHeight, 32);
            compare(findChild(panel, "aboutPanelHero").width, 194);
            compare(findChild(panel, "aboutPanelHero").height, 194);
            compare(findChild(panel, "aboutPanelHero").y, 29);
            compare(findChild(panel, "aboutPanelName").y, 229);
            compare(findChild(panel, "aboutPanelName").height, 28);
            compare(findChild(panel, "aboutPanelLabel0").width, 93);
            compare(findChild(panel, "aboutPanelLabel0").horizontalAlignment, Text.AlignRight);
            compare(findChild(panel, "aboutPanelLabel0").mapToItem(panel, 0, 0).y, 291);
            compare(findChild(panel, "aboutPanelLabel1").mapToItem(panel, 0, 0).y, 307);
            compare(findChild(panel, "aboutPanelValue0").mapToItem(panel, 0, 0).x, 126);
            compare(findChild(panel, "aboutPanelButtonBacking").y, 391);
            compare(findChild(panel, "aboutPanelButtonBacking").width, 94);
            compare(findChild(panel, "aboutPanelButtonBacking").height, 26);
            compare(findChild(panel, "aboutPanelRegulatory").y, 429);
            compare(findChild(panel, "aboutPanelFooter").y, 466);
            compare(findChild(panel, "aboutPanelName").text, "Dragonfruit Book");
            compare(findChild(panel, "aboutPanelValue0").text, "Example CPU");
            compare(findChild(panel, "aboutPanelValue1").text, "16 GB");
            compare(findChild(panel, "aboutPanelValue2").text, "fedora");
            compare(findChild(panel, "aboutPanelValue3").text, "SERIAL-1");
            panel.identity = { deviceName: "A very long localized Dragonfruit machine name that exceeds the panel", hasSerial: false };
            panel.productName = "Système Dragonfruit très long et localisé";
            compare(findChild(panel, "aboutPanelName").text, panel.identity.deviceName);
            compare(findChild(panel, "aboutPanelName").elide, Text.ElideRight);
            compare(findChild(panel, "aboutPanelValue0").text, "Not available");
            compare(findChild(panel, "aboutPanelValue3").text, "Not available");
            var page = createTemporaryObject(pageComponent, stage);
            page.identity = { deviceName: "Dragonfruit", hasSerial: false };
            compare(findChild(page, "aboutPageName").maximumLineCount, 2);
            compare(findChild(page, "aboutPageName").elide, Text.ElideRight);
            compare(findChild(page, "aboutPageProduct").elide, Text.ElideRight);
            compare(findChild(page, "aboutPageDiskEncryption").text, "Not available");
            page.openReport();
            compare(findChild(page, "aboutReportModel").control.children[0].text,
                    "Not available");
        }

        function test_full_page_window_geometry() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            shell.openAboutPage();
            tryVerify(function() { return shell.paneBody.item
                                            && shell.paneBody.item.subPaneId === "about"; });
            compare(shell.appWindow.chromeHeight, 52);
            compare(shell.detailScroll.height, 768);
            compare(shell.aboutOpen, true);
        }

        function test_general_and_about_fit_the_minimum_window() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 720, height: 480 });
            waitForRendering(stage);
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            var pane = paneOf(shell);
            compare(pane.aboutRow.visible, true);
            compare(pane.passwordsRow.visible, true);
            compare(pane.resetRow.visible, true);
            verify(shell.detailScroll.height > 0,
                   "the General list has a visible detail viewport");
            shell.openAboutPage();
            tryVerify(function() { return shell.aboutOpen; });
            var page = pane.subPaneLoader.item;
            verify(page !== null);
            compare(page.width, pane.width);
            verify(page.implicitHeight > 0);
        }

        function test_utility_window_more_info_transition() {
            var window = createTemporaryObject(windowComponent, null);
            verify(window);
            compare(window.visible, false);
            compare(window.aboutWindow.visible, true);
            compare(window.aboutWindow.width, 280);
            compare(window.aboutWindow.height, 503);
            window.aboutPanel.moreInfoRequested();
            tryVerify(function() { return window.visible; });
            compare(window.aboutWindow.visible, false);
            compare(window.width, 740);
            compare(window.height, 820);
            tryVerify(function() { return window.settingsShell.aboutOpen; });
        }

        // -- T264: every published menu action dispatches --------------------

        function test_menu_actions_dispatch_to_the_window() {
            var win = createTemporaryObject(mainWindowComponent, null);
            verify(win !== null);
            tryVerify(function() {
                return win.settingsShell.paneBody.status === Loader.Ready;
            });

            // About opens the utility panel over the main window.
            compare(SettingsMenu.activate("about"), true);
            tryVerify(function() { return win.aboutWindow.visible; });
            win.aboutWindow.close();
            tryVerify(function() { return !win.aboutWindow.visible; });

            // Settings… raises the main window.
            compare(SettingsMenu.activate("settings"), true);

            // Edit verbs apply to the focused editable control.
            win.settingsShell.focusSearch();
            waitForRendering(stage);
            win.settingsShell.searchField.input.text = "abc";
            win.settingsShell.searchField.input.select(0, 0);
            compare(SettingsMenu.activate("edit.select-all"), true);
            tryVerify(function() {
                return win.settingsShell.searchField.input.selectedText === "abc";
            });
            compare(SettingsMenu.activate("edit.copy"), true);
            compare(SettingsMenu.activate("edit.undo"), true);

            // The generated View pane submenu switches panes.
            compare(SettingsMenu.activate("pane.wallpaper"), true);
            tryVerify(function() {
                return win.settingsShell.currentPaneId === "wallpaper";
            });
            compare(SettingsMenu.activate("back"), true);
            tryVerify(function() {
                return win.settingsShell.currentPaneId === "general";
            });
            compare(SettingsMenu.activate("forward"), true);
            tryVerify(function() {
                return win.settingsShell.currentPaneId === "wallpaper";
            });

            // Window and Help verbs.
            compare(SettingsMenu.activate("bring-all-to-front"), true);
            compare(SettingsMenu.activate("help"), true);
            verify(win.settingsShell.lastHelpUrl.indexOf("#pane-wallpaper") >= 0,
                   "Help must open the current pane anchor");
            compare(SettingsMenu.activate("help.debug"), true);
            verify(win.settingsShell.debugView.open);
            compare(SettingsMenu.activate("help.map"), true);
            win.close();
        }

        function test_material_accessibility_and_dark() {
            var shell = make();
            var panel = createTemporaryObject(panelComponent, stage);
            var material = findChild(panel, "aboutUtilityMaterial");
            verify(material);
            Settings.set("accessibility.reduceTransparency", true);
            Settings.set("accessibility.increaseContrast", true);
            compare(material.opaque, true);
            compare(material.contrast, true);
            Settings.set("appearance.colorScheme", "dark");
            tryVerify(function() { return Theme.dark; });
            verify(material.topColor.toString() !== material.bottomColor.toString());
            compare(findChild(panel, "aboutPanelButtonBacking").color.toString(),
                    Theme.color.controlFill.toString());
            Settings.set("appearance.colorScheme", "light");
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
        }

        function test_update_subpane_check_install_restart_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "software-update");
            compare(sub.statusText.text, "1 Update Available");
            compare(sub.updatesRepeater.count, 1);
            compare(sub.checkButton.enabled, true);
            compare(sub.installButton.enabled, true);

            // Check keeps the offered update (the fixture has one).
            sub.checkButton.clicked();
            tryVerify(function() { return sub.statusText.text === "1 Update Available"; });

            // Install clears the list and requires a restart.
            sub.installButton.clicked();
            tryVerify(function() { return pane.rebootRequired === true; });
            compare(sub.statusText.text, "Restart Required");
            compare(sub.restartButton.visible, true);
            compare(sub.installButton.enabled, false);

            // Restart finishes the flow.
            sub.restartButton.clicked();
            tryVerify(function() { return sub.statusText.text === "Up to Date"; });
            compare(pane.rebootRequired, false);
        }

        function test_update_subpane_reports_a_missing_provider_honestly() {
            Settings.setUpdatesProviderFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.updateSummary, "No update provider");
            // The About row stays live because the host identity is present.
            compare(pane.aboutSummary, "Dragonfruit Book");
            compare(pane.aboutRow.visible, true);
            var sub = openSubPane(pane, "software-update");
            compare(sub.providerAvailable, false);
            compare(sub.statusText.text, "No update provider");
            compare(sub.noProviderNote.visible, true);
            compare(sub.checkButton.enabled, false);
            compare(sub.installButton.enabled, false);
            Settings.setUpdatesProviderFixture(true);
        }

        function test_update_state_converges_into_the_pane_row() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.updateSummary, "1 Update Available");
            // Install through the seam; the pane row converges with the view.
            Settings.installUpdates();
            tryVerify(function() { return pane.updateSummary === "Restart Required"; });
            compare(pane.updateRow.secondaryText, "");
            Settings.rebootUpdates();
            tryVerify(function() { return pane.updateSummary === "Up to Date"; });
        }

        function test_absence_keeps_the_rows_visible_with_per_row_summaries() {
            Settings.setUpdatesAbsentFixture(true);
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.identityReady, false);
            compare(pane.aboutRow.visible, true);
            compare(pane.updateRow.visible, true);
            compare(pane.storageRow.visible, true);
            compare(pane.passwordsRow.visible, true);
            compare(pane.aboutSummary, "Unavailable");
            compare(pane.updateSummary, "Unavailable");
            compare(pane.storageSummary, "Unavailable");
            compare(pane.passwordsSummary, "Unavailable");
            compare(pane.listColumn.visible, true);
            Settings.setUpdatesAbsentFixture(false);
        }

        function test_partial_absence_updates_only_keeps_every_row() {
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.identityReady, true);
            compare(pane.aboutSummary, "Dragonfruit Book");
            compare(pane.updateSummary, "1 Update Available");
            compare(pane.storageSummary, "Unavailable");
            compare(pane.passwordsSummary, "Unavailable");
            verify(pane.storageRow.visible, "Storage row stays visible");
            verify(pane.passwordsRow.visible, "Autofill row stays visible");
        }

        function test_partial_absence_general_only_keeps_every_row() {
            Settings.setUpdatesAbsentFixture(true);
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.identityReady, false);
            compare(pane.aboutSummary, "Unavailable");
            compare(pane.updateSummary, "Unavailable");
            compare(pane.storageSummary, "447 GiB used");
            compare(pane.passwordsSummary, "2 providers");
            verify(pane.aboutRow.visible, "About row stays visible");
            verify(pane.updateRow.visible, "Software Update row stays visible");
            Settings.setUpdatesAbsentFixture(false);
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
            compare(pane.storageSummary, "447 GiB used");
            compare(pane.startupSummary, "/dev/nvme0n1 · UEFI");
        }

        function test_software_update_provider_card_and_installed_rows() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "software-update");
            compare(findChild(sub, "softwareUpdateProviderCard").visible, true);
            compare(findChild(sub, "softwareUpdateProviderDetail").text,
                    "1 update available · 1 security update");
            compare(findChild(sub, "softwareUpdateInstalled").control.children[0].text,
                    "Dragonfruit Linux 44");
            compare(sub.updatesRepeater.count, 1);
            compare(findChild(sub, "softwareUpdateBeta").visible, false);
            compare(findChild(sub, "softwareUpdateLicense").visible, true);
            findChild(sub, "softwareUpdateLicense").clicked();
            verify(sub.licenseOpen);
        }

        function test_storage_bar_legend_and_volumes() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "storage");
            compare(sub.usageGroup.visible, true);
            compare(sub.usageChart.visible, true);
            compare(sub.usageChartRepeater.count, 3);
            compare(sub.usageLegendRepeater.count, 4, "three categories plus Free");
            compare(sub.categoriesGroup.visible, true);
            compare(sub.usageRepeater.count, 3);
            compare(sub.manageRow.visible, true);
            compare(sub.manageButton.text, "Show Detail");
            compare(sub.volumesGroup.visible, true);
            compare(sub.volumesRepeater.count >= 1, true);
            compare(sub.removableGroup.visible, true);
        }

        function test_passwords_keyring_banner_and_provider_footer() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "passwords");
            compare(sub.available, true);
            compare(sub.keyringBanner.title,
                    "Passwords and keys are stored in the system keyring");
            compare(sub.keyringRow.label, "System Keyring");
            compare(findChild(sub, "passwordsKeyringStatus").text, "Locked");
            compare(sub.delegateButton.text, "Open Passwords…");
            compare(sub.delegateButton.enabled, true);
            // The AutoFill toggle and Delete After Use are not rendered.
            compare(SettingsMap.rowFor("general", "passwords.autofill").state,
                    "omitted");
            compare(SettingsMap.rowFor("general", "passwords.deleteAfterUse").state,
                    "omitted");
        }

        function test_date_time_source_city_and_zone_group() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "date-time");
            compare(sub.available, true);
            compare(sub.hostGroup.visible, true);
            compare(sub.zoneGroup.visible, true);
            compare(sub.sourceRow.visible, true);
            compare(findChild(sub, "dateTimeSource").text, "Network time (NTP)");
            compare(sub.cityRow.visible, true);
            compare(findChild(sub, "dateTimeCity").text, "Los Angeles");
            compare(sub.clockGroup.visible, true);
            compare(sub.hour24Toggle.enabled, true);
        }

        function test_language_previews_and_omitted_owners() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "language");
            compare(sub.available, true);
            compare(sub.previewGroup.visible, true);
            compare(sub.regionGroup.visible, true);
            compare(sub.languageGroup.visible, true);
            verify(sub.previewText.text.length > 0);
            // The mockup's three rows without an owner are mapped omissions,
            // not disabled controls.
            compare(SettingsMap.rowFor("general", "language.textInImages").state,
                    "omitted");
            compare(SettingsMap.rowFor("general", "language.applications").state,
                    "omitted");
            compare(SettingsMap.rowFor("general", "language.translation").state,
                    "omitted");
        }

        function test_login_items_extension_group_modes() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "login");
            compare(sub.startGroup.title, "Open at Login");
            compare(sub.backgroundGroup.title, "Background App Activity");
            compare(sub.extensionsGroup.title, "Extensions");
            compare(sub.extensionMode, 0);
            compare(sub.extensionsFlatRepeater.visible, true);
            compare(sub.extensionsRepeater.visible, false);
            sub.extensionModeControl.activateIndex(1);
            compare(sub.extensionMode, 1);
            compare(sub.extensionsFlatRepeater.visible, false);
            compare(sub.extensionsRepeater.visible, true);
            sub.extensionModeControl.activateIndex(0);
            compare(sub.extensionMode, 0);
        }

        function test_startup_disk_intro_selected_note_and_advanced() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "startup");
            compare(sub.available, true);
            compare(sub.selectedNote.text, "You have selected fedora.");
            compare(sub.advancedGroup.visible, true);
            compare(findChild(sub, "startupBootModeRow").control.children[0].text,
                    "UEFI");
            compare(findChild(sub, "startupSecureBootRow").control.children[0].text,
                    "On");
            compare(sub.restartButton.visible, true);
        }

        function test_backup_backend_and_absent_states() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "backup");
            compare(sub.backendReady, true);
            compare(sub.addBackupButton.visible, false);
            compare(sub.runButton.visible, true);
            compare(sub.restoreButton.visible, true);
            compare(sub.policyGroup.visible, true);
            compare(sub.statusGroup.visible, true);
            pane.closeSubPane();

            Settings.setGeneralAvailableFixture(false);
            var absent = createTemporaryObject(backupComponent, stage);
            verify(absent);
            compare(absent.available, false);
            compare(absent.addBackupButton.visible, true);
            compare(absent.runButton.enabled, false);
            Settings.setGeneralAvailableFixture(true);
        }

        function test_reset_transfer_assistant_and_linux_erase_note() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "reset");
            compare(sub.transferGroup.title, "Data Transfer");
            compare(findChild(sub, "resetTransferButton").text,
                    "Open Transfer Assistant…");
            compare(findChild(sub, "resetTransferButton").enabled, false);
            compare(sub.resetGroup.visible, true);
            compare(sub.eraseNote.visible, true);
            verify(sub.eraseNote.text.indexOf("reinstall") >= 0);
            verify(sub.eraseNote.text.indexOf("Linux") >= 0);
        }

        function test_general_omitted_rows_are_documented() {
            var omitted = ["device.management", "about.energy", "updates.beta",
                           "sharing.contentCaching", "sharing.bluetooth",
                           "sharing.remoteScripting"];
            for (var i = 0; i < omitted.length; ++i) {
                var row = SettingsMap.rowFor("general", omitted[i]);
                verify(row !== null, omitted[i] + " must be mapped");
                compare(row.state, "omitted");
                compare(row.absence, "omitted");
                verify(row.reason.length > 0,
                       omitted[i] + " must carry an omission reason");
            }
            compare(SettingsMap.rowFor("general", "reset.erase").state, "omitted");
        }

        function test_more_info_reaches_the_canonical_page() {
            var shell = make();
            var panel = createTemporaryObject(panelComponent, stage);
            panel.moreInfoRequested.connect(function() { shell.openAboutPage(); });
            findChild(panel, "aboutPanelMoreInfo").clicked();
            tryVerify(function() { return paneOf(shell).subPaneId === "about"; });
            compare(shell.currentPaneId, "general");
            var page = paneOf(shell).subPaneLoader.item;
            compare(page.reportOpen, false);
            page.openReport();
            compare(page.reportOpen, true);
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

            sub.timezonePicker.activateIndex(2);
            tryVerify(function() {
                return Settings.general.time.timezone === "Europe/Berlin";
            });

            compare(Settings.values["clock.use24Hour"], false);
            sub.hour24Toggle.toggle();
            compare(Settings.values["clock.use24Hour"], true);
            verify(sub.previewText.text.indexOf(":") >= 0);

            sub.manualField.text = "2026-11-07 12:00:00";
            compare(sub.manualError, "");
            compare(sub.setTimeButton.enabled, true);
            sub.setTimeButton.clicked();
            tryVerify(function() {
                return Settings.general.time.localTimeMs
                    === new Date(2026, 10, 7, 12, 0, 0).getTime();
            });
            compare(Settings.generalWriteOperation, "time");
            compare(Settings.generalWriteState, "applied");
        }

        function test_date_time_time_zone_is_searchable() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "date-time");
            compare(sub.timezonePicker.visibleCount, 3);
            sub.timezonePicker.openPicker();
            tryVerify(function() { return sub.timezonePicker.popup.open; });
            sub.timezonePicker.field.text = "berlin";
            compare(sub.timezonePicker.visibleCount, 1);
            compare(sub.timezonePicker.filteredEntries[0].value, "Europe/Berlin");
            sub.timezonePicker.activateHighlighted();
            tryVerify(function() {
                return Settings.general.time.timezone === "Europe/Berlin";
            });
            compare(sub.timezonePicker.popup.open, false);

            // A query with no match is an honest empty state, not a hidden
            // list with a stale highlight.
            sub.timezonePicker.openPicker();
            tryVerify(function() { return sub.timezonePicker.popup.open; });
            sub.timezonePicker.field.text = "nowhere";
            compare(sub.timezonePicker.visibleCount, 0);
            compare(sub.timezonePicker.highlighted, -1);
            sub.timezonePicker.closePicker();
        }

        function test_date_time_validates_the_manual_field() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "date-time");
            sub.ntpToggle.toggle();
            tryVerify(function() { return Settings.general.time.ntpEnabled === false; });

            sub.manualField.text = "2026-13-01 00:00:00";
            verify(sub.manualError.length > 0);
            compare(sub.setTimeButton.enabled, false);

            sub.manualField.text = "07/11/2026 12:00";
            verify(sub.manualError.indexOf("YYYY-MM-DD") >= 0);
            compare(sub.setTimeButton.enabled, false);

            sub.manualField.text = "2026-02-30 12:00:00";
            verify(sub.manualError.length > 0);

            sub.manualField.text = "2026-11-07 12:00:00";
            compare(sub.manualError, "");
            compare(sub.setTimeButton.enabled, true);
        }

        function test_date_time_reports_a_denied_host_write() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "date-time");
            sub.ntpToggle.toggle();
            tryVerify(function() { return Settings.general.time.ntpEnabled === false; });
            sub.manualField.text = "2026-11-07 12:00:00";

            Settings.failNextGeneralWrite("time", "not authorized");
            sub.setTimeButton.clicked();
            tryVerify(function() { return Settings.generalWriteState === "denied"; });
            verify(sub.timeError.indexOf("not authorized") >= 0);
            compare(sub.setTimeButton.enabled, true);
        }

        function test_date_time_keeps_the_clock_rows_without_timedatectl() {
            Settings.set("clock.use24Hour", false);
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "date-time");
            compare(sub.available, false);
            compare(sub.absenceNote.visible, true);
            compare(sub.hostGroup.visible, true);
            compare(sub.clockGroup.visible, true);
            compare(sub.ntpToggle.enabled, false);
            compare(sub.timezonePicker.enabled, false);
            compare(sub.autoZoneToggle.enabled, false);
            compare(sub.hour24Toggle.enabled, true);
            sub.hour24Toggle.toggle();
            compare(Settings.values["clock.use24Hour"], true);
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
            Settings.setList("locale.languages", []);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "language");
            compare(sub.available, true);
            // The preferred order is seeded from the host locale but is not
            // written merely by opening the pane.
            compare(Settings.values["locale.languages"].length, 0);
            compare(sub.effectiveLanguages.length, 1);

            sub.addLanguagePicker.activateIndex(0);
            tryVerify(function() {
                return Settings.values["locale.languages"].length === 2;
            });
            compare(Settings.values["locale.languages"][1], "de_DE.UTF-8");

            // Moving the new language to the front is the host locale write.
            sub.moveLanguage(1, -1);
            tryVerify(function() { return Settings.general.locale.locale === "de_DE.UTF-8"; });
            compare(Settings.values["locale.languages"][0], "de_DE.UTF-8");

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

        function test_language_region_and_preview_follow_the_selected_locale() {
            Settings.setList("locale.languages", []);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "language");
            compare(sub.regionPicker.visibleCount, 3);
            sub.regionPicker.activateIndex(1);
            tryVerify(function() { return Settings.general.locale.locale === "de_DE.UTF-8"; });
            compare(sub.primaryLocale, "de_DE.UTF-8");

            // The preview formats through the selected locale, and the three
            // date lengths are visibly distinct.
            compare(sub.previewDateFor("medium"), "7. November 2026");
            compare(sub.previewDateFor("long"), "Samstag, 7. November 2026");
            verify(sub.previewDateFor("short") !== sub.previewDateFor("medium"));

            sub.dateFormatSelect.activateIndex(0); // short
            verify(sub.previewText.text.indexOf("07.11.26") >= 0);
            sub.dateFormatSelect.activateIndex(2); // long
            verify(sub.previewText.text.indexOf("Samstag") >= 0);

            sub.numberFormatSelect.activateIndex(2); // decimal comma
            verify(sub.previewText.text.indexOf("1.234,5") >= 0);
            sub.numberFormatSelect.activateIndex(1); // decimal point
            verify(sub.previewText.text.indexOf("1,234.5") >= 0);
            sub.numberFormatSelect.activateIndex(0); // locale
            verify(sub.previewText.text.indexOf("1.234,5") >= 0);
        }

        function test_language_remove_and_empty_preferred_list() {
            Settings.setList("locale.languages", []);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "language");
            sub.addLanguagePicker.activateIndex(0);
            tryVerify(function() { return Settings.values["locale.languages"].length === 2; });
            sub.removeLanguage(0);
            tryVerify(function() { return Settings.values["locale.languages"].length === 1; });
            sub.removeLanguage(0);
            tryVerify(function() { return Settings.values["locale.languages"].length === 0; });
            // With an empty preference the pane falls back to the host locale
            // instead of showing an empty Language group.
            compare(sub.effectiveLanguages.length, 1);
        }

        function test_language_keeps_the_format_rows_without_localectl() {
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "language");
            compare(sub.available, false);
            compare(sub.absenceNote.visible, true);
            compare(sub.languageGroup.visible, true);
            compare(sub.regionGroup.visible, true);
            compare(sub.previewGroup.visible, true);
            compare(sub.addLanguagePicker.enabled, false);
            compare(sub.regionPicker.enabled, false);
            compare(sub.calendarSelect.enabled, true);
            sub.calendarSelect.activateIndex(1);
            compare(Settings.values["locale.calendar"], "iso");
            verify(sub.previewText.text.length > 0);
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
            findChild(item, "loginToggle1").toggle();
            tryVerify(function() {
                return Settings.general.login.autostart[1].enabled === true;
            });
            var unit = sub.userUnitRepeater.itemAt(1);
            findChild(unit, "backgroundToggle1").toggle();
            tryVerify(function() {
                return Settings.general.login.userUnits[1].enabled === "enabled";
            });
        }

        function test_login_items_add_remove_and_broken_warning() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "login");
            compare(sub.autostartRepeater.count, 2);
            compare(sub.brokenRow.visible, true);
            verify(sub.brokenRow.description.indexOf("Example Agent") >= 0);

            // Add an installed application: one host write, one new row.
            compare(sub.addPicker.visibleCount, 2);
            sub.addPicker.activateIndex(0);
            tryVerify(function() {
                return Settings.general.login.autostart.length === 3;
            });
            compare(Settings.generalWriteOperation, "login");
            compare(Settings.generalWriteState, "applied");
            compare(sub.autostartRepeater.count, 3);

            // Remove the added item again.
            findChild(sub.autostartRepeater.itemAt(2), "loginRemove2").clicked();
            tryVerify(function() {
                return Settings.general.login.autostart.length === 2;
            });
            compare(sub.autostartRepeater.count, 2);
        }

        function test_login_items_keep_every_group_when_the_adapter_is_absent() {
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "login");
            compare(sub.available, false);
            compare(sub.absenceNote.visible, true);
            compare(sub.startGroup.visible, true);
            compare(sub.backgroundGroup.visible, true);
            compare(sub.extensionsGroup.visible, true);
            compare(sub.addPicker.enabled, false);
        }

        // -- Sharing -------------------------------------------------------

        function test_sharing_lists_only_installed_services_and_toggles_one() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "sharing");
            compare(sub.available, true);
            // The fixture's three installed services land in the mockup's
            // three sections: File, Printer, Remote Login.
            compare(sub.contentRepeater.count, 1);
            compare(sub.accessoriesRepeater.count, 1);
            compare(sub.advancedRepeater.count, 1);
            compare(sub.advancedRepeater.itemAt(0).modelData.id, "remoteLogin");
            var row = sub.advancedRepeater.itemAt(0);
            // The row description is the service description only: state is the
            // toggle and authentication is the Options sheet, never a suffix.
            compare(row.description, "Allow remote login with SSH.");
            verify(row.description.indexOf("Requires authentication") < 0);
            var toggle = findChild(row, "sharingToggle-remoteLogin");
            compare(toggle.checked, true);
            toggle.toggle();
            tryVerify(function() {
                return Settings.general.sharing.services[1].active === false;
            });
            compare(Settings.general.sharing.activeCount, 1);
        }

        function test_sharing_hostname_edit_and_per_service_options() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "sharing");
            compare(sub.hostnameField.text, "dragon");

            sub.hostnameField.text = "dragon-two";
            sub.hostnameButton.clicked();
            tryVerify(function() {
                return Settings.general.sharing.hostname === "dragon-two";
            });
            compare(Settings.generalWriteOperation, "hostname");
            compare(Settings.generalWriteState, "applied");

            // An invalid hostname is refused before the host is asked.
            sub.hostnameField.text = "not a hostname";
            sub.hostnameButton.clicked();
            verify(sub.hostnameError.length > 0);
            compare(Settings.general.sharing.hostname, "dragon-two");

            // Each service owns a read-only Options sheet with its real
            // details; the authentication note lives there.
            var row = sub.advancedRepeater.itemAt(0);
            findChild(row, "sharingOptions-remoteLogin").clicked();
            compare(sub.optionsDialog.open, true);
            verify(sub.optionsService.details.length >= 2);
            sub.optionsDialog.accept();
        }

        function test_sharing_keeps_the_service_list_without_systemd() {
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "sharing");
            compare(sub.available, false);
            compare(sub.servicesGroup.visible, true);
            compare(sub.absenceNote.visible, true);
            compare(sub.hostnameButton.enabled, false);
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

        function test_backup_validates_the_destination_and_disables_policy_without_a_backend() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "backup");
            compare(sub.available, true);

            sub.destinationField.text = "relative/path";
            verify(sub.destinationError.length > 0);
            compare(sub.destinationSave.enabled, false);

            sub.destinationField.text = "/mnt/backup";
            compare(sub.destinationError, "");
            compare(sub.destinationSave.enabled, true);
            sub.destinationSave.clicked();
            compare(Settings.values["backup.destination"], "/mnt/backup");

            sub.destinationField.text = "sftp://host/repo";
            compare(sub.destinationError, "");

            // With the host absent the policy rows stay live but the schedule
            // and automatic toggle name the missing backend and disable.
            Settings.setGeneralAvailableFixture(false);
            tryVerify(function() { return sub.available === false; });
            compare(sub.absenceNote.visible, true);
            compare(sub.policyGroup.visible, true);
            compare(sub.scheduleSelect.enabled, false);
            compare(sub.autoToggle.enabled, false);
            compare(sub.destinationSave.enabled, true);
            verify(sub.absenceNote.text.indexOf("system status") >= 0);
            verify(sub.absenceNote.text.indexOf("backend is installed") < 0);
        }

        function test_backup_reports_a_denied_run() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "backup");
            Settings.failNextGeneralWrite("backup", "not authorized");
            sub.runButton.clicked();
            tryVerify(function() { return Settings.generalWriteState === "denied"; });
            verify(sub.writeError.indexOf("not authorized") >= 0);
        }

        // -- Startup Disk --------------------------------------------------

        function test_startup_disk_lists_disks_and_uses_gib() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "startup");
            compare(sub.available, true);
            // One Startup disk row; the device is in its description, not a
            // duplicate Device row.
            compare(findChild(sub, "startupDiskRow").control.children[0].text, "fedora");
            verify(findChild(sub, "startupDiskRow").description.indexOf("/dev/nvme0n1") >= 0);
            compare(findChild(sub, "startupBootModeRow").control.children[0].text, "UEFI");
            compare(findChild(sub, "startupSecureBootRow").control.children[0].text, "On");
            verify(sub.readOnlyNote.visible);

            compare(sub.diskRepeater.count, 2);
            compare(sub.diskRepeater.itemAt(0).modelData.device, "/dev/nvme0n1");
            compare(sub.diskRepeater.itemAt(0).control.children[0].text, "Startup disk");
            compare(sub.diskRepeater.itemAt(1).control.children[0].text, "Not mounted");
            verify(sub.diskRepeater.itemAt(0).description.indexOf("GiB") >= 0);
            verify(sub.diskRepeater.itemAt(0).description.indexOf(" GB") < 0);
            compare(sub.sizeLabel(512000000000), "477 GiB");
        }

        function test_startup_disk_restart_needs_confirmation() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "startup");
            compare(sub.restartDialog.open, false);
            sub.restartButton.clicked();
            compare(sub.restartDialog.open, true);
            sub.restartConfirm.clicked();
            compare(sub.restartDialog.open, false);
            compare(Settings.generalWriteOperation, "restart");
            compare(Settings.generalWriteState, "applied");
        }

        function test_startup_disk_keeps_the_read_only_structure_when_absent() {
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "startup");
            compare(sub.available, false);
            compare(sub.diskListGroup.visible, true);
            compare(sub.absenceNote.visible, true);
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
            compare(sub.confirmButton.enabled, false);
            sub.confirmField.text = "RESET";
            compare(sub.confirmButton.enabled, true);
            sub.confirmButton.clicked();
            tryVerify(function() {
                return Settings.values["dock.size"] === Settings.schemaFor("dock.size").default;
            });
            compare(sub.doneText.visible, true);
        }

        function test_reset_page_has_the_transfer_structure_and_linux_reason() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "reset");
            compare(sub.transferGroup.visible, true);
            compare(sub.resetGroup.visible, true);
            compare(sub.eraseNote.visible, true);
            verify(sub.eraseNote.text.indexOf("reinstall") >= 0);
            compare(findChild(sub, "resetTransferButton").enabled, false);
            compare(sub.resetButton.variant, "danger");
            // Escape/cancel leaves the settings untouched.
            Settings.set("dock.size", 0.9);
            sub.resetButton.clicked();
            sub.confirmDialog.reject();
            compare(Settings.values["dock.size"], 0.9);
        }

        // -- Absence -------------------------------------------------------

        function test_every_general_sub_pane_degrades_with_the_host_absent() {
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.generalAvailable, false);
            compare(pane.loginRow.visible, true);
            compare(pane.loginSummary, "Unavailable");
            var sharing = createTemporaryObject(sharingComponent, stage);
            verify(sharing);
            compare(sharing.available, false);
            compare(sharing.absenceNote.visible, true);
            var backup = createTemporaryObject(backupComponent, stage);
            verify(backup);
            compare(backup.available, false);
            compare(backup.absenceNote.visible, true);
        }

        function test_every_general_sub_pane_fits_the_740x820_window() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            var pane = paneOf(shell);
            var ids = ["about", "software-update", "passwords", "date-time",
                       "language", "login", "sharing", "backup", "startup",
                       "reset", "storage"];
            for (var i = 0; i < ids.length; ++i) {
                var sub = openSubPane(pane, ids[i]);
                verify(sub, "the " + ids[i] + " sub-pane must load");
                compare(sub.width, pane.width);
                verify(sub.implicitHeight > 0,
                       ids[i] + " must have a measured height");
                verify(shell.detailScroll.height > 0);
                pane.closeSubPane();
                waitForRendering(stage);
            }
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
            compare(pane.usageChart.visible, true);
            compare(pane.usageChartRepeater.count, 3);
            verify(pane.usageSummaryText.text.indexOf("used of") >= 0);
            compare(pane.recommendationsRepeater.count > 0, true);
            compare(pane.manageButton.enabled, true);
        }

        // -- Autofill & Passwords ------------------------------------------

        function test_passwords_lists_providers_and_delegates() {
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "passwords");
            compare(sub.available, true);
            compare(sub.providersRepeater.count, 3);
            compare(sub.keyringRow.visible, true);
            compare(findChild(sub, "passwordsKeyringStatus").text, "Locked");
            compare(sub.delegateProvider.id, "keyring");
            compare(sub.canDelegate, true);

            // The autofill source is a durable settingsd selection, not a
            // secret. Selecting KeePassXC changes the delegate.
            sub.sourceSelect.activateIndex(2);
            compare(Settings.values["defaults.passwordManager"], "keepassxc");
            compare(sub.delegateProvider.id, "keepassxc");

            // The delegate is one host request; Settings stores no secret.
            sub.delegateButton.clicked();
            compare(sub.secretNote.visible, true);
        }

        function test_passwords_pane_degrades_without_the_general_host() {
            Settings.setGeneralAvailableFixture(false);
            var shell = make();
            var pane = paneOf(shell);
            var sub = openSubPane(pane, "passwords");
            compare(sub.available, false);
            compare(sub.absenceNote.visible, true);
            compare(sub.delegateButton.enabled, false);
        }
    }
}
