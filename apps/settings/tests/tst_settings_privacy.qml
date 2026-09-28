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
            compare(pane.categories.length, 14);
            compare(pane.appCount, 5);

            // The first row is Camera with its two-app count.
            compare(findChild(pane.categoryRepeater.itemAt(0), "privacyCategoryLabel").text,
                    "Camera");
            compare(findChild(pane.categoryRepeater.itemAt(0), "privacyCategorySummary").text,
                    "2 apps");
            compare(findChild(pane.categoryRepeater.itemAt(1), "privacyCategorySummary").text,
                    "1 app");
            // A category with no entry is still a row and says `None`.
            compare(findChild(pane.categoryRepeater.itemAt(2), "privacyCategorySummary").text,
                    "1 app");
            compare(findChild(pane.categoryRepeater.itemAt(4), "privacyCategorySummary").text,
                    "None");
        }

        function test_category_dialog_round_trips_a_permission() {
            var shell = make();
            var pane = paneOf(shell);
            pane.openCategory("devices");
            compare(pane.permissionDialog.open, true);
            compare(pane.selectedApps.length, 2);

            // Snapshot is denied in the fixture; allow it.
            compare(pane.selectedApps[0].app, "org.example.Snapshot");
            compare(pane.selectedApps[0].state, "denied");
            var first = pane.permissionRepeater.itemAt(0);
            var select = findChild(first, "privacyPermissionSelect");
            select.activateIndex(0); // Allowed
            tryVerify(function() {
                var category = pane.findCategory("devices");
                return category.resources[0].apps[0].state === "allowed";
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
            compare(pane.categories.length, 14);
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
    }
}