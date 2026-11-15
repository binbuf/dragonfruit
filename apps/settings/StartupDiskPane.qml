// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Startup Disk sub-pane (T226; T270 completion).
//
// The boot device, filesystem, size, firmware mode, and Secure Boot state are
// host reads. Dragonfruit does not rewrite the bootloader or change the startup
// disk; the pane says so and points at the firmware boot menu.
//
// T270: the disk list, a Restart action with a confirmation, GiB labels, and a
// single Startup disk row (the device is part of its description, not a
// duplicate row).
Item {
    id: root

    readonly property var view: Settings.general.startup !== undefined
        ? Settings.general.startup : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var disks: root.view.disks !== undefined
        ? root.view.disks : []
    readonly property string bootDisk: root.view.disk !== undefined
        ? String(root.view.disk) : ""
    readonly property string bootLabel: {
        if (root.view.label !== undefined && String(root.view.label).length > 0)
            return String(root.view.label);
        return root.bootDisk;
    }
    readonly property string diskDetail: {
        const parts = [];
        if (root.bootDisk.length > 0)
            parts.push(root.bootDisk);
        if (root.view.filesystem !== undefined
                && String(root.view.filesystem).length > 0)
            parts.push(String(root.view.filesystem));
        return parts.join(" · ");
    }
    readonly property string writeError: Settings.generalWriteOperation === "restart"
        && (Settings.generalWriteState === "denied"
            || Settings.generalWriteState === "failed")
        ? Settings.generalWriteError : ""
    readonly property bool restarting: Settings.generalWriteOperation === "restart"
        && Settings.generalWriteState === "busy"

    // Binary sizes are GiB, not decimal GB: the host reports bytes and the
    // label must not imply a different unit.
    function sizeLabel(bytes) {
        const value = Number(bytes || 0);
        if (value <= 0)
            return "—";
        const gib = value / (1024 * 1024 * 1024);
        return (gib >= 100 ? gib.toFixed(0) : gib.toFixed(1)) + " GiB";
    }

    property alias diskRow: diskRow
    property alias diskRepeater: diskRepeater
    property alias bootModeRow: bootModeRow
    property alias secureBootRow: secureBootRow
    property alias restartButton: restartButton
    property alias restartDialog: restartDialog
    property alias restartConfirm: restartConfirm
    property alias readOnlyNote: readOnlyNote
    property alias absenceNote: absenceNote
    property alias diskListGroup: diskListGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            width: parent.width
            title: qsTr("Startup Disk")

            SettingsRow {
                id: diskRow
                objectName: "startupDiskRow"
                width: parent.width
                label: qsTr("Startup disk")
                description: root.diskDetail
                controlData: Text {
                    text: root.bootLabel.length > 0 ? root.bootLabel : "—"
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
            id: diskListGroup
            width: parent.width
            title: qsTr("Disks")

            Text {
                width: parent.width
                visible: root.available && root.disks.length === 0
                text: qsTr("No disks were reported.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: diskRepeater
                model: root.disks
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: String(modelData.label || modelData.device)
                    description: String(modelData.device) + " · "
                        + root.sizeLabel(modelData.sizeBytes)
                        + (modelData.model !== undefined
                           && String(modelData.model).length > 0
                           && String(modelData.model) !== String(modelData.label)
                           ? " · " + String(modelData.model) : "")
                    showSeparator: index < root.disks.length - 1
                    controlData: Text {
                        text: modelData.boot === true
                            ? qsTr("Startup disk")
                            : (modelData.mounted === true ? qsTr("Mounted")
                                                          : qsTr("Not mounted"))
                        color: modelData.boot === true ? Theme.color.accent
                                                       : Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Restart")
                description: qsTr("Restarts this computer. The startup disk is chosen in the firmware boot menu.")
                errorText: root.writeError
                showSeparator: false
                controlData: Button {
                    id: restartButton
                    objectName: "startupRestartButton"
                    text: root.restarting ? qsTr("Restarting…") : qsTr("Restart…")
                    enabled: root.available && !root.restarting
                    onClicked: root.restartDialog.show()
                }
            }
        }

        SettingsGroup {
            width: parent.width

            Text {
                id: readOnlyNote
                objectName: "startupReadOnlyNote"
                width: parent.width
                text: qsTr("Changing the startup disk is not supported here. Use the firmware boot menu at power-on.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "startupAbsenceNote"
                kind: "read-only"
                show: !root.available
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Startup Disk information is unavailable on this system.")
            }
        }
    }

    Dialog {
        id: restartDialog
        objectName: "startupRestartDialog"
        title: qsTr("Restart this computer?")
        message: qsTr("Save your work first. The computer restarts immediately.")
        dismissible: true

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: restartDialog.reject()
            }

            Button {
                id: restartConfirm
                objectName: "startupRestartConfirm"
                text: qsTr("Restart")
                variant: "danger"
                onClicked: {
                    Settings.restartSystem();
                    restartDialog.accept();
                }
            }
        }
    }
}
