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
    // The stable identities behind the three numeric selection properties.
    // A live model update can insert, remove, or reorder rows; the key keeps
    // the highlight on the same command instead of the same row number.
    property string highlightedKey: ""
    property string openSubmenuKey: ""
    property string submenuHighlightedKey: ""
    // The identity of a row that armed the submenu hover timer. A row that
    // disappears before the timer fires must not open its replacement.
    property string pendingHoverKey: ""
    property int preferredWidth: Theme.controls.contextMenu.minWidth
    // The rectangle, in parent coordinates, this menu family may occupy.
    // Callers that commit a single popover rectangle (the Dock) pass the
    // intersection of the output area and the render budget. When unset the
    // menu keeps the legacy natural size so standalone callers are unchanged.
    property var availableRect: null
    // An optional hard width cap independent of `availableRect` (a host may
    // know a narrower safe width). -1 means no extra cap.
    property real maxWidth: -1
    readonly property real availableWidth:
        Math.min(availableRect && availableRect.w > 0 ? availableRect.w
                                                      : Number.MAX_VALUE,
                 maxWidth > 0 ? maxWidth : Number.MAX_VALUE)
    readonly property real availableHeight:
        availableRect && availableRect.h > 0 ? availableRect.h : Number.MAX_VALUE
    // The available rectangle translated into this item's coordinates, so a
    // nested panel can decide which side has room and clamp its own box.
    readonly property var localAvailableRect: {
        if (!root.availableRect || root.availableRect.w <= 0
                || root.availableRect.h <= 0)
            return null;
        return {
            x: root.availableRect.x - root.x,
            y: root.availableRect.y - root.y,
            w: root.availableRect.w,
            h: root.availableRect.h
        };
    }
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

    // A stable identity for a row. An explicit `id`/`key` wins (the Dock tags
    // every action); otherwise derive one from the action target or the label
    // so a live update can follow the same command across reordering.
    function stableKey(item, index) {
        var m = item || {};
        if (m.key !== undefined && m.key !== null && String(m.key) !== "")
            return String(m.key);
        if (m.id !== undefined && m.id !== null && String(m.id) !== "")
            return String(m.id);
        if (m.action !== undefined && m.action !== null && String(m.action) !== "") {
            var parts = [String(m.action)];
            var payload = m.payload;
            if (payload && typeof payload === "object") {
                var names = ["windowId", "desktopId", "appId", "path",
                             "target", "position"];
                for (var i = 0; i < names.length; ++i) {
                    if (payload[names[i]] !== undefined
                            && payload[names[i]] !== null)
                        parts.push(names[i] + "=" + String(payload[names[i]]));
                }
            }
            return parts.join("|");
        }
        if (m.type === "separator")
            return "separator";
        return String(m.type || "item") + "|" + String(m.label || "");
    }
    function normalizeModel(list) {
        var out = [];
        if (!list)
            return out;
        var seen = ({});
        for (var i = 0; i < list.length; ++i) {
            var m = list[i] || {};
            var base = root.stableKey(m, i);
            var key = base;
            var occurrence = 0;
            while (seen[key] === true) {
                occurrence += 1;
                key = base + "#" + occurrence;
            }
            seen[key] = true;
            out.push({
                index: i,
                key: key,
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
    readonly property var entries: root.normalizeModel(root.model)
    // The raw children of the currently open submenu (for `triggered`).
    readonly property var submenuModel: {
        if (root.openSubmenuIndex < 0)
            return [];
        var e = root.entries[root.openSubmenuIndex];
        return e && e.submenu ? e.submenu : [];
    }
    // The normalized rows of the open submenu (same shape as `entries`).
    readonly property var submenuEntries: root.normalizeModel(root.submenuModel)
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
        if (root.openSubmenuIndex >= 0 && submenuPanel.width > 0) {
            var p = Qt.point(submenuPanel.x, submenuPanel.y);
            x = Math.min(x, p.x);
            y = Math.min(y, p.y);
            w = Math.max(w, p.x + submenuPanel.width) - x;
            h = Math.max(h, p.y + submenuPanel.height) - y;
        }
        return { x: x, y: y, w: w, h: h };
    }
    // The gap between the parent card and a nested panel.
    readonly property real submenuGap: Theme.primitive.spacing.xxs
    // Room for a nested panel on each side of this menu, in root coordinates.
    // `availableRect` is the binding constraint when a host supplies one;
    // otherwise the legacy parent-width flip is kept.
    readonly property real submenuSpaceRight: root.localAvailableRect
        ? Math.max(0, root.localAvailableRect.x + root.localAvailableRect.w
                      - (root.width + root.submenuGap))
        : Number.MAX_VALUE
    readonly property real submenuSpaceLeft: root.localAvailableRect
        ? Math.max(0, -root.submenuGap - root.localAvailableRect.x)
        : Number.MAX_VALUE
    // Flip the nested panel to the menu's left when that side has more room.
    // A right-edge Dock's menu opens toward the left, so its submenu follows.
    readonly property bool submenuFlips: {
        if (root.localAvailableRect)
            return root.submenuSpaceLeft > root.submenuSpaceRight;
        var parentWidth = root.parent ? root.parent.width : 0;
        if (parentWidth <= 0 || submenuPanel.width <= 0)
            return false;
        return root.x + root.width + root.submenuGap + submenuPanel.width
                > parentWidth;
    }
    readonly property real submenuMaxWidth:
        root.localAvailableRect
            ? Math.max(1, root.submenuFlips ? root.submenuSpaceLeft
                                            : root.submenuSpaceRight)
            : Number.MAX_VALUE

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
    function keyAt(list, index) {
        return index >= 0 && index < list.length ? list[index].key : "";
    }
    function indexOfKey(list, key) {
        if (key === "")
            return -1;
        for (var i = 0; i < list.length; ++i) {
            if (list[i].key === key)
                return i;
        }
        return -1;
    }
    function isSelectableEntry(entry) {
        return entry !== undefined && entry.type !== "separator"
                && entry.enabled !== false;
    }
    // The closest row that can take the highlight after a model update.
    function nearestSelectableIndex(list, preferred) {
        if (list.length === 0)
            return -1;
        var start = Math.max(0, Math.min(list.length - 1, preferred));
        for (var distance = 0; distance < list.length; ++distance) {
            var after = start + distance;
            if (after < list.length && root.isSelectableEntry(list[after]))
                return after;
            var before = start - distance;
            if (distance > 0 && before >= 0
                    && root.isSelectableEntry(list[before]))
                return before;
        }
        return -1;
    }
    // Follow the highlighted command across a live model update. When its
    // identity disappears, move to the nearest surviving enabled row so the
    // highlight and its accessible state never point at a different command.
    function reconcileSelection() {
        var previous = root.highlightedIndex;
        var key = root.highlightedKey;
        if (key === "" && previous >= 0)
            key = root.keyAt(root.entries, previous);
        var index = root.indexOfKey(root.entries, key);
        if (index < 0) {
            if (key === "" && previous < 0) {
                root.highlightedIndex = -1;
                root.highlightedKey = "";
                root.reconcileSubmenuSelection();
                return;
            }
            index = root.nearestSelectableIndex(root.entries, previous);
            if (index < 0) {
                root.highlightedIndex = -1;
                root.highlightedKey = "";
                root.reconcileSubmenuSelection();
                return;
            }
            key = root.keyAt(root.entries, index);
        }
        root.highlightedIndex = index;
        root.highlightedKey = key;
        root.keepRowVisible(index);
        root.reconcileSubmenuSelection();
    }
    function reconcileSubmenuSelection() {
        if (root.openSubmenuIndex < 0) {
            root.openSubmenuKey = "";
            root.submenuHighlightedIndex = -1;
            root.submenuHighlightedKey = "";
            return;
        }
        var openKey = root.openSubmenuKey;
        if (openKey === "")
            openKey = root.keyAt(root.entries, root.openSubmenuIndex);
        var openIndex = root.indexOfKey(root.entries, openKey);
        if (openIndex < 0) {
            root.openSubmenuIndex = -1;
            root.openSubmenuKey = "";
            root.submenuHighlightedIndex = -1;
            root.submenuHighlightedKey = "";
            return;
        }
        root.openSubmenuIndex = openIndex;
        var key = root.submenuHighlightedKey;
        if (key === "" && root.submenuHighlightedIndex >= 0)
            key = root.keyAt(root.submenuEntries, root.submenuHighlightedIndex);
        var index = root.indexOfKey(root.submenuEntries, key);
        if (index < 0) {
            if (key === "" && root.submenuHighlightedIndex < 0)
                return;
            index = root.nearestSelectableIndex(root.submenuEntries,
                                                root.submenuHighlightedIndex);
            key = root.keyAt(root.submenuEntries, index);
        }
        root.submenuHighlightedIndex = index;
        root.submenuHighlightedKey = key;
    }
    function activate(index) {
        // Resolve the identity the caller saw against the current model, so a
        // row that moved since the press still runs its own command.
        var key = root.keyAt(root.entries, index);
        var resolved = key !== "" ? root.indexOfKey(root.entries, key) : index;
        if (resolved < 0)
            return;
        var e = root.entries[resolved];
        if (!e || e.type === "separator" || !e.enabled)
            return;
        if (e.type === "submenu") {
            root.openSubmenu(resolved);
            return;
        }
        root.triggered(e.index, root.model[e.index]);
        // `keepOpen` lets a step (e.g. a confirmation) swap the model without
        // dismissing the menu.
        if (!e.keepOpen)
            root.hide();
    }
    function activateHighlighted() {
        var index = root.indexOfKey(root.entries, root.highlightedKey);
        if (index < 0)
            index = root.highlightedIndex;
        root.activate(index);
    }
    function moveHighlight(delta) {
        submenuTimer.stop();
        pendingHoverIndex = -1;
        pendingHoverKey = "";
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
        submenuTimer.stop();
        pendingHoverIndex = -1;
        pendingHoverKey = "";
        root.highlightedIndex = index;
        root.openSubmenuIndex = index;
        root.submenuHighlightedIndex = -1;
        root.moveSubmenuHighlight(1);
    }
    function closeSubmenu() {
        submenuTimer.stop();
        pendingHoverIndex = -1;
        pendingHoverKey = "";
        root.openSubmenuIndex = -1;
        root.submenuHighlightedIndex = -1;
    }
    function moveSubmenuHighlight(delta) {
        submenuTimer.stop();
        pendingHoverIndex = -1;
        pendingHoverKey = "";
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
        var key = root.keyAt(root.submenuEntries, index);
        var resolved = key !== "" ? root.indexOfKey(root.submenuEntries, key)
                                  : index;
        if (resolved < 0)
            return;
        var e = root.submenuEntries[resolved];
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
    function activateSubmenuHighlighted() {
        var index = root.indexOfKey(root.submenuEntries,
                                    root.submenuHighlightedKey);
        if (index < 0)
            index = root.submenuHighlightedIndex;
        root.activateSubmenu(index);
    }
    // Keep the parent selected while the pointer crosses toward its submenu.
    // Entering the child cancels this pending change, including a diagonal
    // crossing through another row. A stationary sibling still wins on dwell.
    property int pendingHoverIndex: -1
    function onMainRowHovered(index) {
        if (!root.open)
            return;
        if (pendingHoverIndex === index && submenuTimer.running)
            return;
        submenuTimer.stop();
        pendingHoverIndex = index;
        pendingHoverKey = root.keyAt(root.entries, index);
        if (root.openSubmenuIndex >= 0 && root.openSubmenuIndex !== index) {
            submenuTimer.start();
            return;
        }
        var e = root.entries[index];
        root.highlightedIndex = e && e.enabled && e.type !== "separator" ? index : -1;
        if (e && e.type === "submenu" && e.enabled && root.openSubmenuIndex !== index)
            submenuTimer.start();
    }
    function onMainRowLeft() {
        if (root.openSubmenuIndex < 0) {
            submenuTimer.stop();
            pendingHoverIndex = -1;
            pendingHoverKey = "";
        }
    }
    function enterSubmenu() {
        submenuTimer.stop();
        pendingHoverIndex = -1;
        pendingHoverKey = "";
        root.highlightedIndex = root.openSubmenuIndex;
    }
    function commitHover() {
        if (!root.open)
            return;
        // The row that armed the dwell may have disappeared. Never open
        // whatever row now occupies its number.
        var index = root.indexOfKey(root.entries, pendingHoverKey);
        if (index < 0) {
            root.closeSubmenu();
            return;
        }
        root.closeSubmenu();
        var e = root.entries[index];
        root.highlightedIndex = e && e.enabled && e.type !== "separator" ? index : -1;
        if (e && e.enabled && e.type === "submenu")
            root.openSubmenu(index);
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
    // The row's top offset in the scrolled viewport. A nested panel follows
    // its parent row while the menu scrolls.
    function rowViewportY(index) {
        var flickable = rowView.flickable;
        return root.rowY(index) - (flickable ? flickable.contentY : 0);
    }
    // Keep the keyboard-highlighted row fully inside the bounded viewport.
    // The same math the chooser, picker, and stack popover use.
    function keepRowVisible(index) {
        if (!root.scrolls || index < 0 || index >= root.entries.length)
            return;
        var flickable = rowView.flickable;
        if (!flickable || flickable.height <= 0)
            return;
        var padding = Theme.controls.contextMenu.padding;
        var top = root.rowY(index) - padding;
        var entry = root.entries[index];
        var height = entry && entry.type === "separator"
                     ? padding : Theme.controls.contextMenu.rowHeight;
        if (top < flickable.contentY)
            flickable.contentY = Math.max(0, top);
        else if (top + height > flickable.contentY + flickable.height)
            flickable.contentY = Math.min(top + height - flickable.height,
                                          Math.max(0, flickable.contentHeight
                                                      - flickable.height));
    }

    enabled: root.open
    visible: opacity > 0
    // The widest row's natural width. The Column cannot report it once each
    // row stretches to the panel width (a positioner measures child widths),
    // so read the delegates' implicit widths directly and let the panel size
    // itself before the rows are constrained by `availableWidth`.
    readonly property real naturalRowWidth: {
        var width = 0;
        for (var i = 0; i < rowRepeater.count; ++i) {
            var row = rowRepeater.itemAt(i);
            if (row && row.implicitWidth > width)
                width = row.implicitWidth;
        }
        return width;
    }
    readonly property real desiredWidth:
        Math.max(root.preferredWidth,
                 root.naturalRowWidth + 2 * Theme.controls.contextMenu.padding)
    readonly property real desiredHeight:
        rowColumn.implicitHeight + 2 * Theme.controls.contextMenu.padding
    // The menu never grows past the rectangle its host committed. A taller
    // model scrolls inside the card instead of being clipped by the overlay.
    readonly property bool scrolls: root.desiredHeight > root.availableHeight + 0.5
    readonly property real scrollbarGutter: root.scrolls
        ? Theme.controls.scrollView.scrollbarWidth
          + Theme.controls.scrollView.scrollbarMargin
        : 0
    width: Math.min(root.desiredWidth, root.availableWidth)
    height: Math.min(root.desiredHeight, root.availableHeight)
    scale: root.open ? 1.0 : root.closeScale
    transformOrigin: Item.TopLeft
    opacity: root.open ? 1.0 : 0.0
    z: 2000
    focus: root.open
    activeFocusOnTab: root.open

    onHighlightedIndexChanged: {
        var key = root.keyAt(root.entries, root.highlightedIndex);
        if (root.highlightedKey !== key)
            root.highlightedKey = key;
        root.keepRowVisible(root.highlightedIndex);
    }
    onOpenSubmenuIndexChanged: {
        var key = root.keyAt(root.entries, root.openSubmenuIndex);
        if (root.openSubmenuKey !== key)
            root.openSubmenuKey = key;
        root.keepRowVisible(root.openSubmenuIndex);
        Qt.callLater(function() { submenuPanel.keepHighlightVisible(); });
    }
    onSubmenuHighlightedIndexChanged: {
        var key = root.keyAt(root.submenuEntries,
                             root.submenuHighlightedIndex);
        if (root.submenuHighlightedKey !== key)
            root.submenuHighlightedKey = key;
        Qt.callLater(function() { submenuPanel.keepHighlightVisible(); });
    }
    onEntriesChanged: root.reconcileSelection();
    onSubmenuEntriesChanged: {
        if (root.openSubmenuIndex >= 0)
            root.reconcileSubmenuSelection();
    }
    onHeightChanged: root.keepRowVisible(root.highlightedIndex);
    onOpenChanged: {
        if (root.open) {
            root.highlightedIndex = -1;
            root.closeSubmenu();
            if (rowView.flickable)
                rowView.flickable.contentY = 0;
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
        onTriggered: root.commitHover()
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
        border.color: Glass.increaseContrast ? Theme.color.textPrimary : Theme.color.border
        antialiasing: true
    }

    ScrollView {
        id: rowView
        objectName: "contextMenuScroll"
        x: Theme.controls.contextMenu.padding
        y: Theme.controls.contextMenu.padding
        width: Math.max(0, root.width - 2 * Theme.controls.contextMenu.padding)
        height: Math.max(0, root.height - 2 * Theme.controls.contextMenu.padding)
        interactive: root.scrolls
        scrollbarVisible: root.scrolls

        Column {
            id: rowColumn
            width: Math.max(0, rowView.width - root.scrollbarGutter)

            Repeater {
                id: rowRepeater
                model: root.entries
                delegate: MenuRow {
                    required property var modelData
                    objectName: "menuRow" + modelData.index
                    entry: modelData
                    constrainedWidth: parent ? parent.width : -1
                    highlighted: root.highlightedIndex === modelData.index
                    onRowHovered: (index) => root.onMainRowHovered(index)
                    onRowLeft: root.onMainRowLeft()
                    onRowActivated: (index) => root.activate(index)
                }
            }
        }
    }

    // The nested panel for the open submenu, beside its row.
    SubmenuPanel {
        id: submenuPanel
        objectName: "submenuPanel"
        enabled: root.open && root.openSubmenuIndex >= 0
        entriesList: root.submenuEntries
        highlightIndex: root.submenuHighlightedIndex
        availableWidth: root.submenuMaxWidth
        availableHeight: root.availableHeight
        x: root.submenuFlips
           ? -submenuPanel.width - root.submenuGap
           : root.width + root.submenuGap
        y: {
            var y = root.rowViewportY(root.openSubmenuIndex);
            var rect = root.localAvailableRect;
            if (rect && rect.h > 0)
                y = Math.max(rect.y,
                             Math.min(rect.y + rect.h - submenuPanel.height, y));
            return y;
        }
        visible: opacity > 0
        opacity: (root.openSubmenuIndex >= 0 && root.submenuEntries.length > 0) ? 1.0 : 0.0
        scale: root.openSubmenuIndex >= 0 ? 1.0 : 0.97
        transformOrigin: Item.TopLeft
        z: 1
        HoverHandler {
            onHoveredChanged: if (hovered) root.enterSubmenu()
        }
        onRowHovered: (index) => {
            root.enterSubmenu();
            var e = root.submenuEntries[index];
            root.submenuHighlightedIndex = e && e.enabled && e.type !== "separator" ? index : -1;
        }
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
            if (root.openSubmenuIndex < 0) {
                var index = root.indexOfKey(root.entries, root.highlightedKey);
                if (index < 0)
                    index = root.highlightedIndex;
                root.openSubmenu(index);
            }
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
            // Dispatch the selected identity, not the numeric row it had
            // when the highlight last moved.
            if (root.openSubmenuIndex >= 0)
                root.activateSubmenuHighlighted();
            else
                root.activateHighlighted();
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
        property real availableWidth: Number.MAX_VALUE
        property real availableHeight: Number.MAX_VALUE

        signal rowHovered(int index)
        signal rowActivated(int index)

        readonly property real naturalRowWidth: {
            var width = 0;
            for (var i = 0; i < subRepeater.count; ++i) {
                var row = subRepeater.itemAt(i);
                if (row && row.implicitWidth > width)
                    width = row.implicitWidth;
            }
            return width;
        }
        readonly property real desiredWidth:
            Math.max(root.preferredWidth,
                     subPanel.naturalRowWidth + 2 * Theme.controls.contextMenu.padding)
        readonly property real desiredHeight:
            subColumn.implicitHeight + 2 * Theme.controls.contextMenu.padding
        readonly property bool scrolls: subPanel.desiredHeight
                                        > subPanel.availableHeight + 0.5
        readonly property real scrollbarGutter: subPanel.scrolls
            ? Theme.controls.scrollView.scrollbarWidth
              + Theme.controls.scrollView.scrollbarMargin
            : 0
        width: Math.min(subPanel.desiredWidth, subPanel.availableWidth)
        height: Math.min(subPanel.desiredHeight, subPanel.availableHeight)

        // The row's top offset in the scrolled viewport.
        function keepHighlightVisible() {
            if (!subPanel.scrolls || subPanel.highlightIndex < 0
                    || subPanel.highlightIndex >= subPanel.entriesList.length)
                return;
            var flickable = subView.flickable;
            if (!flickable || flickable.height <= 0)
                return;
            var padding = Theme.controls.contextMenu.padding;
            var y = padding;
            for (var i = 0; i < subPanel.highlightIndex; ++i) {
                var e = subPanel.entriesList[i];
                y += (e.type === "separator") ? padding
                                              : Theme.controls.contextMenu.rowHeight;
            }
            var top = y - padding;
            var entry = subPanel.entriesList[subPanel.highlightIndex];
            var height = entry && entry.type === "separator"
                         ? padding : Theme.controls.contextMenu.rowHeight;
            if (top < flickable.contentY)
                flickable.contentY = Math.max(0, top);
            else if (top + height > flickable.contentY + flickable.height)
                flickable.contentY = Math.min(top + height - flickable.height,
                                              Math.max(0, flickable.contentHeight
                                                          - flickable.height));
        }

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
            border.color: Glass.increaseContrast ? Theme.color.textPrimary : Theme.color.border
            antialiasing: true
        }

        ScrollView {
            id: subView
            objectName: "submenuScroll"
            x: Theme.controls.contextMenu.padding
            y: Theme.controls.contextMenu.padding
            width: Math.max(0, subPanel.width - 2 * Theme.controls.contextMenu.padding)
            height: Math.max(0, subPanel.height - 2 * Theme.controls.contextMenu.padding)
            interactive: subPanel.scrolls
            scrollbarVisible: subPanel.scrolls

            Column {
                id: subColumn
                width: Math.max(0, subView.width - subPanel.scrollbarGutter)

                Repeater {
                    id: subRepeater
                    model: subPanel.entriesList
                    delegate: MenuRow {
                        required property var modelData
                        entry: modelData
                        constrainedWidth: parent ? parent.width : -1
                        highlighted: subPanel.highlightIndex === modelData.index
                        onRowHovered: (index) => subPanel.rowHovered(index)
                        onRowActivated: (index) => subPanel.rowActivated(index)
                    }
                }
            }
        }

        Accessible.role: Accessible.PopupMenu
    }
}
