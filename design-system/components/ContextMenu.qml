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
    // T202: the shared detached-menu preset and backdrop ownership. App-owned
    // content menus keep `local` and paint the opaque plate; desktop menus
    // use `desktop` for an alpha-only wallpaper fallback. The shell
    // tray menu and the Dock set `trusted` because the compositor owns their
    // sampled backdrop.
    property string materialPreset: "menuDetached"
    property string materialBackdrop: "local"
    readonly property bool trustedBackdrop: materialBackdrop === "trusted"
    readonly property real cornerRadius: Material.menuRadius(materialPreset)
    readonly property color surfaceFill:
        materialBackdrop === "desktop"
            ? Material.desktopMenuFill(materialBlurActive, materialTier, materialFallback)
            : Material.menuFill(materialPreset, materialBlurActive, materialTier,
                                materialFallback, trustedBackdrop)
    readonly property var glassResolution:
        Material.menuResolution(materialPreset, materialBlurActive, materialTier,
                                materialFallback, trustedBackdrop)
    readonly property real closeScale: Material.menuCloseScale(materialPreset)
    // T-21.2c: the shell applies the compositor's Popup role state. The main
    // panel takes the milky plate when the sampled pass is absent; the nested
    // submenu stays opaque (it already is) and only records the same reason.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

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
    // T202: independently keyed material regions for the shell/Dock overlay.
    // Coordinates are root-local; the owner maps them to its surface. The
    // main panel carries the detached preset and the nested panel is a
    // separate detached card so its rounded corners are not lost in a union.
    readonly property var regions: {
        var radius = root.cornerRadius;
        var out = [{
            key: "menu", preset: root.materialPreset,
            x: 0, y: 0, w: root.width, h: root.height,
            tl: radius, tr: radius, br: radius, bl: radius, overscan: 20
        }];
        if (root.openSubmenuIndex >= 0 && submenuPanel.width > 0) {
            out.push({
                key: "submenu", preset: "menuDetached",
                x: submenuPanel.x, y: submenuPanel.y,
                w: submenuPanel.width, h: submenuPanel.height,
                tl: radius, tr: radius, br: radius, bl: radius, overscan: 20
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
    scale: root.open ? 1.0 : root.closeScale
    transformOrigin: Item.TopLeft
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

    // A menu closes when something else takes focus, or when its host window
    // is deactivated (clicking another application). The per-window dismiss
    // layer below handles clicks on non-focusable background/content.
    onActiveFocusChanged: {
        if (root.open && !root.activeFocus)
            root.hide();
    }

    readonly property var hostWindow: Window.window

    Connections {
        target: root.hostWindow
        function onActiveChanged() {
            if (root.open && root.hostWindow && !root.hostWindow.active)
                root.hide();
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
        radius: root.cornerRadius
        blur: Theme.controls.popup.shadowBlur
    }

    Rectangle {
        objectName: "contextMenuSurface"
        anchors.fill: parent
        radius: root.cornerRadius
        // The compositor material's scrim: resolved preset alpha/tint in the
        // fill so the sampled backdrop and liquid-glass pass read through.
        color: root.surfaceFill
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
            delegate: MenuRow {
                required property var modelData
                objectName: "menuRow" + modelData.index
                entry: modelData
                highlighted: root.highlightedIndex === modelData.index
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
            radius: root.cornerRadius
            blur: Theme.controls.popup.shadowBlur
        }

        Rectangle {
            objectName: "submenuSurface"
            anchors.fill: parent
            radius: root.cornerRadius
            // Submenus are detached cards even when the parent is attached;
            // they use the same resolved detached fill and rounded corners.
            color: root.surfaceFill
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
                delegate: MenuRow {
                    required property var modelData
                    entry: modelData
                    highlighted: subPanel.highlightIndex === modelData.index
                    onRowHovered: (index) => subPanel.rowHovered(index)
                    onRowActivated: (index) => subPanel.rowActivated(index)
                }
            }
        }

        Accessible.role: Accessible.PopupMenu
    }
}
