// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// A scrollable viewport with token-styled scrollbars. Content is supplied
// through the default property and is parented to the Flickable's content
// item; the scrollbars are AT-SPI ScrollBar and support drag, wheel, and
// keyboard scrolling.
FocusScope {
    id: root

    default property alias contentData: flick.contentItem.data
    property alias flickable: flick
    property alias interactive: flick.interactive
    property bool scrollbarVisible: true

    readonly property real maxY: Math.max(0, flick.contentHeight - flick.height)
    readonly property real maxX: Math.max(0, flick.contentWidth - flick.width)
    readonly property real thumbHeight: flick.contentHeight > 0
        ? Math.max(Theme.controls.scrollView.minThumb,
                   flick.height * flick.height / flick.contentHeight) : 0
    readonly property real thumbWidth: flick.contentWidth > 0
        ? Math.max(Theme.controls.scrollView.minThumb,
                   flick.width * flick.width / flick.contentWidth) : 0

    implicitWidth: 320
    implicitHeight: 240
    clip: true
    activeFocusOnTab: true

    function scrollBy(dx, dy) {
        flick.contentX = Math.max(0, Math.min(root.maxX, flick.contentX + dx));
        flick.contentY = Math.max(0, Math.min(root.maxY, flick.contentY + dy));
    }

    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: contentItem.childrenRect.height
        boundsBehavior: Flickable.StopAtBounds
    }

    Rectangle {
        id: vTrack
        visible: root.scrollbarVisible && root.maxY > 0
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.rightMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.topMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.bottomMargin: Theme.controls.scrollView.scrollbarMargin
        width: Theme.controls.scrollView.scrollbarWidth
        color: "transparent"

        Rectangle {
            id: vThumb
            width: parent.width
            height: root.thumbHeight
            radius: Theme.controls.scrollView.scrollbarRadius
            color: vThumbArea.pressed ? Theme.color.accent : Theme.color.controlActive
            y: root.maxY > 0 ? (flick.contentY / root.maxY) * (vTrack.height - height) : 0

            MouseArea {
                id: vThumbArea
                anchors.fill: parent
                property real startY: 0
                property real startContentY: 0
                onPressed: (mouse) => {
                    vThumbArea.startY = vThumbArea.mapToItem(vTrack, mouse.x, mouse.y).y;
                    vThumbArea.startContentY = flick.contentY;
                }
                onPositionChanged: (mouse) => {
                    if (!vThumbArea.pressed || vTrack.height <= vThumb.height)
                        return;
                    var dy = vThumbArea.mapToItem(vTrack, mouse.x, mouse.y).y - vThumbArea.startY;
                    flick.contentY = Math.max(0, Math.min(root.maxY,
                        vThumbArea.startContentY + dy * root.maxY / (vTrack.height - vThumb.height)));
                }
            }

            Accessible.role: Accessible.ScrollBar
            Accessible.name: qsTr("Vertical scroll bar")
            Accessible.focusable: true
        }
    }

    Rectangle {
        id: hTrack
        visible: root.scrollbarVisible && root.maxX > 0
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.rightMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.bottomMargin: Theme.controls.scrollView.scrollbarMargin
        height: Theme.controls.scrollView.scrollbarWidth
        color: "transparent"

        Rectangle {
            id: hThumb
            width: root.thumbWidth
            height: parent.height
            radius: Theme.controls.scrollView.scrollbarRadius
            color: hThumbArea.pressed ? Theme.color.accent : Theme.color.controlActive
            x: root.maxX > 0 ? (flick.contentX / root.maxX) * (hTrack.width - width) : 0

            MouseArea {
                id: hThumbArea
                anchors.fill: parent
                property real startX: 0
                property real startContentX: 0
                onPressed: (mouse) => {
                    hThumbArea.startX = hThumbArea.mapToItem(hTrack, mouse.x, mouse.y).x;
                    hThumbArea.startContentX = flick.contentX;
                }
                onPositionChanged: (mouse) => {
                    if (!hThumbArea.pressed || hTrack.width <= hThumb.width)
                        return;
                    var dx = hThumbArea.mapToItem(hTrack, mouse.x, mouse.y).x - hThumbArea.startX;
                    flick.contentX = Math.max(0, Math.min(root.maxX,
                        hThumbArea.startContentX + dx * root.maxX / (hTrack.width - hThumb.width)));
                }
            }

            Accessible.role: Accessible.ScrollBar
            Accessible.name: qsTr("Horizontal scroll bar")
            Accessible.focusable: true
        }
    }

    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_Down:
            root.scrollBy(0, Theme.primitive.spacing.xxl);
            event.accepted = true;
            break;
        case Qt.Key_Up:
            root.scrollBy(0, -Theme.primitive.spacing.xxl);
            event.accepted = true;
            break;
        case Qt.Key_Right:
            root.scrollBy(Theme.primitive.spacing.xxl, 0);
            event.accepted = true;
            break;
        case Qt.Key_Left:
            root.scrollBy(-Theme.primitive.spacing.xxl, 0);
            event.accepted = true;
            break;
        case Qt.Key_PageDown:
            root.scrollBy(0, flick.height);
            event.accepted = true;
            break;
        case Qt.Key_PageUp:
            root.scrollBy(0, -flick.height);
            event.accepted = true;
            break;
        case Qt.Key_Home:
            flick.contentY = 0;
            event.accepted = true;
            break;
        case Qt.Key_End:
            flick.contentY = root.maxY;
            event.accepted = true;
            break;
        }
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Scroll view")
}
