// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A pointer-anchored menu (right-click, or a toolbar's overflow). It shares
// the MenuBarMenu entry model shape and publishes the same JSON-serializable
// `menuModel`, so context menus and the menu bar cannot diverge in content
// handling. `showAt()` places it at a point in its parent's coordinates.
//
// A row of `type: "submenu"` carries its children in its own `submenu`
// (or `items`) array and opens a nested panel beside the row. The submenu
// opens on a delayed hover (the T-09 drag-through rule) or Right-arrow, and
// `contentRect` reports the union of the menu and its open submenu so a
// caller that commits a single popover rectangle (the Dock) captures both.
Item {
    id: root

    property var model: []
    property int highlightedIndex: -1
    property bool open: false
    // The main-menu row whose submenu is open, -1 when none (section 13).
    property int openSubmenuIndex: -1
    property int submenuHighlightedIndex: -1
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
                checkable: m.checkable === true || m.checked === true,
                keepOpen: m.keepOpen === true,
                submenu: m.submenu || m.items || []
            });
        }
        return out;
    }
    // The raw children of the currently open submenu (for `triggered`).
    readonly property var submenuModel: {
        if (root.openSubmenuIndex < 0)
            return [];
        var e = root.entries[root.openSubmenuIndex];
        return e && e.submenu ? e.submenu : [];
    }
    // The normalized rows of the open submenu (same shape as `entries`).
    readonly property var submenuEntries: {
        var out = [];
        for (var i = 0; i < root.submenuModel.length; ++i) {
            var m = root.submenuModel[i] || {};
            out.push({
                index: i,
                type: m.type === "separator" ? "separator"
                      : (m.type === "submenu" ? "submenu" : "item"),
                label: m.label || "",
                shortcut: m.shortcut || "",
                enabled: m.enabled !== false,
                checked: m.checked === true,
                checkable: m.checkable === true || m.checked === true,
                keepOpen: m.keepOpen === true,
                submenu: m.submenu || m.items || []
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
                checkable: e.checkable,
                keepOpen: e.keepOpen,
                hasSubmenu: e.type === "submenu"
            });
        }
        return out;
    }
    // The bounding box of the menu plus any open submenu, in root
    // coordinates. A caller that commits one popover rectangle uses this so
    // the nested panel is not clipped.
    readonly property var contentRect: {
        var x = 0;
        var y = 0;
        var w = root.width;
        var h = root.height;
        if (submenuPanel.visible && submenuPanel.width > 0) {
            var p = submenuPanel.mapToItem(root, 0, 0);
            x = Math.min(x, p.x);
            y = Math.min(y, p.y);
            w = Math.max(w, p.x + submenuPanel.width) - x;
            h = Math.max(h, p.y + submenuPanel.height) - y;
        }
        return { x: x, y: y, w: w, h: h };
    }
    // Flip the nested panel to the menu's left when it would leave the
    // parent (e.g. a right-edge Dock, whose menu opens toward the left).
    readonly property bool submenuFlips: {
        var parentWidth = root.parent ? root.parent.width : 0;
        if (parentWidth <= 0 || submenuPanel.width <= 0)
            return false;
        return root.x + root.width + Theme.primitive.spacing.xxs + submenuPanel.width
                > parentWidth;
    }

    function show() {
        root.open = true;
    }
    function hide() {
        root.closeSubmenu();
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
        if (e.type === "submenu") {
            root.openSubmenu(index);
            return;
        }
        root.triggered(e.index, root.model[e.index]);
        // `keepOpen` lets a step (e.g. a confirmation) swap the model without
        // dismissing the menu.
        if (!e.keepOpen)
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
    function openSubmenu(index) {
        var e = root.entries[index];
        if (!e || e.type !== "submenu" || !e.enabled)
            return;
        root.openSubmenuIndex = index;
        root.submenuHighlightedIndex = -1;
        root.moveSubmenuHighlight(1);
    }
    function closeSubmenu() {
        submenuTimer.stop();
        root.openSubmenuIndex = -1;
        root.submenuHighlightedIndex = -1;
    }
    function moveSubmenuHighlight(delta) {
        var count = root.submenuEntries.length;
        if (count === 0)
            return;
        var i = root.submenuHighlightedIndex;
        for (var step = 0; step < count; ++step) {
            i = (i + delta + count) % count;
            var e = root.submenuEntries[i];
            if (e.type !== "separator" && e.enabled) {
                root.submenuHighlightedIndex = i;
                return;
            }
        }
    }
    function activateSubmenu(index) {
        var e = root.submenuEntries[index];
        if (!e || e.type === "separator" || !e.enabled)
            return;
        // Nested submenus are a follow-up; the chevron is shown but the row
        // is inert rather than silently activating a parent item.
        if (e.type === "submenu")
            return;
        root.triggered(e.index, root.submenuModel[e.index]);
        if (!e.keepOpen)
            root.hide();
    }
    // A hover on a main-menu row highlights it and, after the submenu delay,
    // opens its submenu; hovering a plain row closes any open submenu. Moving
    // the pointer from the row into the nested panel fires no main-row hover,
    // so the submenu stays open (the drag-through rule).
    function onMainRowHovered(index) {
        root.highlightedIndex = index;
        var e = root.entries[index];
        if (e && e.type === "submenu" && e.enabled) {
            if (root.openSubmenuIndex !== index)
                submenuTimer.restart();
        } else {
            root.closeSubmenu();
        }
    }
    // The row's top offset in root coordinates, matching the Column layout.
    function rowY(index) {
        var y = Theme.controls.contextMenu.padding;
        for (var i = 0; i < index && i < root.entries.length; ++i) {
            var e = root.entries[i];
            y += (e.type === "separator") ? Theme.controls.contextMenu.padding
                                          : Theme.controls.contextMenu.rowHeight;
        }
        return y;
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
            root.closeSubmenu();
            root.moveHighlight(1);
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closeSubmenu();
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

    Timer {
        id: submenuTimer
        interval: Theme.controls.contextMenu.submenuDelay
        repeat: false
        onTriggered: root.openSubmenu(root.highlightedIndex)
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
                highlightIndex: root.highlightedIndex
                onRowHovered: (index) => root.onMainRowHovered(index)
                onRowActivated: (index) => root.activate(index)
            }
        }
    }

    // The nested panel for the open submenu, beside its row.
    SubmenuPanel {
        id: submenuPanel
        objectName: "submenuPanel"
        entriesList: root.submenuEntries
        highlightIndex: root.submenuHighlightedIndex
        x: root.submenuFlips
           ? -submenuPanel.width - Theme.primitive.spacing.xxs
           : root.width + Theme.primitive.spacing.xxs
        y: root.rowY(root.openSubmenuIndex)
        visible: opacity > 0
        opacity: (root.openSubmenuIndex >= 0 && root.submenuEntries.length > 0) ? 1.0 : 0.0
        scale: root.openSubmenuIndex >= 0 ? 1.0 : 0.97
        transformOrigin: Item.TopLeft
        z: 1
        onRowHovered: (index) => root.submenuHighlightedIndex = index
        onRowActivated: (index) => root.activateSubmenu(index)

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.motion.menuOpen.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.menuOpen.curve
            }
        }
        Behavior on scale {
            NumberAnimation {
                duration: Theme.motion.menuOpen.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.menuOpen.curve
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        if (root.openSubmenuIndex >= 0)
            root.closeSubmenu();
        else
            root.hide();
        event.accepted = true;
    }
    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_Down:
            if (root.openSubmenuIndex >= 0)
                root.moveSubmenuHighlight(1);
            else
                root.moveHighlight(1);
            event.accepted = true;
            break;
        case Qt.Key_Up:
            if (root.openSubmenuIndex >= 0)
                root.moveSubmenuHighlight(-1);
            else
                root.moveHighlight(-1);
            event.accepted = true;
            break;
        case Qt.Key_Right:
            if (root.openSubmenuIndex < 0)
                root.openSubmenu(root.highlightedIndex);
            event.accepted = true;
            break;
        case Qt.Key_Left:
            if (root.openSubmenuIndex >= 0)
                root.closeSubmenu();
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
            if (root.openSubmenuIndex >= 0)
                root.activateSubmenu(root.submenuHighlightedIndex);
            else
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
        property int highlightIndex: -1

        signal rowHovered(int index)
        signal rowActivated(int index)

        property bool highlighted: row.highlightIndex === entry.index
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
                    row.rowHovered(row.entry.index);
            }
        }

        TapHandler {
            onTapped: row.rowActivated(row.entry.index)
        }

        Accessible.role: Accessible.MenuItem
        Accessible.name: row.entry.label
        Accessible.checkable: row.entry.checkable
        Accessible.checked: row.entry.checked
        Accessible.onPressAction: row.rowActivated(row.entry.index)
    }

    component SubmenuPanel: Item {
        id: subPanel

        property var entriesList: []
        property int highlightIndex: -1

        signal rowHovered(int index)
        signal rowActivated(int index)

        width: Math.max(root.preferredWidth,
                        subColumn.implicitWidth + 2 * Theme.controls.contextMenu.padding)
        height: subColumn.implicitHeight + 2 * Theme.controls.contextMenu.padding

        Shadow {
            width: subPanel.width
            height: subPanel.height
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
            id: subColumn
            x: Theme.controls.contextMenu.padding
            y: Theme.controls.contextMenu.padding
            width: subPanel.width - 2 * Theme.controls.contextMenu.padding

            Repeater {
                model: subPanel.entriesList
                delegate: ContextRow {
                    required property var modelData
                    entry: modelData
                    highlightIndex: subPanel.highlightIndex
                    onRowHovered: (index) => subPanel.rowHovered(index)
                    onRowActivated: (index) => subPanel.rowActivated(index)
                }
            }
        }

        Accessible.role: Accessible.PopupMenu
    }
}
