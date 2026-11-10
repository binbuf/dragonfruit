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
// and draws the measured 13 pt circles in their 16 pt AX frames at 23 pt
// centre spacing.
Item {
    id: root

    property string title: ""
    property bool active: true
    property string style: "titlebar"

    readonly property real chromeHeight: WindowStyle.chromeHeight(style)
    readonly property real titleRowHeight:
        WindowStyle.titleRowHeight(style) > 0
            ? WindowStyle.titleRowHeight(style) : root.chromeHeight
    readonly property real trafficInset:
        WindowStyle.trafficLightInset(root.titleRowHeight)

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

    Item {
        id: titleRow
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.titleRowHeight
    }

    Row {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.leftMargin: root.trafficInset
        anchors.topMargin: root.trafficInset
        spacing: WindowStyle.trafficLightGap()

        Repeater {
            model: [Theme.color.close, Theme.color.minimize, Theme.color.zoom]
            delegate: Item {
                id: slot
                required property var modelData
                width: WindowStyle.trafficLightSize
                height: width

                Rectangle {
                    anchors.centerIn: parent
                    width: WindowStyle.trafficLightDiameter
                    height: width
                    radius: width / 2
                    color: root.active ? slot.modelData : Theme.color.controlActive
                    antialiasing: true
                }
            }
        }
    }

    Text {
        anchors.centerIn: titleRow
        width: Math.max(0, titleRow.width - 2 * (root.trafficInset
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
