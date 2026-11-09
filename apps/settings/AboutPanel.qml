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
    property string startupDisk: Settings.general.startup && Settings.general.startup.disk
                                 ? String(Settings.general.startup.disk) : ""
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
        x: (root.width - width) / 2
        y: 29
        artworkSize: 194
    }
    Text {
        id: heading
        objectName: "aboutPanelName"
        x: 16; y: 222; width: root.width - 32; height: 32
        text: root.value("deviceName")
        color: Theme.color.textPrimary
        font.pixelSize: 22; font.weight: Font.DemiBold
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        Accessible.name: text
    }
    Text {
        x: 16; y: 254; width: root.width - 32; height: 18
        text: root.productName
        color: Theme.color.textSecondary
        font.pixelSize: 12
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
    }
    Column {
        x: 23; y: 281; width: root.width - 46
        spacing: 5
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
                width: parent.width; height: 19
                Text {
                    width: 91; height: parent.height
                    text: modelData.label
                    font.pixelSize: 11
                    color: Theme.color.textSecondary
                }
                Text {
                    objectName: "aboutPanelValue" + index
                    width: parent.width - 91; height: parent.height
                    text: modelData.value
                    font.pixelSize: 11
                    color: Theme.color.textPrimary
                    elide: Text.ElideRight
                }
            }
        }
    }
    Button {
        id: moreInfo
        objectName: "aboutPanelMoreInfo"
        x: (root.width - width) / 2; y: 404
        width: 94; height: 26
        text: qsTr("More Info…")
        onClicked: root.moreInfoRequested()
    }
    Text {
        x: 20; y: 439; width: root.width - 40; height: 18
        text: qsTr("Regulatory information")
        color: Theme.color.accent
        font.pixelSize: 11
        font.underline: true
        horizontalAlignment: Text.AlignHCenter
        MouseArea { anchors.fill: parent; onClicked: root.regulatoryOpen = true }
    }
    Text {
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
