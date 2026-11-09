// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

Item {
    id: root
    width: 280
    height: 503
    readonly property real chromeHeight: 32
    property var identity: Settings.updates
    readonly property var startupView: Settings.general && Settings.general.startup
                                       ? Settings.general.startup : ({})
    // The root volume's label is the human name; fall back to the parent disk,
    // the root device, then the mount point so a host with a startup disk never
    // reports "Not available" just because one field is empty.
    readonly property string startupDisk: {
        if (String(root.startupView.state || "") !== "available")
            return "";
        const candidates = [root.startupView.label, root.startupView.disk,
                            root.startupView.device, root.startupView.mountPoint];
        for (const candidate of candidates) {
            const text = candidate === undefined || candidate === null
                    ? "" : String(candidate).trim();
            if (text.length > 0)
                return text;
        }
        return "";
    }
    property string productName: qsTr("Dragonfruit System")
    property bool regulatoryOpen: false
    signal moreInfoRequested()
    signal closeRequested()
    function value(field) {
        const raw = root.identity[field];
        return raw === undefined || raw === null || String(raw).trim() === ""
                ? qsTr("Not available") : String(raw);
    }

    AboutUtilityMaterial { anchors.fill: parent }
    TitleBar {
        title: ""
        style: "titlebar"
        fillColor: "transparent"
        borderColor: "transparent"
        onCloseRequested: root.closeRequested()
    }
    AboutHero {
        objectName: "aboutPanelHero"
        x: (root.width - width) / 2
        y: 29
        artworkSize: 194
    }
    Text {
        id: heading
        objectName: "aboutPanelName"
        x: 16; y: 229; width: root.width - 32; height: 28
        text: root.value("deviceName")
        color: Theme.color.textPrimary
        font.pixelSize: 22; font.weight: Font.DemiBold
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        Accessible.name: text
    }
    Text {
        x: 16; y: 259; width: root.width - 32; height: 16
        text: root.productName
        color: Theme.color.textSecondary
        font.pixelSize: 12
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
    Column {
        x: 23; y: 291; width: root.width - 46
        spacing: 0
        Repeater {
            model: [
                { label: qsTr("Chip"), value: root.value("processor") },
                { label: qsTr("Memory"), value: root.value("memoryLabel") },
                { label: qsTr("Startup disk"), value: root.startupDisk || qsTr("Not available") },
                { label: qsTr("Serial number"), value: root.identity.hasSerial === true ? root.value("serial") : qsTr("Not available") },
                { label: qsTr("OS"), value: root.value("osLabel") }
            ]
            delegate: Row {
                required property var modelData
                required property int index
                width: parent.width; height: 16; spacing: 10
                Text {
                    objectName: "aboutPanelLabel" + index
                    width: 93; height: parent.height
                    text: modelData.label
                    font.pixelSize: 11
                    color: Theme.color.textSecondary
                    horizontalAlignment: Text.AlignRight
                    verticalAlignment: Text.AlignVCenter
                }
                Text {
                    objectName: "aboutPanelValue" + index
                    width: parent.width - 103; height: parent.height
                    text: modelData.value
                    font.pixelSize: 11
                    color: Theme.color.textPrimary
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
            }
        }
    }
    Rectangle {
        objectName: "aboutPanelButtonBacking"
        x: (root.width - width) / 2; y: 391
        width: 94; height: 26
        radius: Theme.controls.button.radius
        color: Theme.color.controlFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        Button {
            id: moreInfo
            objectName: "aboutPanelMoreInfo"
            anchors.fill: parent
            variant: "ghost"
            text: qsTr("More Info…")
            onClicked: root.moreInfoRequested()
        }
    }
    Text {
        objectName: "aboutPanelRegulatory"
        x: 20; y: 429; width: root.width - 40; height: 16
        text: qsTr("Regulatory information")
        color: Theme.color.accent
        font.pixelSize: 11
        font.underline: true
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        MouseArea { anchors.fill: parent; onClicked: root.regulatoryOpen = true }
    }
    Text {
        objectName: "aboutPanelFooter"
        x: 18; y: 466; width: root.width - 36; height: 30
        text: qsTr("Dragonfruit Linux\nOpen source system software")
        color: Theme.color.textTertiary
        font.pixelSize: 10
        horizontalAlignment: Text.AlignHCenter
    }
    Rectangle {
        anchors.fill: parent
        visible: root.regulatoryOpen
        z: 10
        radius: 13
        color: Theme.color.surfaceElevated
        Text {
            x: 20; y: 150; width: parent.width - 40
            text: qsTr("Regulatory information")
            color: Theme.color.textPrimary
            font.pixelSize: 17; font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
        }
        Text {
            x: 20; y: 210; width: parent.width - 40
            text: qsTr("No regulatory certification data is available from this host.")
            color: Theme.color.textSecondary
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
        Button {
            x: (parent.width - width) / 2; y: 290
            text: qsTr("Done")
            onClicked: root.regulatoryOpen = false
        }
    }
}
