// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A top-level menu-bar item with a declarative dropdown. The normalized
// `menuModel` is the first-party path into the global-menu broker (FR-4,
// T-22 consumer): publish it as-is, activate entries through `activate()`.
Item {
    id: root

    property string title: ""
    property var model: []
    property alias popup: popup
    property alias open: popup.open
    property int highlightedIndex: -1

    signal triggered(int index, var item)
    signal opened()
    signal closed()

    implicitWidth: barLabel.implicitWidth + 2 * Theme.controls.menuBarMenu.barPaddingH
    implicitHeight: Theme.controls.menuBarMenu.barHeight
    activeFocusOnTab: true

    // Flattened, index-preserving view used for rendering and keyboard nav.
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

    // Publishable, JSON-serializable model (FR-4). No QML objects leak out.
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

    function openMenu() {
        popup.show();
    }

    function closeMenu() {
        popup.hide();
    }

    function activate(index) {
        var e = root.entries[index];
        if (!e || e.type === "separator" || !e.enabled)
            return;
        root.triggered(e.index, root.model[e.index]);
        root.closeMenu();
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

    Rectangle {
        id: barBackground
        anchors.fill: parent
        radius: Theme.controls.focusRing.radius
        color: root.open ? Theme.color.accent
                         : (barHover.hovered ? Theme.color.controlHover : "transparent")
        antialiasing: true
    }

    Text {
        id: barLabel
        anchors.centerIn: parent
        text: root.title
        color: root.open ? Theme.color.accentContent : Theme.color.textPrimary
        font.pixelSize: Theme.controls.titlebar.fontSize
        font.weight: Theme.primitive.font.weightMedium
    }

    HoverHandler { id: barHover }

    TapHandler {
        onTapped: root.open ? root.closeMenu() : root.openMenu()
    }

    FocusRing {
        target: root
        shown: root.activeFocus && !root.open
    }

    Keys.onReturnPressed: (event) => {
        root.open ? root.closeMenu() : root.openMenu();
        event.accepted = true;
    }

    Accessible.role: Accessible.MenuItem
    Accessible.name: root.title
    Accessible.focusable: true
    Accessible.onPressAction: root.openMenu()

    Popup {
        id: popup
        anchorItem: barBackground
        preferredWidth: Theme.controls.menuBarMenu.minWidth
        horizontalPadding: Theme.controls.menuBarMenu.padding
        verticalPadding: Theme.controls.menuBarMenu.padding
        accessibleRole: Accessible.PopupMenu
        accessibleName: root.title

        onOpened: {
            root.highlightedIndex = -1;
            root.moveHighlight(1);
            root.opened();
        }
        onClosed: {
            root.highlightedIndex = -1;
            root.closed();
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

        Repeater {
            model: root.entries

            delegate: MenuRow {
                required property var modelData
                entry: modelData
            }
        }
    }

    component MenuRow: Item {
        id: row

        required property var entry
        property bool highlighted: root.highlightedIndex === entry.index
        readonly property bool isSeparator: entry.type === "separator"
        readonly property bool hasSubmenu: entry.type === "submenu"

        width: parent ? parent.width : implicitWidth
        height: isSeparator ? Theme.controls.menuBarMenu.padding
                            : Theme.controls.menuBarMenu.rowHeight
        implicitWidth: Theme.controls.menuBarMenu.padding * 2
                       + checkGlyph.width + Theme.controls.titlebar.spacing
                       + labelText.implicitWidth
                       + (shortcutText.text.length > 0
                          ? Theme.controls.menuBarMenu.shortcutGap + shortcutText.implicitWidth : 0)
                       + (hasSubmenu ? Theme.controls.titlebar.spacing + chevron.width : 0)
        opacity: entry.enabled ? 1.0 : 0.4

        Rectangle {
            visible: row.isSeparator
            anchors.verticalCenter: parent.verticalCenter
            x: Theme.controls.menuBarMenu.padding
            width: parent.width - 2 * Theme.controls.menuBarMenu.padding
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
            size: Theme.controls.titlebar.fontSize
            color: row.highlighted ? Theme.color.accentContent : Theme.color.accent
            opacity: row.entry.checked ? 1 : 0
            x: Theme.controls.menuBarMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: labelText
            visible: !row.isSeparator
            text: row.entry.label
            color: row.highlighted ? Theme.color.accentContent : Theme.color.textPrimary
            font.pixelSize: Theme.controls.titlebar.fontSize
            anchors.left: checkGlyph.right
            anchors.leftMargin: Theme.controls.titlebar.spacing
            anchors.verticalCenter: parent.verticalCenter
        }

        Icon {
            id: chevron
            visible: !row.isSeparator && row.hasSubmenu
            name: "chevron-right"
            size: Theme.controls.titlebar.fontSize
            color: row.highlighted ? Theme.color.accentContent : Theme.color.textTertiary
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.menuBarMenu.padding
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: shortcutText
            visible: !row.isSeparator && text.length > 0
            text: row.entry.shortcut
            color: row.highlighted ? Theme.color.accentContent : Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            anchors.right: row.hasSubmenu ? chevron.left : parent.right
            anchors.rightMargin: row.hasSubmenu ? Theme.controls.titlebar.spacing
                                                : Theme.controls.menuBarMenu.padding
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
