// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Dock's window chooser (T-10 section 9): an overlay popover anchored to
// an app entry, listing that app's windows across all Spaces, most-recent
// first. The first row is "Show All Windows"; each window row carries a
// checkmark when frontmost, a minimized marker, and its Space name.
//
// The shell renders this into the Dock's `overlay` chrome surface. The
// component owns presentation only: the Dock feeds it `entry` (the app entry
// with its `windowList`) and relays `windowActivated`/`showAllWindows` to the
// shell, which performs the compositor round-trips.
Item {
    id: root

    property var entry: null
    property Item anchorItem: null
    property bool open: false

    signal windowActivated(string windowId)
    signal showAllWindows()
    signal opened()
    signal closed()

    readonly property var windows:
        entry && entry.windowList !== undefined ? entry.windowList : []
    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real arrowSize: Theme.controls.popover.arrowSize
    readonly property real radius: Theme.controls.contextMenu.radius

    function hide() { root.open = false; }
    function activateWindow(index) {
        var row = root.windows[index];
        if (row && row.windowId !== undefined) {
            root.windowActivated(row.windowId);
            root.hide();
        }
    }
    function activateShowAll() {
        root.showAllWindows();
        root.hide();
    }

    onOpenChanged: {
        if (root.open)
            root.opened();
        else
            root.closed();
    }

    width: Math.max(Theme.controls.contextMenu.minWidth,
                    rows.implicitWidth + 2 * root.padding)
    height: root.padding + headerRow.height + separator.height + rows.height
            + root.padding + root.arrowSize / 2
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
        objectName: "chooserSurface"
        anchors.fill: parent
        anchors.bottomMargin: root.arrowSize / 2
        radius: root.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    // The arrow ties the popover to its entry (points down at the Dock).
    Rectangle {
        objectName: "chooserArrow"
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
        objectName: "chooserHeader"
        x: root.padding
        y: root.padding
        width: root.width - 2 * root.padding
        height: root.rowHeight

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.focusRing.radius
            color: headerHover.hovered ? Theme.color.controlFill : "transparent"
        }
        Text {
            id: headerText
            text: qsTr("Show All Windows")
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
        }
        HoverHandler { id: headerHover }
        TapHandler { onTapped: root.activateShowAll() }
        Accessible.role: Accessible.MenuItem
        Accessible.name: qsTr("Show All Windows")
        Accessible.onPressAction: root.activateShowAll()
    }

    Rectangle {
        id: separator
        x: root.padding
        y: headerRow.y + headerRow.height
        width: root.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
        visible: root.windows.length > 0
    }

    Column {
        id: rows
        objectName: "chooserRows"
        x: root.padding
        y: separator.y + separator.height
        width: root.width - 2 * root.padding

        Repeater {
            model: root.windows

            delegate: Item {
                id: row
                required property var modelData
                required property int index

                width: rows.width
                height: root.rowHeight
                readonly property bool focused: modelData.focused === true
                readonly property bool minimized: modelData.minimized === true
                readonly property string spaceName:
                    modelData.workspaceName !== undefined ? modelData.workspaceName : ""

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.controls.focusRing.radius
                    color: rowHover.hovered ? Theme.color.accent : "transparent"
                }
                Icon {
                    id: check
                    visible: row.focused
                    name: "check"
                    size: Theme.controls.button.fontSize
                    color: rowHover.hovered ? Theme.color.accentContent : Theme.color.accent
                    x: Theme.controls.contextMenu.padding
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    id: title
                    text: row.modelData.title !== undefined ? row.modelData.title : ""
                    color: rowHover.hovered ? Theme.color.accentContent : Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    elide: Text.ElideRight
                    anchors.left: check.visible ? check.right : parent.left
                    anchors.leftMargin: check.visible ? Theme.primitive.spacing.sm
                                                      : Theme.controls.contextMenu.padding
                    anchors.right: space.left
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    id: space
                    text: (row.minimized ? qsTr("minimized") + (row.spaceName.length > 0 ? " · " : "")
                                         : "")
                          + row.spaceName
                    visible: text.length > 0
                    color: rowHover.hovered ? Theme.color.accentContent : Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.controls.contextMenu.padding
                    anchors.verticalCenter: parent.verticalCenter
                }
                HoverHandler { id: rowHover }
                TapHandler { onTapped: root.activateWindow(row.index) }
                Accessible.role: Accessible.MenuItem
                Accessible.name: row.modelData.title !== undefined ? row.modelData.title : ""
                Accessible.checkable: true
                Accessible.checked: row.focused
                Accessible.onPressAction: root.activateWindow(row.index)
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.hide();
        event.accepted = true;
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: root.entry && root.entry.name !== undefined ? root.entry.name : ""
}
