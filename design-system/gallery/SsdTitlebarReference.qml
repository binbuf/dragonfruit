// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Stand-in for the compositor's server-side-decoration titlebar (T-13). It is
// a deliberately independent renderer that consumes the same component and
// semantic tokens as the first-party `TitleBar`. The FR-3 screenshot diff
// renders both and requires them to match, which is what makes the
// "unable to drift apart" rule testable before T-13 exists. When the real
// compositor renderer lands it replaces this reference in the diff.
//
// T230b: the reference resolves its height from the same `WindowStyle` table
// and draws the 16 pt lights at the measured 23 pt centre spacing.
Item {
    id: root

    property string title: ""
    property bool active: true
    property string style: "titlebar"

    readonly property real chromeHeight: WindowStyle.chromeHeight(style)

    implicitWidth: 360
    implicitHeight: root.chromeHeight
    visible: root.chromeHeight > 0

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
        anchors.leftMargin: WindowStyle.trafficLightLeadingInset
        anchors.verticalCenter: parent.verticalCenter
        spacing: WindowStyle.trafficLightGap()

        Repeater {
            model: [Theme.color.close, Theme.color.minimize, Theme.color.zoom]
            delegate: Rectangle {
                required property var modelData
                width: WindowStyle.trafficLightSize
                height: width
                radius: width / 2
                color: root.active ? modelData : Theme.color.controlActive
                antialiasing: true
            }
        }
    }

    Text {
        anchors.centerIn: parent
        width: Math.max(0, parent.width - 2 * (WindowStyle.trafficLightLeadingInset
                       + WindowStyle.trafficLightSize * 3
                       + WindowStyle.trafficLightGap() * 2
                       + Theme.controls.titlebar.spacing))
        text: root.title
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideMiddle
        color: root.active ? Theme.color.textPrimary : Theme.color.textTertiary
        font.pixelSize: Theme.controls.titlebar.fontSize
        font.weight: Theme.controls.titlebar.fontWeight
    }
}
