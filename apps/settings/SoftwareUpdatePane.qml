// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Software Update sub-pane (T269).
//
// The surface is an in-place General sub-pane, not a dialog. The host identity
// and the distribution update provider are separate halves of the Updates
// view: a host with no provider still shows the identity rows, and this pane
// says "No update provider" instead of inventing "Up to Date". The provider's
// check/install/restart writes go through the Settings singleton; the pane
// never runs a package manager itself.
Item {
    id: root

    readonly property var view: Settings.updates
    readonly property bool hostAvailable: Settings.updatesAvailable
    readonly property bool providerAvailable: root.view.updatesAvailable === true
    readonly property bool busy: root.view.busy === true
    readonly property bool rebootRequired: root.view.rebootRequired === true
    readonly property var updates: root.view.updates !== undefined
        ? root.view.updates : []
    readonly property int updateCount: root.view.updateCount !== undefined
        ? root.view.updateCount : 0
    readonly property int securityCount: root.view.securityCount !== undefined
        ? root.view.securityCount : 0
    readonly property string statusLabel: {
        if (!root.hostAvailable)
            return qsTr("Unavailable");
        if (!root.providerAvailable)
            return qsTr("No update provider");
        return root.view.label !== undefined ? String(root.view.label) : "";
    }
    readonly property string message: root.view.message !== undefined
        && root.view.message !== null ? String(root.view.message) : ""
    readonly property string lastChecked: {
        const value = Number(root.view.lastCheckedMs || 0);
        if (!root.providerAvailable || !isFinite(value) || value <= 0)
            return qsTr("Not checked");
        return Qt.formatDateTime(new Date(value), "yyyy-MM-dd HH:mm");
    }

    // Test surface (used by tst_settings_general.qml).
    property alias statusText: statusText
    property alias messageText: messageText
    property alias updatesRepeater: updatesRepeater
    property alias checkButton: checkButton
    property alias installButton: installButton
    property alias restartButton: restartButton
    property alias autoCheckToggle: autoCheckToggle
    property alias autoInstallToggle: autoInstallToggle
    property alias securityRow: securityRow
    property alias lastCheckedRow: lastCheckedRow
    property alias progressText: progressText
    property alias noProviderNote: noProviderNote
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Text {
            id: statusText
            objectName: "softwareUpdateStatus"
            width: parent.width
            text: root.statusLabel
            color: Theme.color.textPrimary
            font.pixelSize: 20
            font.weight: Theme.primitive.font.weightMedium
            wrapMode: Text.WordWrap
            Accessible.name: qsTr("Software update status: %1").arg(text)
        }

        Text {
            id: messageText
            objectName: "softwareUpdateMessage"
            width: parent.width
            visible: root.message.length > 0
            text: root.message
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        Text {
            id: progressText
            objectName: "softwareUpdateProgress"
            width: parent.width
            visible: root.busy
            text: qsTr("Working…")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Available Updates")
            visible: root.hostAvailable

            Text {
                id: noProviderNote
                objectName: "softwareUpdateNoProvider"
                width: parent.width
                visible: !root.providerAvailable
                text: qsTr("No update provider is available on this system. Install the distribution's update provider to check for updates.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                visible: root.providerAvailable && root.updates.length === 0
                text: root.statusLabel === qsTr("Up to Date")
                    ? qsTr("All available updates are installed.")
                    : qsTr("No updates are on offer.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: updatesRepeater
                model: root.providerAvailable ? root.updates : []
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    showSeparator: index < root.updates.length - 1
                    label: modelData.name
                    description: qsTr("%1 \u2192 %2 \u00b7 %3")
                        .arg(modelData.currentVersion)
                        .arg(modelData.availableVersion)
                        .arg(modelData.severityLabel)
                }
            }
        }

        Row {
            spacing: Theme.primitive.spacing.md
            visible: root.hostAvailable

            Button {
                id: checkButton
                objectName: "softwareUpdateCheck"
                text: qsTr("Check for Updates")
                enabled: root.providerAvailable && !root.busy
                onClicked: Settings.checkUpdates()
            }

            Button {
                id: installButton
                objectName: "softwareUpdateInstall"
                text: qsTr("Update Now")
                variant: "primary"
                enabled: root.providerAvailable && !root.busy && root.updateCount > 0
                onClicked: Settings.installUpdates()
            }

            Button {
                id: restartButton
                objectName: "softwareUpdateRestart"
                text: qsTr("Restart")
                variant: "primary"
                visible: root.rebootRequired
                onClicked: Settings.rebootUpdates()
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Automatic Updates")
            visible: root.hostAvailable

            SettingsRow {
                width: parent.width
                label: qsTr("Check for updates automatically")
                description: qsTr("Never enabled unless you turn it on here.")
                controlData: Toggle {
                    id: autoCheckToggle
                    objectName: "softwareUpdateAutoCheck"
                    checked: Settings.values["updates.auto"] === true
                    enabled: root.providerAvailable
                    accessibleName: qsTr("Check for updates automatically")
                    onToggled: (checked) => {
                        Settings.set("updates.auto", checked);
                        Settings.applyUpdatePolicy(checked,
                            Settings.values["updates.autoInstall"] === true);
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Install automatically")
                description: qsTr("Requires automatic checks.")
                controlData: Toggle {
                    id: autoInstallToggle
                    objectName: "softwareUpdateAutoInstall"
                    checked: Settings.values["updates.autoInstall"] === true
                    enabled: root.providerAvailable
                        && Settings.values["updates.auto"] === true
                    accessibleName: qsTr("Install updates automatically")
                    onToggled: (checked) => {
                        Settings.set("updates.autoInstall", checked);
                        Settings.applyUpdatePolicy(
                            Settings.values["updates.auto"] === true, checked);
                    }
                }
            }

            SettingsRow {
                id: securityRow
                objectName: "softwareUpdateSecurity"
                width: parent.width
                label: qsTr("Security updates")
                description: root.securityCount > 0
                    ? qsTr("%1 security update(s) are included in the available updates.")
                        .arg(root.securityCount)
                    : qsTr("Security updates are included when the provider offers them.")
                showSeparator: false
                controlData: Text {
                    text: root.providerAvailable
                        ? (root.securityCount > 0 ? qsTr("Included") : qsTr("None pending"))
                        : qsTr("Unavailable")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        SettingsGroup {
            width: parent.width
            visible: root.hostAvailable

            SettingsRow {
                id: lastCheckedRow
                objectName: "softwareUpdateLastChecked"
                width: parent.width
                label: qsTr("Last checked")
                description: qsTr("Scheduled installs are not offered by this provider.")
                showSeparator: false
                controlData: Text {
                    text: root.lastChecked
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        SettingsGroup {
            width: parent.width
            visible: !root.hostAvailable

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "softwareUpdateAbsenceNote"
                kind: "disabled-with-reason"
                reason: qsTr("Software updates are unavailable because the system information service is not running.")
            }
        }
    }
}
