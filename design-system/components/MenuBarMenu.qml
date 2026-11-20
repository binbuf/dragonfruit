// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A top-level menu-bar item with a declarative dropdown. The normalized
// `menuModel` is the first-party path into the global-menu broker (FR-4,
// T-22 consumer): publish it as-is, activate entries through `activate()`.
//
// Submenus recurse to arbitrary supported depth through the shared
// `MenuLevels` model, so a chevron is never a dead end. The dropdown scrolls
// inside the host rectangle, and every level keeps the highlighted row in
// view.
Item {
    id: root

    property string title: ""
    property var model: []
    // The system menu is the OS menu glyph rather than a text title (T-09);
    // `title` is still used for accessibility. `glyph` selects the artwork
    // ("dragonfruit" or "tux") the user picked in System Settings.
    property bool showLogo: false
    property string glyph: "dragonfruit"
    // The application menu's title is bold, the app's own top-level menus are
    // not (the macOS convention).
    property bool emphasized: false
    // T200: the menu-bar foreground chosen from the wallpaper behind the bar.
    // The default keeps the scheme colour for standalone/gallery use.
    property color foreground: Theme.color.textPrimary
    // Whether the label and mark keep the wallpaper contrast treatment. The
    // shell clears it while the compositor veil supplies an opaque-enough
    // plate: the reference measures flat near-black lettering, not a halo.
    // Standalone and no-veil uses keep it because the items are drawn on the
    // wallpaper.
    property bool textTreatmentEnabled: true
    // Test/introspection seam: the resolved treatment on the actual label.
    readonly property bool textShadowVisible: barLabel.shadowEnabled
    readonly property real textOutlineOpacity: barLabel.outlineOpacity
    // The wallpaper text treatment follows the actual lettering: white text
    // takes the dark shadow pair, black text the light one. Computed from the
    // resolved colour so a gallery/standalone instance with the scheme's
    // primary text also gets the correct opposite-tone shadow.
    readonly property bool lightLettering: {
        var c = root.foreground;
        return (0.299 * c.r + 0.587 * c.g + 0.114 * c.b) > 0.5;
    }
    readonly property color labelShadowColor:
        root.lightLettering ? Theme.controls.wallpaperText.shadowColorOnDark
                            : Theme.controls.wallpaperText.shadowColorOnLight
    readonly property real labelShadowOpacity:
        root.lightLettering ? Theme.controls.wallpaperText.shadowOpacityOnDark
                            : Theme.controls.wallpaperText.shadowOpacityOnLight
    // The brand mark shares the label's opposite-tone shadow, slightly
    // lighter because the silhouette covers less area than a glyph fill.
    readonly property color glyphShadowColor:
        Qt.rgba(root.labelShadowColor.r, root.labelShadowColor.g,
                root.labelShadowColor.b, root.labelShadowOpacity * 0.9)
    property alias popup: popup
    property alias open: popup.open
    // The numeric selection mirrors the shared level model. It stays writable
    // for callers that used the old single-level API; the stable keys are what
    // follow a live update.
    property int highlightedIndex: -1
    property int openSubmenuIndex: -1
    property int submenuHighlightedIndex: -1
    property string highlightedKey: ""
    property string openSubmenuKey: ""
    property string submenuHighlightedKey: ""
    // The stable identity of a row that armed the submenu hover timer.
    readonly property int pendingHoverIndex: levels.pendingIndex
    readonly property string pendingHoverKey: levels.pendingKey
    // Test/owner seam for the hover dwell; defaults to the menu token.
    property alias dwellInterval: levels.dwellInterval
    readonly property string typeAheadBuffer: levels.typeAheadBuffer
    // Whether the bar title shows the keyboard FocusRing.
    property bool showFocusRing: true
    // T-21.2c: the shell applies the compositor's Popup role state to the
    // dropdown and every submenu through this one pair of properties.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)
    // T202: the top-level dropdown is the attached preset; nested panels are
    // detached cards. The bar scene has the compositor's trusted backdrop.
    property string materialPreset: "menuAttached"
    readonly property string submenuPreset: "menuDetached"
    readonly property bool trustedBackdrop: true
    readonly property real attachedRadius: Theme.controls.contextMenu.attachedRadius
    readonly property real popupAnchorOffset:
        Math.max(0, Theme.controls.menuBar.height - Theme.controls.menuBarMenu.barHeight)
    readonly property color submenuFill:
        Material.menuFill(submenuPreset, materialBlurActive, materialTier,
                          materialFallback, trustedBackdrop)
    // The host rectangle this dropdown may use, in the bar/content coordinate
    // system. When unset the window content bounds are used, which keeps
    // standalone and gallery menus bounded too.
    property var hostBounds: null
    readonly property var effectiveRect: {
        if (root.hostBounds && root.hostBounds.w > 0 && root.hostBounds.h > 0) {
            var win = Window.window;
            if (!win)
                return root.hostBounds;
            var p = win.contentItem.mapToItem(root, root.hostBounds.x,
                                              root.hostBounds.y);
            return { x: p.x, y: p.y, w: root.hostBounds.w, h: root.hostBounds.h };
        }
        var hostWindow = Window.window;
        if (!hostWindow)
            return null;
        var content = hostWindow.contentItem;
        var origin = content.mapToItem(root, 0, 0);
        return { x: origin.x, y: origin.y, w: content.width, h: content.height };
    }

    signal triggered(int index, var item)
    // The same activation with the full stable identity: the level, the row
    // index on that level, the raw item, and the ancestor key path.
    signal activatedAt(int level, int index, var item, var path)
    signal opened()
    signal closed()
    // Left/Right at the top level asks the owner to move to a sibling menu.
    signal menuSwitchRequested(int delta)

    implicitWidth: (showLogo ? menuGlyph.implicitWidth : barLabel.implicitWidth)
                   + 2 * Theme.controls.menuBarMenu.barPaddingH
    implicitHeight: Theme.controls.menuBarMenu.barHeight
    activeFocusOnTab: true

    // The one shared state model for every level of this dropdown.
    MenuLevels {
        id: levels
        model: root.model
        menuOpen: root.open
        onActivated: (item, entry, ancestors) => {
            root.triggered(entry.index, item);
            root.activatedAt(entry.level, entry.index, item, ancestors);
            if (!entry.keepOpen)
                root.closeMenu();
        }
        onStateChanged: root.syncFromLevels()
    }

    // Flattened, index-preserving view used for rendering and keyboard nav.
    readonly property var entries: levels.levelEntries.length > 0
                                   ? levels.levelEntries[0] : []
    // Introspection/test seam for the whole open chain: the visible level
    // count, the stable keys that opened each level, and the stable key
    // selected on each level.
    readonly property int menuDepth: levels.levelCount
    readonly property var openPath: levels.openPath
    readonly property var highlightPath: levels.highlightKeys
    // The raw children of the currently open submenu (for `triggered`).
    readonly property var submenuModel: levels.levelModels.length > 1
                                        ? levels.levelModels[1] : []
    // The normalized rows of the open submenu (same shape as `entries`).
    readonly property var submenuEntries: levels.levelEntries.length > 1
                                          ? levels.levelEntries[1] : []

    // Publishable, JSON-serializable model (FR-4). No QML objects leak out.
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
                checked: e.checked,
                checkable: e.checkable,
                keepOpen: e.keepOpen,
                disabledReason: e.disabledReason,
                hasSubmenu: e.type === "submenu"
            });
        }
        return out;
    }
    // T202: material regions for the shell overlay, in this item's
    // coordinates. The attached dropdown has square top corners; nested
    // panels are detached cards. The shell maps both to the bar surface.
    readonly property var regions: {
        var out = [];
        if (popup.visible || popup.open) {
            out.push({
                key: "menu", preset: root.materialPreset,
                x: popup.x, y: popup.y, w: popup.width, h: popup.height,
                tl: 0, tr: 0, br: root.attachedRadius, bl: root.attachedRadius,
                overscan: 20
            });
        }
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

    // The bounding box of the dropdown plus every open submenu, in root
    // coordinates. The shell commits this as the `overlay` popup rectangle so
    // nested panels are not clipped.
    readonly property var contentRect: {
        // The logical dropdown rectangle in root coordinates. Using the
        // popup's own x/y (rather than mapToItem) keeps the binding tracking
        // the live position and independent of the open/close scale animation.
        var x = popup.x;
        var y = popup.y;
        var w = popup.width;
        var h = popup.height;
        // Include the submenu as soon as it is open (not once its fade-in has
        // started), so the committed rectangle is stable from the first frame.
        if (submenuPanel.open && submenuPanel.width > 0) {
            var rect = submenuPanel.contentRect;
            x = Math.min(x, submenuPanel.x + rect.x);
            y = Math.min(y, submenuPanel.y + rect.y);
            w = Math.max(w, submenuPanel.x + rect.x + rect.w) - x;
            h = Math.max(h, submenuPanel.y + rect.y + rect.h) - y;
        }
        return { x: x, y: y, w: w, h: h };
    }
    // The dropdown hangs below the bar. Its host rectangle starts at the
    // anchor so a long menu is bounded by the space below the bar instead of
    // growing to the full output height and covering the bar.
    readonly property var popupRect: {
        var rect = root.effectiveRect;
        if (!rect)
            return null;
        var top = Math.max(rect.y, popup.anchorPoint.y);
        return { x: rect.x, y: top, w: rect.w,
                 h: Math.max(0, rect.y + rect.h - top) };
    }
    readonly property real submenuGap: Theme.primitive.spacing.xxs
    // Room on each side of the dropdown for a nested card.
    readonly property real submenuSpaceRight: root.effectiveRect
        ? Math.max(0, root.effectiveRect.x + root.effectiveRect.w
                      - (popup.x + popup.width + root.submenuGap))
        : Number.MAX_VALUE
    readonly property real submenuSpaceLeft: root.effectiveRect
        ? Math.max(0, popup.x - root.submenuGap - root.effectiveRect.x)
        : Number.MAX_VALUE
    // Flip the nested panel to the dropdown's left when it would leave the
    // host rectangle (a menu opened near the right edge).
    readonly property bool submenuFlips:
        root.effectiveRect ? root.submenuSpaceLeft > root.submenuSpaceRight
                           : false
    readonly property real submenuMaxWidth:
        root.effectiveRect
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
        if (root.syncingSelection)
            return;
        if (root.openSubmenuIndex >= 0) {
            if (levels.openIndexAt(0) !== root.openSubmenuIndex)
                levels.openAt(0, root.openSubmenuIndex);
        } else {
            levels.closeFrom(0);
        }
    }
    onSubmenuHighlightedIndexChanged: {
        if (!root.syncingSelection && levels.highlightIndexAt(1)
                !== root.submenuHighlightedIndex)
            levels.setHighlight(1, root.submenuHighlightedIndex);
    }
    onEntriesChanged: levels.reconcile(true)

    function openMenu() {
        popup.show();
    }

    function closeMenu(immediate) {
        popup.hide(immediate);
    }

    function activate(index) {
        levels.activateAt(0, index);
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

    // Type-ahead: printable keys accumulate in a short-lived buffer and move
    // the highlight to the next enabled row whose label starts with it.
    function typeAhead(text) {
        levels.typeAhead(levels.deepestLevel, text);
    }
    function clearTypeAhead() {
        levels.clearTypeAhead();
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
    // The row's top offset in popup coordinates, matching the Column layout.
    function rowY(index) {
        return levels.rowY(0, index, Theme.controls.menuBarMenu.rowHeight,
                           Theme.controls.menuBarMenu.padding);
    }
    function rowViewportY(index) {
        var flickable = rowView.flickable;
        return root.rowY(index) - (flickable ? flickable.contentY : 0);
    }
    // Keep the keyboard-highlighted row fully inside the bounded dropdown.
    function keepRowVisible(index) {
        if (!rowView.interactive || index < 0 || index >= root.entries.length)
            return;
        var flickable = rowView.flickable;
        if (!flickable || flickable.height <= 0)
            return;
        var padding = Theme.controls.menuBarMenu.padding;
        var top = root.rowY(index) - padding;
        var entry = root.entries[index];
        var height = entry && entry.type === "separator"
                     ? padding : Theme.controls.menuBarMenu.rowHeight;
        if (top < flickable.contentY)
            flickable.contentY = Math.max(0, top);
        else if (top + height > flickable.contentY + flickable.height)
            flickable.contentY = Math.min(top + height - flickable.height,
                                          Math.max(0, flickable.contentHeight
                                                      - flickable.height));
    }

    // The widest row's natural width. A Column inside the scrolling
    // viewport cannot report it once rows stretch to the bounded width, so
    // read the delegates' implicit widths directly, like ContextMenu.
    readonly property real naturalRowWidth: {
        var width = 0;
        for (var i = 0; i < rowRepeater.count; ++i) {
            var row = rowRepeater.itemAt(i);
            if (row && row.implicitWidth > width)
                width = row.implicitWidth;
        }
        return width;
    }
    // A live model update reaches `entries` immediately, but the popup's
    // Column keeps reporting the previous model's height until its delegates
    // are polished. Opening the dropdown against that stale height placed it
    // at the wrong edge and left the pointer over a different row. Resolve the
    // height from the entries with the same row math the panel uses.
    readonly property real desiredPopupHeight: {
        var padding = Theme.controls.menuBarMenu.padding;
        var rowHeight = Theme.controls.menuBarMenu.rowHeight;
        var total = 2 * padding;
        for (var i = 0; i < root.entries.length; ++i)
            total += root.entries[i].type === "separator" ? padding : rowHeight;
        return total;
    }

    Rectangle {
        id: barBackground
        anchors.fill: parent
        radius: Theme.controls.focusRing.radius
        // A neutral foreground wash, not an opaque themed chip. The reference
        // bar is a white veil with dark text; the accent belongs to menu-row
        // selection, not to the open title. Hover and open share one ramp so
        // opening a menu cannot flash a different colour.
        readonly property color hoverFill: Qt.rgba(
            root.foreground.r, root.foreground.g, root.foreground.b,
            Theme.controls.menuBar.hoverOpacity)
        readonly property color openFill: Qt.rgba(
            root.foreground.r, root.foreground.g, root.foreground.b,
            Theme.controls.menuBar.openOpacity)
        color: root.open ? openFill
                         : (barHover.hovered ? hoverFill : "transparent")
        antialiasing: true

        Behavior on color {
            enabled: !Theme.reducedMotion
            ColorAnimation {
                duration: Theme.controls.menuBar.hoverDuration
                easing.type: Easing.OutCubic
            }
        }
    }

    SystemMenuGlyph {
        id: menuGlyph
        visible: root.showLogo
        anchors.centerIn: parent
        glyph: root.glyph
        size: Theme.controls.menuBar.iconSize
        color: root.foreground
        // Keep the wallpaper contrast treatment stable across hover/open; the
        // neutral wash already supplies the plate. Toggling the shadow was a
        // visible one-frame flash. On the veil the mark is flat, like the
        // lettering.
        shadowColor: root.textTreatmentEnabled ? root.glyphShadowColor
                                                : "transparent"
        shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)
    }

    WallpaperLabel {
        id: barLabel
        visible: !root.showLogo
        anchors.centerIn: parent
        text: root.title
        color: root.foreground
        // One treatment per state: flat on the veil, the wallpaper contrast
        // pair only when the bar has no plate. Hover/open never toggles it.
        shadowEnabled: root.textTreatmentEnabled
        shadowColor: root.labelShadowColor
        shadowOpacity: root.labelShadowOpacity
        outlineOpacity: root.textTreatmentEnabled
                        ? Theme.controls.wallpaperText.outlineOpacity : 0
        font.pixelSize: Theme.controls.titlebar.fontSize
        font.weight: root.emphasized ? Theme.primitive.font.weightBold
                                     : Theme.primitive.font.weightMedium
    }

    HoverHandler { id: barHover }

    TapHandler {
        onTapped: root.open ? root.closeMenu() : root.openMenu()
    }

    FocusRing {
        target: root
        shown: root.showFocusRing && root.activeFocus && !root.open
    }

    Keys.onReturnPressed: (event) => {
        root.open ? root.closeMenu() : root.openMenu();
        event.accepted = true;
    }
    Keys.onDownPressed: (event) => {
        if (!root.open)
            root.openMenu();
        else
            levels.move(levels.deepestLevel, 1);
        event.accepted = true;
    }

    Accessible.role: Accessible.MenuItem
    Accessible.name: root.title
    Accessible.focusable: true
    Accessible.onPressAction: root.openMenu()

    Popup {
        id: popup
        anchorItem: barBackground
        anchorOffset: root.popupAnchorOffset
        preferredWidth: Theme.controls.menuBarMenu.minWidth
        horizontalPadding: Theme.controls.menuBarMenu.padding
        verticalPadding: Theme.controls.menuBarMenu.padding
        accessibleRole: Accessible.PopupMenu
        accessibleName: root.title
        escapeCloses: false
        motionPreset: "menu"
        materialPreset: root.materialPreset
        materialBackdrop: "trusted"
        materialBlurActive: root.materialBlurActive
        materialTier: root.materialTier
        materialFallback: root.materialFallback
        availableRect: root.popupRect
        // Size from the rows' natural width and the column's natural height.
        // A Column inside a scrolling viewport cannot report the row width
        // once rows stretch to the bounded width, so read it directly.
        width: Math.min(Math.max(preferredWidth,
                                 root.naturalRowWidth + 2 * horizontalPadding),
                        boundWidth)
        height: Math.min(root.desiredPopupHeight, boundHeight)

        onOpened: {
            levels.reset();
            root.syncFromLevels();
            if (rowView.flickable)
                rowView.flickable.contentY = 0;
            levels.move(0, 1);
            root.opened();
        }
        onClosed: {
            levels.reset();
            root.syncFromLevels();
            root.clearTypeAhead();
            root.closed();
        }
        onEscapePressed: {
            if (levels.openPath.length > 0)
                levels.closeDeepest();
            else
                root.closeMenu();
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
                    if (rightIndex >= 0 && levels.openAt(level, rightIndex))
                        event.accepted = true;
                    else if (level === 0)
                        root.menuSwitchRequested(1);
                    else
                        event.accepted = true;
                }
                event.accepted = true;
                break;
            case Qt.Key_Left:
                if (levels.openPath.length > 0)
                    levels.closeDeepest();
                else
                    root.menuSwitchRequested(-1);
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

        // The dropdown scrolls inside the host rectangle. The rows keep their
        // natural width for sizing and stretch to the bounded panel width.
        ScrollView {
            id: rowView
            objectName: "menuBarMenuScroll"
            width: popup.width - 2 * popup.horizontalPadding
            height: rowView.implicitHeight
            implicitWidth: root.naturalRowWidth
            implicitHeight: Math.min(rowColumn.implicitHeight,
                                     popup.maxContentHeight)
            interactive: rowColumn.implicitHeight > height
            scrollbarVisible: interactive

            Column {
                id: rowColumn
                width: Math.max(0, rowView.width
                                   - (rowView.interactive
                                      ? Theme.controls.scrollView.scrollbarWidth
                                        + Theme.controls.scrollView.scrollbarMargin
                                      : 0))

                Repeater {
                    id: rowRepeater
                    model: root.entries

                    delegate: MenuRow {
                        required property var modelData
                        objectName: "menuRow" + modelData.index
                        entry: modelData
                        constrainedWidth: parent ? parent.width : -1
                        highlighted: root.highlightedIndex === modelData.index
                        rowHeight: Theme.controls.menuBarMenu.rowHeight
                        padding: Theme.controls.menuBarMenu.padding
                        labelFontSize: Theme.controls.titlebar.fontSize
                        checkSize: Theme.controls.titlebar.fontSize
                        shortcutGap: Theme.controls.menuBarMenu.shortcutGap
                        onRowHovered: (index) => root.onMainRowHovered(index)
                        onRowLeft: (index) => root.onMainRowLeft(index)
                        onRowActivated: (index) => root.activate(index)
                    }
                }
            }
        }
    }

    // The first nested card. It owns every deeper card through its own
    // recursive child loader. It is a sibling of the popup (not a child) so it
    // can overflow the popup's bounds without being clipped; `contentRect`
    // reports the union for the overlay surface.
    MenuLevelPanel {
        id: submenuPanel
        objectName: "submenuPanel"
        levels: levels
        level: 1
        ready: true
        boundsItem: root
        boundsRect: root.effectiveRect
        maxWidth: root.submenuMaxWidth
        maxHeight: root.popupRect ? root.popupRect.h : Number.MAX_VALUE
        gap: root.submenuGap
        minWidth: Theme.controls.menuBarMenu.minWidth
        rowHeight: Theme.controls.menuBarMenu.rowHeight
        padding: Theme.controls.menuBarMenu.padding
        labelFontSize: Theme.controls.titlebar.fontSize
        checkSize: Theme.controls.titlebar.fontSize
        shortcutGap: Theme.controls.menuBarMenu.shortcutGap
        fill: root.submenuFill
        cornerRadius: Theme.controls.contextMenu.radius
        accessibleName: root.title
        x: popup.x + (root.submenuFlips
                      ? -submenuPanel.width - root.submenuGap
                      : popup.width + root.submenuGap)
        y: {
            var y = popup.y + root.rowViewportY(root.openSubmenuIndex);
            var rect = root.popupRect;
            if (rect && rect.h > 0)
                y = Math.max(rect.y,
                             Math.min(rect.y + rect.h - submenuPanel.height, y));
            return y;
        }
        z: popup.z + 1
    }
}
