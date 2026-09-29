// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Dock's Add Application picker (T-14.7e, ADR 0090): an overlay popover
// listing the installed applications from the shell's app-index corpus, with a
// design-system `SearchField` filter. Selecting a row toggles Dock membership
// through the shell's single `dock.pinned` writer path; the row state and the
// Dock update live. This component owns presentation and local filtering only.
//
// The shell renders this into the Dock's `overlay` chrome surface (the chooser/
// stack pattern); it is anchored to the divider and clamped into the pre-sized
// popover budget so opening it never resizes the offscreen window.
FocusScope {
    id: root

    // [{ desktopId, name, iconPath, pinned }] — the full app-index corpus,
    // built by the shell's pure `buildAppPickerList` (T-14.7e).
    property var items: []
    property Item anchorItem: null
    property bool open: false
    // True when app-index is reachable; false renders the explicit absence
    // state instead of an empty panel (ADR 0090).
    property bool available: true
    // The live filter text. Filtering is local and case-insensitive over name
    // and desktop id, matching the pure helper's contract.
    property string query: ""
    // The highlighted row; -1 = none.
    property int currentIndex: -1
    // Cap the visible rows; taller lists scroll.
    property int maxVisibleRows: 8

    signal pinToggled(string desktopId, bool pinned)
    signal opened()
    signal closed()

    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real arrowSize: Theme.controls.popover.arrowSize
    readonly property real radius: Theme.controls.contextMenu.radius
    readonly property real iconSize: 28
    readonly property int visibleRows: Math.min(filteredItems.length, maxVisibleRows)

    // The rows that survive the filter: name or id contains the query,
    // case-insensitively. The helper layer already sorts/dedupes; this mirrors
    // only its query predicate.
    readonly property var filteredItems: {
        var needle = root.query.trim().toLowerCase();
        if (needle === "")
            return root.items;
        var out = [];
        for (var i = 0; i < root.items.length; ++i) {
            var row = root.items[i];
            var name = row.name !== undefined ? String(row.name).toLowerCase() : "";
            var id = row.desktopId !== undefined ? String(row.desktopId).toLowerCase() : "";
            if (name.indexOf(needle) >= 0 || id.indexOf(needle) >= 0)
                out.push(row);
        }
        return out;
    }
    readonly property bool noMatches: filteredItems.length === 0 && !showAbsence
    readonly property bool showAbsence: !root.available

    function hide() { root.open = false; }
    function clearQuery() { root.query = ""; }
    // Set the search field text (and so the filter) imperatively; the capture
    // seam uses this, and a normal user types into the field.
    function setQueryText(text) { searchField.text = text; }

    // Keep the highlight a valid row as the filter narrows. Runs on every
    // filtered change so Up/Down and Return always act on a real row.
    onFilteredItemsChanged: {
        if (root.filteredItems.length === 0)
            root.currentIndex = -1;
        else if (root.currentIndex >= root.filteredItems.length)
            root.currentIndex = root.filteredItems.length - 1;
    }

    function moveSelection(delta) {
        if (root.filteredItems.length === 0)
            return;
        if (root.currentIndex < 0)
            root.currentIndex = delta > 0 ? 0 : root.filteredItems.length - 1;
        else
            root.currentIndex = Math.max(0, Math.min(root.filteredItems.length - 1,
                                                     root.currentIndex + delta));
        // Keep the highlighted row inside the viewport.
        var flickable = listView.flickable;
        if (flickable) {
            var top = root.currentIndex * root.rowHeight;
            var bottom = top + root.rowHeight;
            if (top < flickable.contentY)
                flickable.contentY = top;
            else if (bottom > flickable.contentY + flickable.height)
                flickable.contentY = Math.min(bottom - flickable.height,
                                              Math.max(0, flickable.contentHeight - flickable.height));
        }
    }

    function toggleRowAt(index) {
        if (index < 0 || index >= root.filteredItems.length)
            return;
        var row = root.filteredItems[index];
        root.pinToggled(row.desktopId, !(row.pinned === true));
    }

    function toggleCurrent() {
        root.toggleRowAt(root.currentIndex);
    }

    onOpenChanged: {
        if (root.open) {
            root.query = "";
            root.currentIndex = root.filteredItems.length > 0 ? 0 : -1;
            root.opened();
            searchField.input.forceActiveFocus();
        } else {
            root.closed();
        }
    }

    width: Math.max(Theme.controls.searchField.minWidth * 2, 300)
    height: root.padding + headerRow.height + searchField.height + separator.height
            + listView.height + root.padding + root.arrowSize / 2
    visible: opacity > 0
    scale: root.open ? 1.0 : 0.97
    transformOrigin: Item.Bottom
    opacity: root.open ? 1.0 : 0.0
    z: 2500

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
        objectName: "appPickerSurface"
        anchors.fill: parent
        anchors.bottomMargin: root.arrowSize / 2
        radius: root.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    Rectangle {
        objectName: "appPickerArrow"
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
        objectName: "appPickerHeader"
        x: root.padding
        y: root.padding
        width: root.width - 2 * root.padding
        height: root.rowHeight

        Text {
            id: headerText
            text: qsTr("Add Application")
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.bold: true
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
        }
    }

    SearchField {
        id: searchField
        objectName: "appPickerSearch"
        x: root.padding
        y: headerRow.y + headerRow.height
        width: root.width - 2 * root.padding
        placeholderText: qsTr("Search applications")
        onTextChanged: root.query = text
        onAccepted: root.toggleCurrent()
    }

    Rectangle {
        id: separator
        x: root.padding
        y: searchField.y + searchField.height
        width: root.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
    }

    // The list viewport. `height` caps at `maxVisibleRows`; taller lists scroll
    // through the design-system ScrollView's Flickable.
    ScrollView {
        id: listView
        objectName: "appPickerRows"
        x: root.padding
        y: separator.y + separator.height
        width: root.width - 2 * root.padding
        height: root.visibleRows > 0 ? root.visibleRows * root.rowHeight
                                     : root.rowHeight
        interactive: root.filteredItems.length > root.maxVisibleRows

        Column {
            width: listView.width
            // The viewport clips at the popover edge; the Flickable lives on
            // the ScrollView, so rows just stack.
            Repeater {
                model: root.filteredItems

                delegate: Item {
                    id: row
                    required property int index
                    required property var modelData

                    readonly property bool pinned: modelData.pinned === true
                    readonly property bool active: row.index === root.currentIndex

                    width: listView.width
                    height: root.rowHeight

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.controls.focusRing.radius
                        color: row.active ? Theme.color.controlActive
                             : (rowHover.hovered ? Theme.color.controlFill : "transparent")
                    }
                    DockGlyph {
                        id: rowIcon
                        kind: "app"
                        name: row.modelData.name !== undefined ? row.modelData.name : ""
                        appId: row.modelData.desktopId !== undefined ? row.modelData.desktopId : ""
                        desktopId: row.modelData.desktopId !== undefined ? row.modelData.desktopId : ""
                        iconPath: row.modelData.iconPath !== undefined ? row.modelData.iconPath : ""
                        size: root.iconSize
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.contextMenu.padding
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        id: nameText
                        text: row.modelData.name !== undefined ? row.modelData.name : ""
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideMiddle
                        anchors.left: rowIcon.right
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.right: pinnedText.left
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        id: pinnedText
                        text: row.pinned ? qsTr("In Dock") : qsTr("Add")
                        color: row.pinned ? Theme.color.accent : Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.contextMenu.padding
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    HoverHandler { id: rowHover }
                    TapHandler {
                        onTapped: root.toggleRowAt(row.index)
                    }
                    Accessible.role: Accessible.ListItem
                    Accessible.name: (row.modelData.name !== undefined
                                      ? row.modelData.name : "")
                                     + (row.pinned ? qsTr(", In Dock") : "")
                    Accessible.selected: row.active
                    Accessible.onPressAction: root.toggleRowAt(row.index)
                }
            }
        }
    }

    // The empty-filter state is a disabled row, never an empty panel.
    Item {
        objectName: "appPickerEmptyRow"
        visible: root.noMatches
        x: root.padding
        y: listView.y
        width: root.width - 2 * root.padding
        height: root.rowHeight
        Text {
            text: qsTr("No applications found")
            color: Theme.color.textTertiary
            font.pixelSize: Theme.controls.button.fontSize
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    // app-index absent (or the corpus empty): an explicit absence state rather
    // than a blank panel (ADR 0090).
    Item {
        objectName: "appPickerUnavailableRow"
        visible: root.showAbsence
        x: root.padding
        y: listView.y
        width: root.width - 2 * root.padding
        height: root.rowHeight
        Text {
            text: qsTr("Application index unavailable")
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
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.toggleCurrent();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Add Application")
}