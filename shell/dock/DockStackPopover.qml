// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Dock's folder stack popover (T-10 section 17, T-14.7h): an overlay
// popover anchored to the folder entry, listing the folder newest-first.
// Selecting a row opens the item; the header names the folder and carries the
// explicit **Open in Files** action. The shell renders this into the Dock's
// `overlay` chrome surface.
//
// The component owns presentation only: the Dock feeds it `items`
// (name/path/isDir) and relays `itemActivated`/`openFolder` to the shell. Long
// folders scroll (up to `maxItems` visible rows); the overflow summary states
// how many items do not fit, and an empty folder shows the empty state.
FocusScope {
    id: root

    property var items: []
    property string title: qsTr("Downloads")
    property Item anchorItem: null
    property bool open: false
    // The visible-row cap; a longer folder scrolls through the rest and
    // summarizes the remainder.
    property int maxItems: 8
    // The highlighted row; -1 is the header's "Open in Files" action. It makes
    // the header reachable by keyboard alongside the rows.
    property int currentIndex: -1

    signal itemActivated(string path)
    signal openFolder()
    signal opened()
    signal closed()

    readonly property int rowCount: items.length
    readonly property bool isEmpty: rowCount === 0
    readonly property int overflowCount: Math.max(0, rowCount - maxItems)
    readonly property int visibleRows: Math.min(rowCount, maxItems)
    readonly property real listHeight: isEmpty ? 0 : visibleRows * rowHeight
    readonly property real overflowHeight: overflowCount > 0 ? rowHeight : 0
    // The empty state is a real row, so the surface must reserve room for it.
    readonly property real emptyHeight: isEmpty ? rowHeight : 0
    readonly property bool scrolls: rowCount > maxItems

    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real arrowSize: Theme.controls.popover.arrowSize
    readonly property real radius: Theme.controls.contextMenu.radius
    readonly property real rowIconSize: Theme.controls.sidebar.iconSize

    function hide() { root.open = false; }

    function activateItem(index) {
        var row = root.items[index];
        if (!row || row.path === undefined)
            return;
        root.itemActivated(row.path);
        root.hide();
    }

    function activateFolder() {
        root.openFolder();
        root.hide();
    }

    // Keyboard traversal over the header action (-1) and every row.
    function moveSelection(delta) {
        var last = root.rowCount - 1;
        var next = root.currentIndex + delta;
        root.currentIndex = Math.max(-1, Math.min(last, next));
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
            root.activateFolder();
        else
            root.activateItem(root.currentIndex);
    }

    onOpenChanged: {
        if (root.open) {
            root.currentIndex = -1;
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }

    width: Math.max(Theme.controls.contextMenu.minWidth + Theme.primitive.spacing.xxxl,
                    headerText.implicitWidth + 2 * root.padding
                    + root.rowIconSize + Theme.primitive.spacing.md
                    + Theme.controls.contextMenu.shortcutGap)
    height: root.padding + headerRow.height + separator.height + root.listHeight
            + root.overflowHeight + root.emptyHeight + root.padding
            + root.arrowSize / 2
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
        color: Qt.rgba(Theme.color.surfaceElevated.r, Theme.color.surfaceElevated.g,
                       Theme.color.surfaceElevated.b, Theme.material.popupOpacity)
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    Rectangle {
        objectName: "stackArrow"
        width: root.arrowSize
        height: root.arrowSize
        rotation: 45
        color: Qt.rgba(Theme.color.surfaceElevated.r, Theme.color.surfaceElevated.g,
                       Theme.color.surfaceElevated.b, Theme.material.popupOpacity)
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

    // The header: the folder icon, the elided folder name, and the explicit
    // "Open in Files" action. The whole row activates it; it is also reachable
    // with Up/Down (index -1) and Return.
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
            color: root.currentIndex === -1 ? Theme.color.controlActive
                 : (headerHover.hovered ? Theme.color.controlFill : "transparent")
        }
        Icon {
            id: headerIcon
            objectName: "stackHeaderIcon"
            name: "folder"
            color: Theme.color.accent
            size: root.rowIconSize
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }
        Text {
            id: headerText
            objectName: "stackHeaderText"
            text: root.title
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.bold: true
            elide: Text.ElideRight
            anchors.left: headerIcon.right
            anchors.leftMargin: Theme.primitive.spacing.sm
            anchors.right: openAction.left
            anchors.rightMargin: Theme.primitive.spacing.sm
            anchors.verticalCenter: parent.verticalCenter
        }
        Text {
            id: openAction
            objectName: "stackOpenAction"
            text: qsTr("Open in Files")
            color: Theme.color.accent
            font.pixelSize: Theme.primitive.font.sizeSm
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.contextMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }
        HoverHandler { id: headerHover }
        TapHandler { onTapped: root.activateFolder() }
        Accessible.role: Accessible.MenuItem
        Accessible.name: qsTr("%1, Open in Files").arg(root.title)
        Accessible.onPressAction: root.activateFolder()
    }

    Rectangle {
        id: separator
        x: root.padding
        y: headerRow.y + headerRow.height
        width: root.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.border
    }

    // The row viewport. A folder longer than `maxItems` scrolls through it.
    ScrollView {
        id: rowsView
        objectName: "stackRows"
        x: root.padding
        y: separator.y + separator.height
        width: root.width - 2 * root.padding
        height: root.listHeight
        interactive: root.scrolls
        clip: true

        Column {
            width: rowsView.width

            Repeater {
                model: root.items

                delegate: Item {
                    id: row
                    required property int index
                    required property var modelData

                    readonly property var entryData: row.modelData
                    readonly property bool isDir: entryData.isDir === true
                    readonly property bool active: row.index === root.currentIndex

                    width: rowsView.width
                    height: root.rowHeight

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.controls.focusRing.radius
                        color: row.active ? Theme.color.controlActive
                             : (rowHover.hovered ? Theme.color.controlFill : "transparent")
                    }
                    Icon {
                        id: rowIcon
                        objectName: "stackRowIcon"
                        name: row.isDir ? "folder" : "file"
                        color: Theme.color.textSecondary
                        size: root.rowIconSize
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.contextMenu.padding
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        id: nameText
                        objectName: "stackRowName"
                        text: row.entryData.name !== undefined ? row.entryData.name : ""
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideMiddle
                        anchors.left: rowIcon.right
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.right: kindText.left
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        id: kindText
                        text: row.isDir ? qsTr("folder") : ""
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.contextMenu.padding
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    HoverHandler { id: rowHover }
                    TapHandler { onTapped: root.activateItem(row.index) }
                    Accessible.role: Accessible.MenuItem
                    Accessible.name: row.entryData.name !== undefined ? row.entryData.name : ""
                    Accessible.selected: row.active
                    Accessible.onPressAction: root.activateItem(row.index)
                }
            }
        }
    }

    // A summary row below the viewport when the folder has more than
    // `maxItems` items.
    Item {
        objectName: "stackOverflowRow"
        visible: root.overflowCount > 0
        x: root.padding
        y: rowsView.y + root.listHeight
        width: root.width - 2 * root.padding
        height: root.overflowHeight
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
        visible: root.isEmpty
        x: root.padding
        y: rowsView.y
        width: root.width - 2 * root.padding
        height: root.isEmpty ? root.rowHeight : 0
        Text {
            text: qsTr("Empty")
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
    Accessible.name: root.title
}