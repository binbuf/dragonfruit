// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Backup sub-pane (T226; T270 completion).
//
// Dragonfruit does not implement a backup engine: the host adapter detects
// restic, Pika Backup, and Timeshift behind one seam. The pane owns the durable
// policy (backend, destination, schedule, automatic) as settingsd keys and the
// explicit Run/Restore entries; the selected backend does the work.
//
// T270: a destination picker and path validation, a disabled schedule/automatic
// pair when no backend is installed, a backend selection, and an absence note
// that names the missing host rather than the missing backend. The settingsd
// policy rows stay live when the host is absent.
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
    readonly property bool backendReady: root.detectedBackend.length > 0
    readonly property string selectedBackend: Settings.values["backup.backend"] !== undefined
        ? String(Settings.values["backup.backend"]) : "auto"
    readonly property string destination: Settings.values["backup.destination"] !== undefined
        ? String(Settings.values["backup.destination"]) : ""
    readonly property string schedule: Settings.values["backup.schedule"] !== undefined
        ? String(Settings.values["backup.schedule"]) : "off"
    readonly property bool automatic: Settings.values["backup.auto"] === true
    readonly property bool canRun: root.available && root.view.canRun === true
        && root.view.running !== true
    readonly property bool canRestore: root.available && root.view.canRestore === true
    readonly property bool chooserAvailable: Settings.wallpaperChooserAvailable

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

    // The destination is a local absolute path or a remote repository URL. A
    // relative path is never accepted: the backend would resolve it against a
    // different working directory.
    function validateDestination(value) {
        const text = String(value || "").trim();
        if (text.length === 0)
            return qsTr("Choose a destination first.");
        if (text.startsWith("/"))
            return "";
        if (/^[A-Za-z][A-Za-z0-9+.-]*:\/\//.test(text))
            return "";
        return qsTr("Use an absolute path or a repository URL.");
    }

    readonly property string destinationDraft: destinationField.text
    readonly property string destinationError: root.validateDestination(
        root.destinationDraft)
    readonly property string writeError: Settings.generalWriteOperation === "backup"
        && (Settings.generalWriteState === "denied"
            || Settings.generalWriteState === "failed")
        ? Settings.generalWriteError : ""
    readonly property bool backupBusy: Settings.generalWriteOperation === "backup"
        && Settings.generalWriteState === "busy"

    function indexOf(model, value) {
        for (var i = 0; i < model.length; ++i) {
            if (model[i].value === value)
                return i;
        }
        return 0;
    }

    function runBackup() {
        const destination = root.destination.length > 0
            ? root.destination
            : (root.view.detectedDestination !== undefined
                ? String(root.view.detectedDestination) : "");
        Settings.runBackup(destination);
    }

    property alias backendSelect: backendSelect
    property alias destinationField: destinationField
    property alias destinationSave: destinationSave
    property alias destinationChoose: destinationChoose
    property alias scheduleSelect: scheduleSelect
    property alias autoToggle: autoToggle
    property alias runButton: runButton
    property alias restoreButton: restoreButton
    property alias messageText: messageText
    property alias noBackendNote: noBackendNote
    property alias absenceNote: absenceNote
    property alias destinationRow: destinationRow
    property alias policyGroup: policyGroup
    property alias statusGroup: statusGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Connections {
        target: Settings
        function onBackupDestinationChosen(path) {
            root.destinationField.text = path;
            if (root.validateDestination(path).length === 0)
                Settings.set("backup.destination", path);
        }
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
            id: policyGroup
            width: parent.width
            title: qsTr("Backup")

            Text {
                id: noBackendNote
                objectName: "backupNoBackend"
                width: parent.width
                visible: root.available && !root.backendReady
                text: qsTr("No backup backend is installed. Install restic, Pika Backup, or Timeshift to run a backup.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Backup backend")
                description: root.backendReady
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
                id: destinationRow
                objectName: "backupDestinationRow"
                width: parent.width
                label: qsTr("Destination")
                description: root.view.detectedDestination !== undefined
                    && String(root.view.detectedDestination).length > 0
                    ? qsTr("Backend reports %1").arg(root.view.detectedDestination)
                    : qsTr("An absolute path or a repository URL.")
                errorText: root.destinationDraft.length > 0 ? root.destinationError : ""
                controlData: Row {
                    spacing: Theme.primitive.spacing.xs

                    Rectangle {
                        width: 150
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
                            Accessible.name: qsTr("Backup destination")
                        }
                    }

                    Button {
                        id: destinationChoose
                        objectName: "backupDestinationChoose"
                        text: qsTr("Choose…")
                        enabled: root.chooserAvailable
                        onClicked: Settings.chooseBackupDestination()
                    }

                    Button {
                        id: destinationSave
                        objectName: "backupDestinationSave"
                        text: qsTr("Save")
                        enabled: root.destinationError.length === 0
                        onClicked: Settings.set("backup.destination",
                                                destinationField.text.trim())
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Schedule")
                description: root.backendReady
                    ? ""
                    : qsTr("Install a backup backend to schedule a backup.")
                controlData: Select {
                    id: scheduleSelect
                    objectName: "backupScheduleSelect"
                    model: root.scheduleModel
                    currentIndex: root.indexOf(root.scheduleModel, root.schedule)
                    enabled: root.backendReady
                    accessibleName: qsTr("Backup schedule")
                    onActivated: (index) => Settings.set(
                        "backup.schedule", root.scheduleModel[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Back up automatically")
                description: root.backendReady
                    ? qsTr("Off by default; the backend only runs on this schedule when you turn it on.")
                    : qsTr("Install a backup backend first.")
                showSeparator: false
                controlData: Toggle {
                    id: autoToggle
                    objectName: "backupAutoToggle"
                    checked: root.automatic
                    enabled: root.backendReady
                    accessibleName: qsTr("Back up automatically")
                    onToggled: (checked) => Settings.set("backup.auto", checked)
                }
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "backupAbsenceNote"
                kind: "disabled-with-reason"
                show: !root.available
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("The backup backend is unavailable because the system status service is not running. The policy rows still apply.")
            }
        }

        SettingsGroup {
            id: statusGroup
            width: parent.width
            title: qsTr("Status")

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
                            : (root.backendReady ? qsTr("Ready") : qsTr("Unavailable")))
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        Text {
            width: parent.width
            visible: root.writeError.length > 0
            text: root.writeError
            color: Theme.color.danger
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        Row {
            spacing: Theme.primitive.spacing.md

            Button {
                id: runButton
                objectName: "backupRunButton"
                text: root.backupBusy ? qsTr("Starting…") : qsTr("Back Up Now")
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
    }
}
