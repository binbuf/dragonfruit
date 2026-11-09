// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Canonical machine identity in Settings → General → About. The surrounding
// SettingsWindow owns the 740×820 unified window and its 768 pt scroll area.
Item {
    id: root
    width: parent ? parent.width : 480
    implicitHeight: page.implicitHeight
    property var identity: Settings.updates
    property string productName: qsTr("Dragonfruit System")
    property bool showDetails: false
    property bool showReport: false
    property bool showLicenses: false
    property bool showRegulatory: false
    readonly property var storage: Settings.general.storage || ({})
    readonly property string freeSpace: Number(root.storage.freeBytes) > 0
        ? qsTr("%1 GB").arg((Number(root.storage.freeBytes) / 1000000000).toLocaleString(Qt.locale(), 'f', 1))
        : qsTr("Not available")
    signal navigateRequested(string paneId)
    function value(field) {
        const raw = root.identity[field];
        return raw === undefined || raw === null || String(raw).trim() === ""
                ? qsTr("Not available") : String(raw);
    }
    Column {
        id: page
        width: parent.width
        spacing: 18
        Row {
            width: parent.width; height: 140; spacing: 20
            AboutHero { artworkSize: 128 }
            Column {
                width: parent.width - 148
                spacing: 7
                Text {
                    objectName: "aboutPageName"
                    width: parent.width
                    text: root.value("deviceName")
                    color: Theme.color.textPrimary
                    font.pixelSize: 24; font.weight: Font.DemiBold
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                }
                Text { text: root.productName; color: Theme.color.textSecondary; font.pixelSize: 13 }
                Text { text: qsTr("Chip: %1").arg(root.value("processor")); color: Theme.color.textSecondary; font.pixelSize: 12 }
                Text { text: qsTr("Memory: %1").arg(root.value("memoryLabel")); color: Theme.color.textSecondary; font.pixelSize: 12 }
                Text {
                    objectName: "aboutPageSerial"
                    text: qsTr("Serial number: %1").arg(root.identity.hasSerial === true ? root.value("serial") : qsTr("Not available"))
                    color: Theme.color.textSecondary; font.pixelSize: 12
                }
            }
        }
        Rectangle { width: parent.width; height: 1; color: Theme.color.separator }
        Text { text: qsTr("Operating System"); color: Theme.color.textPrimary; font.pixelSize: 17; font.weight: Font.DemiBold }
        Text {
            objectName: "aboutPageOs"
            width: parent.width
            text: root.value("osLabel")
            color: Theme.color.textPrimary
            font.pixelSize: 14
            wrapMode: Text.WordWrap
        }
        Text { text: qsTr("Version and compatibility details depend on the installed distribution."); color: Theme.color.textSecondary; font.pixelSize: 12; wrapMode: Text.WordWrap; width: parent.width }
        Button { text: qsTr("Details…"); onClicked: root.showDetails = !root.showDetails }
        Column {
            visible: root.showDetails
            width: parent.width; spacing: 6
            Text { text: qsTr("Kernel: %1").arg(root.value("kernel")); color: Theme.color.textSecondary }
            Text { text: qsTr("Architecture: %1").arg(root.value("architecture")); color: Theme.color.textSecondary }
            Text { objectName: "aboutPageModel"; text: qsTr("Model: %1").arg(root.value("deviceModel")); color: Theme.color.textSecondary }
            Text { text: qsTr("GPU: %1").arg(root.value("gpuLabel")); color: Theme.color.textSecondary }
        }
        Rectangle { width: parent.width; height: 1; color: Theme.color.separator }
        Text { text: qsTr("Displays"); color: Theme.color.textPrimary; font.pixelSize: 17; font.weight: Font.DemiBold }
        Text { text: qsTr("Display: Not available"); color: Theme.color.textSecondary; font.pixelSize: 13 }
        Button { text: qsTr("Display Settings…"); onClicked: root.navigateRequested("displays") }
        Rectangle { width: parent.width; height: 1; color: Theme.color.separator }
        Text { text: qsTr("Storage"); color: Theme.color.textPrimary; font.pixelSize: 17; font.weight: Font.DemiBold }
        Text {
            objectName: "aboutPageStorage"
            text: qsTr("Total: %1").arg(root.value("diskLabel"))
            color: Theme.color.textSecondary; font.pixelSize: 13
        }
        Text { text: qsTr("Free space: %1").arg(root.freeSpace); color: Theme.color.textSecondary; font.pixelSize: 13 }
        Button { text: qsTr("Storage Settings…"); onClicked: root.navigateRequested("storage") }
        Rectangle { width: parent.width; height: 1; color: Theme.color.separator }
        Button { text: qsTr("System Report…"); onClicked: root.showReport = !root.showReport }
        Column {
            visible: root.showReport
            width: parent.width; spacing: 6
            Text { text: qsTr("Computer name: %1").arg(root.value("hostName")); color: Theme.color.textSecondary }
            Text { text: qsTr("Model: %1").arg(root.value("deviceModel")); color: Theme.color.textSecondary }
            Text { text: qsTr("Kernel: %1").arg(root.value("kernel")); color: Theme.color.textSecondary }
            Text { text: qsTr("Architecture: %1").arg(root.value("architecture")); color: Theme.color.textSecondary }
            Text { text: qsTr("GPU: %1").arg(root.value("gpuLabel")); color: Theme.color.textSecondary }
        }
        Row {
            spacing: 10
            Button { text: qsTr("Licenses…"); variant: "ghost"; onClicked: root.showLicenses = !root.showLicenses }
            Button { text: qsTr("Regulatory information…"); variant: "ghost"; onClicked: root.showRegulatory = !root.showRegulatory }
        }
        Text {
            visible: root.showLicenses
            width: parent.width
            text: qsTr("Dragonfruit source files use their declared open source licenses; installed host packages retain their own licenses.")
            color: Theme.color.textSecondary; wrapMode: Text.WordWrap
        }
        Text {
            visible: root.showRegulatory
            width: parent.width
            text: qsTr("No regulatory certification data is available from this host.")
            color: Theme.color.textSecondary; wrapMode: Text.WordWrap
        }
        Text { text: qsTr("Dragonfruit Linux · Open source system software"); color: Theme.color.textTertiary; font.pixelSize: 11 }
        Item { width: 1; height: 16 }
    }
}
