// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Stand-in for the compositor's server-side-decoration titlebar (T-13). It is
// a deliberately independent renderer that consumes the same component and
// semantic tokens as the first-party `TitleBar`. The FR-3 screenshot diff
// renders both and requires them to match, which is what makes the
// "unable to drift apart" rule testable before T-13 exists. When the real
// compositor renderer lands it replaces this reference in the diff.
Item {
    id: root

    property string title: ""
    property bool active: true

    implicitWidth: 360
    implicitHeight: Theme.controls.titlebar.height

    Item {
        id: chrome
        anchors.fill: parent
        opacity: Theme.material.chromeOpacity

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.titlebar.cornerRadius
            color: Theme.color.chrome
            antialiasing: true
        }
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: Theme.controls.titlebar.cornerRadius
            color: Theme.color.chrome
        }
    }

    Row {
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.trafficLights.inset
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.controls.trafficLights.gap

        Repeater {
            model: [Theme.color.close, Theme.color.minimize, Theme.color.zoom]
            delegate: Rectangle {
                required property var modelData
                width: Theme.controls.trafficLights.diameter
                height: width
                radius: width / 2
                color: root.active ? modelData : Theme.color.controlActive
                antialiasing: true
            }
        }
    }

    Text {
        anchors.centerIn: parent
        width: Math.max(0, parent.width - 2 * (Theme.controls.trafficLights.inset
                       + Theme.controls.trafficLights.diameter * 3
                       + Theme.controls.trafficLights.gap * 2
                       + Theme.controls.titlebar.spacing))
        text: root.title
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideMiddle
        color: root.active ? Theme.color.textPrimary : Theme.color.textTertiary
        font.pixelSize: Theme.controls.titlebar.fontSize
        font.weight: Theme.controls.titlebar.fontWeight
    }
}
