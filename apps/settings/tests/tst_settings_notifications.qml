// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Notifications and Focus pane tests (T-15.7b). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_NOTIFICATIONS_FIXTURE` (see CMakeLists.txt), so
// the `Settings` singleton serves a deterministic in-process notifications
// adapter with no bus. The cases cover Notifications preferences
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
            Settings.setFocusProfile("personal");
            Settings.setFocusApp("Mail", false);
            Settings.setFocusApp("chat", false);
            Settings.set("notifications.showPreviews", "when-unlocked");
            Settings.set("notifications.showWhenSleeping", false);
            Settings.set("notifications.showWhenLocked", true);
            Settings.set("notifications.showWhenMirroring", false);
            Settings.set("notifications.appOptions", "");
            Settings.setList("focus.customProfiles", []);
            Settings.set("focus.customProfile", "");
            Settings.set("focus.schedule.enabled", false);
            Settings.set("focus.schedule.start", "22:00");
            Settings.set("focus.schedule.end", "07:00");
            Settings.setList("focus.schedule.days", ["mon", "tue", "wed", "thu",
                                                    "fri", "sat", "sun"]);
            Settings.set("focus.shortcut", "");
            for (var i = 0; i < 3; ++i) {
                var name = ["Mail", "chat", "Pager"][i];
                Settings.setNotificationAppPreference(name, "banner", true,
                                                      true, true);
            }
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
            verify(pane.ready, "Notifications view must be available");
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

        function test_focus_list_is_one_real_control_surface() {
            var shell = make("focus");
            var pane = paneOf(shell);
            compare(pane.mode, "focus");
            // Off + Do Not Disturb, Personal, Work, Sleep, Custom.
            compare(pane.focusList.count, 6);
            verify(pane.focusList.itemAt(0).width > 0);
            compare(pane.focusList.itemAt(0).active, false);
            compare(pane.focusList.itemAt(2).active, true); // Personal
            compare(pane.focusList.itemAt(2).Accessible.checked, true);
            verify(pane.focusList.itemAt(2).Accessible.description.length > 0);
            verify(pane.summary.text.indexOf("Personal") >= 0);

            pane.activateFocus("dnd");
            tryVerify(function() { return Settings.notifications.mode === "dnd"; },
                      2000);
            compare(pane.focusList.itemAt(1).active, true);
            verify(pane.summary.text.indexOf("Do Not Disturb") >= 0);

            pane.activateFocus("off");
            tryVerify(function() { return Settings.notifications.mode === "off"; },
                      2000);
            verify(pane.summary.text.indexOf("normally") >= 0);
        }

        function test_focus_add_and_delete_a_named_focus() {
            var shell = make("focus");
            var pane = paneOf(shell);
            pane.addFocusButton.clicked();
            waitForRendering(stage);
            compare(pane.addFocusDialog.open, true);
            pane.addFocusInput.text = "Writing";
            pane.addFocus();
            tryVerify(function() {
                return Settings.values["focus.customProfiles"].indexOf("Writing") >= 0;
            }, 2000);
            tryVerify(function() {
                return Settings.notifications.profile === "custom";
            }, 2000);
            compare(pane.activeEntry.name, "Writing");
            verify(pane.deleteFocusButton.enabled);

            pane.deleteFocusButton.clicked();
            waitForRendering(stage);
            compare(pane.deleteFocusDialog.open, true);
            pane.deleteActiveFocus();
            tryVerify(function() {
                return Settings.values["focus.customProfiles"].indexOf("Writing") === -1;
            }, 2000);
            tryVerify(function() {
                return Settings.notifications.mode === "off";
            }, 2000);
        }

        function test_focus_add_validates_an_empty_or_duplicate_name() {
            var shell = make("focus");
            var pane = paneOf(shell);
            pane.addFocusButton.clicked();
            waitForRendering(stage);
            pane.addFocusInput.text = "";
            pane.addFocus();
            compare(pane.addFocusDialog.open, true);
            verify(pane.addError.length > 0);

            pane.addFocusInput.text = "Work";
            pane.addFocus();
            compare(pane.addFocusDialog.open, true);
            verify(pane.addError.indexOf("already exists") >= 0);
            pane.addFocusDialog.reject();
        }

        function test_focus_app_and_people_filters() {
            var shell = make("focus");
            var pane = paneOf(shell);
            compare(pane.allowList.count, 3);
            pane.appFilter = "allowed";
            compare(pane.allowList.count, 1); // Pager
            pane.appFilter = "blocked";
            compare(pane.allowList.count, 2); // chat, Mail
            pane.appFilter = "all";

            compare(pane.peopleList.count, 2);
            pane.peopleFilter = "allowed";
            compare(pane.peopleList.count, 1); // Ada
            pane.peopleFilter = "blocked";
            compare(pane.peopleList.count, 1); // Grace
            pane.peopleFilter = "all";
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

        // -- T229a: per-app preferences and the Focus link ---------------------

        function test_app_detail_sheet_round_trips_allow_style_sound_badge() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            var mail = pane.apps[1];
            compare(mail.name, "Mail");

            pane.openAppDetail(mail);
            waitForRendering(stage);
            compare(pane.appDialog.open, true);
            compare(pane.appDialogTitle.text, "Mail");
            compare(pane.appAllowToggle.checked, true);

            pane.appAllowToggle.toggle();
            tryVerify(function() {
                var app = pane.appByName("Mail");
                return app && app.status === "Off";
            }, 2000, "muting Mail turns its row Off");

            pane.appStyleSelect.activateIndex(1); // Alerts
            tryVerify(function() {
                var app = pane.appByName("Mail");
                return app && app.style === "alert" && app.styleLabel === "Alerts";
            }, 2000, "the style write reaches the service view");

            pane.appSoundToggle.toggle();
            pane.appBadgeToggle.toggle();
            tryVerify(function() {
                var app = pane.appByName("Mail");
                return app && app.sound === false && app.badge === false;
            }, 2000, "the sound and badge writes reach the service view");

            // Reopening seeds the sheet from the authoritative view.
            pane.appDialog.reject();
            waitForRendering(stage);
            pane.openAppDetail(pane.appByName("Mail"));
            waitForRendering(stage);
            compare(pane.appAllowToggle.checked, false);
            compare(pane.appStyleSelect.currentIndex, 1);
            compare(pane.appSoundToggle.checked, false);
            compare(pane.appBadgeToggle.checked, false);
            pane.appDialog.reject();
        }

        function test_app_detail_seeds_from_the_service_state() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            Settings.setNotificationAppPreference("Pager", "none", false,
                                                  false, false);
            tryVerify(function() {
                var app = pane.appByName("Pager");
                return app && app.status === "Off" && app.style === "none";
            }, 2000);
            pane.openAppDetail(pane.appByName("Pager"));
            waitForRendering(stage);
            compare(pane.appStyleSelect.currentIndex, 2); // None
            compare(pane.appSoundToggle.checked, false);
            compare(pane.appBadgeToggle.checked, false);
            pane.appDialog.reject();
        }

        function test_app_detail_full_sheet_round_trips_the_four_options() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            pane.openAppDetail(pane.appByName("Mail"));
            waitForRendering(stage);

            compare(pane.appLockScreenToggle.checked, true);
            pane.appLockScreenToggle.toggle();
            tryVerify(function() {
                var options = pane.parseAppOptions();
                return options["mail"] && options["mail"].lockScreen === false;
            }, 2000, "the lock-screen choice is stored per app");

            pane.appCenterSelect.activateIndex(1); // 1
            pane.appPreviewsSelect.activateIndex(2); // Never
            pane.appGroupingSelect.activateIndex(1); // By App
            tryVerify(function() {
                var entry = pane.parseAppOptions()["mail"];
                return entry && entry.centerCount === "1"
                        && entry.previews === "never"
                        && entry.grouping === "by-app";
            }, 2000, "the remaining per-app options round trip");

            // Reopening seeds the sheet from the stored options.
            pane.appDialog.reject();
            waitForRendering(stage);
            pane.openAppDetail(pane.appByName("Mail"));
            waitForRendering(stage);
            compare(pane.appLockScreenToggle.checked, false);
            compare(pane.appCenterSelect.currentIndex, 1);
            compare(pane.appPreviewsSelect.currentIndex, 2);
            compare(pane.appGroupingSelect.currentIndex, 1);
            pane.appDialog.reject();
        }

        function test_app_row_opens_the_sheet_from_the_whole_row() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            var row = pane.appList.itemAt(1);
            mouseClick(row, row.width * 0.3, row.height / 2);
            waitForRendering(stage);
            compare(pane.appDialog.open, true);
            compare(pane.appDialogTitle.text, "Mail");
            pane.appDialog.reject();
        }

        function test_global_preferences_are_marked_stored_policy() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            verify(pane.storedPolicyNoteItem.visible);
            verify(pane.storedPolicyNoteItem.text.indexOf("stored") >= 0);
        }

        function test_focus_link_opens_the_focus_pane() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            verify(pane.focusRow.visible);
            pane.focusButton.clicked();
            tryCompare(shell, "currentPaneId", "focus");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            compare(paneOf(shell).mode, "focus");
        }

        function test_long_app_name_stays_inside_the_sheet() {
            var shell = make("notifications");
            var pane = paneOf(shell);
            var longName = "An application with an extremely long display name "
                         + "that should elide instead of widening the sheet";
            pane.openAppDetail({ name: longName, notificationsAllowed: true,
                                 style: "banner", sound: true, badge: true,
                                 notifications: 0, status: "Default" });
            waitForRendering(stage);
            compare(pane.appDialogTitle.text, longName);
            compare(pane.appDialogTitle.elide, Text.ElideRight);
            verify(pane.appDialog.card.width <= stage.width,
                   "the sheet card must stay inside the capture stage");
            pane.appDialog.reject();
        }

        // -- T229a: the Focus modes, people, schedule, and shortcut -----------

        function test_focus_profiles_select_through_the_one_list() {
            var shell = make("focus");
            var pane = paneOf(shell);
            // The fixture starts on the Personal profile.
            compare(pane.profile, "personal");
            verify(pane.profileSummary.text.indexOf("Personal") >= 0);

            pane.activateFocus("work");
            tryVerify(function() {
                return Settings.notifications.profile === "work";
            }, 2000, "selecting Work installs the named profile");
            compare(pane.mode, "focus");

            pane.activateFocus("sleep");
            tryVerify(function() {
                return Settings.notifications.profile === "sleep";
            }, 2000);
            compare(pane.mode, "dnd");

            pane.activateFocus("off");
            tryVerify(function() {
                return Settings.notifications.mode === "off";
            }, 2000);
            compare(pane.profile, "");
        }

        function test_allowed_people_list_round_trips_through_the_adapter() {
            var shell = make("focus");
            var pane = paneOf(shell);
            verify(pane.peopleGroup.visible);
            verify(pane.peopleList.count > 0);
            // Ada is on the fixture allowed-people list; Grace is not.
            compare(pane.peopleList.itemAt(0).modelData.name, "Ada");
            compare(pane.peopleList.itemAt(0).modelData.allowed, true);
            compare(pane.peopleList.itemAt(1).modelData.allowed, false);

            pane.peopleList.itemAt(1).personToggle.toggle();
            tryVerify(function() {
                return Settings.notifications.allowedPeople.indexOf("Grace") >= 0;
            }, 2000, "allowing Grace adds the person to the policy");
            pane.peopleList.itemAt(1).personToggle.toggle();
            tryVerify(function() {
                return Settings.notifications.allowedPeople.indexOf("Grace") === -1;
            }, 2000);
        }

        function test_schedule_validates_and_round_trips() {
            var shell = make("focus");
            var pane = paneOf(shell);
            Settings.set("focus.schedule.enabled", true);
            compare(pane.scheduleToggle.checked, true);

            // An invalid time reverts nothing and shows the reason.
            pane.scheduleStartDraft = "25:99";
            pane.scheduleApplyButton.clicked();
            verify(pane.scheduleErrorText.visible);
            compare(Settings.values["focus.schedule.start"], "22:00");

            // A valid time writes both keys and clears the error.
            pane.scheduleStartDraft = "21:30";
            pane.scheduleEndDraft = "06:15";
            pane.scheduleApplyButton.clicked();
            compare(Settings.values["focus.schedule.start"], "21:30");
            compare(Settings.values["focus.schedule.end"], "06:15");
            compare(pane.scheduleErrorText.visible, false);
            verify(pane.scheduleSummary.text.indexOf("21:30") >= 0);
        }

        function test_schedule_days_round_trip_as_a_list() {
            var shell = make("focus");
            var pane = paneOf(shell);
            Settings.set("focus.schedule.enabled", true);
            pane.dayRepeater.itemAt(6).clicked(); // Sunday off
            tryVerify(function() {
                return Settings.values["focus.schedule.days"].indexOf("sun") === -1;
            }, 2000);
            pane.dayRepeater.itemAt(6).clicked(); // back on
            tryVerify(function() {
                return Settings.values["focus.schedule.days"].indexOf("sun") >= 0;
            }, 2000);
        }

        function test_shortcut_validates_and_round_trips() {
            var shell = make("focus");
            var pane = paneOf(shell);
            pane.shortcutDraft = "F";
            pane.shortcutApplyButton.clicked();
            verify(pane.shortcutErrorText.visible);
            compare(Settings.values["focus.shortcut"], "");

            pane.shortcutDraft = "Ctrl+Alt+F";
            pane.shortcutApplyButton.clicked();
            compare(Settings.values["focus.shortcut"], "Ctrl+Alt+F");
            compare(pane.shortcutErrorText.visible, false);

            // Clearing disables the shortcut.
            pane.shortcutDraft = "";
            pane.shortcutApplyButton.clicked();
            compare(Settings.values["focus.shortcut"], "");
        }
    }
}