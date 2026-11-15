// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A scrollable viewport with token-styled scrollbars. Content is supplied
// through the default property and is parented to the Flickable's content
// item; the scrollbars are AT-SPI ScrollBar and support drag, wheel, track
// click, and keyboard scrolling.
//
// T266: the three `appearance.scrollBars` modes are distinct.
//   automatic      overflow while the view is focused, hovered, or moving;
//   when-scrolling overflow while the view moves or a thumb is dragged;
//   always         any overflow, at rest or not.
// `scrollBarClickBehavior` selects macOS's two track-click behaviours
// (`jump-to-spot` and `jump-to-next-page`). The horizontal path is live when
// an owner opts into `horizontalScroll`.
FocusScope {
    id: root

    default property alias contentData: flick.contentItem.data
    property alias flickable: flick
    property alias interactive: flick.interactive
    property alias verticalTrack: vTrack
    property alias verticalThumb: vThumb
    property alias horizontalTrack: hTrack
    property alias horizontalThumb: hThumb
    property alias focusRing: scrollFocusRing
    // T266: the horizontal path is live when an owner opts in. A content item
    // wider than the viewport then scrolls; the default keeps the legacy
    // vertical-only behavior so no existing surface starts scrolling sideways.
    property bool horizontalScroll: false
    property bool scrollbarVisible: true
    // T227: `appearance.scrollBars`. See the mode contract above.
    property string scrollBars: Theme.scrollBars
    // T266: reserve the window's resize corner for the grips. The scrollbars
    // end before this square, so a bottom-right resize press and a scrollbar
    // thumb never share a pointer target.
    property real cornerInset: 0
    // T266: the scroll-bar click behavior key. A track press either jumps the
    // thumb to the clicked spot or advances one viewport page.
    property string scrollBarClickBehavior: "jump-to-spot"

    readonly property bool overflowsVertically: root.maxY > 0
    readonly property bool overflowsHorizontally: root.maxX > 0
    readonly property bool scrollingActive: flick.moving
                                             || vThumbArea.pressed
                                             || hThumbArea.pressed
    readonly property bool pointerOver: scrollHover.hovered
    // T266: the three modes are distinct.
    //   automatic      overflowing and the view is focused, hovered, or moving;
    //   when-scrolling overflowing and the view is moving or a thumb is held;
    //   always         overflowing, at rest or not.
    readonly property bool showBars: {
        if (!root.scrollbarVisible)
            return false;
        if (!root.overflowsVertically && !root.overflowsHorizontally)
            return false;
        if (root.scrollBars === "always")
            return true;
        if (root.scrollBars === "when-scrolling")
            return root.scrollingActive;
        return root.activeFocus || root.pointerOver || root.scrollingActive;
    }

    readonly property real maxY: Math.max(0, flick.contentHeight - flick.height)
    readonly property real maxX: Math.max(0, flick.contentWidth - flick.width)
    readonly property real thumbHeight: flick.contentHeight > 0
        ? Math.min(flick.height, Math.max(Theme.controls.scrollView.minThumb,
                   flick.height * flick.height / flick.contentHeight)) : 0
    readonly property real thumbWidth: flick.contentWidth > 0
        ? Math.min(flick.width, Math.max(Theme.controls.scrollView.minThumb,
                   flick.width * flick.width / flick.contentWidth)) : 0
    // The public scrollbar state (T266): a value in [0, 1] and the axes that
    // currently have a bar. Tests and owners read these instead of poking the
    // internal thumb rectangles.
    readonly property real scrollBarValue:
        root.maxY > 0 ? flick.contentY / root.maxY : 0
    readonly property real horizontalScrollBarValue:
        root.maxX > 0 ? flick.contentX / root.maxX : 0
    readonly property string scrollBarOrientation:
        (root.overflowsVertically && root.overflowsHorizontally) ? "both"
      : root.overflowsHorizontally ? "horizontal" : "vertical"

    implicitWidth: 320
    implicitHeight: 240
    clip: true
    activeFocusOnTab: true

    function scrollBy(dx, dy) {
        flick.contentX = Math.max(0, Math.min(root.maxX, flick.contentX + dx));
        flick.contentY = Math.max(0, Math.min(root.maxY, flick.contentY + dy));
    }

    function clickVerticalTrack(y) {
        if (root.maxY <= 0 || vTrack.height <= vThumb.height)
            return;
        if (root.scrollBarClickBehavior === "jump-to-next-page") {
            var thumbMiddle = vThumb.y + vThumb.height / 2;
            root.scrollBy(0, y < thumbMiddle ? -flick.height : flick.height);
            return;
        }
        var travel = vTrack.height - vThumb.height;
        var target = (y - vThumb.height / 2) / travel;
        flick.contentY = Math.max(0, Math.min(root.maxY, target * root.maxY));
    }

    function clickHorizontalTrack(x) {
        if (root.maxX <= 0 || hTrack.width <= hThumb.width)
            return;
        if (root.scrollBarClickBehavior === "jump-to-next-page") {
            var thumbMiddle = hThumb.x + hThumb.width / 2;
            root.scrollBy(x < thumbMiddle ? -flick.width : flick.width, 0);
            return;
        }
        var travel = hTrack.width - hThumb.width;
        var target = (x - hThumb.width / 2) / travel;
        flick.contentX = Math.max(0, Math.min(root.maxX, target * root.maxX));
    }

    HoverHandler { id: scrollHover }

    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        contentWidth: root.horizontalScroll
                      ? Math.max(width, contentItem.childrenRect.width) : width
        contentHeight: contentItem.childrenRect.height
        boundsBehavior: Flickable.StopAtBounds
    }

    Rectangle {
        id: vTrack
        objectName: "verticalScrollTrack"
        visible: root.showBars && root.maxY > 0
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.rightMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.topMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.bottomMargin: Theme.controls.scrollView.scrollbarMargin
                          + root.cornerInset
        width: Theme.controls.scrollView.scrollbarWidth
        color: "transparent"

        MouseArea {
            id: vTrackArea
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton
            onPressed: (mouse) => root.clickVerticalTrack(mouse.y)
        }

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
        objectName: "horizontalScrollTrack"
        visible: root.showBars && root.maxX > 0
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.rightMargin: Theme.controls.scrollView.scrollbarMargin
                          + root.cornerInset
        anchors.bottomMargin: Theme.controls.scrollView.scrollbarMargin
        height: Theme.controls.scrollView.scrollbarWidth
        color: "transparent"

        MouseArea {
            id: hTrackArea
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton
            onPressed: (mouse) => root.clickHorizontalTrack(mouse.x)
        }

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

    FocusRing {
        id: scrollFocusRing
        target: root
        cornerRadius: Theme.controls.scrollView.scrollbarRadius
        shown: root.activeFocus
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
