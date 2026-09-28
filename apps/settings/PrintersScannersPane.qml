// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Printers & Scanners pane (T-15.12b).
//
// The pane and the Control Center tile are one functional unit: both read the
// same host-stack adapter (CUPS queues + SANE devices) through the bridge host.
// The pane mirrors the macOS pane (SystemSettings_PrintersScanners.md /
// System_Preferences.md): a `Default printer` popup, a `Default paper size`
// popup, a `Printers` list whose rows open a per-printer detail, and a scanner
// list. The three queue writes (set default, accept/reject jobs, cancel a job)
// go through the `Settings` singleton to the adapter — never to D-Bus from QML.
// The paper size is the one settingsd key the pane owns (ADR 0141); CUPS and
// SANE own their own state.
//
// Deviations from the macOS capture reference (ADR 0122), recorded so later
// tasks do not re-litigate them:
//   * `Add Printer, Scanner, or Fax...` is not shown: there is no CUPS
//     discovery seam in the adapter yet, and a disabled button would be a dead
//     control (follow-up).
//   * the AirPrint wording is dropped; CUPS discovers the same printers over
//     generic IPP/DNS-SD.
//   * the trailing `?` help is omitted project-wide.
//   * `Last Printer Used` appears as the popup's value when CUPS names no
//     system default; clearing a default back to it is not a CUPS client-tool
//     operation, so it is shown as state, not offered as a choice.
Item {
    id: root

    // The bridge host's Printers and Scanners view (empty when absent).
    readonly property var view: Settings.printers
    readonly property bool available: Settings.printersAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    // The two halves degrade independently.
    readonly property bool printersAvailable: root.view.printersAvailable === true
    readonly property bool scannersAvailable: root.view.scannersAvailable === true
    readonly property var printers: root.view.printers !== undefined ? root.view.printers : []
    readonly property var scanners: root.view.scanners !== undefined ? root.view.scanners : []
    readonly property string defaultPrinter: root.view.defaultPrinter !== undefined
        ? String(root.view.defaultPrinter) : ""

    // The printer whose detail dialog is open, keyed by name and derived from
    // the live view so a write's round-trip converges in the open dialog.
    property string selectedPrinterName: ""
    readonly property var selectedPrinter: root.selectedPrinterName.length > 0
        ? root.findPrinter(root.selectedPrinterName) : null

    // Test surface (used by tst_settings_printers.qml).
    property alias defaultSelect: defaultSelect
    property alias paperSizeSelect: paperSizeSelect
    property alias printersGroup: printersGroup
    property alias printerRepeater: printerRepeater
    property alias noPrintersNote: noPrintersNote
    property alias scannersGroup: scannersGroup
    property alias scannerRepeater: scannerRepeater
    property alias noScannersNote: noScannersNote
    property alias scannerAbsentNote: scannerAbsentNote
    property alias absenceNote: absenceNote
    property alias printerDialog: printerDialog
    property alias acceptJobsToggle: acceptJobsToggle
    property alias setDefaultButton: setDefaultButton
    property alias jobsRepeater: jobsRepeater
    property alias noJobsNote: noJobsNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    // The paper-size choices (the stable ids of the `printers.defaultPaperSize`
    // schema key).
    readonly property var paperSizeModel: [
        { label: qsTr("US Letter"), value: "us-letter" },
        { label: qsTr("US Legal"), value: "us-legal" },
        { label: qsTr("A3"), value: "a3" },
        { label: qsTr("A4"), value: "a4" },
        { label: qsTr("A5"), value: "a5" }
    ]

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshPrinters()

    function findPrinter(name) {
        for (var i = 0; i < root.printers.length; ++i)
            if (root.printers[i].name === name)
                return root.printers[i];
        return null;
    }

    // The `Default printer` popup's current selection. When CUPS names no
    // default the first entry is `Last Printer Used` (value empty).
    function defaultIndex() {
        for (var i = 0; i < root.printers.length; ++i)
            if (root.printers[i].name === root.defaultPrinter)
                return i;
        return 0;
    }

    function paperSizeIndex() {
        var current = String(Settings.values["printers.defaultPaperSize"] || "us-letter");
        for (var i = 0; i < root.paperSizeModel.length; ++i)
            if (root.paperSizeModel[i].value === current)
                return i;
        return 0;
    }

    function stateColor(state) {
        if (state === "idle")
            return Theme.color.success;
        if (state === "processing")
            return Theme.color.accent;
        if (state === "stopped")
            return Theme.color.warning;
        return Theme.color.textTertiary;
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Default printer / Default paper size ──────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.ready

            SettingsRow {
                objectName: "defaultPrinterRow"
                width: parent.width
                label: qsTr("Default printer")
                controlData: Select {
                    id: defaultSelect
                    objectName: "defaultPrinterSelect"
                    accessibleName: qsTr("Default printer")
                    enabled: root.printersAvailable && root.printers.length > 0
                    model: {
                        var entries = [];
                        if (root.defaultPrinter.length === 0)
                            entries.push({ label: qsTr("Last Printer Used"), value: "" });
                        for (var i = 0; i < root.printers.length; ++i)
                            entries.push({
                                label: root.printers[i].displayName,
                                value: root.printers[i].name
                            });
                        return entries;
                    }
                    onSelected: (value) => {
                        if (String(value).length > 0)
                            Settings.setDefaultPrinter(String(value));
                    }

                    Binding {
                        target: defaultSelect
                        property: "currentIndex"
                        value: root.defaultIndex()
                    }
                }
            }

            SettingsRow {
                objectName: "defaultPaperSizeRow"
                width: parent.width
                label: qsTr("Default paper size")
                showSeparator: false
                controlData: Select {
                    id: paperSizeSelect
                    objectName: "paperSizeSelect"
                    accessibleName: qsTr("Default paper size")
                    model: root.paperSizeModel
                    onSelected: (value) => Settings.set("printers.defaultPaperSize", value)

                    Binding {
                        target: paperSizeSelect
                        property: "currentIndex"
                        value: root.paperSizeIndex()
                    }
                }
            }
        }

        // ── Printers ──────────────────────────────────────────────────────
        SettingsGroup {
            id: printersGroup
            width: parent.width
            title: qsTr("Printers")
            visible: root.ready
            reserveBottomMargin: false

            Text {
                id: noPrintersNote
                objectName: "printersNoPrinters"
                width: parent.width
                visible: root.printersAvailable && root.printers.length === 0
                text: qsTr("No printers are configured.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Text {
                id: cupsAbsentNote
                objectName: "printersCupsAbsent"
                width: parent.width
                visible: !root.printersAvailable
                text: qsTr("Printer support is unavailable because CUPS is not running.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: printerRepeater
                model: root.printers
                delegate: Item {
                    id: printerRow
                    required property var modelData
                    required property int index
                    readonly property var printer: modelData
                    width: parent ? parent.width : 0
                    height: Math.max(Theme.controls.settingsRow.height, 48)
                    Accessible.role: Accessible.Grouping
                    Accessible.name: printerRow.printer.displayName

                    Icon {
                        id: printerIcon
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        name: "printer"
                        size: 22
                        color: Theme.color.textSecondary
                    }

                    Column {
                        anchors.left: printerIcon.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: printerChevron.left
                        anchors.rightMargin: Theme.controls.settingsRow.controlGap
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            objectName: "printerNameText"
                            width: parent.width
                            text: printerRow.printer.displayName
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }

                        Row {
                            width: parent.width
                            spacing: Theme.primitive.spacing.xs

                            Rectangle {
                                objectName: "printerStateDot"
                                anchors.verticalCenter: parent.verticalCenter
                                width: 8
                                height: 8
                                radius: 4
                                color: root.stateColor(String(printerRow.printer.state))
                            }

                            Text {
                                objectName: "printerStateText"
                                text: {
                                    var message = printerRow.printer.stateMessage !== undefined
                                        ? String(printerRow.printer.stateMessage) : "";
                                    var label = printerRow.printer.stateLabel !== undefined
                                        ? String(printerRow.printer.stateLabel) : "";
                                    var text = message.length > 0 ? message : label;
                                    if (printerRow.printer.isDefault === true)
                                        text = text.length > 0
                                            ? qsTr("%1 \u00b7 Default").arg(text)
                                            : qsTr("Default");
                                    return text;
                                }
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }
                        }
                    }

                    Icon {
                        id: printerChevron
                        objectName: "printerDisclosure"
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        name: "chevron-right"
                        size: 14
                        color: Theme.color.textTertiary
                    }

                    MouseArea {
                        objectName: "printerRowArea"
                        anchors.fill: parent
                        onClicked: {
                            root.selectedPrinterName = String(printerRow.printer.name);
                            root.printerDialog.show();
                        }
                    }

                    Rectangle {
                        visible: printerRow.index < root.printers.length - 1
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.bottom: parent.bottom
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                    }
                }
            }
        }

        // ── Scanners ──────────────────────────────────────────────────────
        SettingsGroup {
            id: scannersGroup
            width: parent.width
            title: qsTr("Scanners")
            visible: root.ready
            reserveBottomMargin: false

            Text {
                id: scannerAbsentNote
                objectName: "scannersAbsent"
                width: parent.width
                visible: !root.scannersAvailable
                text: qsTr("Scanner support is unavailable because SANE is not running.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Text {
                id: noScannersNote
                objectName: "scannersNone"
                width: parent.width
                visible: root.scannersAvailable && root.scanners.length === 0
                text: qsTr("No scanners were found.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: scannerRepeater
                model: root.scanners
                delegate: Item {
                    id: scannerRow
                    required property var modelData
                    required property int index
                    readonly property var scanner: modelData
                    width: parent ? parent.width : 0
                    height: Math.max(Theme.controls.settingsRow.height, 48)
                    Accessible.role: Accessible.Grouping
                    Accessible.name: scannerRow.scanner.displayName

                    Icon {
                        id: scannerIcon
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        name: "scanner"
                        size: 22
                        color: Theme.color.textSecondary
                    }

                    Column {
                        anchors.left: scannerIcon.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            objectName: "scannerNameText"
                            width: parent.width
                            text: scannerRow.scanner.displayName
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }

                        Text {
                            objectName: "scannerKindText"
                            width: parent.width
                            text: scannerRow.scanner.kindLabel !== undefined
                                ? String(scannerRow.scanner.kindLabel) : ""
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Rectangle {
                        visible: scannerRow.index < root.scanners.length - 1
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.bottom: parent.bottom
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                    }
                }
            }
        }

        // ── Host absent ───────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            Text {
                id: absenceNote
                objectName: "printersAbsenceNote"
                width: parent.width
                text: qsTr("The printing service is not running, so printers and "
                           + "scanners are unavailable.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }

    // ── The per-printer detail dialog ─────────────────────────────────────
    Dialog {
        id: printerDialog
        objectName: "printerDialog"
        title: root.selectedPrinter !== null
            ? String(root.selectedPrinter.displayName) : qsTr("Printer")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            SettingsGroup {
                width: parent.width
                reserveBottomMargin: false

                SettingsRow {
                    objectName: "printerStateRow"
                    width: parent.width
                    label: qsTr("Status")
                    description: root.selectedPrinter !== null
                        && String(root.selectedPrinter.location).length > 0
                        ? String(root.selectedPrinter.location) : ""
                    controlData: Text {
                        objectName: "printerStateValue"
                        text: {
                            if (root.selectedPrinter === null)
                                return "";
                            var message = root.selectedPrinter.stateMessage !== undefined
                                ? String(root.selectedPrinter.stateMessage) : "";
                            var label = root.selectedPrinter.stateLabel !== undefined
                                ? String(root.selectedPrinter.stateLabel) : "";
                            return message.length > 0 ? message : label;
                        }
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }

                SettingsRow {
                    objectName: "printerModelRow"
                    width: parent.width
                    label: qsTr("Model")
                    visible: root.selectedPrinter !== null
                        && String(root.selectedPrinter.makeAndModel).length > 0
                    controlData: Text {
                        objectName: "printerModelValue"
                        text: root.selectedPrinter !== null
                            ? String(root.selectedPrinter.makeAndModel) : ""
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }

                SettingsRow {
                    objectName: "printerAcceptJobsRow"
                    width: parent.width
                    label: qsTr("Accept New Jobs")
                    description: qsTr("Pause the queue by rejecting new jobs.")
                    showSeparator: false
                    controlData: Toggle {
                        id: acceptJobsToggle
                        objectName: "printerAcceptJobsToggle"
                        accessibleName: qsTr("Accept New Jobs")
                        onToggled: (checked) => {
                            if (root.selectedPrinter !== null)
                                Settings.setPrinterAcceptingJobs(
                                    String(root.selectedPrinter.name), checked);
                        }

                        Binding {
                            target: acceptJobsToggle
                            property: "checked"
                            value: root.selectedPrinter !== null
                                && root.selectedPrinter.acceptingJobs === true
                        }
                    }
                }
            }

            // ── The queue's jobs ──────────────────────────────────────────
            SettingsGroup {
                width: parent.width
                title: qsTr("Jobs")
                reserveBottomMargin: false

                Text {
                    id: noJobsNote
                    objectName: "printerNoJobs"
                    width: parent.width
                    visible: root.selectedPrinter !== null
                        && root.selectedPrinter.jobCount === 0
                    text: qsTr("No jobs are waiting.")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    topPadding: Theme.primitive.spacing.xs
                    bottomPadding: Theme.primitive.spacing.xs
                }

                Repeater {
                    id: jobsRepeater
                    model: root.selectedPrinter !== null
                        && root.selectedPrinter.jobs !== undefined
                        ? root.selectedPrinter.jobs : []
                    delegate: SettingsRow {
                        id: jobRow
                        required property var modelData
                        required property int index
                        width: parent.width
                        label: qsTr("Job #%1").arg(jobRow.modelData.id)
                        description: qsTr("%1 \u00b7 %2 bytes")
                            .arg(String(jobRow.modelData.user))
                            .arg(Number(jobRow.modelData.size))
                        showSeparator: true
                        controlData: Button {
                            id: cancelJobButton
                            objectName: "cancelJobButton"
                            text: qsTr("Cancel")
                            variant: "ghost"
                            accessibleName: qsTr("Cancel job %1").arg(jobRow.modelData.id)
                            onClicked: Settings.cancelPrinterJob(Number(jobRow.modelData.id))
                        }
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                id: setDefaultButton
                objectName: "setDefaultPrinterButton"
                text: qsTr("Set as Default Printer")
                enabled: root.selectedPrinter !== null
                    && root.selectedPrinter.isDefault !== true
                onClicked: {
                    if (root.selectedPrinter !== null)
                        Settings.setDefaultPrinter(String(root.selectedPrinter.name));
                }
            }

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: printerDialog.accept()
            }
        }
    }
}