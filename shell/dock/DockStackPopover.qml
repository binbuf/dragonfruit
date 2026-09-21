// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Dock's Downloads stack popover (T-10 section 17): an overlay popover
// anchored to the Downloads stack entry, listing the folder newest-first.
// Selecting a row opens the file; the header opens the folder itself.
//
// The shell renders this into the Dock's `overlay` chrome surface. The
// component owns presentation only: the Dock feeds it `items` (name/path/
// isDir) and relays `itemActivated`/`openFolder` to the shell.
Item {
    id: root

    property var items: []
    property string title: qsTr("Downloads")
    property Item anchorItem: null
    property bool open: false
    // Cap the visible rows; the rest are summarized as "N more…".
    property int maxItems: 8

    signal itemActivated(string path)
    signal openFolder()
    signal opened()
    signal closed()

    readonly property int visibleCount: Math.min(items.length, maxItems)
    readonly property int overflowCount: Math.max(0, items.length - maxItems)
    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real arrowSize: Theme.controls.popover.arrowSize
    readonly property real radius: Theme.controls.contextMenu.radius

    function hide() { root.open = false; }
    function activateItem(index) {
        var row = root.items[index];
        if (row && row.path !== undefined) {
            root.itemActivated(row.path);
            root.hide();
        }
    }
    function activateFolder() {
        root.openFolder();
        root.hide();
    }

    onOpenChanged: {
        if (root.open)
            root.opened();
        else
            root.closed();
    }

    width: Math.max(Theme.controls.contextMenu.minWidth,
                    Math.max(headerText.implicitWidth, rows.implicitWidth)
                    + 2 * root.padding)
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
        objectName: "stackSurface"
        anchors.fill: parent
        anchors.bottomMargin: root.arrowSize / 2
        radius: root.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    Rectangle {
        objectName: "stackArrow"
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
        objectName: "stackHeader"
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
            text: root.title
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.bold: true
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
        }
        HoverHandler { id: headerHover }
        TapHandler { onTapped: root.activateFolder() }
        Accessible.role: Accessible.MenuItem
        Accessible.name: root.title
        Accessible.onPressAction: root.activateFolder()
    }

    Rectangle {
        id: separator
        x: root.padding
        y: headerRow.y + headerRow.height
        width: root.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
    }

    Column {
        id: rows
        objectName: "stackRows"
        x: root.padding
        y: separator.y + separator.height
        width: root.width - 2 * root.padding

        Repeater {
            model: root.visibleCount

            delegate: Item {
                id: row
                required property int index

                readonly property var entryData: root.items[row.index]
                readonly property bool isDir: entryData.isDir === true

                width: rows.width
                height: root.rowHeight

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.controls.focusRing.radius
                    color: rowHover.hovered ? Theme.color.accent : "transparent"
                }
                Text {
                    id: nameText
                    text: row.entryData.name !== undefined ? row.entryData.name : ""
                    color: rowHover.hovered ? Theme.color.accentContent : Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    elide: Text.ElideMiddle
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.controls.contextMenu.padding
                    anchors.right: kindText.left
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    id: kindText
                    text: row.isDir ? qsTr("folder") : ""
                    color: rowHover.hovered ? Theme.color.accentContent : Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.controls.contextMenu.padding
                    anchors.verticalCenter: parent.verticalCenter
                }
                HoverHandler { id: rowHover }
                TapHandler { onTapped: root.activateItem(row.index) }
                Accessible.role: Accessible.MenuItem
                Accessible.name: row.entryData.name !== undefined ? row.entryData.name : ""
                Accessible.onPressAction: root.activateItem(row.index)
            }
        }

        // A disabled summary row when the folder has more than `maxItems`.
        Item {
            objectName: "stackOverflowRow"
            visible: root.overflowCount > 0
            width: rows.width
            height: root.overflowCount > 0 ? root.rowHeight : 0
            Text {
                text: qsTr("%1 more…").arg(root.overflowCount)
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                anchors.left: parent.left
                anchors.leftMargin: Theme.controls.contextMenu.padding
                anchors.verticalCenter: parent.verticalCenter
            }
        }

        // The empty-folder state is a disabled row, never an empty panel.
        Item {
            objectName: "stackEmptyRow"
            visible: root.items.length === 0
            width: rows.width
            height: root.items.length === 0 ? root.rowHeight : 0
            Text {
                text: qsTr("Empty")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.controls.button.fontSize
                anchors.left: parent.left
                anchors.leftMargin: Theme.controls.contextMenu.padding
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.hide();
        event.accepted = true;
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: root.title
}
