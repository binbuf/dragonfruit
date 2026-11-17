// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A reusable, virtualized list viewport for design-system popups.
//
// A `Repeater` inside a `Column` creates one delegate per model row, so a
// folder with thousands of entries or a full application inventory pays for
// every row even though only a handful fit. This viewport owns one `ListView`
// instead: delegate count follows the visible rows plus a small cache margin,
// while scrolling, keyboard selection, and accessibility stay on the viewport.
//
// The owner supplies a plain array (`model`) and the stable role that
// identifies a row (`keyRole`). The array is reconciled by
// `IncrementalListModel`, so a live snapshot update changes only the affected
// rows and leaves the other delegates in place. `delegate` is a normal
// `ListView` delegate; use `ListView.view.width` for the row width.
//
// Scrollbars use the same `appearance.scrollBars` tokens and click behaviour
// as `ScrollView`, so a popup list and a settings page scroll identically.
FocusScope {
    id: root

    property var model: []
    property Component delegate: null
    // The role that identifies a row across snapshots.
    property string keyRole: "id"
    // Every role the owner may publish (see `IncrementalListModel`).
    property var roleNames: []
    // While false, a source change does not build the model. A closed popover
    // sets this from its `open` state and calls `sync()` when it opens.
    property bool live: true
    property int currentIndex: -1
    property real rowHeight: Theme.controls.contextMenu.rowHeight
    property int maxVisibleRows: 8
    property bool scrollbarVisible: true
    property string scrollBars: Theme.scrollBars
    property string scrollBarClickBehavior: Theme.scrollBarClick

    property alias interactive: list.interactive
    property alias flickable: list
    property alias listView: list
    property alias contentY: list.contentY
    property alias count: list.count

    readonly property bool overflows: list.contentHeight > list.height
    readonly property bool scrollingActive: list.moving || vThumbArea.pressed
    readonly property bool pointerOver: viewHover.hovered
    readonly property bool showBars: {
        if (!root.scrollbarVisible || !root.overflows)
            return false;
        if (root.scrollBars === "always")
            return true;
        if (root.scrollBars === "when-scrolling")
            return root.scrollingActive;
        return root.activeFocus || root.pointerOver || root.scrollingActive;
    }
    readonly property real maxY: Math.max(0, list.contentHeight - list.height)
    readonly property real thumbHeight: list.contentHeight > 0
        ? Math.min(list.height, Math.max(Theme.controls.scrollView.minThumb,
                   list.height * list.height / list.contentHeight)) : 0
    readonly property real scrollBarValue:
        root.maxY > 0 ? list.contentY / root.maxY : 0
    // The delegates the view currently owns. Tests use this to prove that the
    // object count follows the viewport instead of the model length.
    readonly property int liveDelegateCount:
        list.contentItem ? list.contentItem.children.length : 0

    implicitWidth: 320
    implicitHeight: root.maxVisibleRows * root.rowHeight
    clip: true
    activeFocusOnTab: true

    function sync() { syncedModel.sync(); }

    // Scroll the selected row into view immediately. The old popovers set
    // `contentY` directly; a `ListView.positionViewAtIndex()` call can animate
    // and outlive the interaction that asked for it, so the shared viewport
    // keeps the immediate contract and the bounded cache still instantiates
    // only the destination window.
    function positionViewAtIndex(index) {
        if (index < 0 || index >= list.count)
            return;
        var top = index * root.rowHeight;
        var bottom = top + root.rowHeight;
        if (top < list.contentY)
            list.contentY = Math.max(0, top);
        else if (bottom > list.contentY + list.height)
            list.contentY = Math.min(bottom - list.height, root.maxY);
    }

    function scrollBy(dx, dy) {
        list.contentX = Math.max(0, Math.min(list.contentWidth - list.width,
                                             list.contentX + dx));
        list.contentY = Math.max(0, Math.min(root.maxY, list.contentY + dy));
    }

    function clickVerticalTrack(y) {
        if (root.maxY <= 0 || vTrack.height <= vThumb.height)
            return;
        if (root.scrollBarClickBehavior === "jump-to-next-page") {
            var thumbMiddle = vThumb.y + vThumb.height / 2;
            root.scrollBy(0, y < thumbMiddle ? -list.height : list.height);
            return;
        }
        var travel = vTrack.height - vThumb.height;
        var target = (y - vThumb.height / 2) / travel;
        list.contentY = Math.max(0, Math.min(root.maxY, target * root.maxY));
    }

    HoverHandler { id: viewHover }

    ListView {
        id: list
        anchors.fill: parent
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: syncedModel
        delegate: root.delegate
        currentIndex: root.currentIndex
        highlightFollowsCurrentItem: false
        // One screen ahead is enough to make a wheel or keyboard jump cheap
        // without instantiating the rest of a large folder.
        cacheBuffer: root.rowHeight * 2
    }

    IncrementalListModel {
        id: syncedModel
        source: root.model
        keyRole: root.keyRole
        roleNames: root.roleNames
        live: root.live
    }

    Rectangle {
        id: vTrack
        objectName: "listVerticalScrollTrack"
        visible: root.showBars
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.rightMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.topMargin: Theme.controls.scrollView.scrollbarMargin
        anchors.bottomMargin: Theme.controls.scrollView.scrollbarMargin
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
            objectName: "listVerticalScrollThumb"
            width: parent.width
            height: root.thumbHeight
            radius: Theme.controls.scrollView.scrollbarRadius
            color: vThumbArea.pressed ? Theme.color.accent
                                      : Theme.color.controlActive
            y: root.maxY > 0
               ? (list.contentY / root.maxY) * (vTrack.height - height) : 0

            MouseArea {
                id: vThumbArea
                anchors.fill: parent
                property real startY: 0
                property real startContentY: 0
                onPressed: (mouse) => {
                    vThumbArea.startY =
                        vThumbArea.mapToItem(vTrack, mouse.x, mouse.y).y;
                    vThumbArea.startContentY = list.contentY;
                }
                onPositionChanged: (mouse) => {
                    if (!vThumbArea.pressed || vTrack.height <= vThumb.height)
                        return;
                    var dy = vThumbArea.mapToItem(vTrack, mouse.x, mouse.y).y
                             - vThumbArea.startY;
                    list.contentY = Math.max(0, Math.min(root.maxY,
                        vThumbArea.startContentY
                        + dy * root.maxY / (vTrack.height - vThumb.height)));
                }
            }

            Accessible.role: Accessible.ScrollBar
            Accessible.name: qsTr("Vertical scroll bar")
            Accessible.focusable: true
        }
    }

    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_PageDown:
            root.scrollBy(0, list.height);
            event.accepted = true;
            break;
        case Qt.Key_PageUp:
            root.scrollBy(0, -list.height);
            event.accepted = true;
            break;
        case Qt.Key_Home:
            list.contentY = 0;
            event.accepted = true;
            break;
        case Qt.Key_End:
            list.contentY = root.maxY;
            event.accepted = true;
            break;
        default:
            break;
        }
    }

    Accessible.role: Accessible.List
}
