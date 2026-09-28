// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Users & Groups pane (T-15.11b).
//
// The pane and the Control Center tile are one functional unit: both read the
// same host-stack adapter through the bridge host. The pane mirrors the macOS
// pane (SystemSettings_UsesGroups.md / System_Preferences.md) as a user list
// with info buttons, the `Add Group...` / `Add User...` buttons, an
// `Automatically log in as` popup, and the group list. Every control applies
// live through `Settings`, so there is no Apply button.
//
// Deviations from the macOS capture reference (ADR 0122), recorded so later
// tasks do not re-litigate them:
//   * the Apple Account/iCloud sign-in row and the sidebar `iCloud` /
//     `Internet Accounts` rows are dropped (Apple-only).
//   * the FileVault helper text on the auto-login row is omitted: the adapter
//     has no disk-encryption provider to read, so there is no honest state to
//     show. The popup is fully functional.
//   * `Network account server` is omitted: there is no enterprise directory
//     provider to join, and a disabled `Edit...` would be a dead control.
//   * the trailing `?` help is omitted project-wide.
//   * system/daemon accounts are not listed (the macOS capture shows only the
//     human account); they remain in the adapter snapshot for completeness.
Item {
    id: root

    // The bridge host's Users and Groups view (empty when absent).
    readonly property var view: Settings.accounts
    readonly property bool available: Settings.accountsAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property bool groupsAvailable: root.view.groupsAvailable === true
    readonly property var users: root.view.users !== undefined ? root.view.users : []
    readonly property var groups: root.view.groups !== undefined ? root.view.groups : []
    // The human users, in the adapter's order (human first, by uid).
    readonly property var humanUsers: {
        var out = [];
        for (var i = 0; i < root.users.length; ++i)
            if (root.users[i].system !== true)
                out.push(root.users[i]);
        return out;
    }
    readonly property string automaticLoginUser: root.view.automaticLoginUser !== undefined
        ? String(root.view.automaticLoginUser) : ""
    readonly property int automaticLoginUid: root.view.automaticLoginUid !== undefined
        ? Number(root.view.automaticLoginUid) : 0

    // The user/group the info dialog edits. Keyed by uid/name and derived from
    // the live view, so a write's round-trip is reflected in the open dialog
    // without copying a stale entry.
    property int selectedUid: -1
    property string selectedGroupName: ""
    readonly property var selectedUser: root.selectedUid >= 0
        ? root.findUser(root.selectedUid) : null
    readonly property var selectedGroup: root.selectedGroupName.length > 0
        ? root.findGroup(root.selectedGroupName) : null

    // Test surface (used by tst_settings_users.qml).
    property alias userRepeater: userRepeater
    property alias groupRepeater: groupRepeater
    property alias addUserButton: addUserButton
    property alias addGroupButton: addGroupButton
    property alias autoLoginSelect: autoLoginSelect
    property alias userDialog: userDialog
    property alias userTypeSelect: userTypeSelect
    property alias userLockedToggle: userLockedToggle
    property alias userAutoLoginToggle: userAutoLoginToggle
    property alias userDeleteButton: userDeleteButton
    property alias deleteUserDialog: deleteUserDialog
    property alias deleteUserConfirm: deleteUserConfirm
    property alias addUserDialog: addUserDialog
    property alias addUserNameInput: addUserNameInput
    property alias addRealNameInput: addRealNameInput
    property alias addUserTypeSelect: addUserTypeSelect
    property alias addUserCreateButton: addUserCreateButton
    property alias groupDialog: groupDialog
    property alias groupMemberRepeater: groupMemberRepeater
    property alias groupDeleteButton: groupDeleteButton
    property alias addGroupDialog: addGroupDialog
    property alias addGroupNameInput: addGroupNameInput
    property alias addGroupCreateButton: addGroupCreateButton
    property alias groupsAbsentNote: groupsAbsentNote
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    // The account-type choices, shared by the user dialog and the add dialog.
    readonly property var accountTypeModel: [
        { label: qsTr("Standard"), value: "standard" },
        { label: qsTr("Administrator"), value: "administrator" }
    ]

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshAccounts()

    function findUser(uid) {
        for (var i = 0; i < root.users.length; ++i)
            if (Number(root.users[i].uid) === Number(uid))
                return root.users[i];
        return null;
    }

    function findGroup(name) {
        for (var i = 0; i < root.groups.length; ++i)
            if (root.groups[i].name === name)
                return root.groups[i];
        return null;
    }

    function accountTypeIndex(id) {
        return id === "administrator" ? 1 : 0;
    }

    // The current auto-login popup selection: `0` is `Off`, else the uid.
    function autoLoginIndex() {
        if (root.automaticLoginUser.length === 0)
            return 0;
        for (var i = 0; i < root.humanUsers.length; ++i)
            if (root.humanUsers[i].userName === root.automaticLoginUser)
                return i + 1;
        return 0;
    }

    // Toggle one member of a group and write the whole membership list (the
    // adapter's write is a whole-list replace).
    function toggleGroupMember(groupName, member, allowed) {
        var group = root.findGroup(groupName);
        if (group === null)
            return;
        var members = group.members !== undefined ? group.members.slice() : [];
        var at = members.indexOf(member);
        if (allowed && at < 0)
            members.push(member);
        else if (!allowed && at >= 0)
            members.splice(at, 1);
        else
            return;
        Settings.setAccountGroupMembers(groupName, members);
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The user list ─────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.ready
            reserveBottomMargin: false

            Repeater {
                id: userRepeater
                model: root.humanUsers
                delegate: Item {
                    id: userRow
                    required property var modelData
                    required property int index
                    readonly property var user: modelData
                    width: parent ? parent.width : 0
                    height: Math.max(Theme.controls.settingsRow.height, 40)
                    Accessible.role: Accessible.Grouping
                    Accessible.name: userRow.user.displayName

                    Rectangle {
                        id: avatar
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        width: 32
                        height: 32
                        radius: 16
                        color: Theme.color.accentMuted

                        Text {
                            objectName: "userAvatarInitial"
                            anchors.centerIn: parent
                            text: userRow.user.initial !== undefined
                                ? String(userRow.user.initial) : "?"
                            color: Theme.color.accent
                            font.pixelSize: Theme.primitive.font.sizeMd
                            font.weight: Theme.primitive.font.weightSemibold
                        }
                    }

                    Column {
                        anchors.left: avatar.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: userInfo.left
                        anchors.rightMargin: Theme.controls.settingsRow.controlGap
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            objectName: "userNameText"
                            width: parent.width
                            text: userRow.user.displayName
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }

                        Text {
                            objectName: "userRoleText"
                            width: parent.width
                            text: {
                                var role = userRow.user.accountTypeLabel !== undefined
                                    ? String(userRow.user.accountTypeLabel) : "";
                                if (userRow.user.locked === true)
                                    return role.length > 0
                                        ? qsTr("%1 \u00b7 Locked").arg(role) : qsTr("Locked");
                                return role;
                            }
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Button {
                        id: userInfo
                        objectName: "userInfoButton"
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        icon: "info"
                        variant: "ghost"
                        accessibleName: qsTr("Edit %1").arg(userRow.user.displayName)
                        onClicked: {
                            root.selectedUid = Number(userRow.user.uid);
                            root.userDialog.show();
                        }
                    }

                    Rectangle {
                        visible: userRow.index < root.humanUsers.length - 1
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.bottom: parent.bottom
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                    }
                }
            }
        }

        // ── Add User / Add Group ──────────────────────────────────────────
        Row {
            width: parent.width
            visible: root.ready
            spacing: Theme.primitive.spacing.md

            Button {
                id: addUserButton
                objectName: "addUserButton"
                text: qsTr("Add User\u2026")
                onClicked: root.addUserDialog.show()
            }

            Button {
                id: addGroupButton
                objectName: "addGroupButton"
                text: qsTr("Add Group\u2026")
                enabled: root.groupsAvailable
                onClicked: root.addGroupDialog.show()
            }
        }

        // ── Automatically log in as ───────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.ready

            SettingsRow {
                objectName: "autoLoginRow"
                width: parent.width
                label: qsTr("Automatically log in as")
                description: qsTr("The account that logs in without a prompt.")
                showSeparator: false
                controlData: Select {
                    id: autoLoginSelect
                    objectName: "autoLoginSelect"
                    accessibleName: qsTr("Automatically log in as")
                    enabled: root.humanUsers.length > 0
                    model: {
                        var entries = [{ label: qsTr("Off"), value: 0 }];
                        for (var i = 0; i < root.humanUsers.length; ++i)
                            entries.push({
                                label: root.humanUsers[i].displayName,
                                value: root.humanUsers[i].uid
                            });
                        return entries;
                    }
                    onSelected: (value) => {
                        if (Number(value) === 0) {
                            if (root.automaticLoginUid > 0)
                                Settings.setAccountAutomaticLogin(
                                    root.automaticLoginUid, false);
                        } else {
                            Settings.setAccountAutomaticLogin(Number(value), true);
                        }
                    }
                }
            }

            // The popup reflects the live auto-login state; the Select writes
            // its own currentIndex on interaction, so bind it explicitly.
            Binding {
                target: autoLoginSelect
                property: "currentIndex"
                value: root.autoLoginIndex()
            }
        }

        // ── The group list ────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.ready && root.groupsAvailable
            title: qsTr("Groups")
            reserveBottomMargin: false

            Repeater {
                id: groupRepeater
                model: root.groups
                delegate: Item {
                    id: groupRow
                    required property var modelData
                    required property int index
                    readonly property var group: modelData
                    width: parent ? parent.width : 0
                    height: Math.max(Theme.controls.settingsRow.height, 40)
                    Accessible.role: Accessible.Grouping
                    Accessible.name: groupRow.group.name

                    Column {
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: groupInfo.left
                        anchors.rightMargin: Theme.controls.settingsRow.controlGap
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            objectName: "groupNameText"
                            width: parent.width
                            text: groupRow.group.name
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }

                        Text {
                            objectName: "groupMembersText"
                            width: parent.width
                            text: groupRow.group.memberCount === 1
                                ? qsTr("1 member")
                                : qsTr("%1 members").arg(groupRow.group.memberCount)
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Button {
                        id: groupInfo
                        objectName: "groupInfoButton"
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        icon: "info"
                        variant: "ghost"
                        accessibleName: qsTr("Edit %1").arg(groupRow.group.name)
                        onClicked: {
                            root.selectedGroupName = String(groupRow.group.name);
                            root.groupDialog.show();
                        }
                    }

                    Rectangle {
                        visible: groupRow.index < root.groups.length - 1
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.bottom: parent.bottom
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                    }
                }
            }
        }

        // ── Group provider absent ─────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.ready && !root.groupsAvailable

            Text {
                id: groupsAbsentNote
                objectName: "usersGroupsAbsentNote"
                width: parent.width
                text: qsTr("Group management is unavailable because no group "
                           + "provider is running. User accounts stay live.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Host absent ───────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            Text {
                id: absenceNote
                objectName: "usersGroupsAbsenceNote"
                width: parent.width
                text: qsTr("The account service is not running, so users and "
                           + "groups are unavailable.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }

    // ── The per-user dialog ───────────────────────────────────────────────
    Dialog {
        id: userDialog
        objectName: "userDialog"
        title: root.selectedUser !== null
            ? String(root.selectedUser.displayName) : qsTr("User")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            SettingsGroup {
                width: parent.width
                reserveBottomMargin: false

                SettingsRow {
                    objectName: "userNameRow"
                    width: parent.width
                    label: qsTr("User Name")
                    controlData: Text {
                        objectName: "userNameValue"
                        text: root.selectedUser !== null
                            ? String(root.selectedUser.userName) : ""
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }

                SettingsRow {
                    objectName: "userTypeRow"
                    width: parent.width
                    label: qsTr("Account Type")
                    controlData: Select {
                        id: userTypeSelect
                        objectName: "userTypeSelect"
                        accessibleName: qsTr("Account Type")
                        model: root.accountTypeModel
                        onSelected: (value) => {
                            if (root.selectedUser !== null)
                                Settings.setAccountType(Number(root.selectedUser.uid), value);
                        }
                    }
                }

                SettingsRow {
                    objectName: "userLockedRow"
                    width: parent.width
                    label: qsTr("Disabled")
                    description: qsTr("A disabled account cannot log in.")
                    controlData: Toggle {
                        id: userLockedToggle
                        objectName: "userLockedToggle"
                        accessibleName: qsTr("Disabled")
                        onToggled: (checked) => {
                            if (root.selectedUser !== null)
                                Settings.setAccountLocked(Number(root.selectedUser.uid),
                                                          checked);
                        }
                    }
                }

                SettingsRow {
                    objectName: "userAutoLoginRow"
                    width: parent.width
                    label: qsTr("Automatically Log In")
                    showSeparator: false
                    controlData: Toggle {
                        id: userAutoLoginToggle
                        objectName: "userAutoLoginToggle"
                        accessibleName: qsTr("Automatically Log In")
                        onToggled: (checked) => {
                            if (root.selectedUser !== null)
                                Settings.setAccountAutomaticLogin(
                                    Number(root.selectedUser.uid), checked);
                        }
                    }
                }
            }

            Binding {
                target: userTypeSelect
                property: "currentIndex"
                value: root.selectedUser !== null
                    ? root.accountTypeIndex(String(root.selectedUser.accountType)) : 0
            }
            Binding {
                target: userLockedToggle
                property: "checked"
                value: root.selectedUser !== null && root.selectedUser.locked === true
            }
            Binding {
                target: userAutoLoginToggle
                property: "checked"
                value: root.selectedUser !== null
                    && root.selectedUser.automaticLogin === true
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                id: userDeleteButton
                objectName: "userDeleteButton"
                text: qsTr("Delete User\u2026")
                enabled: root.selectedUser !== null
                    && root.selectedUser.system !== true
                onClicked: root.deleteUserDialog.show()
            }

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: userDialog.accept()
            }
        }
    }

    // ── The delete-user confirmation ──────────────────────────────────────
    Dialog {
        id: deleteUserDialog
        objectName: "deleteUserDialog"
        title: qsTr("Delete User")
        message: root.selectedUser !== null
            ? qsTr("\u201c%1\u201d will be removed. This cannot be undone.")
                .arg(String(root.selectedUser.userName))
            : ""
        dismissible: true

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: deleteUserDialog.reject()
            }
            Button {
                id: deleteUserConfirm
                objectName: "deleteUserConfirm"
                text: qsTr("Delete")
                variant: "primary"
                onClicked: {
                    if (root.selectedUser !== null)
                        Settings.deleteAccount(Number(root.selectedUser.uid));
                    deleteUserDialog.accept();
                    userDialog.accept();
                }
            }
        }
    }

    // ── The add-user dialog ───────────────────────────────────────────────
    Dialog {
        id: addUserDialog
        objectName: "addUserDialog"
        title: qsTr("New User")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            AccountField {
                id: addUserNameInput
                objectName: "addUserNameInput"
                placeholder: qsTr("User Name")
            }

            AccountField {
                id: addRealNameInput
                objectName: "addRealNameInput"
                placeholder: qsTr("Full Name")
            }

            SettingsGroup {
                width: parent.width
                reserveBottomMargin: false

                SettingsRow {
                    objectName: "addUserTypeRow"
                    width: parent.width
                    label: qsTr("Account Type")
                    showSeparator: false
                    controlData: Select {
                        id: addUserTypeSelect
                        objectName: "addUserTypeSelect"
                        accessibleName: qsTr("Account Type")
                        model: root.accountTypeModel
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: addUserDialog.reject()
            }
            Button {
                id: addUserCreateButton
                objectName: "addUserCreateButton"
                text: qsTr("Create User")
                variant: "primary"
                enabled: addUserNameInput.text.trim().length > 0
                onClicked: {
                    Settings.createAccount(addUserNameInput.text.trim(),
                                           addRealNameInput.text.trim(),
                                           addUserTypeSelect.currentValue !== undefined
                                               ? String(addUserTypeSelect.currentValue)
                                               : "standard");
                    addUserDialog.accept();
                }
            }
        }

        onOpened: {
            addUserNameInput.text = "";
            addRealNameInput.text = "";
            addUserTypeSelect.currentIndex = 0;
            addUserNameInput.input.forceActiveFocus();
        }
    }

    // ── The per-group dialog ──────────────────────────────────────────────
    Dialog {
        id: groupDialog
        objectName: "groupDialog"
        title: root.selectedGroup !== null
            ? String(root.selectedGroup.name) : qsTr("Group")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                objectName: "groupDialogHint"
                width: parent.width
                text: qsTr("Choose the users that belong to this group.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsGroup {
                width: parent.width
                reserveBottomMargin: false

                Repeater {
                    id: groupMemberRepeater
                    model: root.humanUsers
                    delegate: SettingsRow {
                        id: memberRow
                        required property var modelData
                        required property int index
                        width: parent.width
                        label: memberRow.modelData.displayName
                        showSeparator: memberRow.index < root.humanUsers.length - 1
                        controlData: Toggle {
                            id: memberToggle
                            objectName: "groupMemberToggle"
                            accessibleName: memberRow.modelData.displayName
                            onToggled: (checked) => {
                                if (root.selectedGroup !== null)
                                    root.toggleGroupMember(String(root.selectedGroup.name),
                                                           String(memberRow.modelData.userName),
                                                           checked);
                            }

                            Binding {
                                target: memberToggle
                                property: "checked"
                                value: root.selectedGroup !== null
                                    && root.selectedGroup.members !== undefined
                                    && root.selectedGroup.members.indexOf(
                                           memberRow.modelData.userName) >= 0
                            }
                        }
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                id: groupDeleteButton
                objectName: "groupDeleteButton"
                text: qsTr("Delete Group")
                enabled: root.selectedGroup !== null
                    && root.selectedGroup.system !== true
                onClicked: {
                    if (root.selectedGroup !== null)
                        Settings.deleteAccountGroup(String(root.selectedGroup.name));
                    groupDialog.accept();
                }
            }

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: groupDialog.accept()
            }
        }
    }

    // ── The add-group dialog ──────────────────────────────────────────────
    Dialog {
        id: addGroupDialog
        objectName: "addGroupDialog"
        title: qsTr("New Group")
        dismissible: true

        contentData: AccountField {
            id: addGroupNameInput
            objectName: "addGroupNameInput"
            placeholder: qsTr("Group Name")
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: addGroupDialog.reject()
            }
            Button {
                id: addGroupCreateButton
                objectName: "addGroupCreateButton"
                text: qsTr("Create Group")
                variant: "primary"
                enabled: addGroupNameInput.text.trim().length > 0
                onClicked: {
                    Settings.createAccountGroup(addGroupNameInput.text.trim());
                    addGroupDialog.accept();
                }
            }
        }

        onOpened: {
            addGroupNameInput.text = "";
            addGroupNameInput.input.forceActiveFocus();
        }
    }

    // A single-line field styled after the lock-screen message editor
    // (T-15.8b); the design system has no generic text field yet.
    component AccountField: Rectangle {
        property alias text: input.text
        property alias input: input
        property string placeholder: ""
        width: parent ? parent.width : 0
        height: 34
        radius: Theme.primitive.radius.md
        color: Theme.color.controlFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        TextInput {
            id: input
            objectName: parent.objectName + "Input"
            anchors.fill: parent
            anchors.leftMargin: Theme.primitive.spacing.md
            anchors.rightMargin: Theme.primitive.spacing.md
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.color.textPrimary
            selectionColor: Theme.color.selection
            selectedTextColor: Theme.color.textPrimary
            selectByMouse: true
            clip: true
            font.pixelSize: Theme.controls.button.fontSize
        }

        Text {
            anchors.fill: input
            visible: input.text.length === 0
            text: parent.placeholder
            color: Theme.color.textTertiary
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: Theme.controls.button.fontSize
        }
    }
}