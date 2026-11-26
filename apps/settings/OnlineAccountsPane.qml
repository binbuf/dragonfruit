// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Online Accounts pane (T228).
//
// GNOME Online Accounts is optional. When `org.gnome.OnlineAccounts` is absent. df-allow-desktop-name: third-party service name
// the catalog hides this pane (not a dead entry); if it is mounted anyway the
// pane shows the same honest absence note. When GOA is present:
//
//  * the provider list comes from the installed `.provider` descriptors;
//  * the account list and sync capabilities come from GOA's object manager;
//  * Add/Remove and the sync toggles are explicit GOA writes.
//
// The pane never implements OAuth and never stores a credential. GOA owns the
// account; this is its projection.
Item {
    id: root

    readonly property var view: Settings.onlineAccounts
    readonly property bool available: Settings.onlineAccountsAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var providers: root.view.providers !== undefined
        ? root.view.providers : []
    readonly property var accounts: root.view.accounts !== undefined
        ? root.view.accounts : []

    property string selectedProvider: ""
    property string selectedAccountId: ""
    property string addError: ""
    // The account whose details sheet is open (T279).
    property string detailsAccountId: ""
    readonly property var detailsAccount: root.detailsAccountId.length > 0
        ? root.accountById(root.detailsAccountId) : null

    // Test surface.
    property alias accountRepeater: accountRepeater
    property alias addButton: addButton
    property alias addDialog: addDialog
    property alias providerSelect: providerSelect
    property alias identityField: identityField
    property alias addErrorNote: addErrorNote
    property alias removeDialog: removeDialog
    property alias absenceNote: absenceGroup
    property alias absenceButton: absenceButton
    property alias delegateDialog: delegateDialog
    property alias emptyNote: emptyNote
    property alias detailsDialog: detailsDialog
    property alias detailsProvider: detailsProvider
    property alias detailsIdentity: detailsIdentity
    property alias detailsCapabilityRepeater: detailsCapabilityRepeater
    property alias detailsSignInButton: detailsSignInButton

    // The shell reparents dialogs to its root so the detail ScrollView's clip
    // cannot cut them off (T227 note).

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    readonly property var providerModel: {
        var out = [];
        for (var i = 0; i < root.providers.length; ++i)
            out.push({
                label: String(root.providers[i].name),
                value: String(root.providers[i].id)
            });
        return out;
    }

    Component.onCompleted: Settings.refreshOnlineAccounts()

    function openAdd() {
        root.addError = "";
        root.selectedProvider = root.providers.length > 0
            ? String(root.providers[0].id) : "";
        identityField.text = "";
        addDialog.show();
    }

    function submitAdd() {
        root.addError = "";
        if (root.selectedProvider.length === 0) {
            root.addError = qsTr("Choose a provider.");
            return;
        }
        if (identityField.text.trim().length === 0) {
            root.addError = qsTr("Enter the account identity.");
            return;
        }
        Settings.addOnlineAccount(root.selectedProvider, identityField.text.trim());
        addDialog.accept();
    }

    function accountById(id) {
        for (var i = 0; i < root.accounts.length; ++i)
            if (String(root.accounts[i].id) === String(id))
                return root.accounts[i];
        return null;
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Row {
            width: parent.width
            visible: root.ready

            Text {
                width: parent.width - addButton.width - Theme.primitive.spacing.sm
                anchors.verticalCenter: parent.verticalCenter
                text: root.accounts.length === 0
                    ? qsTr("No online accounts are configured.")
                    : qsTr("%1 online account(s).").arg(root.accounts.length)
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Button {
                id: addButton
                objectName: "onlineAccountsAddButton"
                text: qsTr("Add Account\u2026")
                enabled: root.ready && root.providers.length > 0
                onClicked: root.openAdd()
            }
        }

        Text {
            id: emptyNote
            objectName: "onlineAccountsEmptyNote"
            width: parent.width
            visible: root.ready && root.accounts.length === 0
            text: root.providers.length === 0
                ? qsTr("No Online Accounts providers are installed.")
                : qsTr("Add an account to sync mail, calendar, contacts, and files.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        Repeater {
            id: accountRepeater
            model: root.accounts
            delegate: SettingsGroup {
                id: accountGroup
                required property var modelData
                required property int index
                width: root.width
                title: (modelData.providerName !== undefined
                        ? String(modelData.providerName) : String(modelData.provider))
                       + (modelData.presentationIdentity !== undefined
                          && String(modelData.presentationIdentity).length > 0
                          ? " \u2014 " + String(modelData.presentationIdentity) : "")

                SettingsRow {
                    width: parent.width
                    label: modelData.identity !== undefined
                        ? String(modelData.identity) : ""
                    description: modelData.attentionNeeded === true
                        ? qsTr("This account needs attention; sign in again.")
                        : qsTr("Signed in.")
                    showSeparator: true
                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Button {
                            objectName: "onlineAccountsSignInButton"
                            text: qsTr("Sign in again\u2026")
                            variant: "ghost"
                            visible: modelData.attentionNeeded === true
                            enabled: modelData.locked !== true
                            onClicked: Settings.ensureOnlineAccountCredentials(
                                String(modelData.id))
                        }

                        Button {
                            objectName: "onlineAccountsDetailsButton"
                            text: qsTr("Details\u2026")
                            variant: "ghost"
                            onClicked: {
                                root.detailsAccountId = String(modelData.id);
                                root.detailsDialog.show();
                            }
                        }

                        Button {
                            objectName: "onlineAccountsRemoveButton"
                            text: qsTr("Remove\u2026")
                            variant: "ghost"
                            enabled: modelData.locked !== true
                            onClicked: {
                                root.selectedAccountId = String(modelData.id);
                                removeDialog.show();
                            }
                        }
                    }
                }

                Repeater {
                    model: modelData.capabilities !== undefined
                        ? modelData.capabilities : []
                    delegate: SettingsRow {
                        required property var modelData
                        required property int index
                        width: parent.width
                        label: modelData.label !== undefined
                            ? String(modelData.label) : String(modelData.id)
                        description: qsTr("Sync this service for the account.")
                        showSeparator: index < accountGroup.modelData.capabilities.length - 1
                        controlData: Toggle {
                            objectName: "onlineAccountsSyncToggle"
                            accessibleName: qsTr("Sync %1").arg(modelData.label)
                            checked: modelData.enabled === true
                            onToggled: (value) => Settings.setOnlineAccountSync(
                                String(accountGroup.modelData.id),
                                String(modelData.id), value)
                        }
                    }
                }
            }
        }

        // T279: GOA is optional, but the pane never disappears. The absent
        // state keeps its header, states why, and offers the delegate action
        // that explains how the distribution installs the service.
        SettingsGroup {
            id: absenceGroup
            objectName: "onlineAccountsAbsenceNote"
            width: parent.width
            visible: !root.ready

            Text {
                objectName: "onlineAccountsAbsenceText"
                width: parent.width
                text: qsTr("Online accounts are not installed.")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                text: qsTr("Online Accounts is an optional service. The pane stays "
                           + "visible so the feature is discoverable; account sign-in "
                           + "is delegated to the provider once it is installed.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Button {
                id: absenceButton
                objectName: "onlineAccountsAbsenceButton"
                text: qsTr("How to install\u2026")
                onClicked: root.delegateDialog.show()
            }
        }
    }

    // ── Account details (read-only projection + sign-in delegate) ────────
    Dialog {
        id: detailsDialog
        objectName: "onlineAccountsDetailsDialog"
        title: root.detailsAccount !== null
            ? String(root.detailsAccount.providerName !== undefined
                     ? root.detailsAccount.providerName
                     : root.detailsAccount.provider)
            : qsTr("Account Details")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            SettingsRow {
                width: parent.width
                label: qsTr("Provider")
                showSeparator: true
                controlData: Text {
                    id: detailsProvider
                    objectName: "onlineAccountsDetailsProvider"
                    text: root.detailsAccount !== null
                        && root.detailsAccount.providerName !== undefined
                        ? String(root.detailsAccount.providerName) : ""
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Identity")
                showSeparator: true
                controlData: Text {
                    id: detailsIdentity
                    objectName: "onlineAccountsDetailsIdentity"
                    text: root.detailsAccount !== null
                        && root.detailsAccount.identity !== undefined
                        ? String(root.detailsAccount.identity) : ""
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            Text {
                width: parent.width
                text: qsTr("The provider owns the sign-in and the credential; this "
                           + "pane never implements OAuth.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: detailsCapabilityRepeater
                model: root.detailsAccount !== null
                    && root.detailsAccount.capabilities !== undefined
                    ? root.detailsAccount.capabilities : []
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.label !== undefined
                        ? String(modelData.label) : String(modelData.id)
                    showSeparator: false
                    controlData: Text {
                        text: modelData.enabled === true ? qsTr("Syncing")
                                                         : qsTr("Off")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                id: detailsSignInButton
                objectName: "onlineAccountsDetailsSignIn"
                text: qsTr("Sign in again\u2026")
                visible: root.detailsAccount !== null
                    && root.detailsAccount.attentionNeeded === true
                onClicked: {
                    if (root.detailsAccount !== null)
                        Settings.ensureOnlineAccountCredentials(
                            String(root.detailsAccount.id));
                }
            }

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: detailsDialog.accept()
            }
        }
    }

    // ── The absent-service delegate ──────────────────────────────────────
    Dialog {
        id: delegateDialog
        objectName: "onlineAccountsDelegateDialog"
        title: qsTr("Online accounts are not installed")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("Install the distribution's Online Accounts service to add "
                           + "mail, calendar, contacts, and file accounts. The service "
                           + "is an optional package on every supported distribution.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                text: qsTr("Settings does not install packages or implement OAuth; the "
                           + "provider owns both.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: delegateDialog.accept()
            }
        }
    }

    // ── Add account ──────────────────────────────────────────────────────
    Dialog {
        id: addDialog
        objectName: "onlineAccountsAddDialog"
        title: qsTr("Add Account")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("The provider owns the sign-in; this pane only asks "
                           + "Online Accounts to create the account.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Select {
                id: providerSelect
                objectName: "onlineAccountsProviderSelect"
                width: parent.width
                accessibleName: qsTr("Provider")
                model: root.providerModel
                onSelected: (value) => root.selectedProvider = String(value)
            }

            Rectangle {
                width: parent.width
                height: Theme.controls.button.height
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border

                TextInput {
                    id: identityField
                    objectName: "onlineAccountsIdentity"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: root.submitAdd()
                }
            }

            Text {
                id: addErrorNote
                objectName: "onlineAccountsAddError"
                width: parent.width
                visible: root.addError.length > 0
                text: root.addError
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Cancel")
                onClicked: addDialog.reject()
            }
            Button {
                objectName: "onlineAccountsAddConfirm"
                text: qsTr("Add")
                variant: "primary"
                onClicked: root.submitAdd()
            }
        }
    }

    // ── Remove account confirmation ──────────────────────────────────────
    Dialog {
        id: removeDialog
        objectName: "onlineAccountsRemoveDialog"
        title: qsTr("Remove Account")
        dismissible: true

        contentData: Text {
            width: parent.width
            text: qsTr("Remove this account from Online Accounts? Its local "
                       + "sync state is removed; the remote account is unchanged.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Cancel")
                onClicked: removeDialog.reject()
            }
            Button {
                objectName: "onlineAccountsRemoveConfirm"
                text: qsTr("Remove")
                variant: "primary"
                onClicked: {
                    Settings.removeOnlineAccount(root.selectedAccountId);
                    removeDialog.accept();
                }
            }
        }
    }
}
