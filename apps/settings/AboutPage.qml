// SPDX-License-Identifier: GPL-3.0-or-later
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
    // The session type is the product's own fact (the desktop is Wayland);
    // the host identity view does not repeat it.
    readonly property string sessionLabel: qsTr("Wayland")
    // Security updates are the update provider's fact. With no provider the
    // row says so instead of claiming "Automatic".
    readonly property string securityUpdatesLabel: {
        if (root.identity.updatesAvailable !== true)
            return qsTr("Not available");
        return root.identity.securityCount > 0
            ? qsTr("Included in the available updates")
            : qsTr("Automatic");
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

        // The mockup hero: original Dragonfruit artwork, the machine name, and
        // the product line. It replaces the old header card; the toolbar title
        // is the only page title.
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
                    property string label: qsTr("Name")
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
            }
        }

        SettingsGroup {
            width: parent.width

            SettingsRow {
                objectName: "aboutPageHostNameRow"
                width: parent.width
                settingsId: "about.deviceName"
                label: qsTr("Name")
                controlData: Text {
                    objectName: "aboutPageHostName"
                    text: root.value("deviceName")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                objectName: "aboutPageProcessorRow"
                width: parent.width
                settingsId: "about.hardware"
                label: qsTr("Processor")
                controlData: Text {
                    objectName: "aboutPageProcessor"
                    text: root.value("processor")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                objectName: "aboutPageMemoryRow"
                width: parent.width
                label: qsTr("Memory")
                controlData: Text {
                    objectName: "aboutPageMemory"
                    text: root.value("memoryLabel")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                objectName: "aboutPageDiskEncryptionRow"
                width: parent.width
                settingsId: "about.diskEncryption"
                label: qsTr("Disk encryption")
                description: root.identity.diskEncryption !== undefined
                    && String(root.identity.diskEncryption).length > 0
                    ? "" : qsTr("The host does not report disk encryption.")
                controlData: Text {
                    objectName: "aboutPageDiskEncryption"
                    text: root.value("diskEncryption")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                objectName: "aboutPageSecurityUpdatesRow"
                width: parent.width
                settingsId: "about.securityUpdates"
                label: qsTr("Security updates")
                controlData: Text {
                    objectName: "aboutPageSecurityUpdates"
                    text: root.securityUpdatesLabel
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Dragonfruit OS")

            SettingsRow {
                objectName: "aboutPageVersionRow"
                width: parent.width
                label: qsTr("Version")
                controlData: Text {
                    objectName: "aboutPageOs"
                    text: root.value("osLabel")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                objectName: "aboutPageKernelRow"
                width: parent.width
                label: qsTr("Kernel")
                controlData: Text {
                    objectName: "aboutPageKernel"
                    text: qsTr("%1 · %2").arg(root.value("kernel"))
                        .arg(root.value("architecture"))
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                objectName: "aboutPageSessionRow"
                width: parent.width
                settingsId: "about.session"
                label: qsTr("Session")
                controlData: Text {
                    objectName: "aboutPageSession"
                    text: root.sessionLabel
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Displays")

            SettingsRow {
                objectName: "aboutPageDisplayRow"
                width: parent.width
                leadingIcon: "displays"
                label: qsTr("Built-in display")
                description: root.displayName
                trailingAction: qsTr("Display Settings…")
                controlData: Text {
                    objectName: "aboutPageDisplay"
                    text: root.displayResolution
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
                onTrailingActionClicked: root.navigateRequested("displays")
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Storage")

            SettingsRow {
                objectName: "aboutPageStorageRow"
                width: parent.width
                leadingIcon: "storage"
                label: qsTr("System")
                description: qsTr("%1 of %2 used · %3 available")
                    .arg(root.usedLabel).arg(root.totalLabel).arg(root.freeLabel)
                trailingAction: qsTr("Storage Settings…")
                controlData: Text {
                    objectName: "aboutPageVolume"
                    text: qsTr("Volume: %1").arg(root.volumeName)
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
                onTrailingActionClicked: root.navigateRequested("storage")
            }
        }

        SettingsFooter {
            width: parent.width
            alignment: "right"

            Button {
                id: systemReportButton
                objectName: "aboutPageSystemReport"
                property string settingsId: "about.moreInfo"
                property string label: qsTr("System Report…")
                text: qsTr("System Report…")
                onClicked: root.openReport()
            }
        }

        // The mockup's about-foot links. Energy Compliance is omitted because
        // this build has no host energy document; the map records the reason.
        Row {
            width: parent.width
            spacing: Theme.primitive.spacing.md

            Text {
                objectName: "aboutFootLicenses"
                text: qsTr("Software License")
                color: Theme.color.accent
                font.pixelSize: Theme.primitive.font.sizeSm
                TapHandler { onTapped: root.licensesOpen = !root.licensesOpen }
            }
            Text {
                objectName: "aboutFootRegulatory"
                text: qsTr("Regulatory Certification")
                color: Theme.color.accent
                font.pixelSize: Theme.primitive.font.sizeSm
                TapHandler { onTapped: root.regulatoryOpen = !root.regulatoryOpen }
            }
            Text {
                objectName: "aboutFootAgreement"
                text: qsTr("License Agreement")
                color: Theme.color.accent
                font.pixelSize: Theme.primitive.font.sizeSm
                TapHandler { onTapped: root.licensesOpen = !root.licensesOpen }
            }
        }
        Text {
            visible: root.licensesOpen
            width: parent.width
            text: qsTr("Dragonfruit source files use their declared open source licenses; installed host packages retain their own licenses.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
        Text {
            visible: root.regulatoryOpen
            width: parent.width
            text: qsTr("No regulatory certification data is available from this host.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
        Text {
            objectName: "aboutPageCopyright"
            width: parent.width
            elide: Text.ElideRight
            text: qsTr("© 2026 Dragonfruit contributors. All rights reserved.")
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
