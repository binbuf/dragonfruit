// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Window
import Dragonfruit
import Dragonfruit.Settings

// Canonical machine identity in Settings → General → About (T269).
//
// The page reads the host identity from the Updates view (the host identity is
// read even when the distribution update provider is absent), the filesystem
// usage from the General view, and the display name/resolution from the live
// screen. Missing host values say "Not available" and carry the reason in the
// accessible description. System Report is a real sheet, not an inline toggle.
Item {
    id: root
    width: parent ? parent.width : 480
    implicitHeight: page.implicitHeight

    property var identity: Settings.updates
    property string productName: qsTr("Dragonfruit System")
    property bool reportOpen: false
    property bool licensesOpen: false
    property bool regulatoryOpen: false
    property alias reportDialog: reportDialog
    readonly property var storage: Settings.general.storage || ({})
    readonly property var storageVolume: {
        if (root.storage.volumes === undefined || root.storage.volumes.length === 0)
            return ({});
        for (var i = 0; i < root.storage.volumes.length; ++i) {
            if (String(root.storage.volumes[i].mountPoint || "") === "/")
                return root.storage.volumes[i];
        }
        return root.storage.volumes[0];
    }
    readonly property string volumeName: {
        const candidates = [root.storageVolume.label, root.storageVolume.mountPoint,
                            root.storageVolume.device];
        for (const candidate of candidates) {
            if (candidate !== undefined && candidate !== null
                    && String(candidate).trim().length > 0)
                return String(candidate);
        }
        return qsTr("Not available");
    }
    readonly property real totalBytes: Number(root.storage.totalBytes
        || root.storageVolume.totalBytes || root.identity.diskBytes || 0)
    readonly property real usedBytes: Number(root.storage.usedBytes
        || root.storageVolume.usedBytes || 0)
    readonly property real freeBytes: Number(root.storage.freeBytes
        || root.storageVolume.availableBytes || 0)
    readonly property string totalLabel: root.totalBytes > 0
        ? root.formatBytes(root.totalBytes) : qsTr("Not available")
    readonly property string usedLabel: root.usedBytes > 0
        ? root.formatBytes(root.usedBytes) : qsTr("Not available")
    readonly property string freeLabel: root.freeBytes > 0
        ? root.formatBytes(root.freeBytes) : qsTr("Not available")
    readonly property string displayName: {
        const name = String(Screen.name || "").trim();
        return name.length > 0 ? name : qsTr("Not available");
    }
    readonly property string displayResolution: {
        const width = Number(Screen.width || 0);
        const height = Number(Screen.height || 0);
        if (width <= 0 || height <= 0)
            return qsTr("Not available");
        const ratio = Number(Screen.devicePixelRatio || 1);
        const physical = ratio > 1
            ? qsTr(" · %1 × %2 physical").arg(Math.round(width * ratio))
                .arg(Math.round(height * ratio))
            : "";
        return qsTr("%1 × %2%3").arg(Math.round(width)).arg(Math.round(height)).arg(physical);
    }

    signal navigateRequested(string paneId)

    function value(field) {
        const raw = root.identity[field];
        return raw === undefined || raw === null || String(raw).trim() === ""
                ? qsTr("Not available") : String(raw);
    }

    function reason(field) {
        const raw = root.identity[field];
        return raw === undefined || raw === null || String(raw).trim() === ""
            ? qsTr("%1 is not reported by this host.").arg(field) : "";
    }

    function formatBytes(bytes) {
        const value = Number(bytes || 0);
        if (value <= 0)
            return qsTr("Not available");
        const gib = value / (1024 * 1024 * 1024);
        return (gib >= 10 ? Math.round(gib) : Math.round(gib * 10) / 10) + " GiB";
    }

    function openReport() {
        root.reportOpen = true;
        reportDialog.open = true;
    }

    Column {
        id: page
        width: parent.width
        spacing: 18

        Row {
            width: parent.width; height: 140; spacing: 20
            AboutHero { objectName: "aboutPageHero"; artworkSize: 128 }
            Column {
                width: parent.width - 148
                spacing: 6
                Text {
                    id: deviceNameText
                    objectName: "aboutPageName"
                    // The map row `about.deviceName` lands on this rendered
                    // field; the label is the row label, not the value.
                    property string settingsId: "about.deviceName"
                    property string label: qsTr("Device name")
                    property bool highlighted: false
                    width: parent.width
                    text: root.value("deviceName")
                    color: deviceNameText.highlighted ? Theme.color.accent
                                                      : Theme.color.textPrimary
                    font.pixelSize: 24; font.weight: Font.DemiBold
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    Accessible.description: root.reason("deviceName")
                    function focusFirstControl() { forceActiveFocus(); }
                }
                Text {
                    objectName: "aboutPageProduct"
                    width: parent.width; elide: Text.ElideRight
                    text: root.productName; color: Theme.color.textSecondary; font.pixelSize: 13
                }
                Text {
                    objectName: "aboutPageModel"
                    width: parent.width; elide: Text.ElideRight
                    text: qsTr("Model: %1").arg(root.value("deviceModel"))
                    color: Theme.color.textSecondary; font.pixelSize: 12
                    Accessible.description: root.reason("deviceModel")
                }
                Text {
                    objectName: "aboutPageProcessor"
                    width: parent.width; elide: Text.ElideRight
                    text: qsTr("Processor: %1").arg(root.value("processor"))
                    color: Theme.color.textSecondary; font.pixelSize: 12
                    Accessible.description: root.reason("processor")
                }
                Text {
                    objectName: "aboutPageMemory"
                    width: parent.width; elide: Text.ElideRight
                    text: qsTr("Memory: %1").arg(root.value("memoryLabel"))
                    color: Theme.color.textSecondary; font.pixelSize: 12
                    Accessible.description: root.reason("memoryLabel")
                }
                Text {
                    objectName: "aboutPageGraphics"
                    width: parent.width; elide: Text.ElideRight
                    text: qsTr("Graphics: %1").arg(root.value("gpuLabel"))
                    color: Theme.color.textSecondary; font.pixelSize: 12
                    Accessible.description: root.reason("gpuLabel")
                }
                Text {
                    objectName: "aboutPageSerial"
                    width: parent.width; elide: Text.ElideRight
                    text: qsTr("Serial number: %1").arg(root.identity.hasSerial === true
                        ? root.value("serial") : qsTr("Not available"))
                    color: Theme.color.textSecondary; font.pixelSize: 12
                    Accessible.description: root.identity.hasSerial === true
                        ? "" : qsTr("The host does not report a serial number.")
                }
            }
        }

        Rectangle { width: parent.width; height: 1; color: Theme.color.separator }

        Text {
            width: parent.width
            text: qsTr("Operating System")
            color: Theme.color.textPrimary
            font.pixelSize: 17
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }
        Text {
            objectName: "aboutPageOs"
            width: parent.width
            text: root.value("osLabel")
            color: Theme.color.textPrimary
            font.pixelSize: 14
            wrapMode: Text.WordWrap
        }
        Text {
            objectName: "aboutPageKernel"
            width: parent.width
            elide: Text.ElideRight
            text: qsTr("Kernel: %1 · Architecture: %2")
                .arg(root.value("kernel")).arg(root.value("architecture"))
            color: Theme.color.textSecondary
            font.pixelSize: 12
        }
        Text {
            width: parent.width
            text: qsTr("Version and compatibility details depend on the installed distribution.")
            color: Theme.color.textSecondary
            font.pixelSize: 12
            wrapMode: Text.WordWrap
        }

        Rectangle { width: parent.width; height: 1; color: Theme.color.separator }

        Text {
            width: parent.width
            text: qsTr("Displays")
            color: Theme.color.textPrimary
            font.pixelSize: 17
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }
        Text {
            objectName: "aboutPageDisplay"
            width: parent.width
            elide: Text.ElideRight
            text: qsTr("%1 · %2").arg(root.displayName).arg(root.displayResolution)
            color: Theme.color.textSecondary
            font.pixelSize: 13
            Accessible.description: root.displayName === qsTr("Not available")
                ? qsTr("The compositor does not report an output name.") : ""
        }
        Button {
            text: qsTr("Display Settings…")
            onClicked: root.navigateRequested("displays")
        }

        Rectangle { width: parent.width; height: 1; color: Theme.color.separator }

        Text {
            width: parent.width
            text: qsTr("Storage")
            color: Theme.color.textPrimary
            font.pixelSize: 17
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }
        Text {
            objectName: "aboutPageVolume"
            width: parent.width
            elide: Text.ElideRight
            text: qsTr("Volume: %1").arg(root.volumeName)
            color: Theme.color.textSecondary
            font.pixelSize: 13
        }
        Text {
            objectName: "aboutPageStorage"
            width: parent.width
            elide: Text.ElideRight
            text: qsTr("%1 of %2 used · %3 available")
                .arg(root.usedLabel).arg(root.totalLabel).arg(root.freeLabel)
            color: Theme.color.textSecondary
            font.pixelSize: 13
        }
        Button {
            text: qsTr("Storage Settings…")
            onClicked: root.navigateRequested("storage")
        }

        Rectangle { width: parent.width; height: 1; color: Theme.color.separator }

        Row {
            spacing: 10
            Button {
                id: systemReportButton
                objectName: "aboutPageSystemReport"
                property string settingsId: "about.moreInfo"
                property string label: qsTr("More Info…")
                text: qsTr("System Report…")
                onClicked: root.openReport()
            }
            Button {
                text: qsTr("Licenses…")
                variant: "ghost"
                onClicked: root.licensesOpen = !root.licensesOpen
            }
            Button {
                text: qsTr("Regulatory information…")
                variant: "ghost"
                onClicked: root.regulatoryOpen = !root.regulatoryOpen
            }
        }
        Text {
            visible: root.licensesOpen
            width: parent.width
            text: qsTr("Dragonfruit source files use their declared open source licenses; installed host packages retain their own licenses.")
            color: Theme.color.textSecondary
            wrapMode: Text.WordWrap
        }
        Text {
            visible: root.regulatoryOpen
            width: parent.width
            text: qsTr("No regulatory certification data is available from this host.")
            color: Theme.color.textSecondary
            wrapMode: Text.WordWrap
        }
        Text {
            width: parent.width
            elide: Text.ElideRight
            text: qsTr("Dragonfruit Linux · Open source system software")
            color: Theme.color.textTertiary
            font.pixelSize: 11
        }
        Item { width: 1; height: 16 }
    }

    // ── The System Report sheet ──────────────────────────────────────────
    Dialog {
        id: reportDialog
        objectName: "aboutSystemReportDialog"
        title: qsTr("System Report")
        dismissible: true

        contentData: Column {
            width: 460
            spacing: Theme.primitive.spacing.sm

            SettingsGroup {
                width: parent.width
                reserveBottomMargin: false

                SettingsRow {
                    objectName: "aboutReportHostName"
                    width: parent.width
                    label: qsTr("Computer name")
                    controlData: Text {
                        text: root.value("hostName")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportModel"
                    width: parent.width
                    label: qsTr("Model")
                    controlData: Text {
                        text: root.value("deviceModel")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportProcessor"
                    width: parent.width
                    label: qsTr("Processor")
                    controlData: Text {
                        text: root.value("processor")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportMemory"
                    width: parent.width
                    label: qsTr("Memory")
                    controlData: Text {
                        text: root.value("memoryLabel")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportGraphics"
                    width: parent.width
                    label: qsTr("Graphics")
                    controlData: Text {
                        text: root.value("gpuLabel")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportSerial"
                    width: parent.width
                    label: qsTr("Serial number")
                    controlData: Text {
                        text: root.identity.hasSerial === true
                            ? root.value("serial") : qsTr("Not available")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportOs"
                    width: parent.width
                    label: qsTr("Operating system")
                    controlData: Text {
                        text: root.value("osLabel")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportKernel"
                    width: parent.width
                    label: qsTr("Kernel")
                    controlData: Text {
                        text: root.value("kernel")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportArchitecture"
                    width: parent.width
                    label: qsTr("Architecture")
                    controlData: Text {
                        text: root.value("architecture")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportDisplay"
                    width: parent.width
                    label: qsTr("Display")
                    controlData: Text {
                        text: qsTr("%1 · %2").arg(root.displayName)
                            .arg(root.displayResolution)
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
                SettingsRow {
                    objectName: "aboutReportStorage"
                    width: parent.width
                    label: qsTr("Storage")
                    showSeparator: false
                    controlData: Text {
                        text: qsTr("%1 · %2 of %3 used")
                            .arg(root.volumeName).arg(root.usedLabel)
                            .arg(root.totalLabel)
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
                onClicked: {
                    reportDialog.accept();
                    root.reportOpen = false;
                }
            }
        }
        onClosed: root.reportOpen = false
    }
}
