// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Settings common apply path UI tests (T224).
//
// Headless with `DF_SETTINGS_FIXTURE` and `DF_ACCOUNTS_FIXTURE`: the one write
// path validates and reverts in-process, the five absence kinds render their
// policy, and a privileged account write goes through the authorizer seam
// (cancel / wrong password / deny / allow) with no password field in the app.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsApplyUI"
        when: windowShown

        Component { id: dockPane; DesktopDockPane { } }
        Component { id: appearancePane; AppearancePane { } }
        Component { id: inputPane; InputPane { } }
        Component { id: usersPane; UsersGroupsPane { } }
        Component { id: menuBarPane; MenuBarPane { } }

        // A five-kind absence harness: one notice and one control per kind.
        Component {
            id: absenceHarness
            Item {
                width: 640
                height: 480
                property alias liveNotice: liveNotice
                property alias disabledNotice: disabledNotice
                property alias hiddenNotice: hiddenNotice
                property alias readOnlyNotice: readOnlyNotice
                property alias delegatedNotice: delegatedNotice
                property alias liveToggle: liveToggle
                property alias disabledToggle: disabledToggle
                property alias hiddenToggle: hiddenToggle
                property alias readOnlyToggle: readOnlyToggle
                property alias delegatedControl: delegatedControl
                property alias delegateButton: delegateButton

                Column {
                    width: parent.width
                    spacing: 8

                    SettingsAbsenceNotice {
                        id: liveNotice
                        kind: "live-on-defaults"
                        showWhenLive: true
                        reason: "running on schema defaults"
                    }
                    Toggle {
                        id: liveToggle
                        text: "live"
                        visible: liveNotice.controlsVisible
                        enabled: liveNotice.controlsEnabled
                    }

                    SettingsAbsenceNotice {
                        id: disabledNotice
                        kind: "disabled-with-reason"
                        reason: "the owner is not running"
                    }
                    Toggle {
                        id: disabledToggle
                        text: "disabled"
                        visible: disabledNotice.controlsVisible
                        enabled: disabledNotice.controlsEnabled
                    }

                    SettingsAbsenceNotice {
                        id: hiddenNotice
                        kind: "hidden"
                        reason: "this group is not shown"
                    }
                    Toggle {
                        id: hiddenToggle
                        text: "hidden"
                        visible: hiddenNotice.controlsVisible
                    }

                    SettingsAbsenceNotice {
                        id: readOnlyNotice
                        kind: "read-only"
                        reason: "reads only while the owner is absent"
                    }
                    Toggle {
                        id: readOnlyToggle
                        text: "read only"
                        visible: readOnlyNotice.controlsVisible
                        enabled: readOnlyNotice.controlsEnabled
                    }

                    SettingsAbsenceNotice {
                        id: delegatedNotice
                        kind: "delegated"
                        reason: "managed by another service"
                    }
                    Toggle {
                        id: delegatedControl
                        text: "delegated"
                        visible: delegatedNotice.controlsVisible
                    }
                    Button {
                        id: delegateButton
                        text: "Open\u2026"
                        visible: delegatedNotice.delegated
                    }
                }
            }
        }

        function make(component, width) {
            var item = createTemporaryObject(component, stage,
                                             { width: width || stage.width });
            waitForRendering(stage);
            return item;
        }

        function findRow(item, key) {
            if (!item)
                return null;
            if (item.settingsKey !== undefined && item.settingsKey === key
                    && item.errorText !== undefined)
                return item;
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i) {
                var found = findRow(kids[i], key);
                if (found)
                    return found;
            }
            return null;
        }

        function countKeyRows(item) {
            if (!item)
                return 0;
            var count = (item.settingsKey !== undefined
                         && String(item.settingsKey).length > 0) ? 1 : 0;
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i)
                count += countKeyRows(kids[i]);
            return count;
        }

        function collectKeyRows(item, out) {
            if (!item)
                return out;
            // Only rows on the common write path are collected. A disabled
            // row names the task that owns its runtime and never writes a
            // key, so it is not a fake control (T227).
            if (item.settingsKey !== undefined
                    && String(item.settingsKey).length > 0
                    && item.errorText !== undefined
                    && item.enabled !== false)
                out.push(item);
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i)
                collectKeyRows(kids[i], out);
            return out;
        }

        function hasPasswordField(item) {
            if (!item)
                return false;
            if (item.echoMode !== undefined && item.echoMode === 2)
                return true; // TextInput.Password
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i)
                if (hasPasswordField(kids[i]))
                    return true;
            return false;
        }

        function userByUid(uid) {
            var users = Settings.accounts.users || [];
            for (var i = 0; i < users.length; ++i)
                if (Number(users[i].uid) === Number(uid))
                    return users[i];
            return null;
        }

        function init() {
            Settings.setAuthFixture("allow");
            Settings.setAvailableFixture(true);
            Settings.resetAccountsFixture();
            // Clear any write state left by the previous case.
            Settings.apply("dock.size", 0.5);
            Settings.apply("dock.autohide", false);
        }

        // -- Validation and the inline error surface ---------------------------

        function test_validation_rejects_ranges_and_enums_next_to_the_control() {
            var pane = make(dockPane, 720);
            var row = findRow(pane, "dock.size");
            verify(row, "the Dock size row must be a SettingsKeyRow");

            compare(Settings.apply("dock.size", 9.0), false);
            compare(Settings.values["dock.size"], 0.5);
            verify(Settings.errorFor("dock.size").indexOf("outside") >= 0);
            compare(row.errorText, Settings.errorFor("dock.size"));
            verify(row.errorLabel.visible, "the row must show the validation message");

            compare(Settings.apply("appearance.colorScheme", "purple"), false);
            verify(Settings.errorFor("appearance.colorScheme").indexOf("not one of") >= 0);
            compare(Settings.values["appearance.colorScheme"], "auto");

            // A valid write clears the message and updates the value.
            compare(Settings.apply("dock.size", 0.8), true);
            compare(Settings.values["dock.size"], 0.8);
            compare(Settings.errorFor("dock.size"), "");
            verify(!row.errorLabel.visible);
        }

        function test_failed_owner_write_is_optimistic_then_reverts_with_message() {
            var pane = make(dockPane, 720);
            var row = findRow(pane, "dock.autohide");
            verify(row);
            compare(Settings.values["dock.autohide"], false);

            Settings.failNextWrite("dock.autohide",
                                   "The settings service rejected the change.");
            compare(Settings.apply("dock.autohide", true), true);
            compare(Settings.values["dock.autohide"], true,
                    "the control is optimistic before the owner answers");

            tryVerify(function() {
                return Settings.values["dock.autohide"] === false;
            });
            compare(Settings.stateFor("dock.autohide"), "error");
            compare(Settings.errorFor("dock.autohide"),
                    "The settings service rejected the change.");
            compare(row.errorText, Settings.errorFor("dock.autohide"));
            verify(row.errorLabel.visible);
        }

        function test_three_panes_use_the_common_apply_row() {
            var dock = make(dockPane, 720);
            var appearance = make(appearancePane, 720);
            var input = make(inputPane, 720);
            verify(countKeyRows(dock) >= 10);
            verify(countKeyRows(appearance) >= 3);
            verify(countKeyRows(input) >= 10);

            // Every common row writes a key the schema owns and has a real
            // control in its slot; there is no display-only fake row.
            var rows = [];
            collectKeyRows(dock, rows);
            collectKeyRows(appearance, rows);
            collectKeyRows(input, rows);
            verify(rows.length >= 23);
            for (var i = 0; i < rows.length; ++i) {
                var row = rows[i];
                verify(Settings.schemaFor(row.settingsKey).key === row.settingsKey,
                       row.settingsKey + " must be a schema key");
                compare(Settings.validationError(row.settingsKey,
                                                 Settings.values[row.settingsKey]),
                        "");
                verify(row.control.children.length > 0,
                       row.settingsKey + " must own a real control");
            }
        }

        // -- The five absence kinds --------------------------------------------

        function test_all_five_absence_kinds_render_their_policy() {
            compare(SettingsAbsence.kinds.length, 5);
            for (var i = 0; i < SettingsAbsence.kinds.length; ++i)
                verify(SettingsAbsence.valid(SettingsAbsence.kinds[i]));

            var harness = make(absenceHarness, 640);
            waitForRendering(harness);

            // live-on-defaults: controls stay live, note may explain defaults.
            verify(harness.liveNotice.controlsVisible);
            verify(harness.liveNotice.controlsEnabled);
            verify(harness.liveNotice.visible);
            verify(harness.liveToggle.visible);
            verify(harness.liveToggle.enabled);

            // disabled-with-reason: visible but inert, with the reason.
            verify(harness.disabledNotice.controlsVisible);
            verify(!harness.disabledNotice.controlsEnabled);
            verify(harness.disabledNotice.visible);
            verify(harness.disabledNotice.text.indexOf("owner") >= 0);
            verify(harness.disabledToggle.visible);
            verify(!harness.disabledToggle.enabled,
                   "a disabled-with-reason control is disabled, not a fake toggle");

            // hidden: neither the controls nor a note.
            verify(!harness.hiddenNotice.controlsVisible);
            verify(!harness.hiddenNotice.visible);
            verify(!harness.hiddenToggle.visible);

            // read-only: reads visible, writes disabled, with the reason.
            verify(harness.readOnlyNotice.controlsVisible);
            verify(!harness.readOnlyNotice.controlsEnabled);
            verify(harness.readOnlyNotice.visible);
            verify(harness.readOnlyToggle.visible);
            verify(!harness.readOnlyToggle.enabled);

            // delegated: no fake control, an explicit hand-off.
            verify(!harness.delegatedNotice.controlsVisible);
            verify(harness.delegatedNotice.delegated);
            verify(harness.delegatedNotice.visible);
            verify(!harness.delegatedControl.visible);
            verify(harness.delegateButton.visible);
        }

        function test_every_shipped_pane_and_row_declares_a_valid_absence_kind() {
            for (var i = 0; i < SettingsPanes.shippedPanes.length; ++i) {
                var pane = SettingsPanes.shippedPanes[i];
                verify(SettingsAbsence.valid(String(pane.absence)),
                       pane.id + " has an unknown absence kind");
                var rows = SettingsMap.visibleRowsFor(pane.id);
                for (var r = 0; r < rows.length; ++r)
                    verify(SettingsAbsence.valid(String(rows[r].absence)),
                           pane.id + "/" + rows[r].id + " has an unknown absence kind");
            }
        }

        // The five kinds are not just a harness: the executable map actually
        // assigns every one of them to a pane or row.
        function test_the_map_uses_all_five_absence_kinds() {
            var used = {};
            for (var i = 0; i < SettingsMap.panes.length; ++i) {
                var pane = SettingsMap.panes[i];
                used[String(pane.absence)] = true;
                var rows = SettingsMap.rowsFor(pane.id);
                for (var r = 0; r < rows.length; ++r)
                    used[String(rows[r].absence)] = true;
            }
            var kinds = ["live-on-defaults", "disabled-with-reason",
                         "hidden", "read-only", "delegated"];
            for (var k = 0; k < kinds.length; ++k)
                verify(used[kinds[k]],
                       "the map must use the " + kinds[k] + " absence kind");
        }

        // A service that disappears while the pane is open flips to its
        // absence state without losing the settingsd-backed controls, and the
        // return resync clears the deferred write.
        function test_owner_disappearance_flips_absence_without_losing_controls() {
            compare(Settings.available, true);
            var pane = make(menuBarPane, 720);
            verify(!pane.absenceNote.visible);
            var before = Settings.values["menu.showBackground"];

            Settings.setAvailableFixture(false);
            tryVerify(function() { return Settings.available === false; });
            waitForRendering(stage);
            verify(pane.absenceNote.visible);
            verify(pane.absenceNote.text.length > 0);
            verify(pane.backgroundToggle.enabled,
                   "live-on-defaults controls stay usable while the owner is away");
            compare(Settings.values["menu.showBackground"], before);

            pane.backgroundToggle.toggle();
            tryVerify(function() {
                return Settings.stateFor("menu.showBackground") === "deferred";
            });

            Settings.setAvailableFixture(true);
            tryVerify(function() { return Settings.available === true; });
            tryVerify(function() {
                return Settings.stateFor("menu.showBackground") === "idle";
            });
            verify(!pane.absenceNote.visible);
        }

        // -- Privileged actions: the system prompt, never an app password ------

        function test_privileged_account_action_cancel_deny_wrong_password_allow() {
            var pane = make(usersPane, 720);
            tryVerify(function() { return pane.ready === true; });
            compare(userByUid(1000).accountType, "administrator");

            Settings.setAuthFixture("cancel");
            Settings.setAccountType(1000, "standard");
            tryVerify(function() {
                return Settings.accountsError.indexOf("cancelled") >= 0;
            });
            compare(userByUid(1000).accountType, "administrator",
                    "a cancelled prompt leaves the account unchanged");
            verify(pane.authErrorNote.visible);
            verify(pane.authErrorNote.text.indexOf("cancelled") >= 0);

            Settings.setAuthFixture("wrong-password");
            Settings.setAccountType(1000, "standard");
            tryVerify(function() {
                return Settings.accountsError.indexOf("incorrect") >= 0;
            });
            compare(userByUid(1000).accountType, "administrator");
            verify(pane.authErrorNote.text.indexOf("incorrect") >= 0);

            Settings.setAuthFixture("deny");
            Settings.setAccountType(1000, "standard");
            tryVerify(function() {
                return Settings.accountsError.indexOf("denied") >= 0;
            });
            compare(userByUid(1000).accountType, "administrator");

            Settings.setAuthFixture("allow");
            Settings.setAccountType(1000, "standard");
            tryVerify(function() {
                return userByUid(1000).accountType === "standard";
            });
            compare(Settings.accountsError, "");

            // The app never renders its own password field for an admin action.
            verify(!hasPasswordField(pane),
                   "privileged actions must use the system prompt, not an app password field");
        }
    }
}
