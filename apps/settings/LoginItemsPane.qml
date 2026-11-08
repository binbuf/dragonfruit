// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Login Items sub-pane (T226).
//
// XDG autostart entries (`~/.config/autostart`, `/etc/xdg/autostart`) and
// systemd user units are the host's. A system entry is disabled with a user
// override, never edited in place. Extensions are a delegated portal surface,
// so the group carries a hand-off note instead of fake rows.
Item {
    id: root

    readonly property var view: Settings.general.login !== undefined
        ? Settings.general.login : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var autostart: root.view.autostart !== undefined
        ? root.view.autostart : []
    readonly property var userUnits: root.view.userUnits !== undefined
        ? root.view.userUnits : []
    readonly property string extensionsNote: root.view.extensionsNote !== undefined
        ? String(root.view.extensionsNote) : ""

    property alias autostartRepeater: autostartRepeater
    property alias userUnitRepeater: userUnitRepeater
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
            text: qsTr("Choose which apps open at login and which background services run with your session.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Start at Login")
            visible: root.available

            Text {
                width: parent.width
                visible: root.autostart.length === 0
                text: qsTr("No apps open at login.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: autostartRepeater
                model: root.autostart
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.name
                    description: (modelData.source === "system" ? qsTr("System") : qsTr("You"))
                        + " · " + modelData.command
                    showSeparator: index < root.autostart.length - 1
                    controlData: Toggle {
                        checked: modelData.enabled === true
                        enabled: modelData.canToggle !== false
                        accessibleName: qsTr("Open %1 at login").arg(modelData.name)
                        onToggled: (checked) => Settings.setAutostartEnabled(
                            modelData.id, checked)
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Background Services")
            visible: root.available

            Text {
                width: parent.width
                visible: root.userUnits.length === 0
                text: qsTr("No systemd user services are installed.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: userUnitRepeater
                model: root.userUnits
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.name
                    description: modelData.id + " · " + modelData.active
                    showSeparator: index < root.userUnits.length - 1
                    controlData: Toggle {
                        checked: modelData.enabled === "enabled"
                        enabled: modelData.canToggle !== false
                        accessibleName: qsTr("Enable %1").arg(modelData.name)
                        onToggled: (checked) => Settings.setUserUnitEnabled(
                            modelData.id, checked)
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Extensions")
            visible: root.available

            Text {
                id: extensionsText
                objectName: "loginExtensionsNote"
                width: parent.width
                text: root.extensionsNote.length > 0
                    ? root.extensionsNote
                    : qsTr("Extensions are managed by the desktop portal and installed apps.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        SettingsGroup {
            width: parent.width
            visible: !root.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "loginAbsenceNote"
                kind: "disabled-with-reason"
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Login Items are unavailable because the session services are not running.")
            }
        }
    }
}
