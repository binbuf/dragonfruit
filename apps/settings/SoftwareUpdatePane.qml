// SPDX-License-Identifier: GPL-3.0-or-later
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
    readonly property bool betaAvailable: root.view.betaAvailable === true
    readonly property string installedLabel: root.view.osLabel !== undefined
        && String(root.view.osLabel).length > 0
        ? String(root.view.osLabel) : qsTr("Not available")
    readonly property string providerDetail: {
        if (!root.providerAvailable)
            return qsTr("No update provider");
        var parts = [];
        if (root.updateCount > 0)
            parts.push(root.updateCount === 1
                       ? qsTr("1 update available")
                       : qsTr("%1 updates available").arg(root.updateCount));
        if (root.securityCount > 0)
            parts.push(root.securityCount === 1
                       ? qsTr("1 security update")
                       : qsTr("%1 security updates").arg(root.securityCount));
        if (root.rebootRequired)
            parts.push(qsTr("Restart required"));
        return parts.length > 0 ? parts.join(" · ") : qsTr("Up to date");
    }
    property bool licenseOpen: false

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

        // The provider card: tile, status, the real offer detail, and the one
        // set of update actions. It is not a fake "Dragonfruit OS 0.3.0"
        // version: every line comes from the provider or says it is missing.
        Rectangle {
            id: providerCard
            objectName: "softwareUpdateProviderCard"
            width: parent.width
            implicitHeight: providerLayout.implicitHeight
                            + 2 * Theme.primitive.spacing.md
            visible: root.hostAvailable
            radius: Theme.controls.settingsGroup.radius
            color: Theme.color.surfaceElevated
            border.width: Theme.controls.window.borderWidth
            border.color: Glass.increaseContrast ? Theme.color.textPrimary
                                                 : Theme.color.border
            antialiasing: true

            Row {
                id: providerLayout
                x: Theme.primitive.spacing.md
                y: Theme.primitive.spacing.md
                width: Math.max(0, parent.width - 2 * Theme.primitive.spacing.md)
                spacing: Theme.primitive.spacing.md

                SettingsCategoryIcon {
                    width: 44
                    height: 44
                    source: SettingsPanes.rowStyle("update").source
                    gradientStart: SettingsPanes.rowStyle("update").gradientStart
                    gradientEnd: SettingsPanes.rowStyle("update").gradientEnd
                }

                Column {
                    width: Math.max(0, parent.width - 56)
                    spacing: Theme.primitive.spacing.xs

                    Text {
                        id: statusText
                        objectName: "softwareUpdateStatus"
                        width: parent.width
                        text: root.statusLabel
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        font.weight: Theme.primitive.font.weightSemibold
                        wrapMode: Text.WordWrap
                        Accessible.name: qsTr("Software update status: %1").arg(text)
                    }

                    Text {
                        objectName: "softwareUpdateProviderDetail"
                        width: parent.width
                        text: root.providerDetail
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
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

                    Row {
                        spacing: Theme.primitive.spacing.sm

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
                            enabled: root.providerAvailable && !root.busy
                                && root.updateCount > 0
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
                }
            }
        }

        Text {
            id: noProviderNote
            objectName: "softwareUpdateNoProvider"
            width: parent.width
            visible: root.hostAvailable && !root.providerAvailable
            text: qsTr("No update provider is available on this system. Install the distribution's update provider to check for updates.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        // Only the provider's real additional items. An empty offer is stated
        // by the card, not by an empty group.
        SettingsGroup {
            width: parent.width
            title: qsTr("Also Available")
            visible: root.hostAvailable && root.providerAvailable
                && root.updates.length > 0

            Repeater {
                id: updatesRepeater
                model: root.providerAvailable ? root.updates : []
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.name
                    description: qsTr("%1 \u2192 %2 \u00b7 %3")
                        .arg(modelData.currentVersion)
                        .arg(modelData.availableVersion)
                        .arg(modelData.severityLabel)
                    showSeparator: index < root.updates.length - 1
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Installed")
            visible: root.hostAvailable

            SettingsRow {
                objectName: "softwareUpdateInstalled"
                width: parent.width
                settingsId: "updates.installed"
                label: qsTr("Installed")
                controlData: Text {
                    text: root.installedLabel
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "updates.auto"
                label: qsTr("Automatic Updates")
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

            // Beta Updates is rendered only when the provider advertises a
            // beta channel; it is a read-only provider fact, not a stored
            // settingsd key. With no beta provider the row is a map omission.
            SettingsRow {
                objectName: "softwareUpdateBeta"
                width: parent.width
                visible: root.betaAvailable
                settingsId: "updates.beta"
                label: qsTr("Beta Updates")
                controlData: Text {
                    text: root.view.betaLabel !== undefined
                        ? String(root.view.betaLabel) : qsTr("Available")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
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
                controlData: Text {
                    text: root.providerAvailable
                        ? (root.securityCount > 0 ? qsTr("Included") : qsTr("None pending"))
                        : qsTr("Unavailable")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

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

        SettingsNote {
            width: parent.width
            visible: root.hostAvailable
            text: qsTr("Use of this software is subject to the license agreement that accompanied the software being updated.")
        }

        SettingsFooter {
            width: parent.width
            visible: root.hostAvailable
            alignment: "right"

            Button {
                objectName: "softwareUpdateLicense"
                text: qsTr("License agreement")
                variant: "ghost"
                onClicked: root.licenseOpen = !root.licenseOpen
            }
        }

        SettingsNote {
            width: parent.width
            visible: root.licenseOpen
            text: qsTr("Installed host packages retain the licenses declared by their distributions.")
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
