// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Privacy & Security pane tests (T-15.13b). Headless with `DF_SETTINGS_FIXTURE`
// and `DF_PRIVACY_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton
// and the host-stack client are deterministic and in-process with no bus (and
// no portal). The cases cover the flat category list, the per-category dialog,
// and the tristate/removal round-trip.
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
        // starts on the known four-category / five-permission store.
        function init() {
            Settings.resetPrivacyFixture();
            Settings.setPrivacyAvailableFixture(true);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
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

        function test_pane_opens_with_the_flat_category_list() {
            var shell = make();
            compare(shell.currentPaneId, "privacy");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.mappedCategories.length, 15);
            compare(pane.appCount, 5);

            // The macOS rows are the pane's order: Location first, Camera at
            // index 5 with its two-app count.
            compare(findChild(pane.categoryRepeater.itemAt(0), "privacyCategoryLabel").text,
                    "Location");
            compare(findChild(pane.categoryRepeater.itemAt(0), "privacyCategorySummary").text,
                    "1 app");
            compare(findChild(pane.categoryRepeater.itemAt(5), "privacyCategoryLabel").text,
                    "Camera");
            compare(findChild(pane.categoryRepeater.itemAt(5), "privacyCategorySummary").text,
                    "2 apps");
            // A delegated category says so instead of inventing a count.
            compare(findChild(pane.categoryRepeater.itemAt(1), "privacyCategorySummary").text,
                    "Delegated");
            // The real portal tables that have no macOS row remain listed.
            compare(pane.otherCategories.length > 0, true);
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
            compare(findChild(pane.categoryRepeater.itemAt(0), "privacyCategorySummary").text,
                    "None");
        }

        function test_absence_note_hidden_with_a_ready_view() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.absenceNote.visible, false);
            compare(pane.emptyNote.visible, false);
        }

        function test_macos_rows_state_the_linux_mapping() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.mappedCategories.length, 15);
            var location = pane.findCategory("location");
            compare(location.label, "Location");
            verify(location.difference.length > 0);
            compare(location.delegated, false);
            var contacts = pane.findCategory("contacts");
            compare(contacts.delegated, true);
            compare(contacts.summary, "Delegated");
            verify(contacts.linuxOwner.length > 0);
            var camera = pane.findCategory("camera");
            compare(camera.summary, "2 apps");
            // The real portal tables without a macOS row stay visible.
            var hasNotifications = false;
            for (var i = 0; i < pane.otherCategories.length; ++i)
                if (pane.otherCategories[i].id === "notifications")
                    hasNotifications = true;
            compare(hasNotifications, true);
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
    }
}