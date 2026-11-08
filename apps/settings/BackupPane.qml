// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Backup sub-pane (T226).
//
// Dragonfruit does not implement a backup engine: the host adapter detects
// restic, Pika Backup, and Timeshift behind one seam. The pane owns the durable
// policy (backend, destination, schedule, automatic) as settingsd keys and the
// explicit Run/Restore entries; the selected backend does the work.
Item {
    id: root

    readonly property var view: Settings.general.backup !== undefined
        ? Settings.general.backup : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var candidates: root.view.candidates !== undefined
        ? root.view.candidates : []
    readonly property string detectedBackend: root.view.backend !== undefined
        ? String(root.view.backend) : ""
    readonly property string selectedBackend: Settings.values["backup.backend"] !== undefined
        ? String(Settings.values["backup.backend"]) : "auto"
    readonly property string destination: Settings.values["backup.destination"] !== undefined
        ? String(Settings.values["backup.destination"]) : ""
    readonly property string schedule: Settings.values["backup.schedule"] !== undefined
        ? String(Settings.values["backup.schedule"]) : "off"
    readonly property bool automatic: Settings.values["backup.auto"] === true
    readonly property bool canRun: root.view.canRun === true
        && root.view.running !== true
    readonly property bool canRestore: root.view.canRestore === true

    readonly property var backendModel: {
        var out = [{ label: qsTr("Automatic"), value: "auto", enabled: true }];
        for (var i = 0; i < root.candidates.length; ++i) {
            var candidate = root.candidates[i];
            out.push({
                label: candidate.label + (candidate.installed === true ? "" : " (" + qsTr("not installed") + ")"),
                value: candidate.id,
                enabled: candidate.installed === true
            });
        }
        return out;
    }
    readonly property var scheduleModel: [
        { label: qsTr("Off"), value: "off" },
        { label: qsTr("Hourly"), value: "hourly" },
        { label: qsTr("Daily"), value: "daily" },
        { label: qsTr("Weekly"), value: "weekly" },
        { label: qsTr("Monthly"), value: "monthly" }
    ]

    function indexOf(model, value) {
        for (var i = 0; i < model.length; ++i) {
            if (model[i].value === value)
                return i;
        }
        return 0;
    }

    property alias backendSelect: backendSelect
    property alias destinationField: destinationField
    property alias destinationSave: destinationSave
    property alias scheduleSelect: scheduleSelect
    property alias autoToggle: autoToggle
    property alias runButton: runButton
    property alias restoreButton: restoreButton
    property alias messageText: messageText
    property alias noBackendNote: noBackendNote
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function runBackup() {
        const destination = root.destination.length > 0
            ? root.destination
            : (root.view.detectedDestination !== undefined
                ? String(root.view.detectedDestination) : "");
        Settings.runBackup(destination);
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Text {
            width: parent.width
            text: qsTr("Dragonfruit delegates the backup itself to restic, Pika Backup, or Timeshift. Choose a backend and the policy here.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Backup")
            visible: root.available

            Text {
                id: noBackendNote
                objectName: "backupNoBackend"
                width: parent.width
                visible: root.detectedBackend.length === 0
                text: qsTr("No backup backend is installed. Install restic, Pika Backup, or Timeshift to use this pane.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Backup backend")
                description: root.detectedBackend.length > 0
                    ? qsTr("Detected: %1").arg(root.view.backendLabel || root.detectedBackend)
                    : qsTr("None detected")
                controlData: Select {
                    id: backendSelect
                    objectName: "backupBackendSelect"
                    model: root.backendModel
                    currentIndex: root.indexOf(root.backendModel, root.selectedBackend)
                    accessibleName: qsTr("Backup backend")
                    onActivated: (index) => Settings.set(
                        "backup.backend", root.backendModel[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Destination")
                description: root.view.detectedDestination !== undefined
                    && String(root.view.detectedDestination).length > 0
                    ? qsTr("Backend reports %1").arg(root.view.detectedDestination)
                    : qsTr("A path or device the backend can write to.")
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Rectangle {
                        width: 210
                        height: Theme.controls.select.height
                        radius: Theme.controls.select.radius
                        color: Theme.color.controlFill
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        TextInput {
                            id: destinationField
                            objectName: "backupDestinationField"
                            anchors.fill: parent
                            anchors.leftMargin: Theme.controls.select.paddingH
                            anchors.rightMargin: Theme.controls.select.paddingH
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            text: root.destination
                            selectByMouse: true
                        }
                    }

                    Button {
                        id: destinationSave
                        objectName: "backupDestinationSave"
                        text: qsTr("Save")
                        onClicked: Settings.set("backup.destination", destinationField.text)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Schedule")
                controlData: Select {
                    id: scheduleSelect
                    objectName: "backupScheduleSelect"
                    model: root.scheduleModel
                    currentIndex: root.indexOf(root.scheduleModel, root.schedule)
                    accessibleName: qsTr("Backup schedule")
                    onActivated: (index) => Settings.set(
                        "backup.schedule", root.scheduleModel[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Back up automatically")
                description: qsTr("Off by default; the backend only runs on this schedule when you turn it on.")
                showSeparator: false
                controlData: Toggle {
                    id: autoToggle
                    objectName: "backupAutoToggle"
                    checked: root.automatic
                    accessibleName: qsTr("Back up automatically")
                    onToggled: (checked) => Settings.set("backup.auto", checked)
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Status")
            visible: root.available

            SettingsRow {
                width: parent.width
                label: qsTr("Last backup")
                controlData: Text {
                    text: root.view.lastRun !== undefined
                        && String(root.view.lastRun).length > 0
                        ? String(root.view.lastRun) : qsTr("Never")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Progress")
                showSeparator: false
                controlData: Text {
                    id: messageText
                    objectName: "backupMessage"
                    text: root.view.running === true
                        ? qsTr("Running…")
                        : (root.view.message !== undefined && String(root.view.message).length > 0
                            ? String(root.view.message)
                            : (root.detectedBackend.length > 0 ? qsTr("Ready") : qsTr("Unavailable")))
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        Row {
            spacing: Theme.primitive.spacing.md
            visible: root.available

            Button {
                id: runButton
                objectName: "backupRunButton"
                text: qsTr("Back Up Now")
                variant: "primary"
                enabled: root.canRun
                onClicked: root.runBackup()
            }

            Button {
                id: restoreButton
                objectName: "backupRestoreButton"
                text: qsTr("Restore…")
                enabled: root.canRestore
                onClicked: Settings.openBackupRestore()
            }
        }

        SettingsGroup {
            width: parent.width
            visible: !root.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "backupAbsenceNote"
                kind: "disabled-with-reason"
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Backup is unavailable because no backup backend is installed.")
            }
        }
    }
}
