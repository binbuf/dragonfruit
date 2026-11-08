// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Startup Disk sub-pane (T226).
//
// Read-only first pass: the boot device, filesystem, size, firmware mode, and
// Secure Boot state are host reads. Dragonfruit does not rewrite the
// bootloader or change the startup disk; the pane says so and points at the
// firmware boot menu.
Item {
    id: root

    readonly property var view: Settings.general.startup !== undefined
        ? Settings.general.startup : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"

    function sizeLabel(bytes) {
        const value = Number(bytes || 0);
        if (value <= 0)
            return "—";
        const gib = Math.round(value / (1024 * 1024 * 1024));
        return gib + " GB";
    }

    property alias diskRow: diskRow
    property alias bootModeRow: bootModeRow
    property alias secureBootRow: secureBootRow
    property alias readOnlyNote: readOnlyNote
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            width: parent.width
            visible: root.available

            SettingsRow {
                id: diskRow
                objectName: "startupDiskRow"
                width: parent.width
                label: qsTr("Startup disk")
                controlData: Text {
                    text: root.view.disk !== undefined && String(root.view.disk).length > 0
                        ? String(root.view.disk)
                        : String(root.view.device || "—")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Device")
                controlData: Text {
                    text: String(root.view.device || "—")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Filesystem")
                controlData: Text {
                    text: String(root.view.filesystem || "—")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Size")
                controlData: Text {
                    text: root.sizeLabel(root.view.totalBytes)
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                id: bootModeRow
                objectName: "startupBootModeRow"
                width: parent.width
                label: qsTr("Firmware")
                controlData: Text {
                    text: String(root.view.bootMode || "—")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                id: secureBootRow
                objectName: "startupSecureBootRow"
                width: parent.width
                label: qsTr("Secure Boot")
                showSeparator: false
                controlData: Text {
                    text: String(root.view.secureBoot || qsTr("Unknown"))
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        SettingsGroup {
            width: parent.width
            visible: root.available

            Text {
                id: readOnlyNote
                objectName: "startupReadOnlyNote"
                width: parent.width
                text: qsTr("Changing the startup disk is not supported here. Use the firmware boot menu at power-on.")
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
                objectName: "startupAbsenceNote"
                kind: "read-only"
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Startup Disk information is unavailable on this system.")
            }
        }
    }
}
