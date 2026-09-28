// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Notifications and Focus pane tests (T-15.7b). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_NOTIFICATIONS_FIXTURE` (see CMakeLists.txt), so
// the `Settings` singleton serves a deterministic in-process notifications
// adapter with no bus. The cases cover the Notification Center preferences
// round-tripping through settingsd, the Focus mode and per-app allow list
// round-tripping through the adapter, an external change converging, and the
// absence note.
Item {
    id: stage
    width: 900
    height: 1400

    TestCase {
        id: testCase
        name: "SettingsNotifications"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture client is process-global; reset the mode and the allow
        // list before every case so the panes always open on a known state.
        function init() {
            Settings.setFocusMode("focus");
            Settings.setFocusApp("Mail", false);
            Settings.setFocusApp("chat", false);
            Settings.set("notifications.showPreviews", "when-unlocked");
            Settings.set("notifications.showWhenSleeping", false);
            Settings.set("notifications.showWhenLocked", true);
            Settings.set("notifications.showWhenMirroring", false);
        }

        function make(paneId) {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane(paneId);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_notifications_pane_opens_with_the_prefs_and_apps() {
            var shell = make("notifications");
            compare(shell.currentPaneId, "notifications");
            var pane = paneOf(shell);
            verify(pane.ready, "the notifications view must be available");
            compare(pane.showPreviews, "when-unlocked");
            compare(pane.previewSelect.currentIndex, 1);
            compare(pane.previewSelect.currentLabel, "When Unlocked");
            compare(pane.lockedToggle.checked, true);
            compare(pane.apps.length, 3);
            verify(pane.appList.itemAt(0).width > 0,
                   "the per-app inventory rows must have a real width");
            compare(pane.absenceNote.visible, false);
        }

        function test_notification_center_prefs_round_trip_through_settingsd() {
            var shell = make("notifications");
            var pane = paneOf(shell);

            pane.previewSelect.activateIndex(2); // Never
            compare(Settings.values["notifications.showPreviews"], "never");
            compare(pane.showPreviews, "never");

            pane.sleepingToggle.toggle();
            compare(Settings.values["notifications.showWhenSleeping"], true);
            pane.lockedToggle.toggle();
            compare(Settings.values["notifications.showWhenLocked"], false);
            pane.mirroringToggle.toggle();
            compare(Settings.values["notifications.showWhenMirroring"], true);
        }

        function test_external_pref_change_converges_into_the_controls() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            Settings.set("notifications.showPreviews", "always");
            compare(pane.previewSelect.currentIndex, 0);
            compare(pane.previewSelect.currentLabel, "Always");
            Settings.set("notifications.showWhenLocked", false);
            compare(pane.lockedToggle.checked, false);
        }

        function test_focus_mode_selector_round_trips_through_the_adapter() {
            var shell = make("focus");
            var pane = paneOf(shell);
            compare(pane.mode, "focus");
            compare(pane.modeSelector.currentIndex, 1);
            verify(pane.allowList.itemAt(0).width > 0,
                   "the allowed-app rows must have a real width");

            pane.modeSelector.activateIndex(2); // Do Not Disturb
            compare(Settings.notifications.mode, "dnd");
            compare(pane.mode, "dnd");
            verify(pane.summary.text.indexOf("Do Not Disturb") >= 0);

            pane.modeSelector.activateIndex(0); // Off
            compare(Settings.notifications.mode, "off");
            verify(pane.summary.text.indexOf("normally") >= 0);
        }

        function test_focus_allow_list_toggle_round_trips_through_the_adapter() {
            var shell = make("focus");
            var pane = paneOf(shell);
            compare(Settings.notifications.allowList.length, 1); // Pager

            // The second app row is Mail (case-insensitive sort: chat, Mail,
            // Pager). Allowing it appends to the list.
            var mailToggle = pane.allowList.itemAt(1).allowToggle;
            verify(mailToggle, "the allow toggle must exist on the Mail row");
            mailToggle.toggle();
            tryVerify(function() {
                return Settings.notifications.allowList.indexOf("Mail") >= 0;
            }, 2000, "allowing Mail adds it to the adapter allow list");
            compare(pane.allowList.itemAt(1).modelData.allowed, true);

            // Toggling back removes exactly that entry. The view re-emits after the
            // write, so the delegate is rebuilt; re-fetch the toggle.
            pane.allowList.itemAt(1).allowToggle.toggle();
            tryVerify(function() {
                return Settings.notifications.allowList.indexOf("Mail") === -1;
            }, 2000, "disallowing Mail removes it from the adapter allow list");
        }

        function test_notifications_pane_reflects_the_focus_allow_list() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            // Pager is on the fixture allow list, so its inventory row says
            // Allowed; Mail is not.
            compare(pane.apps[2].name, "Pager");
            compare(pane.apps[2].status, "Allowed");
            compare(pane.apps[1].name, "Mail");
            compare(pane.apps[1].status, "Default");
        }

        function test_absence_note_hides_when_the_service_is_present() {
            // In fixture mode the bridge client is available, so the panes show
            // their controls. The no-host path is covered by
            // tst_settings_absence.qml.
            var shell = make("focus");
            var pane = paneOf(shell);
            compare(Settings.notificationsAvailable, true);
            compare(pane.absenceNote.visible, false);
        }
    }
}