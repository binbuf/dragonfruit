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
    // The system menu is the brand mark rather than a text title (T-09);
    // `title` is still used for accessibility.
    property bool showLogo: false
    // The application menu's title is bold, the app's own top-level menus are
    // not (the macOS convention).
    property bool emphasized: false
    property alias popup: popup
    property alias open: popup.open
    property int highlightedIndex: -1
    // The main-menu row whose submenu is open, -1 when none (section 13).
    property int openSubmenuIndex: -1
    property int submenuHighlightedIndex: -1
    // Whether the bar title shows the keyboard FocusRing. The shell turns it
    // off until it implements Tab navigation across the bar (T-09 polish).
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

    signal triggered(int index, var item)
    signal opened()
    signal closed()

    implicitWidth: (showLogo ? logo.implicitWidth : barLabel.implicitWidth)
                   + 2 * Theme.controls.menuBarMenu.barPaddingH
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
                checkable: e.checkable,
                keepOpen: e.keepOpen,
                hasSubmenu: e.type === "submenu"
            });
        }
        return out;
    }
    // T202: material regions for the shell overlay, in this item's
    // coordinates. The attached dropdown has square top corners; the nested
    // panel is a detached card. The shell maps both to the bar surface.
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
        if (root.openSubmenuIndex >= 0 && submenuPanel.width > 0) {
            out.push({
                key: "submenu", preset: root.submenuPreset,
                x: submenuPanel.x, y: submenuPanel.y,
                w: submenuPanel.width, h: submenuPanel.height,
                tl: Theme.controls.contextMenu.radius,
                tr: Theme.controls.contextMenu.radius,
                br: Theme.controls.contextMenu.radius,
                bl: Theme.controls.contextMenu.radius, overscan: 20
            });
        }
        return out;
    }

    // The bounding box of the dropdown plus any open submenu, in root
    // coordinates. The shell commits this as the `overlay` popup rectangle so
    // the nested panel is not clipped.
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
        if (root.openSubmenuIndex >= 0 && submenuPanel.width > 0) {
            var px = submenuPanel.x;
            var py = submenuPanel.y;
            x = Math.min(x, px);
            y = Math.min(y, py);
            w = Math.max(w, px + submenuPanel.width) - x;
            h = Math.max(h, py + submenuPanel.height) - y;
        }
        return { x: x, y: y, w: w, h: h };
    }
    // Flip the nested panel to the dropdown's left when it would leave the
    // window (a menu opened near the right edge).
    readonly property bool submenuFlips: {
        var bounds = Window.window ? Window.window.width : 0;
        if (bounds <= 0 || submenuPanel.width <= 0)
            return false;
        var tracked = popup.x + popup.y;
        var right = popup.mapToItem(null, popup.width, 0).x;
        return right + Theme.primitive.spacing.xxs + submenuPanel.width > bounds;
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
        if (e.type === "submenu") {
            root.openSubmenu(index);
            return;
        }
        root.triggered(e.index, root.model[e.index]);
        if (!e.keepOpen)
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
            root.closeMenu();
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

    // The row's top offset in popup coordinates, matching the Column layout.
    function rowY(index) {
        var y = Theme.controls.menuBarMenu.padding;
        for (var i = 0; i < index && i < root.entries.length; ++i) {
            var e = root.entries[i];
            y += (e.type === "separator") ? Theme.controls.menuBarMenu.padding
                                          : Theme.controls.menuBarMenu.rowHeight;
        }
        return y;
    }

    Rectangle {
        id: barBackground
        anchors.fill: parent
        radius: Theme.controls.focusRing.radius
        color: root.open ? Theme.color.accent
                         : (barHover.hovered ? Theme.color.controlHover : "transparent")
        antialiasing: true
    }

    DragonfruitLogo {
        id: logo
        visible: root.showLogo
        anchors.centerIn: parent
        size: Theme.controls.menuBar.iconSize
        color: root.open ? Theme.color.accentContent : Theme.color.textPrimary
    }

    Text {
        id: barLabel
        visible: !root.showLogo
        anchors.centerIn: parent
        text: root.title
        color: root.open ? Theme.color.accentContent : Theme.color.textPrimary
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

        onOpened: {
            root.highlightedIndex = -1;
            root.closeSubmenu();
            root.moveHighlight(1);
            root.opened();
        }
        onClosed: {
            root.highlightedIndex = -1;
            root.closeSubmenu();
            root.closed();
        }
        onEscapePressed: {
            if (root.openSubmenuIndex >= 0)
                root.closeSubmenu();
            else
                root.closeMenu();
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

        Repeater {
            model: root.entries

            delegate: MenuRow {
                required property var modelData
                objectName: "menuRow" + modelData.index
                entry: modelData
                highlighted: root.highlightedIndex === modelData.index
                rowHeight: Theme.controls.menuBarMenu.rowHeight
                padding: Theme.controls.menuBarMenu.padding
                labelFontSize: Theme.controls.titlebar.fontSize
                checkSize: Theme.controls.titlebar.fontSize
                shortcutGap: Theme.controls.menuBarMenu.shortcutGap
                onRowHovered: (index) => root.onMainRowHovered(index)
                onRowActivated: (index) => root.activate(index)
            }
        }
    }

    Timer {
        id: submenuTimer
        interval: Theme.controls.contextMenu.submenuDelay
        repeat: false
        onTriggered: root.openSubmenu(root.highlightedIndex)
    }

    // The nested panel for the open submenu, beside its row. It is a sibling
    // of the popup (not a child) so it can overflow the popup's bounds without
    // being clipped; `contentRect` reports the union for the overlay surface.
    SubmenuPanel {
        id: submenuPanel
        objectName: "submenuPanel"
        entriesList: root.submenuEntries
        highlightIndex: root.submenuHighlightedIndex
        x: popup.x + (root.submenuFlips
                      ? -submenuPanel.width - Theme.primitive.spacing.xxs
                      : popup.width + Theme.primitive.spacing.xxs)
        y: popup.y + root.rowY(root.openSubmenuIndex)
        visible: opacity > 0
        opacity: (root.openSubmenuIndex >= 0 && root.submenuEntries.length > 0) ? 1.0 : 0.0
        scale: root.openSubmenuIndex >= 0 ? 1.0 : 0.97
        transformOrigin: Item.TopLeft
        z: popup.z + 1
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

    component SubmenuPanel: Item {
        id: subPanel

        property var entriesList: []
        property int highlightIndex: -1

        signal rowHovered(int index)
        signal rowActivated(int index)

        width: Math.max(Theme.controls.menuBarMenu.minWidth,
                        subColumn.implicitWidth + 2 * Theme.controls.menuBarMenu.padding)
        height: subColumn.implicitHeight + 2 * Theme.controls.menuBarMenu.padding

        Shadow {
            width: subPanel.width
            height: subPanel.height
            radius: Theme.controls.contextMenu.radius
            blur: Theme.controls.popup.shadowBlur
        }

        Rectangle {
            objectName: "submenuSurface"
            anchors.fill: parent
            radius: Theme.controls.contextMenu.radius
            // Submenus are detached cards: all-rounded corners and the same
            // resolved detached fill as the context/tray family.
            color: root.submenuFill
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        Column {
            id: subColumn
            x: Theme.controls.menuBarMenu.padding
            y: Theme.controls.menuBarMenu.padding
            width: subPanel.width - 2 * Theme.controls.menuBarMenu.padding

            Repeater {
                model: subPanel.entriesList
                delegate: MenuRow {
                    required property var modelData
                    entry: modelData
                    highlighted: subPanel.highlightIndex === modelData.index
                    rowHeight: Theme.controls.menuBarMenu.rowHeight
                    padding: Theme.controls.menuBarMenu.padding
                    labelFontSize: Theme.controls.titlebar.fontSize
                    checkSize: Theme.controls.titlebar.fontSize
                    shortcutGap: Theme.controls.menuBarMenu.shortcutGap
                    onRowHovered: (index) => subPanel.rowHovered(index)
                    onRowActivated: (index) => subPanel.rowActivated(index)
                }
            }
        }

        Accessible.role: Accessible.PopupMenu
    }

}
