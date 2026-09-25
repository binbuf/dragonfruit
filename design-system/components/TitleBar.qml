// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The first-party titlebar. It owns no windowing behavior: it reports intent
// (move / zoom / window-menu) so the host application can forward it to the
// compositor over the private protocol. The compositor's SSD renderer draws
// from the same tokens (FR-3), so the two cannot drift apart.
Item {
    id: root

    property string title: ""
    property bool active: true
    property bool showTrafficLights: true
    property alias trafficLights: lights
    property alias hovered: dragArea.containsMouse

    signal closeRequested()
    signal minimizeRequested()
    signal zoomRequested()
    signal menuRequested(real x, real y)
    signal moveRequested(real x, real y)

    implicitWidth: 360
    implicitHeight: Theme.controls.titlebar.height

    // A titlebar spans its window: when it is parented into a chrome slot
    // (e.g. `AppWindow.titleBarData`, whose slot is the window width) it fills
    // the parent width, so the drag area, centered title, and traffic-light
    // cluster cover the whole top strip. A standalone instance keeps its
    // implicit width; an explicit `width` still overrides this.
    width: parent ? parent.width : implicitWidth

    // Top corners follow the window radius; the bottom edge is square so the
    // titlebar reads as one sheet with the content below. The two rectangles
    // are composited inside one item so the shared opacity does not stack.
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

    MouseArea {
        id: dragArea
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        hoverEnabled: true
        onPressed: (mouse) => {
            if (mouse.button === Qt.LeftButton)
                root.moveRequested(mouse.x, mouse.y);
        }
        onDoubleClicked: root.zoomRequested()
        onClicked: (mouse) => {
            if (mouse.button === Qt.RightButton)
                root.menuRequested(mouse.x, mouse.y);
        }
    }

    TrafficLights {
        id: lights
        visible: root.showTrafficLights
        active: root.active
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.trafficLights.inset
        anchors.verticalCenter: parent.verticalCenter

        onCloseClicked: root.closeRequested()
        onMinimizeClicked: root.minimizeRequested()
        onZoomClicked: root.zoomRequested()
    }

    Text {
        anchors.centerIn: parent
        width: Math.max(0, parent.width - 2 * (Theme.controls.trafficLights.inset
                       + (lights.visible ? lights.width + Theme.controls.titlebar.spacing : 0)))
        text: root.title
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideMiddle
        color: root.active ? Theme.color.textPrimary : Theme.color.textTertiary
        font.pixelSize: Theme.controls.titlebar.fontSize
        font.weight: Theme.controls.titlebar.fontWeight

        Behavior on color {
            ColorAnimation {
                duration: Theme.motion.hover.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.hover.curve
            }
        }
    }

    Accessible.role: Accessible.TitleBar
    Accessible.name: root.title
}
