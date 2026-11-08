// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The General group (T-15.10b / T226).
//
// General mirrors the macOS pane as grouped disclosure rows: About and
// Software Update open dialogs, Storage jumps to the Storage pane, and the
// remaining reference rows open their own sub-pane in place (Login Items,
// Language & Region, Date & Time, Sharing, Backup, Startup Disk, Reset). The
// rows are a live summary of the same bridge-host view the sub-panes render.
//
// Deviations from the macOS capture reference (ADR 0122), recorded so later
// tasks do not re-litigate them:
//   * `Storage` is a first-class sidebar pane in Dragonfruit (T-15.2b), so
//     its General row navigates there instead of duplicating the pane.
//   * the About illustration is our own `computer` glyph, and the `Chip` value
//     is the host's processor (never `Apple M5`); `OS` reads the host's own
//     release.
//   * the `Regulatory Certification` / copyright footer is omitted.
//   * the trailing `?` help is omitted project-wide.
Item {
    id: root

    // The bridge host's General/About/Updates view (empty when absent).
    readonly property var view: Settings.updates
    readonly property bool available: Settings.updatesAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property string deviceName: root.view.deviceName !== undefined
        ? String(root.view.deviceName) : ""
    readonly property string processor: root.view.processor !== undefined
        ? String(root.view.processor) : ""
    readonly property string memoryLabel: root.view.memoryLabel !== undefined
        ? String(root.view.memoryLabel) : ""
    readonly property string osLabel: root.view.osLabel !== undefined
        ? String(root.view.osLabel) : ""
    readonly property string kernel: root.view.kernel !== undefined
        ? String(root.view.kernel) : ""
    readonly property bool hasSerial: root.view.hasSerial === true
    readonly property string serial: root.view.serial !== undefined
        ? String(root.view.serial) : ""
    readonly property string gpuLabel: root.view.gpuLabel !== undefined
        ? String(root.view.gpuLabel) : ""
    readonly property string diskLabel: root.view.diskLabel !== undefined
        ? String(root.view.diskLabel) : ""

    // The distribution update provider, when the host runs one.
    readonly property bool providerAvailable: root.view.updatesAvailable === true
    readonly property var updates: root.view.updates !== undefined
        ? root.view.updates : []
    readonly property bool busy: root.view.busy === true
    readonly property bool rebootRequired: root.view.rebootRequired === true
    readonly property int updateCount: root.view.updateCount !== undefined
        ? root.view.updateCount : 0
    readonly property int securityCount: root.view.securityCount !== undefined
        ? root.view.securityCount : 0
    readonly property string updateLabel: root.view.label !== undefined
        ? String(root.view.label) : ""
    readonly property string updateMessage: root.view.message !== undefined
        && root.view.message !== null ? String(root.view.message) : ""

    // The General sub-panes view (T226). Each sub-view carries its own state.
    readonly property var general: Settings.general
    readonly property bool generalAvailable: Settings.generalAvailable
    readonly property var timeView: root.general.time !== undefined
        ? root.general.time : ({})
    readonly property var localeView: root.general.locale !== undefined
        ? root.general.locale : ({})
    readonly property var loginView: root.general.login !== undefined
        ? root.general.login : ({})
    readonly property var sharingView: root.general.sharing !== undefined
        ? root.general.sharing : ({})
    readonly property var backupView: root.general.backup !== undefined
        ? root.general.backup : ({})
    readonly property var storageView: root.general.storage !== undefined
        ? root.general.storage : ({})
    readonly property var startupView: root.general.startup !== undefined
        ? root.general.startup : ({})

    // The in-place sub-pane ("" is the General list).
    property string subPaneId: ""

    readonly property string updateSummary: {
        if (!root.ready)
            return qsTr("Unavailable");
        if (!root.providerAvailable)
            return qsTr("No update provider");
        return root.updateLabel;
    }
    readonly property string timeSummary: root.generalAvailable
        && root.timeView.state === "available"
        ? String(root.timeView.label || qsTr("Date & Time"))
        : qsTr("Unavailable")
    readonly property string dateTimeSummary: root.timeSummary
    readonly property string localeSummary: root.generalAvailable
        && root.localeView.state === "available"
        ? String(root.localeView.label || qsTr("Language & Region"))
        : qsTr("Unavailable")
    readonly property string loginSummary: root.generalAvailable
        && root.loginView.state === "available"
        ? String(root.loginView.label || qsTr("Login Items"))
        : qsTr("Unavailable")
    readonly property string sharingSummary: root.generalAvailable
        && root.sharingView.state === "available"
        ? String(root.sharingView.label || qsTr("Sharing"))
        : qsTr("Unavailable")
    readonly property string backupSummary: root.generalAvailable
        && root.backupView.state === "available"
        ? String(root.backupView.label || qsTr("Backup"))
        : qsTr("Unavailable")
    readonly property string storageSummary: root.generalAvailable
        && root.storageView.state === "available"
        ? String(root.storageView.label || qsTr("Storage"))
        : qsTr("Unavailable")
    readonly property string startupSummary: {
        if (!root.generalAvailable || root.startupView.state !== "available")
            return qsTr("Unavailable");
        const disk = root.startupView.disk !== undefined && String(root.startupView.disk).length > 0
            ? String(root.startupView.disk)
            : String(root.startupView.device || "");
        const mode = String(root.startupView.bootMode || "");
        return mode.length > 0 ? disk + " · " + mode : disk;
    }

    // Test surface (used by tst_settings_general.qml).
    property alias aboutRow: aboutRow
    property alias aboutDialog: aboutDialog
    property alias aboutHeading: aboutHeading
    property alias aboutOsRow: aboutOsRow
    property alias aboutMemoryRow: aboutMemoryRow
    property alias aboutSerialRow: aboutSerialRow
    property alias moreInfoButton: moreInfoButton
    property alias moreInfoDialog: moreInfoDialog
    property alias updateRow: updateRow
    property alias updateDialog: updateDialog
    property alias updateStatusText: updateStatusText
    property alias updatesRepeater: updatesRepeater
    property alias noProviderNote: noProviderNote
    property alias checkButton: checkButton
    property alias installButton: installButton
    property alias restartButton: restartButton
    property alias absenceNote: absenceNote
    property alias autoCheckToggle: autoCheckToggle
    property alias autoInstallToggle: autoInstallToggle
    property alias storageRow: storageRow
    property alias loginRow: loginRow
    property alias languageRow: languageRow
    property alias dateTimeRow: dateTimeRow
    property alias sharingRow: sharingRow
    property alias backupRow: backupRow
    property alias startupRow: startupRow
    property alias resetRow: resetRow
    property alias subPaneLoader: subPaneLoader
    property alias listColumn: listColumn
    property alias backButton: backButton

    signal navigateRequested(string paneId)

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function openSubPane(id) {
        root.subPaneId = id;
    }

    function closeSubPane() {
        root.subPaneId = "";
    }

    function subPaneComponent(id) {
        switch (id) {
        case "login": return loginComponent;
        case "language": return languageComponent;
        case "date-time": return dateTimeComponent;
        case "sharing": return sharingComponent;
        case "backup": return backupComponent;
        case "startup": return startupComponent;
        case "reset": return resetComponent;
        default: return null;
        }
    }

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: {
        Settings.refreshUpdates();
        Settings.refreshGeneral();
        Settings.applyUpdatePolicy(Settings.values["updates.auto"] === true,
                                   Settings.values["updates.autoInstall"] === true);
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The General list ──────────────────────────────────────────────
        Column {
            id: listColumn
            width: parent.width
            spacing: Theme.primitive.spacing.lg
            visible: root.subPaneId === ""

            SettingsGroup {
                width: parent.width
                visible: root.ready

                SettingsRow {
                    id: aboutRow
                    objectName: "generalAboutRow"
                    width: parent.width
                    label: qsTr("About")
                    description: root.deviceName
                    disclosure: true
                    onDisclosureClicked: aboutDialog.show()
                }

                SettingsRow {
                    id: updateRow
                    objectName: "generalUpdateRow"
                    width: parent.width
                    label: qsTr("Software Update")
                    description: root.updateSummary
                    showSeparator: false
                    disclosure: true
                    onDisclosureClicked: updateDialog.show()
                }
            }

            SettingsGroup {
                width: parent.width
                visible: root.generalAvailable

                SettingsRow {
                    id: storageRow
                    objectName: "generalStorageRow"
                    width: parent.width
                    label: qsTr("Storage")
                    description: root.storageSummary
                    disclosure: true
                    onDisclosureClicked: root.navigateRequested("storage")
                }

                SettingsRow {
                    id: loginRow
                    objectName: "generalLoginRow"
                    width: parent.width
                    label: qsTr("Login Items")
                    description: root.loginSummary
                    disclosure: true
                    onDisclosureClicked: root.openSubPane("login")
                }

                SettingsRow {
                    id: languageRow
                    objectName: "generalLanguageRow"
                    width: parent.width
                    label: qsTr("Language & Region")
                    description: root.localeSummary
                    disclosure: true
                    onDisclosureClicked: root.openSubPane("language")
                }

                SettingsRow {
                    id: dateTimeRow
                    objectName: "generalDateTimeRow"
                    width: parent.width
                    label: qsTr("Date & Time")
                    description: root.timeSummary
                    disclosure: true
                    onDisclosureClicked: root.openSubPane("date-time")
                }

                SettingsRow {
                    id: sharingRow
                    objectName: "generalSharingRow"
                    width: parent.width
                    label: qsTr("Sharing")
                    description: root.sharingSummary
                    disclosure: true
                    onDisclosureClicked: root.openSubPane("sharing")
                }

                SettingsRow {
                    id: backupRow
                    objectName: "generalBackupRow"
                    width: parent.width
                    label: qsTr("Backup")
                    description: root.backupSummary
                    disclosure: true
                    onDisclosureClicked: root.openSubPane("backup")
                }

                SettingsRow {
                    id: startupRow
                    objectName: "generalStartupRow"
                    width: parent.width
                    label: qsTr("Startup Disk")
                    description: root.startupSummary
                    disclosure: true
                    onDisclosureClicked: root.openSubPane("startup")
                }

                SettingsRow {
                    id: resetRow
                    objectName: "generalResetRow"
                    width: parent.width
                    label: qsTr("Reset")
                    description: qsTr("Reset Dragonfruit settings")
                    showSeparator: false
                    disclosure: true
                    onDisclosureClicked: root.openSubPane("reset")
                }
            }

            // ── Absence ──────────────────────────────────────────────────
            SettingsGroup {
                width: parent.width
                visible: !root.ready && !root.generalAvailable

                Text {
                    id: absenceNote
                    objectName: "generalAbsenceNote"
                    width: parent.width
                    text: qsTr("The system information service is not running, "
                               + "so system details and updates are unavailable.")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    wrapMode: Text.WordWrap
                }
            }
        }

        // ── One in-place sub-pane ─────────────────────────────────────────
        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.lg
            visible: root.subPaneId !== ""

            Button {
                id: backButton
                objectName: "generalSubPaneBack"
                text: qsTr("‹ General")
                onClicked: root.closeSubPane()
            }

            Loader {
                id: subPaneLoader
                width: parent.width
                sourceComponent: root.subPaneComponent(root.subPaneId)
            }
        }
    }

    // ── The About dialog (About This System) ──────────────────────────────
    Dialog {
        id: aboutDialog
        objectName: "generalAboutDialog"
        title: root.deviceName.length > 0 ? root.deviceName : qsTr("About This System")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.md

            Icon {
                anchors.horizontalCenter: parent.horizontalCenter
                name: "computer"
                size: 72
                color: Theme.color.textSecondary
            }

            Text {
                id: aboutHeading
                objectName: "generalAboutHeading"
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: root.deviceName.length > 0
                    ? root.deviceName : qsTr("This System")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeXl
                font.weight: Theme.primitive.font.weightSemibold
                wrapMode: Text.WordWrap
            }

            Text {
                objectName: "generalAboutSublabel"
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                visible: root.processor.length > 0
                text: root.processor
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsGroup {
                width: parent.width
                reserveBottomMargin: false

                SettingsRow {
                    objectName: "generalAboutChipRow"
                    width: parent.width
                    label: qsTr("Chip")
                    controlData: Text {
                        objectName: "generalAboutChipValue"
                        text: root.processor.length > 0
                            ? root.processor : qsTr("Unknown")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }

                SettingsRow {
                    id: aboutMemoryRow
                    objectName: "generalAboutMemoryRow"
                    width: parent.width
                    label: qsTr("Memory")
                    controlData: Text {
                        objectName: "generalAboutMemoryValue"
                        text: root.memoryLabel.length > 0
                            ? root.memoryLabel : qsTr("Unknown")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }

                SettingsRow {
                    id: aboutSerialRow
                    objectName: "generalAboutSerialRow"
                    width: parent.width
                    label: qsTr("Serial number")
                    visible: root.hasSerial
                    controlData: Text {
                        objectName: "generalAboutSerialValue"
                        text: root.serial
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }

                SettingsRow {
                    id: aboutOsRow
                    objectName: "generalAboutOsRow"
                    width: parent.width
                    label: qsTr("OS")
                    showSeparator: false
                    controlData: Text {
                        objectName: "generalAboutOsValue"
                        text: root.osLabel.length > 0
                            ? root.osLabel : qsTr("Unknown")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
            }

            Text {
                objectName: "generalAboutKernel"
                width: parent.width
                visible: root.kernel.length > 0
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("Kernel %1").arg(root.kernel)
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Button {
                id: moreInfoButton
                objectName: "generalAboutMoreInfo"
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("More Info…")
                onClicked: {
                    aboutDialog.accept();
                    moreInfoDialog.show();
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: aboutDialog.accept()
            }
        }
    }

    // ── More Info (the full host read, no invented Apple hardware) ────────
    Dialog {
        id: moreInfoDialog
        objectName: "generalMoreInfoDialog"
        title: qsTr("System Info")
        dismissible: true

        contentData: SettingsGroup {
            width: parent.width
            reserveBottomMargin: false

            Repeater {
                model: [
                    { label: qsTr("Computer name"), value: root.view.hostName || "—" },
                    { label: qsTr("Model"), value: root.view.deviceModel || "—" },
                    { label: qsTr("OS"), value: root.osLabel || "—" },
                    { label: qsTr("Kernel"), value: root.kernel || "—" },
                    { label: qsTr("Architecture"), value: root.view.architecture || "—" },
                    { label: qsTr("Processor"), value: root.processor || "—" },
                    { label: qsTr("Memory"), value: root.memoryLabel || "—" },
                    { label: qsTr("GPU"), value: root.gpuLabel || "—" },
                    { label: qsTr("Disk"), value: root.diskLabel || "—" },
                    { label: qsTr("Serial number"), value: root.hasSerial ? root.serial : qsTr("Not available") }
                ]
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.label
                    showSeparator: index < 9
                    controlData: Text {
                        text: modelData.value
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: moreInfoDialog.accept()
            }
        }
    }

    // ── The Software Update dialog ────────────────────────────────────────
    Dialog {
        id: updateDialog
        objectName: "generalUpdateDialog"
        title: qsTr("Software Update")
        dismissible: true

        contentData: Column {
            width: 380
            spacing: Theme.primitive.spacing.md

            Text {
                id: updateStatusText
                objectName: "generalUpdateStatus"
                width: parent.width
                text: root.updateLabel
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                font.weight: Theme.primitive.font.weightMedium
                wrapMode: Text.WordWrap
            }

            Text {
                objectName: "generalUpdateMessage"
                width: parent.width
                visible: root.updateMessage.length > 0
                text: root.updateMessage
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                objectName: "generalUpdateProgress"
                width: parent.width
                visible: root.busy
                text: qsTr("Working…")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                id: noProviderNote
                objectName: "generalUpdateNoProvider"
                width: parent.width
                visible: !root.providerAvailable
                text: qsTr("No update provider is available on this system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsGroup {
                width: parent.width
                reserveBottomMargin: false
                visible: root.providerAvailable && root.updates.length > 0

                Repeater {
                    id: updatesRepeater
                    model: root.updates
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

            SettingsGroup {
                width: parent.width
                title: qsTr("Automatic Updates")
                reserveBottomMargin: false
                visible: root.providerAvailable

                SettingsRow {
                    width: parent.width
                    label: qsTr("Check for updates automatically")
                    description: qsTr("Never enabled unless you turn it on here.")
                    controlData: Toggle {
                        id: autoCheckToggle
                        objectName: "generalUpdateAutoCheck"
                        checked: Settings.values["updates.auto"] === true
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
                    showSeparator: false
                    controlData: Toggle {
                        id: autoInstallToggle
                        objectName: "generalUpdateAutoInstall"
                        checked: Settings.values["updates.autoInstall"] === true
                        enabled: Settings.values["updates.auto"] === true
                        accessibleName: qsTr("Install updates automatically")
                        onToggled: (checked) => Settings.set(
                            "updates.autoInstall", checked)
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                id: checkButton
                objectName: "generalUpdateCheck"
                text: qsTr("Check for Updates")
                enabled: root.providerAvailable && !root.busy
                onClicked: Settings.checkUpdates()
            }

            Button {
                id: installButton
                objectName: "generalUpdateInstall"
                text: qsTr("Install")
                variant: "primary"
                enabled: root.providerAvailable && !root.busy
                    && root.updateCount > 0
                onClicked: Settings.installUpdates()
            }

            Button {
                id: restartButton
                objectName: "generalUpdateRestart"
                text: qsTr("Restart")
                variant: "primary"
                visible: root.rebootRequired
                onClicked: Settings.rebootUpdates()
            }

            Button {
                text: qsTr("Done")
                onClicked: updateDialog.accept()
            }
        }
    }

    Component { id: loginComponent; LoginItemsPane { } }
    Component { id: languageComponent; LanguageRegionPane { } }
    Component { id: dateTimeComponent; DateTimePane { } }
    Component { id: sharingComponent; SharingPane { } }
    Component { id: backupComponent; BackupPane { } }
    Component { id: startupComponent; StartupDiskPane { } }
    Component { id: resetComponent; ResetPane { } }
}
