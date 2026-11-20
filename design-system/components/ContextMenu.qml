// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A pointer-anchored menu (right-click, or a toolbar's overflow). It shares
// the MenuBarMenu entry model shape and publishes the same JSON-serializable
// `menuModel`, so context menus and the menu bar cannot diverge in content
// handling. `showAt()` places it at a point in its parent's coordinates.
//
// A row of `type: "submenu"` carries its children in its own `submenu`
// (or `items`) array and opens a nested panel beside the row. Nested panels
// recurse to arbitrary supported depth through the shared `MenuLevels` model,
// and `contentRect` reports the union of the whole chain so a caller that
// commits a single popover rectangle (the Dock) captures every open level.
Item {
    id: root

    property var model: []
    property bool open: false
    // The numeric selection mirrors the shared level model. They stay
    // writable for callers and tests that used the old single-level API; the
    // stable keys below are what follow a live model update.
    property int highlightedIndex: -1
    property int openSubmenuIndex: -1
    property int submenuHighlightedIndex: -1
    property string highlightedKey: ""
    property string openSubmenuKey: ""
    property string submenuHighlightedKey: ""
    // The stable identity of a row that armed the submenu hover timer.
    readonly property string pendingHoverKey: levels.pendingKey
    readonly property int pendingHoverIndex: levels.pendingIndex
    // Test/owner seam for the hover dwell; defaults to the menu token.
    property alias dwellInterval: levels.dwellInterval
    property int preferredWidth: Theme.controls.contextMenu.minWidth
    // T266: when a `Select` (or another owner) sets `anchorItem`, opening the
    // menu reparents it to the window overlay and places it below the anchor,
    // flipping above when the lower edge has no room. The owner stays local
    // for the shell's own menus, which already live on an overlay.
    property Item anchorItem: null
    property real anchorOffset: Theme.controls.popup.offset
    property Item previousFocusItem: null
    // The pointer point that opened a pointer-anchored menu, in parent
    // coordinates. It is consumed once when the menu opens so the row under
    // the cursor is selected immediately instead of showing row zero until
    // the first motion event (which reads as a jump/flicker).
    property real pointerOpenY: -1
    // The rectangle, in parent coordinates, this menu family may occupy.
    // Callers that commit a single popover rectangle (the Dock) pass the
    // intersection of the output area and the render budget. When unset the
    // menu derives the host bounds so a pointer-opened menu is still clamped.
    property var availableRect: null
    // An optional hard width cap independent of `availableRect` (a host may
    // know a narrower safe width). -1 means no extra cap.
    property real maxWidth: -1
    // The host rectangle the menu may use, in the coordinates of the menu's
    // current parent. An explicit `availableRect` wins; otherwise the window
    // overlay (for anchored menus) or the parent supplies the bounds.
    readonly property var effectiveRect: {
        if (root.availableRect && root.availableRect.w > 0
                && root.availableRect.h > 0)
            return root.availableRect;
        if (root.anchorItem) {
            var layer = OverlayLayer.layerFor(root);
            if (layer)
                return { x: 0, y: 0, w: layer.width, h: layer.height };
        }
        if (root.parent)
            return { x: 0, y: 0, w: root.parent.width, h: root.parent.height };
        return null;
    }
    readonly property real availableWidth:
        Math.min(effectiveRect && effectiveRect.w > 0 ? effectiveRect.w
                                                      : Number.MAX_VALUE,
                 maxWidth > 0 ? maxWidth : Number.MAX_VALUE)
    readonly property real availableHeight:
        effectiveRect && effectiveRect.h > 0 ? effectiveRect.h
                                             : Number.MAX_VALUE
    // The available rectangle translated into this item's coordinates, so a
    // nested panel can decide which side has room and clamp its own box.
    readonly property var localAvailableRect: {
        var rect = root.effectiveRect;
        if (!rect || rect.w <= 0 || rect.h <= 0)
            return null;
        return {
            x: rect.x - root.x,
            y: rect.y - root.y,
            w: rect.w,
            h: rect.h
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
    // The same activation with the full stable identity: the level, the row
    // index on that level, the raw item, and the ancestor key path.
    signal activatedAt(int level, int index, var item, var path)
    signal opened()
    signal closed()

    // The one shared state model for every level of this menu.
    MenuLevels {
        id: levels
        model: root.model
        menuOpen: root.open
        onActivated: (item, entry, ancestors) => {
            root.triggered(entry.index, item);
            root.activatedAt(entry.level, entry.index, item, ancestors);
            if (!entry.keepOpen)
                root.hide();
        }
        onStateChanged: root.syncFromLevels()
    }

    readonly property var entries: levels.levelEntries.length > 0
                                   ? levels.levelEntries[0] : []
    // Introspection/test seam for the whole open chain: the visible level
    // count, the stable keys that opened each level, and the stable key
    // selected on each level.
    readonly property int menuDepth: levels.levelCount
    readonly property var openPath: levels.openPath
    readonly property var highlightPath: levels.highlightKeys
    readonly property var submenuModel: levels.levelModels.length > 1
                                        ? levels.levelModels[1] : []
    readonly property var submenuEntries: levels.levelEntries.length > 1
                                          ? levels.levelEntries[1] : []
    readonly property var menuModel: {
        var out = [];
        for (var i = 0; i < root.entries.length; ++i) {
            var e = root.entries[i];
            out.push({
                id: e.index,
                key: e.key,
                level: e.level,
                path: e.path,
                type: e.type,
                label: e.label,
                shortcut: e.shortcut,
                enabled: e.enabled,
                disabledReason: e.disabledReason,
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
    // main panel carries the detached preset and every nested panel is a
    // separate detached card so its rounded corners are not lost in a union.
    readonly property var regions: {
        var radius = root.cornerRadius;
        var out = [{
            key: "menu", preset: root.materialPreset,
            x: 0, y: 0, w: root.width, h: root.height,
            tl: radius, tr: radius, br: radius, bl: radius, overscan: 20
        }];
        if (submenuPanel.open && submenuPanel.width > 0) {
            var childRegions = submenuPanel.regions;
            for (var i = 0; i < childRegions.length; ++i) {
                var region = childRegions[i];
                out.push({
                    key: region.key, preset: region.preset,
                    x: submenuPanel.x + region.x,
                    y: submenuPanel.y + region.y,
                    w: region.w, h: region.h,
                    tl: region.tl, tr: region.tr,
                    br: region.br, bl: region.bl,
                    overscan: region.overscan
                });
            }
        }
        return out;
    }

    // The bounding box of the menu plus every open submenu, in root
    // coordinates. A caller that commits one popover rectangle uses this so
    // nested panels are not clipped.
    readonly property var contentRect: {
        var x = 0;
        var y = 0;
        var w = root.width;
        var h = root.height;
        if (submenuPanel.open && submenuPanel.width > 0) {
            var rect = submenuPanel.contentRect;
            x = Math.min(x, submenuPanel.x + rect.x);
            y = Math.min(y, submenuPanel.y + rect.y);
            w = Math.max(w, submenuPanel.x + rect.x + rect.w) - x;
            h = Math.max(h, submenuPanel.y + rect.y + rect.h) - y;
        }
        return { x: x, y: y, w: w, h: h };
    }
    // The gap between the parent card and a nested panel.
    readonly property real submenuGap: Theme.primitive.spacing.xxs
    // Room for a nested panel on each side of this menu, in root coordinates.
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

    // Keep the legacy numeric API in step with the shared level model.
    property bool syncingSelection: false
    function syncFromLevels() {
        if (root.syncingSelection)
            return;
        root.syncingSelection = true;
        root.highlightedIndex = levels.highlightIndices.length > 0
                                ? levels.highlightIndices[0] : -1;
        root.openSubmenuIndex = levels.openPath.length > 0
                                ? levels.openIndexAt(0) : -1;
        root.submenuHighlightedIndex = levels.levelCount > 1
                                       ? levels.highlightIndexAt(1) : -1;
        root.highlightedKey = levels.highlightKeys.length > 0
                              ? levels.highlightKeys[0] : "";
        root.openSubmenuKey = levels.openPath.length > 0
                              ? levels.openPath[0] : "";
        root.submenuHighlightedKey = levels.highlightKeys.length > 1
                                     ? levels.highlightKeys[1] : "";
        root.syncingSelection = false;
    }
    onHighlightedIndexChanged: {
        if (!root.syncingSelection && levels.highlightIndexAt(0)
                !== root.highlightedIndex)
            levels.setHighlight(0, root.highlightedIndex);
        root.keepRowVisible(root.highlightedIndex);
    }
    onOpenSubmenuIndexChanged: {
        if (!root.syncingSelection) {
            if (root.openSubmenuIndex >= 0) {
                if (levels.openIndexAt(0) !== root.openSubmenuIndex)
                    levels.openAt(0, root.openSubmenuIndex);
            } else {
                levels.closeFrom(0);
            }
        }
        root.keepRowVisible(root.openSubmenuIndex);
        Qt.callLater(function() { submenuPanel.keepHighlightVisible(); });
    }
    onSubmenuHighlightedIndexChanged: {
        if (!root.syncingSelection && levels.highlightIndexAt(1)
                !== root.submenuHighlightedIndex)
            levels.setHighlight(1, root.submenuHighlightedIndex);
        Qt.callLater(function() { submenuPanel.keepHighlightVisible(); });
    }
    onEntriesChanged: levels.reconcile(true)

    function show() {
        root.adoptOverlay();
        root.placeAtAnchor();
        root.open = true;
    }
    function hide(immediate) {
        levels.closeFrom(0);
        root.open = false;
        if (immediate === true) {
            opacityAnimation.complete();
            scaleAnimation.complete();
        }
    }
    // Place a pointer-opened menu at a point and clamp it into the host
    // rectangle. Anchored menus are placed by `placeAtAnchor()` instead.
    function showAt(x, y) {
        // Keep the pointer point before clamping. A menu that flips or slides
        // up still highlights the row the cursor is actually over.
        root.pointerOpenY = y;
        var bounds = root.effectiveRect;
        if (bounds) {
            var w = Math.min(root.desiredWidth, root.availableWidth);
            var h = Math.min(root.desiredHeight, root.availableHeight);
            x = Math.max(bounds.x, Math.min(bounds.x + bounds.w - w, x));
            y = Math.max(bounds.y, Math.min(bounds.y + bounds.h - h, y));
        }
        root.x = x;
        root.y = y;
        root.show();
    }
    // Move the menu into the window overlay layer when its owner is anchored.
    // A pane's Select sits inside a clipped ScrollView; the menu must outlive
    // that clip. The layer owns the menu until the owner is destroyed.
    function adoptOverlay() {
        if (!root.anchorItem)
            return;
        var layer = OverlayLayer.layerFor(root);
        if (!layer)
            return;
        if (root.parent !== layer) {
            var hostWindow = root.Window.window;
            root.previousFocusItem = hostWindow ? hostWindow.activeFocusItem : null;
            root.parent = layer;
        }
    }
    // Place the menu below its anchor, clamped to the overlay and flipped above
    // when the lower half has no room.
    function placeAtAnchor() {
        if (!root.anchorItem)
            return;
        var layer = OverlayLayer.layerFor(root);
        if (!layer)
            return;
        var bounds = { x: 0, y: 0, w: layer.width, h: layer.height };
        var point = root.anchorItem.mapToItem(layer, 0, root.anchorItem.height);
        var menuWidth = Math.min(root.desiredWidth, root.availableWidth);
        var menuHeight = Math.min(root.desiredHeight, root.availableHeight);
        var x = Math.max(bounds.x, Math.min(bounds.x + bounds.w - menuWidth,
                                           point.x));
        var below = point.y + root.anchorOffset;
        var above = point.y - menuHeight - root.anchorOffset;
        var y = below;
        if (below + menuHeight > bounds.y + bounds.h && above >= bounds.y)
            y = above;
        y = Math.max(bounds.y, Math.min(bounds.y + bounds.h - menuHeight, y));
        root.x = x;
        root.y = y;
    }
    function keyAt(list, index) {
        return index >= 0 && index < list.length ? list[index].key : "";
    }
    function indexOfKey(list, key) {
        return levels.indexOfKey(list, key);
    }
    function isSelectableEntry(entry) {
        return levels.isSelectable(entry);
    }
    function nearestSelectableIndex(list, preferred) {
        return levels.nearestSelectable(list, preferred);
    }
    // Follow the highlighted command across a live model update. The shared
    // model moves to the nearest surviving enabled row when its identity
    // disappears, so the highlight never points at a different command.
    function reconcileSelection() {
        levels.reconcile();
        root.syncFromLevels();
    }
    function reconcileSubmenuSelection() {
        levels.reconcile();
        root.syncFromLevels();
    }
    function activate(index) {
        // Resolve the identity the caller saw against the current model, so a
        // row that moved since the press still runs its own command.
        levels.activateAt(0, index);
    }
    function activateHighlighted() {
        var index = levels.highlightIndexAt(0);
        if (index < 0)
            index = root.highlightedIndex;
        root.activate(index);
    }
    // Type-ahead: printable keys accumulate in a short-lived buffer and move
    // the highlight to the next enabled row whose label starts with it.
    function typeAhead(text) {
        levels.typeAhead(levels.deepestLevel, text);
    }
    function clearTypeAhead() {
        levels.clearTypeAhead();
    }
    function moveHighlight(delta) {
        levels.move(0, delta);
    }
    function openSubmenu(index) {
        levels.openAt(0, index);
    }
    function closeSubmenu() {
        levels.closeFrom(0);
    }
    function moveSubmenuHighlight(delta) {
        levels.move(1, delta);
    }
    function activateSubmenu(index) {
        levels.activateAt(1, index);
    }
    function activateSubmenuHighlighted() {
        var index = levels.highlightIndexAt(1);
        if (index < 0)
            index = root.submenuHighlightedIndex;
        root.activateSubmenu(index);
    }
    // Keep the parent selected while the pointer crosses toward its submenu.
    // Entering the child cancels this pending change, including a diagonal
    // crossing through another row. A stationary sibling still wins on dwell.
    function onMainRowHovered(index) {
        levels.hover(0, index);
    }
    function onMainRowLeft(index) {
        levels.leave(0, index);
    }
    function enterSubmenu() {
        levels.enterChild(0);
    }
    function commitHover() {
        levels.commitHover();
    }
    // The row's top offset in root coordinates, matching the Column layout.
    function rowY(index) {
        return levels.rowY(0, index, Theme.controls.contextMenu.rowHeight,
                           Theme.controls.contextMenu.padding);
    }
    // The root row at a y coordinate, including the click that opened a
    // pointer menu. A point in the padding or outside the card is not a row.
    function rowIndexAtY(y) {
        var padding = Theme.controls.contextMenu.padding;
        var top = padding;
        for (var i = 0; i < root.entries.length; ++i) {
            var height = root.entries[i].type === "separator"
                         ? padding : Theme.controls.contextMenu.rowHeight;
            if (y >= top && y < top + height)
                return i;
            top += height;
        }
        return -1;
    }
    // The row a menu selects on open: the row under a pointer-open point when
    // it names one, otherwise the checked/current row (a Select opens on its
    // live value), otherwise the first row through the normal move path.
    function initialHighlightIndex() {
        if (root.pointerOpenY >= 0) {
            var at = root.rowIndexAtY(root.pointerOpenY - root.y);
            if (at >= 0)
                return at;
        }
        for (var i = 0; i < root.entries.length; ++i) {
            if (root.entries[i].checked === true)
                return i;
        }
        return -1;
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
    // A menu is a focus scope, not a Tab stop; its owner focuses it and the
    // arrow/Escape/type-ahead handler owns the keys (T267).
    activeFocusOnTab: false

    onHeightChanged: root.keepRowVisible(root.highlightedIndex)
    onOpenChanged: {
        if (root.open) {
            if (root.previousFocusItem === null) {
                var hostWindow = root.Window.window;
                root.previousFocusItem = hostWindow ? hostWindow.activeFocusItem : null;
            }
            levels.reset();
            root.syncFromLevels();
            if (rowView.flickable)
                rowView.flickable.contentY = 0;
            // Open on the row the cursor is over (a pointer-opened menu) or
            // on the checked/current value (a Select). Only fall back to the
            // first row when neither names one.
            var initial = root.initialHighlightIndex();
            root.pointerOpenY = -1;
            if (initial >= 0)
                levels.setHighlight(0, initial);
            else
                levels.move(0, 1);
            root.forceActiveFocus();
            root.opened();
        } else {
            levels.reset();
            root.syncFromLevels();
            // Only restore the focus the menu borrowed. A menu closed by a
            // click elsewhere has already lost active focus, so it must not
            // steal focus back from the new target (T266).
            if (root.activeFocus && root.previousFocusItem
                    && root.previousFocusItem.visible)
                root.previousFocusItem.forceActiveFocus();
            root.previousFocusItem = null;
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
            id: opacityAnimation
            duration: root.open ? Theme.motion.menuOpen.duration : Theme.motion.menuClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.menuOpen.curve : Theme.motion.menuClose.curve
        }
    }
    Behavior on scale {
        NumberAnimation {
            id: scaleAnimation
            duration: root.open ? Theme.motion.menuOpen.duration : Theme.motion.menuClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.menuOpen.curve : Theme.motion.menuClose.curve
        }
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
                    onRowLeft: (index) => root.onMainRowLeft(index)
                    onRowActivated: (index) => root.activate(index)
                }
            }
        }
    }

    // The first nested card. It owns every deeper card through its own
    // recursive child loader.
    MenuLevelPanel {
        id: submenuPanel
        objectName: "submenuPanel"
        levels: levels
        level: 1
        ready: true
        boundsItem: root
        boundsRect: root.effectiveRect
        maxWidth: root.submenuMaxWidth
        maxHeight: root.availableHeight
        gap: root.submenuGap
        minWidth: root.preferredWidth
        rowHeight: Theme.controls.contextMenu.rowHeight
        padding: Theme.controls.contextMenu.padding
        labelFontSize: Theme.controls.button.fontSize
        checkSize: Theme.controls.button.fontSize
        shortcutGap: Theme.controls.contextMenu.shortcutGap
        fill: root.surfaceFill
        cornerRadius: root.cornerRadius
        accessibleName: root.accessibleName
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
        z: 1
    }

    Keys.onEscapePressed: (event) => {
        if (levels.openPath.length > 0)
            levels.closeDeepest();
        else
            root.hide();
        event.accepted = true;
    }
    Keys.onPressed: (event) => {
        var level = levels.deepestLevel;
        var list = levels.levelEntries[level] || [];
        switch (event.key) {
        case Qt.Key_Down:
            levels.move(level, 1);
            event.accepted = true;
            break;
        case Qt.Key_Up:
            levels.move(level, -1);
            event.accepted = true;
            break;
        case Qt.Key_Right:
            if (levels.openPath.length <= level) {
                var rightIndex = levels.highlightIndexAt(level);
                if (rightIndex >= 0)
                    levels.openAt(level, rightIndex);
            }
            event.accepted = true;
            break;
        case Qt.Key_Left:
            if (levels.openPath.length > 0)
                levels.closeDeepest();
            event.accepted = true;
            break;
        case Qt.Key_Home:
            levels.setHighlight(level, -1);
            levels.move(level, 1);
            event.accepted = true;
            break;
        case Qt.Key_End:
            levels.setHighlight(level, list.length);
            levels.move(level, -1);
            event.accepted = true;
            break;
        case Qt.Key_Return:
        case Qt.Key_Enter:
        case Qt.Key_Space:
            // Dispatch the selected identity, not the numeric row it had
            // when the highlight last moved.
            levels.activateAt(level, levels.highlightIndexAt(level));
            event.accepted = true;
            break;
        default:
            if (event.text.length > 0
                    && !(event.modifiers & (Qt.ControlModifier
                                            | Qt.AltModifier
                                            | Qt.MetaModifier))) {
                levels.typeAhead(level, event.text);
                event.accepted = true;
            }
            break;
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: root.accessibleName
    Accessible.description: qsTr("Menu with %1 items").arg(root.entries.length)
    Accessible.focusable: true
}
