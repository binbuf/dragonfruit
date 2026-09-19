// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// A pointer-anchored menu (right-click, or a toolbar's overflow). It shares
// the MenuBarMenu entry model shape and publishes the same JSON-serializable
// `menuModel`, so context menus and the menu bar cannot diverge in content
// handling. `showAt()` places it at a point in its parent's coordinates.
Item {
    id: root

    property var model: []
    property int highlightedIndex: -1
    property bool open: false
    property int preferredWidth: Theme.controls.contextMenu.minWidth
    property string accessibleName: ""

    signal triggered(int index, var item)
    signal opened()
    signal closed()

    readonly property var entries: {
        var out = [];
        for (var i = 0; i < root.model.length; ++i) {
            var m = root.model[i] || {};
            out.push({
                index: i,
                type: m.type === "separator" ? "separator"
                      : (m.type === "submenu" ? "submenu" : "item"),
                label: m.label || "",
                shortcut: m.shortcut || "",
                enabled: m.enabled !== false,
                checked: m.checked === true,
                checkable: m.checkable === true || m.checked === true
            });
        }
        return out;
    }
    readonly property var menuModel: {
        var out = [];
        for (var i = 0; i < root.entries.length; ++i) {
            var e = root.entries[i];
            out.push({
                id: e.index,
                type: e.type,
                label: e.label,
                shortcut: e.shortcut,
                enabled: e.enabled,
                checked: e.checked,
                checkable: e.checkable
            });
        }
        return out;
    }

    function show() {
        root.open = true;
    }
    function hide() {
        root.open = false;
    }
    function showAt(x, y) {
        root.x = x;
        root.y = y;
        root.show();
    }
    function activate(index) {
        var e = root.entries[index];
        if (!e || e.type === "separator" || !e.enabled)
            return;
        root.triggered(e.index, root.model[e.index]);
        root.hide();
    }
    function moveHighlight(delta) {
        var count = root.entries.length;
        if (count === 0)
            return;
        var i = root.highlightedIndex;
        for (var step = 0; step < count; ++step) {
            i = (i + delta + count) % count;
            var e = root.entries[i];
            if (e.type !== "separator" && e.enabled) {
                root.highlightedIndex = i;
                return;
            }
        }
    }

    visible: opacity > 0
    width: Math.max(root.preferredWidth,
                    rowColumn.implicitWidth + 2 * Theme.controls.contextMenu.padding)
    height: rowColumn.implicitHeight + 2 * Theme.controls.contextMenu.padding
    scale: root.open ? 1.0 : 0.97
    transformOrigin: Item.Top
    opacity: root.open ? 1.0 : 0.0
    z: 2000
    focus: root.open
    activeFocusOnTab: root.open

    onOpenChanged: {
        if (root.open) {
            root.highlightedIndex = -1;
            root.moveHighlight(1);
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: root.open ? Theme.motion.menuOpen.duration : Theme.motion.menuClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.menuOpen.curve : Theme.motion.menuClose.curve
        }
    }
    Behavior on scale {
        NumberAnimation {
            duration: root.open ? Theme.motion.menuOpen.duration : Theme.motion.menuClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.menuOpen.curve : Theme.motion.menuClose.curve
        }
    }

    Shadow {
        width: root.width
        height: root.height
        radius: Theme.controls.contextMenu.radius
        blur: Theme.controls.popup.shadowBlur
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.contextMenu.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    Column {
        id: rowColumn
        x: Theme.controls.contextMenu.padding
        y: Theme.controls.contextMenu.padding
        width: root.width - 2 * Theme.controls.contextMenu.padding

        Repeater {
            model: root.entries
            delegate: ContextRow {
                required property var modelData
                entry: modelData
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.hide();
        event.accepted = true;
    }
    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_Down:
            root.moveHighlight(1);
            event.accepted = true;
            break;
        case Qt.Key_Up:
            root.moveHighlight(-1);
            event.accepted = true;
            break;
        case Qt.Key_Home:
            root.highlightedIndex = -1;
            root.moveHighlight(1);
            event.accepted = true;
            break;
        case Qt.Key_End:
            root.highlightedIndex = root.entries.length;
            root.moveHighlight(-1);
            event.accepted = true;
            break;
        case Qt.Key_Return:
        case Qt.Key_Enter:
        case Qt.Key_Space:
            root.activate(root.highlightedIndex);
            event.accepted = true;
            break;
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: root.accessibleName

    component ContextRow: Item {
        id: row

        required property var entry
        property bool highlighted: root.highlightedIndex === entry.index
        readonly property bool isSeparator: entry.type === "separator"
        readonly property bool hasSubmenu: entry.type === "submenu"

        width: parent ? parent.width : implicitWidth
        height: isSeparator ? Theme.controls.contextMenu.padding
                            : Theme.controls.contextMenu.rowHeight
        implicitWidth: Theme.controls.contextMenu.padding * 2
                       + checkGlyph.width + Theme.primitive.spacing.sm
                       + labelText.implicitWidth
                       + (shortcutText.text.length > 0
                          ? Theme.controls.contextMenu.shortcutGap + shortcutText.implicitWidth : 0)
                       + (hasSubmenu ? Theme.primitive.spacing.sm + chevron.width : 0)
        opacity: entry.enabled ? 1.0 : 0.4

        Rectangle {
            visible: row.isSeparator
            anchors.verticalCenter: parent.verticalCenter
            x: Theme.controls.contextMenu.padding
            width: parent.width - 2 * Theme.controls.contextMenu.padding
            height: Theme.controls.window.borderWidth
            color: Theme.color.separator
        }

        Rectangle {
            visible: !row.isSeparator
            anchors.fill: parent
            radius: Theme.controls.focusRing.radius
            color: row.highlighted ? Theme.color.accent : "transparent"
            antialiasing: true
        }

        Icon {
            id: checkGlyph
            visible: !row.isSeparator
            name: "check"
            size: Theme.controls.button.fontSize
            color: row.highlighted ? Theme.color.accentContent : Theme.color.accent
            opacity: row.entry.checked ? 1 : 0
            x: Theme.controls.contextMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: labelText
            visible: !row.isSeparator
            text: row.entry.label
            color: row.highlighted ? Theme.color.accentContent : Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            anchors.left: checkGlyph.right
            anchors.leftMargin: Theme.primitive.spacing.sm
            anchors.verticalCenter: parent.verticalCenter
        }

        Icon {
            id: chevron
            visible: !row.isSeparator && row.hasSubmenu
            name: "chevron-right"
            size: Theme.controls.button.fontSize
            color: row.highlighted ? Theme.color.accentContent : Theme.color.textTertiary
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.contextMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: shortcutText
            visible: !row.isSeparator && text.length > 0
            text: row.entry.shortcut
            color: row.highlighted ? Theme.color.accentContent : Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            anchors.right: row.hasSubmenu ? chevron.left : parent.right
            anchors.rightMargin: row.hasSubmenu ? Theme.primitive.spacing.sm
                                                : Theme.controls.contextMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }

        HoverHandler {
            onHoveredChanged: {
                if (hovered && !row.isSeparator && row.entry.enabled)
                    root.highlightedIndex = row.entry.index;
            }
        }

        TapHandler {
            onTapped: root.activate(row.entry.index)
        }

        Accessible.role: Accessible.MenuItem
        Accessible.name: row.entry.label
        Accessible.checkable: row.entry.checkable
        Accessible.checked: row.entry.checked
        Accessible.onPressAction: root.activate(row.entry.index)
    }
}
