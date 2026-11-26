// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Privacy & Security pane tests (T-15.13b; T279 completion). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_PRIVACY_FIXTURE` (see CMakeLists.txt), so the
// `Settings` singleton and the host-stack client are deterministic and
// in-process with no bus (and no portal). The cases cover the grouped category
// list, the per-category dialog, the tristate/removal round-trip, the Location
// Services master, the analytics default, the USBGuard accessory row, and the
// keyboard path into a category.
Item {
    id: stage
    width: 1000
    height: 1600

    TestCase {
        id: testCase
        name: "SettingsPrivacy"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture is process-global; reset it before every case so each
        // starts on the known four-category / five-permission store and the
        // schema defaults.
        function init() {
            Settings.resetPrivacyFixture();
            Settings.setPrivacyAvailableFixture(true);
            Settings.set("privacy.locationServices", true);
            Settings.set("privacy.analytics", false);
        }

        function make(width, height) {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: width || stage.width,
                                                height: height || stage.height });
            waitForRendering(stage);
            shell.selectPane("privacy");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Privacy & Security pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_grouped_category_list() {
            var shell = make();
            compare(shell.currentPaneId, "privacy");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.mappedCategories.length, 15);
            compare(pane.appCount, 5);

            // Location Services is its own group with the master gate and the
            // one disclosure row.
            compare(pane.locationGroup.title, "Location Services");
            compare(pane.locationRow.label, "Location");
            compare(findChild(pane.locationRow, "privacyLocationSummary").text,
                    "1 app");
            compare(pane.locationMasterToggle.checked, true);

            // App access is a second group; the location row is not repeated.
            compare(pane.appGroup.title, "App access");
            compare(pane.appCategories.length, 14);
            // Contacts is the first row and is delegated; Camera is the fifth
            // and carries its two-app count.
            compare(pane.categoryRepeater.itemAt(0).label, "Contacts");
            compare(findChild(pane.categoryRepeater.itemAt(0),
                              "privacyCategorySummary").text, "Delegated");
            compare(pane.categoryRepeater.itemAt(4).label, "Camera");
            compare(findChild(pane.categoryRepeater.itemAt(4),
                              "privacyCategorySummary").text, "2 apps");

            // Distinct category glyphs, not one repeated mark.
            compare(pane.categoryRepeater.itemAt(0).leadingIcon, "privacy-contacts");
            compare(pane.categoryRepeater.itemAt(4).leadingIcon, "privacy-camera");
            compare(pane.categoryRepeater.itemAt(5).leadingIcon, "privacy-microphone");
            compare(pane.locationRow.leadingIcon, "privacy-location");

            // The real portal tables that have no macOS row remain listed.
            compare(pane.otherCategories.length > 0, true);
        }

        function test_every_category_row_is_a_keyboard_stop() {
            var shell = make();
            var pane = paneOf(shell);
            var row = pane.categoryRepeater.itemAt(4); // Camera
            compare(row.activeFocusOnTab, true);
            compare(row.Accessible.name, "Camera");
            row.forceActiveFocus();
            tryVerify(function() { return row.activeFocus; });
            keyClick(Qt.Key_Return);
            compare(pane.permissionDialog.open, true);
            compare(pane.selectedCategoryId, "camera");
            pane.permissionDialog.accept();
        }

        function test_category_dialog_round_trips_a_permission() {
            var shell = make();
            var pane = paneOf(shell);
            pane.openCategory("camera");
            compare(pane.permissionDialog.open, true);
            compare(pane.selectedApps.length, 2);

            // Snapshot is denied in the fixture; allow it.
            compare(pane.selectedApps[0].app, "org.example.Snapshot");
            compare(pane.selectedApps[0].state, "denied");
            var first = pane.permissionRepeater.itemAt(0);
            var select = findChild(first, "privacyPermissionSelect");
            select.activateIndex(0); // Allowed
            tryVerify(function() {
                var category = pane.findCategory("camera");
                return category.apps[0].state === "allowed";
            });
        }

        function test_not_set_removes_the_permission() {
            var shell = make();
            var pane = paneOf(shell);
            pane.openCategory("location");
            compare(pane.selectedApps.length, 1);
            var select = findChild(pane.permissionRepeater.itemAt(0), "privacyPermissionSelect");
            select.activateIndex(3); // Not Set
            tryVerify(function() {
                return pane.findCategory("location").summary === "None";
            });
            // Removing the only location app drops the store's app count.
            tryVerify(function() { return pane.appCount === 4; });
        }

        function test_empty_store_still_lists_every_category() {
            var shell = make();
            var pane = paneOf(shell);
            // Remove every known application permission.
            Settings.deletePrivacyPermission("devices", "camera", "org.example.Snapshot");
            Settings.deletePrivacyPermission("devices", "camera", "org.mozilla.firefox");
            Settings.deletePrivacyPermission("location", "location", "org.example.Maps");
            Settings.deletePrivacyPermission("notifications", "notification",
                                             "org.example.Calendar");
            Settings.deletePrivacyPermission("screencast", "screencast",
                                             "com.obsproject.Studio");
            tryVerify(function() { return pane.appCount === 0; });
            compare(pane.mappedCategories.length, 15);
            compare(pane.emptyNote.visible, true);
            compare(findChild(pane.locationRow, "privacyLocationSummary").text,
                    "None");
            compare(findChild(pane.categoryRepeater.itemAt(4),
                              "privacyCategorySummary").text, "None");
        }

        function test_absence_note_hidden_with_a_ready_view() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.absenceNote.visible, false);
            compare(pane.emptyNote.visible, false);
        }

        function test_location_master_gates_per_app_writes() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.locationEnabled, true);

            pane.locationMasterToggle.toggle();
            tryVerify(function() { return pane.locationEnabled === false; });
            compare(Settings.values["privacy.locationServices"], false);
            compare(pane.locationMasterNote.visible, true);

            // The per-app location dialog stays readable but its writes are
            // disabled while the master is off.
            pane.openCategory("location");
            compare(pane.permissionDialog.open, true);
            var select = findChild(pane.permissionRepeater.itemAt(0),
                                   "privacyPermissionSelect");
            verify(select);
            compare(select.enabled, false);
            pane.permissionDialog.accept();

            pane.locationMasterToggle.toggle();
            tryVerify(function() { return pane.locationEnabled === true; });
            compare(Settings.values["privacy.locationServices"], true);
        }

        function test_analytics_is_opt_in_and_defaults_off() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.analyticsGroup.title, "Analytics & Improvements");
            compare(pane.analyticsEnabled, false);
            compare(Settings.values["privacy.analytics"], false);
            compare(pane.analyticsToggle.checked, false);

            pane.analyticsToggle.toggle();
            tryVerify(function() { return pane.analyticsEnabled === true; });
            compare(Settings.values["privacy.analytics"], true);
        }

        function test_security_rows_and_sources_dialog() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.securityReady, true);
            compare(pane.encryption.label, "Encrypted (LUKS)");
            compare(pane.hardening.label, "Enforcing");
            compare(pane.extensions.length, 1);
            compare(pane.securitySources.length, 2);
            compare(pane.sourceSummary(), "Fedora repositories, Flathub (system)");

            pane.sourcesButton.clicked();
            compare(pane.sourcesDialog.open, true);
            compare(pane.sourceRepeater.count, 2);
            var toggle = findChild(pane.sourceRepeater.itemAt(1), "privacySourceToggle");
            verify(toggle);
            compare(toggle.checked, true);
            toggle.toggled(false);
            tryVerify(function() { return pane.enabledSourceCount === 1; });
            pane.sourcesDialog.accept();
        }

        function test_disk_encryption_and_accessories_delegate_sheets() {
            var shell = make();
            var pane = paneOf(shell);

            // USB accessories (USBGuard) report the policy and delegate edits.
            compare(pane.securityReady, true);
            compare(pane.accessories.available, true);
            compare(pane.accessoriesButton.enabled, true);
            pane.accessoriesButton.clicked();
            compare(pane.accessoriesDialog.open, true);
            compare(findChild(pane.accessoriesDialog, "privacyAccessoriesPolicy").text,
                    "2 devices managed");
            pane.accessoriesDialog.accept();

            // Disk encryption: read-only status plus the device sheet.
            compare(pane.encryptionButton.enabled, true);
            pane.encryptionButton.clicked();
            compare(pane.encryptionDialog.open, true);
            compare(pane.encryptionDeviceRepeater.count, 1);
            pane.encryptionDialog.accept();
        }

        function test_accessories_absent_disables_only_that_row() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setAccessoriesAvailableFixture(false);
            tryVerify(function() { return pane.accessories.available === false; });
            compare(pane.accessoriesButton.enabled, false);
            compare(pane.sourcesButton.enabled, true);
            compare(pane.securityReady, true);
        }

        function test_an_external_permission_change_reflects_in_the_pane() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.findCategory("camera").summary, "2 apps");

            // An app (or another portal client) changes the store; the pane is
            // open and must converge without a reload.
            Settings.deletePrivacyPermission("devices", "camera",
                                             "org.mozilla.firefox");
            tryVerify(function() {
                return pane.findCategory("camera").summary === "1 app";
            });
            Settings.setPrivacyPermission("devices", "camera",
                                          "org.gimp.GIMP", "ask");
            tryVerify(function() {
                return pane.findCategory("camera").summary === "2 apps";
            });
            var camera = pane.findCategory("camera");
            compare(camera.apps[1].app, "org.gimp.GIMP");
            compare(camera.apps[1].state, "ask");
        }

        function test_portal_absent_keeps_the_security_half_live() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setPrivacyAvailableFixture(false);
            tryVerify(function() { return pane.ready === false; });
            compare(pane.absenceNote.visible, true);
            compare(pane.securityReady, true);
            compare(pane.securityGroup.visible, true);
            compare(pane.sourcesButton.enabled, true);
            compare(pane.encryption.label, "Encrypted (LUKS)");
        }

        // T279: the measured 740×820 window must not clip the grouped rows.
        function test_measured_window_740x820_keeps_every_row_reachable() {
            var shell = make(740, 820);
            var pane = paneOf(shell);
            verify(pane.width > 0 && pane.width <= 740);
            verify(pane.implicitWidth <= 740);
            var groups = [pane.locationGroup, pane.appGroup, pane.analyticsGroup,
                          pane.securityGroup];
            for (var g = 0; g < groups.length; ++g) {
                verify(groups[g].width > 0, "group " + g + " must have width");
                var rows = groups[g].rows.children;
                for (var i = 0; i < rows.length; ++i) {
                    if (rows[i].control === undefined)
                        continue;
                    verify(rows[i].width > 0, "row " + i + " in group " + g);
                    verify(rows[i].width <= 740);
                }
            }
        }
    }
}
