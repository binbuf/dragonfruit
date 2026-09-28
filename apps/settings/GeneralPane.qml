// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The General pane (T-15.10b).
//
// The pane and the Control Center Software Update tile are one functional unit:
// both read the same host-stack adapter through the bridge host. General
// mirrors the macOS pane as grouped disclosure rows — `About` and `Software
// Update` — each opening the matching dialog, because that is the reference
// anatomy (SystemSettings_General.md / AboutDialog.md). The durable preferences
// are none: About is a live host read and the update writes are explicit
// actions over the adapter, so every control applies live through `Settings`.
//
// Deviations from the macOS capture reference (ADR 0122), recorded so later
// tasks do not re-litigate them:
//   * `Storage` is a first-class sidebar pane in Dragonfruit (T-15.2b), so it
//     is not repeated as a General row.
//   * the other General rows (`AutoFill & Passwords`, `Date & Time`, `Language
//     & Region`, `Login Items & Extensions`, `Sharing`, `Startup Disk`) have no
//     provider yet; they are omitted rather than shipped as dead rows.
//   * the About illustration is our own `computer` glyph, and the `Chip` value
//     is the host's processor (never `Apple M5`); `macOS` reads the host's own
//     OS name/version (`osLabel`).
//   * the `More Info...` button and the `Regulatory Certification` / copyright
//     footer are omitted: there is no System Information app or legal page to
//     open, so they would be dead controls.
//   * the trailing `?` help is omitted project-wide.
Item {
    id: root

    // The bridge host's General/About/Updates view (empty when absent).
    readonly property var view: Settings.updates
    readonly property bool available: Settings.updatesAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    // The identity half is always read when the host answers.
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

    // The Software Update row's summary (the pane reflects state live).
    readonly property string updateSummary: {
        if (!root.ready)
            return qsTr("Unavailable");
        if (!root.providerAvailable)
            return qsTr("No update provider");
        return root.updateLabel;
    }

    // Test surface (used by tst_settings_general.qml).
    property alias aboutRow: aboutRow
    property alias aboutDialog: aboutDialog
    property alias aboutHeading: aboutHeading
    property alias aboutOsRow: aboutOsRow
    property alias aboutMemoryRow: aboutMemoryRow
    property alias aboutSerialRow: aboutSerialRow
    property alias updateRow: updateRow
    property alias updateDialog: updateDialog
    property alias updateStatusText: updateStatusText
    property alias updatesRepeater: updatesRepeater
    property alias noProviderNote: noProviderNote
    property alias checkButton: checkButton
    property alias installButton: installButton
    property alias restartButton: restartButton
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshUpdates()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The disclosure rows ───────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.ready

            SettingsRow {
                id: aboutRow
                objectName: "generalAboutRow"
                width: parent.width
                label: qsTr("About")
                controlData: Button {
                    icon: "chevron-right"
                    variant: "ghost"
                    accessibleName: qsTr("About This System")
                    onClicked: aboutDialog.show()
                }
            }

            SettingsRow {
                id: updateRow
                objectName: "generalUpdateRow"
                width: parent.width
                label: qsTr("Software Update")
                description: root.updateSummary
                showSeparator: false
                controlData: Button {
                    icon: "chevron-right"
                    variant: "ghost"
                    accessibleName: qsTr("Software Update")
                    onClicked: updateDialog.show()
                }
            }
        }

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

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

    // ── The Software Update dialog ────────────────────────────────────────
    Dialog {
        id: updateDialog
        objectName: "generalUpdateDialog"
        title: qsTr("Software Update")
        dismissible: true

        contentData: Column {
            width: parent.width
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
}