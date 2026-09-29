// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Dock's "More Windows" overflow list (T-14.7q): an overlay popover
// anchored to the terminal overflow cell, listing the running groups that did
// not fit on the Dock. It is the affordance that keeps every hidden group
// reachable, instead of the legacy silent drop (ADR 0103 supersedes legacy
// T-10 section 5.1 for the running-groups case).
//
// The list is deliberately simpler than the window chooser: one row per hidden
// group with its icon, name, and window count. Choosing a single-window group
// activates that window; choosing a multi-window group asks the Dock to open
// the T-14.7m window chooser anchored to the overflow cell. The shell renders
// this into the Dock's `overlay` chrome surface (the chooser/stack pattern);
// the Dock feeds it `groups` and relays `groupActivated`.
FocusScope {
    id: root

    // [{ id, appId, name, icon, iconPath, windowCount, windowList }]
    property var groups: []
    property Item anchorItem: null
    property bool open: false
    // The visible-row cap; a longer list scrolls through the rest.
    property int maxRows: Theme.controls.dock.overflow.maxRows
    // The highlighted row; -1 is none. The list has no pinned header.
    property int currentIndex: -1

    signal groupActivated(var group)
    signal opened()
    signal closed()

    readonly property int rowCount: groups.length
    readonly property int visibleRows: Math.min(rowCount, maxRows)
    readonly property real listHeight: visibleRows * rowHeight
    readonly property real emptyHeight: rowCount === 0 ? rowHeight : 0
    readonly property bool scrolls: rowCount > maxRows

    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real arrowSize: Theme.controls.popover.arrowSize
    readonly property real radius: Theme.controls.contextMenu.radius
    readonly property real rowIconSize: Theme.controls.sidebar.iconSize

    function hide() { root.open = false; }

    function groupWindowCount(group) {
        if (!group)
            return 0;
        if (group.windowCount !== undefined)
            return group.windowCount;
        return group.windowList !== undefined && group.windowList !== null
               ? group.windowList.length : 0;
    }

    function windowCountLabel(count) {
        return count === 1 ? qsTr("1 window") : qsTr("%1 windows").arg(count);
    }

    function activateGroup(index) {
        if (index < 0 || index >= root.groups.length)
            return;
        root.groupActivated(root.groups[index]);
    }

    // Keyboard traversal over every row.
    function moveSelection(delta) {
        if (root.rowCount === 0)
            return;
        if (root.currentIndex < 0)
            root.currentIndex = delta > 0 ? 0 : root.rowCount - 1;
        else
            root.currentIndex = Math.max(0, Math.min(root.rowCount - 1,
                                                     root.currentIndex + delta));
        keepSelectionVisible();
    }

    function keepSelectionVisible() {
        if (root.currentIndex < 0)
            return;
        var flickable = rowsView.flickable;
        if (!flickable)
            return;
        var top = root.currentIndex * root.rowHeight;
        var bottom = top + root.rowHeight;
        if (top < flickable.contentY)
            flickable.contentY = top;
        else if (bottom > flickable.contentY + flickable.height)
            flickable.contentY = Math.min(bottom - flickable.height,
                                          Math.max(0, flickable.contentHeight - flickable.height));
    }

    function activateCurrent() {
        if (root.currentIndex < 0)
            root.currentIndex = root.rowCount > 0 ? 0 : -1;
        root.activateGroup(root.currentIndex);
    }

    onOpenChanged: {
        if (root.open) {
            root.currentIndex = root.rowCount > 0 ? 0 : -1;
            if (rowsView.flickable)
                rowsView.flickable.contentY = 0;
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }

    width: Math.max(Theme.controls.contextMenu.minWidth,
                    rowsContent.implicitWidth + 2 * root.padding)
    height: root.padding + headerRow.height + separator.height + root.listHeight
            + root.emptyHeight + root.padding + root.arrowSize / 2
    visible: opacity > 0
    scale: root.open ? 1.0 : 0.97
    transformOrigin: Item.Bottom
    opacity: root.open ? 1.0 : 0.0
    z: 2000

    Behavior on opacity {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve
                                          : Theme.motion.popupClose.curve
        }
    }
    Behavior on scale {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve
                                          : Theme.motion.popupClose.curve
        }
    }

    Shadow {
        width: root.width
        height: root.height
        radius: root.radius
        blur: Theme.controls.popup.shadowBlur
    }

    Rectangle {
        objectName: "overflowSurface"
        anchors.fill: parent
        anchors.bottomMargin: root.arrowSize / 2
        radius: root.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    // The arrow ties the popover to the overflow cell (points down at the
    // Dock).
    Rectangle {
        objectName: "overflowArrow"
        width: root.arrowSize
        height: root.arrowSize
        rotation: 45
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        x: {
            if (!root.anchorItem || !root.parent)
                return root.width / 2 - width / 2;
            var p = root.anchorItem.mapToItem(root.parent, root.anchorItem.width / 2, 0);
            return Math.max(root.radius,
                            Math.min(root.width - root.radius, p.x - root.x)) - width / 2;
        }
        y: root.height - height
        z: -1
    }

    Item {
        id: headerRow
        objectName: "overflowHeader"
        x: root.padding
        y: root.padding
        width: root.width - 2 * root.padding
        height: root.rowHeight

        Text {
            id: headerText
            objectName: "overflowHeaderText"
            text: qsTr("More Windows")
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.bold: true
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
        }
        Accessible.role: Accessible.StaticText
        Accessible.name: qsTr("More Windows")
    }

    Rectangle {
        id: separator
        x: root.padding
        y: headerRow.y + headerRow.height
        width: root.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
    }

    // The bounded row viewport. The header stays pinned above it.
    ScrollView {
        id: rowsView
        objectName: "overflowRows"
        x: root.padding
        y: separator.y + separator.height
        width: root.width - 2 * root.padding
        height: root.listHeight
        interactive: root.scrolls
        scrollbarVisible: root.scrolls

        Column {
            id: rowsContent
            width: rowsView.width

            Repeater {
                model: root.groups

                delegate: Item {
                    id: row
                    required property var modelData
                    required property int index

                    readonly property int groupWindowCount: root.groupWindowCount(row.modelData)
                    readonly property bool active: row.index === root.currentIndex
                    readonly property string groupName:
                        modelData.name !== undefined ? modelData.name : ""

                    width: rowsContent.width
                    height: root.rowHeight

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.controls.focusRing.radius
                        color: row.active ? Theme.color.controlActive
                             : (rowHover.hovered ? Theme.color.controlFill : "transparent")
                    }
                    DockGlyph {
                        id: rowIcon
                        objectName: "overflowRowIcon"
                        kind: "app"
                        name: row.groupName
                        appId: row.modelData.appId !== undefined ? row.modelData.appId : ""
                        desktopId: row.modelData.desktopId !== undefined
                                   ? row.modelData.desktopId
                                   : (row.modelData.appId !== undefined
                                      ? row.modelData.appId : "")
                        iconPath: row.modelData.iconPath !== undefined
                                  ? row.modelData.iconPath : ""
                        size: root.rowIconSize
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.contextMenu.padding
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        id: nameText
                        objectName: "overflowRowName"
                        text: row.groupName
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                        anchors.left: rowIcon.right
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.right: countText.left
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        id: countText
                        objectName: "overflowRowCount"
                        text: root.windowCountLabel(row.groupWindowCount)
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.contextMenu.padding
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    HoverHandler { id: rowHover }
                    TapHandler { onTapped: root.activateGroup(row.index) }
                    Accessible.role: Accessible.MenuItem
                    Accessible.name: row.groupName + qsTr(", %1")
                                      .arg(root.windowCountLabel(row.groupWindowCount))
                    Accessible.onPressAction: root.activateGroup(row.index)
                }
            }
        }
    }

    // An empty list is a disabled row, never a blank panel.
    Item {
        objectName: "overflowEmptyRow"
        visible: root.rowCount === 0
        x: root.padding
        y: rowsView.y
        width: root.width - 2 * root.padding
        height: root.rowCount === 0 ? root.rowHeight : 0
        Text {
            text: qsTr("No hidden windows")
            color: Theme.color.textTertiary
            font.pixelSize: Theme.controls.button.fontSize
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Escape) {
            root.hide();
            event.accepted = true;
        } else if (event.key === Qt.Key_Down) {
            root.moveSelection(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Up) {
            root.moveSelection(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                   || event.key === Qt.Key_Space) {
            root.activateCurrent();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: qsTr("More Windows")
}