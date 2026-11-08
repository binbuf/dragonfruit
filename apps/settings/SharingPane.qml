// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Sharing sub-pane (T226).
//
// The host only reports services that are installed: an absent SMB, SSH, CUPS,
// or remote-desktop unit is not a row. Each toggle is one explicit
// `systemctl enable/disable --now` through the bridge host, which may ask for
// administrator authentication.
Item {
    id: root

    readonly property var view: Settings.general.sharing !== undefined
        ? Settings.general.sharing : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var services: root.view.services !== undefined
        ? root.view.services : []

    property alias servicesRepeater: servicesRepeater
    property alias emptyNote: emptyNote
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Text {
            width: parent.width
            text: qsTr("Share this computer's files, screen, printers, and remote login with other devices.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width
            visible: root.available

            Text {
                id: emptyNote
                objectName: "sharingEmptyNote"
                width: parent.width
                visible: root.services.length === 0
                text: qsTr("No sharing services are installed on this system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: servicesRepeater
                model: root.services
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.name
                    description: modelData.description
                        + " · " + (modelData.active === true ? qsTr("On") : qsTr("Off"))
                        + (modelData.needsAuth === true ? " · " + qsTr("Requires authentication") : "")
                    showSeparator: index < root.services.length - 1
                    controlData: Toggle {
                        checked: modelData.active === true
                        enabled: modelData.canToggle !== false
                        accessibleName: modelData.name
                        onToggled: (checked) => Settings.setSharingEnabled(
                            modelData.id, checked)
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            visible: !root.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "sharingAbsenceNote"
                kind: "disabled-with-reason"
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Sharing is unavailable because systemd is not reachable.")
            }
        }
    }
}
