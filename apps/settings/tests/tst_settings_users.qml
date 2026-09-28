// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Users & Groups pane tests (T-15.11b). Headless with `DF_SETTINGS_FIXTURE`
// and `DF_ACCOUNTS_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton
// and the host-stack client are deterministic and in-process with no bus (and
// no AccountsService). The cases cover the user list, the per-user dialog's
// account-type/lock/auto-login round-trips, adding and deleting a user, the
// group list and membership writes, and the `Automatically log in as` popup.
Item {
    id: stage
    width: 1000
    height: 1600

    TestCase {
        id: testCase
        name: "SettingsUsers"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture is process-global; reset it before every case so each
        // starts on the known two-user / two-group workstation.
        function init() {
            Settings.resetAccountsFixture();
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("users-groups");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Users & Groups pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_user_list_and_auto_login() {
            var shell = make();
            compare(shell.currentPaneId, "users-groups");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.humanUsers.length, 2);
            compare(pane.autoLoginSelect.currentIndex, 1);
            compare(pane.automaticLoginUser, "dan");

            compare(findChild(pane.userRepeater.itemAt(0), "userNameText").text,
                    "Dan Doe");
            compare(findChild(pane.userRepeater.itemAt(0), "userRoleText").text,
                    "Admin");
            compare(findChild(pane.userRepeater.itemAt(1), "userNameText").text,
                    "Sam Smith");
            compare(findChild(pane.userRepeater.itemAt(1), "userRoleText").text,
                    "Standard \u00b7 Locked");

            // System/daemon accounts are not listed.
            for (var i = 0; i < pane.humanUsers.length; ++i)
                compare(pane.humanUsers[i].system, false);

            // The group list shows both groups (user group first).
            compare(pane.groups.length, 2);
            compare(findChild(pane.groupRepeater.itemAt(0), "groupNameText").text,
                    "wheel");
            compare(findChild(pane.groupRepeater.itemAt(0), "groupMembersText").text,
                    "1 member");
        }

        function test_user_dialog_account_type_and_lock_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            findChild(pane.userRepeater.itemAt(0), "userInfoButton").clicked();
            compare(pane.userDialog.open, true);
            compare(pane.selectedUser.userName, "dan");

            pane.userTypeSelect.activateIndex(0); // Standard
            tryVerify(function() { return pane.selectedUser.accountType === "standard"; });

            pane.userLockedToggle.toggle();
            tryVerify(function() { return pane.selectedUser.locked === true; });

            // The pane row converges with the view.
            compare(findChild(pane.userRepeater.itemAt(0), "userRoleText").text,
                    "Standard \u00b7 Locked");
        }

        function test_user_dialog_automatic_login_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            // Sam is not the automatic login user; open Sam.
            findChild(pane.userRepeater.itemAt(1), "userInfoButton").clicked();
            compare(pane.selectedUser.userName, "sam");
            compare(pane.userAutoLoginToggle.checked, false);

            pane.userAutoLoginToggle.toggle();
            tryVerify(function() { return pane.selectedUser.automaticLogin === true; });
            compare(pane.automaticLoginUser, "sam");

            // AccountsService keeps one automatic login user: Dan is cleared.
            compare(pane.findUser(1000).automaticLogin, false);
        }

        function test_add_user_creates_a_live_row() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addUserButton.clicked();
            compare(pane.addUserDialog.open, true);
            pane.addUserNameInput.text = "kim";
            pane.addRealNameInput.text = "Kim Lee";
            pane.addUserTypeSelect.activateIndex(1); // Administrator
            pane.addUserCreateButton.clicked();

            tryVerify(function() { return pane.humanUsers.length === 3; });
            var kim = pane.findUser(1002);
            verify(kim !== null, "the new user is in the live view");
            compare(kim.displayName, "Kim Lee");
            compare(kim.accountType, "administrator");
        }

        function test_delete_user_removes_the_row() {
            var shell = make();
            var pane = paneOf(shell);
            findChild(pane.userRepeater.itemAt(1), "userInfoButton").clicked();
            compare(pane.selectedUser.userName, "sam");
            pane.userDeleteButton.clicked();
            compare(pane.deleteUserDialog.open, true);
            pane.deleteUserConfirm.clicked();

            tryVerify(function() { return pane.humanUsers.length === 1; });
            compare(pane.findUser(1001), null);
        }

        function test_add_group_creates_a_live_group() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addGroupButton.clicked();
            compare(pane.addGroupDialog.open, true);
            pane.addGroupNameInput.text = "devs";
            pane.addGroupCreateButton.clicked();

            tryVerify(function() { return pane.groups.length === 3; });
            verify(pane.findGroup("devs") !== null, "the new group is live");
        }

        function test_group_membership_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.findGroup("wheel").memberCount, 1);
            findChild(pane.groupRepeater.itemAt(0), "groupInfoButton").clicked();
            compare(pane.groupDialog.open, true);
            compare(pane.selectedGroup.name, "wheel");

            // Add Sam (the second human user) to `wheel`.
            var toggle = findChild(pane.groupMemberRepeater.itemAt(1),
                                   "groupMemberToggle");
            verify(toggle !== null, "the membership toggle exists");
            toggle.toggle();
            tryVerify(function() { return pane.selectedGroup.memberCount === 2; });
            verify(pane.findGroup("wheel").members.indexOf("sam") >= 0);

            // Remove Dan again.
            findChild(pane.groupMemberRepeater.itemAt(0),
                      "groupMemberToggle").toggle();
            tryVerify(function() { return pane.findGroup("wheel").memberCount === 1; });
        }

        function test_auto_login_popup_switches_and_clears() {
            var shell = make();
            var pane = paneOf(shell);
            // Off (index 0) clears the automatic login user.
            pane.autoLoginSelect.activateIndex(0);
            tryVerify(function() { return pane.automaticLoginUser === ""; });

            // Selecting Sam (index 2) sets him.
            pane.autoLoginSelect.activateIndex(2);
            tryVerify(function() { return pane.automaticLoginUser === "sam"; });
            compare(pane.findUser(1001).automaticLogin, true);
            compare(pane.findUser(1000).automaticLogin, false);
        }

        function test_absence_note_replaces_the_lists_without_a_host() {
            // The absence case itself lives in tst_settings_absence.qml (no
            // fixture, no bus); this asserts the gate is wired so a ready view
            // never shows the note.
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.absenceNote.visible, false);
            compare(pane.groupsAbsentNote.visible, false);
        }
    }
}