// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One detached submenu card inside a shared `MenuLevels` tree.
//
// Level 1 is the card beside the root menu; the card lazily creates its own
// child at level 2 when a row opens another submenu, and so on. The recursion
// is driven by the shared model, so a live model update or an Escape unwinds
// exactly the level it names instead of a copied panel-local index.
Item {
    id: panel

    property var levels: null
    property int level: 1
    // A dynamically loaded child starts inert until its level and bindings
    // are installed. Without this gate the child's own loader would see the
    // default level and recurse forever during construction.
    property bool ready: false
    // The item whose coordinate system `boundsRect` uses (the menu owner).
    property Item boundsItem: null
    // The rectangle, in `boundsItem` coordinates, this panel family may use.
    property var boundsRect: null
    // Maximum size the parent allocated for this panel.
    property real maxWidth: Number.MAX_VALUE
    property real maxHeight: Number.MAX_VALUE
    property real gap: Theme.primitive.spacing.xxs
    property real minWidth: Theme.controls.contextMenu.minWidth
    property int rowHeight: Theme.controls.contextMenu.rowHeight
    property int padding: Theme.controls.contextMenu.padding
    property int labelFontSize: Theme.controls.button.fontSize
    property int checkSize: Theme.controls.button.fontSize
    property int shortcutGap: Theme.controls.contextMenu.shortcutGap
    property color fill: Theme.color.surfaceElevated
    property real cornerRadius: Theme.controls.contextMenu.radius
    property string accessibleName: ""

    readonly property var entries: levels && levels.levelEntries.length > level
                                   ? levels.levelEntries[level] : []
    readonly property int highlightIndex:
        levels ? levels.highlightIndexAt(level) : -1
    readonly property int openIndex: levels ? levels.openIndexAt(level) : -1
    readonly property bool open:
        entries.length > 0 && levels && levels.levelCount > level

    // The row's top offset within this card.
    function rowY(index) {
        return levels ? levels.rowY(level, index, rowHeight, padding) : padding;
    }

    // The row's top offset in the scrolled viewport. A nested panel follows
    // its parent row while the card scrolls.
    function rowViewportY(index) {
        var flickable = rowView.flickable;
        return rowY(index) - (flickable ? flickable.contentY : 0);
    }

    function keepHighlightVisible() {
        if (!scrolls || highlightIndex < 0 || highlightIndex >= entries.length)
            return;
        var flickable = rowView.flickable;
        if (!flickable || flickable.height <= 0)
            return;
        var top = rowY(highlightIndex) - padding;
        var entry = entries[highlightIndex];
        var height = entry && entry.type === "separator" ? padding : rowHeight;
        if (top < flickable.contentY)
            flickable.contentY = Math.max(0, top);
        else if (top + height > flickable.contentY + flickable.height)
            flickable.contentY = Math.min(top + height - flickable.height,
                                          Math.max(0, flickable.contentHeight
                                                      - flickable.height));
    }

    // The bounds rectangle translated into this card's coordinates.
    readonly property var localBounds: {
        if (!panel.boundsItem || !panel.boundsRect
                || panel.boundsRect.w <= 0 || panel.boundsRect.h <= 0)
            return null;
        var p = panel.boundsItem.mapToItem(panel, panel.boundsRect.x,
                                           panel.boundsRect.y);
        return { x: p.x, y: p.y, w: panel.boundsRect.w, h: panel.boundsRect.h };
    }
    // Room for a child card on each side of this one.
    readonly property real spaceRight: localBounds
        ? Math.max(0, localBounds.x + localBounds.w
                      - (panel.width + panel.gap))
        : Number.MAX_VALUE
    readonly property real spaceLeft: localBounds
        ? Math.max(0, -panel.gap - localBounds.x)
        : Number.MAX_VALUE
    // Flip a child card to the left when that side has more room.
    readonly property bool flips: localBounds
        ? panel.spaceLeft > panel.spaceRight : false

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
        Math.max(panel.minWidth, panel.naturalRowWidth + 2 * panel.padding)
    readonly property real desiredHeight:
        rowColumn.implicitHeight + 2 * panel.padding
    readonly property bool scrolls: panel.desiredHeight > panel.maxHeight + 0.5
    readonly property real scrollbarGutter: panel.scrolls
        ? Theme.controls.scrollView.scrollbarWidth
          + Theme.controls.scrollView.scrollbarMargin
        : 0
    width: Math.min(panel.desiredWidth, panel.maxWidth)
    height: Math.min(panel.desiredHeight, panel.maxHeight)
    visible: opacity > 0
    opacity: panel.open ? 1.0 : 0.0
    scale: panel.open ? 1.0 : 0.97
    transformOrigin: Item.TopLeft
    z: 1

    // The child card's top offset, clamped so it stays inside the bounds.
    function childY(childHeight) {
        var y = panel.rowViewportY(panel.openIndex);
        var bounds = panel.localBounds;
        if (bounds)
            y = Math.max(bounds.y,
                         Math.min(bounds.y + bounds.h - childHeight, y));
        return y;
    }

    // The union of this card and every open child card, in this card's
    // coordinates. The menu owner commits this as one popover rectangle.
    readonly property var contentRect: {
        var x = 0;
        var y = 0;
        var w = panel.width;
        var h = panel.height;
        var child = childLoader.item;
        if (child) {
            var rect = child.contentRect;
            x = Math.min(x, child.x + rect.x);
            y = Math.min(y, child.y + rect.y);
            w = Math.max(w, child.x + rect.x + rect.w) - x;
            h = Math.max(h, child.y + rect.y + rect.h) - y;
        }
        return { x: x, y: y, w: w, h: h };
    }

    // Independently keyed material regions for every card in the chain.
    readonly property var regions: {
        var out = [{
            key: panel.level === 1 ? "submenu" : "submenu" + panel.level,
            preset: "menuDetached",
            x: 0, y: 0, w: panel.width, h: panel.height,
            tl: panel.cornerRadius, tr: panel.cornerRadius,
            br: panel.cornerRadius, bl: panel.cornerRadius, overscan: 20
        }];
        var child = childLoader.item;
        if (child) {
            var childRegions = child.regions;
            for (var i = 0; i < childRegions.length; ++i) {
                var region = childRegions[i];
                out.push({
                    key: region.key, preset: region.preset,
                    x: child.x + region.x, y: child.y + region.y,
                    w: region.w, h: region.h,
                    tl: region.tl, tr: region.tr,
                    br: region.br, bl: region.bl,
                    overscan: region.overscan
                });
            }
        }
        return out;
    }

    onHighlightIndexChanged: Qt.callLater(panel.keepHighlightVisible)

    HoverHandler {
        enabled: panel.open
        onHoveredChanged: if (hovered && panel.levels)
                              panel.levels.enterChild(panel.level - 1)
    }

    Shadow {
        width: panel.width
        height: panel.height
        radius: panel.cornerRadius
        blur: Theme.controls.popup.shadowBlur
    }

    Rectangle {
        objectName: "submenuSurface"
        anchors.fill: parent
        radius: panel.cornerRadius
        color: panel.fill
        border.width: Theme.controls.window.borderWidth
        border.color: Glass.increaseContrast ? Theme.color.textPrimary
                                             : Theme.color.border
        antialiasing: true
    }

    ScrollView {
        id: rowView
        objectName: "submenuScroll"
        x: panel.padding
        y: panel.padding
        width: Math.max(0, panel.width - 2 * panel.padding)
        height: Math.max(0, panel.height - 2 * panel.padding)
        interactive: panel.scrolls
        scrollbarVisible: panel.scrolls

        Column {
            id: rowColumn
            width: Math.max(0, rowView.width - panel.scrollbarGutter)

            Repeater {
                id: rowRepeater
                model: panel.entries
                delegate: MenuRow {
                    required property var modelData
                    objectName: "menuRow" + panel.level + "_" + modelData.index
                    entry: modelData
                    constrainedWidth: parent ? parent.width : -1
                    rowHeight: panel.rowHeight
                    padding: panel.padding
                    labelFontSize: panel.labelFontSize
                    checkSize: panel.checkSize
                    shortcutGap: panel.shortcutGap
                    highlighted: panel.highlightIndex === modelData.index
                    onRowHovered: (index) => {
                        if (panel.levels)
                            panel.levels.hover(panel.level, index);
                    }
                    onRowLeft: if (panel.levels)
                                    panel.levels.leave(panel.level)
                    onRowActivated: (index) => {
                        if (panel.levels)
                            panel.levels.activateAt(panel.level, index);
                    }
                }
            }
        }
    }

    // The next card in the chain. A URL loader is the recursive instantiation
    // seam: QML forbids a file type referring to itself directly, but a Loader
    // may load the same component again. It stays inactive until a row opens a
    // child, so a two-level menu never instantiates the third level.
    Loader {
        id: childLoader
        active: panel.ready && panel.openIndex >= 0
                && panel.levels && panel.levels.levelCount > panel.level + 1
        source: "MenuLevelPanel.qml"
        onLoaded: {
            var child = item;
            child.levels = Qt.binding(function() { return panel.levels; });
            child.level = panel.level + 1;
            child.boundsItem = Qt.binding(function() { return panel.boundsItem; });
            child.boundsRect = Qt.binding(function() { return panel.boundsRect; });
            child.maxWidth = Qt.binding(function() {
                return panel.flips ? panel.spaceLeft : panel.spaceRight;
            });
            child.maxHeight = Qt.binding(function() { return panel.maxHeight; });
            child.gap = Qt.binding(function() { return panel.gap; });
            child.minWidth = Qt.binding(function() { return panel.minWidth; });
            child.rowHeight = Qt.binding(function() { return panel.rowHeight; });
            child.padding = Qt.binding(function() { return panel.padding; });
            child.labelFontSize = Qt.binding(function() {
                return panel.labelFontSize;
            });
            child.checkSize = Qt.binding(function() { return panel.checkSize; });
            child.shortcutGap = Qt.binding(function() {
                return panel.shortcutGap;
            });
            child.fill = Qt.binding(function() { return panel.fill; });
            child.cornerRadius = Qt.binding(function() {
                return panel.cornerRadius;
            });
            child.accessibleName = Qt.binding(function() {
                return panel.accessibleName;
            });
            child.x = Qt.binding(function() {
                return panel.flips ? -child.width - panel.gap
                                   : panel.width + panel.gap;
            });
            child.y = Qt.binding(function() { return panel.childY(child.height); });
            child.z = 1;
            child.ready = true;
        }
    }

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

    Accessible.role: Accessible.PopupMenu
    Accessible.name: panel.accessibleName
}
