// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Passwords & Keys sub-pane (T269).
//
// Settings stores no credentials. The pane inventories the installed password
// managers, shows the Secret Service keyring state, records which provider
// Autofill delegates to, and opens that provider's own application. Every
// provider read and the delegate request go through the Settings singleton to
// the General host adapter; QML never touches D-Bus or a keyring.
Item {
    id: root

    readonly property var view: Settings.general.passwords !== undefined
        ? Settings.general.passwords : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var providers: root.view.providers !== undefined
        ? root.view.providers : []
    readonly property bool secretServiceAvailable:
        root.view.secretServiceAvailable === true
    readonly property string keyringLabel: root.view.keyringLabel !== undefined
        ? String(root.view.keyringLabel) : qsTr("Not reported")
    readonly property string message: root.view.message !== undefined
        && root.view.message !== null ? String(root.view.message) : ""
    readonly property string selectedSource: Settings.values["defaults.passwordManager"] !== undefined
        ? String(Settings.values["defaults.passwordManager"]) : ""

    readonly property var sourceModel: {
        var out = [{ label: qsTr("Automatic (active provider)"), value: "" }];
        for (var i = 0; i < root.providers.length; ++i) {
            if (root.providers[i].installed !== true)
                continue;
            out.push({ label: root.providers[i].label, value: root.providers[i].id });
        }
        return out;
    }
    readonly property int sourceIndex: {
        for (var i = 0; i < root.sourceModel.length; ++i) {
            if (root.sourceModel[i].value === root.selectedSource)
                return i;
        }
        return 0;
    }
    // The provider the Open action uses: the chosen source, else the active
    // provider, else the first installed provider that ships an application.
    readonly property var delegateProvider: {
        var fallback = null;
        for (var i = 0; i < root.providers.length; ++i) {
            var provider = root.providers[i];
            if (provider.installed !== true)
                continue;
            if (root.selectedSource.length > 0
                    && provider.id === root.selectedSource)
                return provider;
            if (fallback === null && provider.active === true)
                fallback = provider;
            if (fallback === null && String(provider.launch || "").length > 0)
                fallback = provider;
        }
        return fallback;
    }
    readonly property bool canDelegate: root.available
        && root.delegateProvider !== null
        && String(root.delegateProvider.launch || "").length > 0

    // Test surface (used by tst_settings_general.qml).
    property alias providersRepeater: providersRepeater
    property alias sourceSelect: sourceSelect
    property alias keyringRow: keyringRow
    property alias delegateButton: delegateButton
    property alias noProviderNote: noProviderNote
    property alias secretNote: keyringBanner.bodyText
    property alias keyringBanner: keyringBanner
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function openProvider(id) {
        if (!root.available || !id)
            return;
        Settings.openPasswordManager(id);
    }

    function delegate() {
        if (!root.canDelegate)
            return;
        root.openProvider(root.delegateProvider.id);
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsBanner {
            id: keyringBanner
            objectName: "passwordsKeyringBanner"
            width: parent.width
            title: qsTr("Passwords and keys are stored in the system keyring")
            text: qsTr("Use the system keyring to manage passwords, passkeys, and verification codes for apps and websites. Settings stores no credentials.")
            visible: root.available
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Keyring")
            visible: root.available

            Text {
                id: noProviderNote
                objectName: "passwordsNoProvider"
                width: parent.width
                visible: root.providers.length === 0
                text: root.message.length > 0
                    ? root.message
                    : qsTr("No password provider is installed.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsRow {
                objectName: "passwordsProviderRow"
                width: parent.width
                settingsId: "passwords.source"
                label: qsTr("Password provider")
                description: qsTr("The provider Passwords & Keys delegates to. No secret is stored here.")
                controlData: Select {
                    id: sourceSelect
                    objectName: "passwordsSourceSelect"
                    model: root.sourceModel
                    currentIndex: root.sourceIndex
                    enabled: root.providers.length > 0
                    accessibleName: qsTr("Password provider")
                    onActivated: (index) => Settings.set(
                        "defaults.passwordManager", root.sourceModel[index].value)
                }
            }

            SettingsRow {
                id: keyringRow
                objectName: "passwordsKeyringRow"
                width: parent.width
                settingsId: "passwords.keyring"
                leadingIcon: "lock"
                label: qsTr("System Keyring")
                description: root.secretServiceAvailable
                    ? qsTr("The Secret Service is running.")
                    : qsTr("The Secret Service is not running, so providers cannot share saved credentials.")
                showSeparator: false
                controlData: Text {
                    objectName: "passwordsKeyringStatus"
                    text: root.keyringLabel
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Providers")
            visible: root.available

            Repeater {
                id: providersRepeater
                model: root.providers
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    showSeparator: index < root.providers.length - 1
                    label: modelData.label
                    description: modelData.description + " · "
                        + (modelData.installed === true
                            ? (modelData.active === true
                                ? qsTr("Active") : qsTr("Installed"))
                            : qsTr("Not installed"))
                    controlData: Button {
                        objectName: "passwordsOpenProvider"
                        text: qsTr("Open")
                        enabled: modelData.installed === true
                            && String(modelData.launch || "").length > 0
                        onClicked: root.openProvider(modelData.id)
                    }
                }
            }
        }

        SettingsFooter {
            width: parent.width
            visible: root.available
            alignment: "right"

            Button {
                id: delegateButton
                objectName: "passwordsDelegate"
                text: qsTr("Open Passwords…")
                variant: "primary"
                enabled: root.canDelegate
                onClicked: root.delegate()
            }
        }

        Text {
            width: parent.width
            visible: root.available && root.message.length > 0
                && root.providers.length > 0
            text: root.message
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width
            visible: !root.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "passwordsAbsenceNote"
                kind: "disabled-with-reason"
                reason: qsTr("Passwords & Keys is unavailable because the system information service is not running.")
            }
        }
    }
}
