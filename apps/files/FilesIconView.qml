// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files icon view (T-10.4b/T-10.4c): a centered grid of icon tiles (a
// thumbnail or glyph over a two-line label) over the files-core listing.
// Selection is the set of node ids the shell owns, so it survives the view
// switch (T-10.4c multi-select); opening is a double click; right-click opens
// the context menu; Return/`renamingId` swaps the label for an inline editor.
FocusScope {
    id: root

    // The `FilesDirectoryModel` (files-core facade). The view renders it and
    // never touches the filesystem.
    property var directory: null
    // The node id the shell treats as primary (the last clicked).
    property real selectedId: 0
    // The full selected set (multi-select, T-10.4c).
    property var selectedIds: []
    // The node currently being renamed inline, or 0.
    property real renamingId: 0

    // Desktop presentation. The desktop labels sit directly on the wallpaper,
    // so they use the menu bar's sampled luminance and its light/dark pair
    // instead of the app's scheme text colour. Files windows keep the normal
    // scheme label and the hover wash.
    property bool desktopStyle: false
    // The wallpaper luma behind the view (0..1), from the same sampled source
    // the menu bar uses. Desktop labels are near-white until this crosses the
    // menu bar's flip threshold, where they become black.
    property real backgroundLuma: 0.5
    property bool hoverHighlight: true
    // Free placement: the grid remains the base layout, but a tile can be
    // dragged away from its cell. The offset is remembered per URI for the
    // life of the view; `cleanUp()` returns every tile to its grid slot.
    property bool freePlacement: true
    property var freePositions: ({})
    readonly property int dragThreshold: Math.max(2, Theme.primitive.spacing.xs)

    signal selected(real nodeId, int modifiers)
    signal activated(string uri, bool isDir)
    signal contextRequested(real nodeId, string uri, bool isDir, real x, real y)
    signal backgroundContextRequested(real x, real y)
    signal renameSubmitted(real nodeId, string name)
    signal renameCancelled()
    // Rubber-band selection (T-19.3): the node ids enclosed by the marquee,
    // with the live modifier state so the shell can toggle (Cmd) or extend
    // (Shift) instead of replacing.
    signal marqueeStarted(int modifiers)
    signal marqueeUpdated(var nodeIds, int modifiers)
    signal marqueeSelected(var nodeIds, int modifiers)

    readonly property int tileIconSize: 48
    readonly property int cellWidth: 116
    readonly property int cellHeight: 104
    // Wallpaper labels follow the menu bar's measured polarity: almost white
    // on ordinary scenes, black only on very bright wallpaper, with a slight
    // opposite-tone drop shadow so the glyphs stay legible on busy images.
    readonly property bool desktopLabelDark:
        root.backgroundLuma >= Theme.controls.menuBar.foregroundFlipThresholdLight
    readonly property color desktopLabelColor:
        root.desktopLabelDark ? Theme.controls.menuBar.foregroundOnLight
                              : Theme.controls.menuBar.foregroundOnDark
    readonly property color desktopLabelShadowColor:
        root.desktopLabelDark ? Qt.rgba(1, 1, 1, 0.70)
                              : Qt.rgba(0, 0, 0, 0.70)
    readonly property color tileSelectionColor:
        Qt.rgba(Theme.color.selection.r, Theme.color.selection.g,
                Theme.color.selection.b, 0.32)
    // The virtualizing view, exposed for the windowed-rendering check (T-10.5).
    property alias gridView: grid
    // The live marquee rectangle, exposed so tests can assert the drag-selection
    // visual (macOS Tahoe neutral wash + square grey perimeter).
    property alias marqueeItem: marquee

    // The live rubber band, in view coordinates. `bandActive` gates the
    // marquee and is false for a plain click.
    property real bandStartX: 0
    property real bandStartY: 0
    property bool bandActive: false
    property bool bandMoved: false
    property rect bandRect: Qt.rect(0, 0, 0, 0)

    function isSelected(nodeId) {
        return root.selectedIds.indexOf(nodeId) >= 0;
    }

    function freeOffsetFor(uri) {
        return root.freePositions[uri] || null;
    }

    function setFreeOffset(uri, x, y) {
        var next = {};
        for (var key in root.freePositions)
            next[key] = root.freePositions[key];
        if (Math.abs(x) < 0.5 && Math.abs(y) < 0.5)
            delete next[uri];
        else
            next[uri] = { x: x, y: y };
        root.freePositions = next;
    }

    // Finder's Clean Up: keep the current order but return every free-placed
    // tile to the grid. It is deliberately the only reflow rule: users can
    // otherwise put icons wherever they like, including overlapping.
    function cleanUp() {
        root.freePositions = ({});
    }

    // The tile node ids whose delegate rectangle intersects `rect` (view
    // coordinates). GridView only instantiates visible delegates, so a
    // marquee selects what is on screen, exactly like a Files window.
    function enclosedIds(rect) {
        var ids = [];
        if (!root.directory)
            return ids;
        var right = rect.x + rect.width;
        var bottom = rect.y + rect.height;
        for (var i = 0; i < grid.count; ++i) {
            var tile = grid.itemAtIndex(i);
            if (!tile)
                continue;
            var p = root.mapFromItem(tile, 0, 0);
            if (p.x < right && p.x + tile.width > rect.x
                    && p.y < bottom && p.y + tile.height > rect.y)
                ids.push(tile.nodeId);
        }
        return ids;
    }

    // The tile delegate whose rectangle contains `x, y` (view coordinates), or
    // null. The background band consults this so a press that lands on a tile
    // is left for the tile's own MouseArea (T-19.3).
    function tileAt(x, y) {
        if (!root.directory)
            return null;
        for (var i = 0; i < grid.count; ++i) {
            var tile = grid.itemAtIndex(i);
            if (!tile)
                continue;
            var p = root.mapFromItem(tile, 0, 0);
            if (x >= p.x && x < p.x + tile.width
                    && y >= p.y && y < p.y + tile.height)
                return tile;
        }
        return null;
    }

    // Commit a rubber-band selection: resolve the enclosed ids and emit them
    // with the live modifiers. Exposed so tests (and the desktop shell, once
    // it lands) can drive the same path the pointer band uses.
    function rubberBandSelect(x, y, width, height, modifiers) {
        var rect = Qt.rect(x, y, width, height);
        root.bandActive = false;
        root.marqueeSelected(root.enclosedIds(rect), modifiers);
    }

    // Keep a programmatic selection (the T-10.6c reveal) on screen.
    function scrollToNode(nodeId) {
        var row = root.directory ? root.directory.rowForNodeId(nodeId) : -1;
        if (row >= 0)
            grid.positionViewAtIndex(row, GridView.Center);
    }

    // Empty-space interactions (T-19.3): right-click opens the background
    // menu; a left-button drag paints a rubber band. This sits *above* the
    // grid (z: 1) so a press on the empty background reaches it even though
    // the GridView fills the view; a press that lands on a tile is rejected
    // here so the tile's own MouseArea wins. `preventStealing` keeps the band
    // grab while the pointer sweeps across tiles.
    MouseArea {
        id: band
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        preventStealing: true
        z: 1

        readonly property real threshold: Math.max(2, Theme.primitive.spacing.xs)

        onPressed: (mouse) => {
            if (root.tileAt(mouse.x, mouse.y)) {
                mouse.accepted = false;
                return;
            }
            root.bandMoved = false;
            root.bandActive = false;
            root.bandStartX = mouse.x;
            root.bandStartY = mouse.y;
        }
        onPositionChanged: (mouse) => {
            if (!pressed || !(mouse.buttons & Qt.LeftButton))
                return;
            if (!root.bandActive) {
                if (Math.abs(mouse.x - root.bandStartX) < threshold
                        && Math.abs(mouse.y - root.bandStartY) < threshold)
                    return;
                root.bandActive = true;
                // Capture the pre-drag selection so live Cmd/Shift updates
                // toggle/extend from a stable base instead of flipping the
                // same tile on every pointer move.
                root.marqueeStarted(mouse.modifiers);
            }
            root.bandRect = Qt.rect(Math.min(root.bandStartX, mouse.x),
                                    Math.min(root.bandStartY, mouse.y),
                                    Math.abs(mouse.x - root.bandStartX),
                                    Math.abs(mouse.y - root.bandStartY));
            // Live selection: the shell highlights every enclosed tile while
            // the button is still held, not only after release.
            root.marqueeUpdated(root.enclosedIds(root.bandRect),
                                mouse.modifiers);
        }
        onReleased: (mouse) => {
            if (mouse.button === Qt.LeftButton && root.bandActive) {
                root.bandMoved = true;
                root.rubberBandSelect(root.bandRect.x, root.bandRect.y,
                                      root.bandRect.width, root.bandRect.height,
                                      mouse.modifiers);
            } else {
                root.bandActive = false;
            }
        }
        onClicked: (mouse) => {
            if (mouse.button === Qt.RightButton)
                root.backgroundContextRequested(mouse.x, mouse.y);
            else if (mouse.button === Qt.LeftButton && !root.bandMoved)
                root.selected(0, mouse.modifiers);
        }
    }

    // The marquee rectangle, drawn over the grid while a band is in progress.
    // macOS Tahoe's drag-selection look: a very light, semi-transparent grey
    // wash inside a slightly darker, thicker and less transparent grey square
    // perimeter (no rounding). This is deliberately neutral, not accent-tinted:
    // the band is a transient region marker, not a committed selection.
    Rectangle {
        id: marquee
        visible: root.bandActive
        x: root.bandRect.x
        y: root.bandRect.y
        width: root.bandRect.width
        height: root.bandRect.height
        radius: Theme.controls.marquee.radius
        color: Theme.color.marqueeFill
        border.width: Theme.controls.marquee.borderWidth
        border.color: Theme.color.marqueeBorder
        z: 10
    }

    GridView {
        id: grid
        anchors.fill: parent
        anchors.margins: Theme.primitive.spacing.lg
        clip: true
        cellWidth: root.cellWidth
        cellHeight: root.cellHeight
        model: root.directory
        focus: true

        Accessible.role: Accessible.List
        Accessible.name: qsTr("Icon view")

        delegate: Item {
            id: tile
            required property int nodeId
            required property string name
            required property string uri
            required property bool isDir
            required property string icon
            required property string thumbnail

            width: grid.cellWidth
            height: grid.cellHeight

            readonly property bool isSelected: root.isSelected(tile.nodeId)
            readonly property bool isRenaming: root.renamingId === tile.nodeId
            readonly property var freeOffset: root.freeOffsetFor(tile.uri)
            property real dragOffsetX: 0
            property real dragOffsetY: 0
            property bool dragging: false
            property bool suppressClick: false

            // Free placement translates the tile from its grid slot. The
            // transform (not x/y) keeps GridView's layout ownership intact.
            transform: Translate {
                x: (tile.freeOffset ? tile.freeOffset.x : 0) + tile.dragOffsetX
                y: (tile.freeOffset ? tile.freeOffset.y : 0) + tile.dragOffsetY
            }
            // A dragged item is a floating, semi-transparent copy, like Finder.
            opacity: tile.dragging ? 0.72 : 1.0
            z: tile.dragging ? 100 : (tile.isSelected ? 1 : 0)

            // The icon's own translucent selection square. The label gets a
            // separate pill below; the full tile is never washed in accent.
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                y: Theme.primitive.spacing.lg - Theme.primitive.spacing.sm
                width: root.tileIconSize + 2 * Theme.primitive.spacing.sm
                height: root.tileIconSize + 2 * Theme.primitive.spacing.sm
                radius: Theme.primitive.radius.md
                color: tile.isSelected ? root.tileSelectionColor
                                       : (root.hoverHighlight && hover.hovered
                                          ? Theme.color.controlHover : "transparent")
            }

            Icon {
                anchors.horizontalCenter: parent.horizontalCenter
                y: Theme.primitive.spacing.lg
                name: tile.icon
                size: root.tileIconSize
                color: Theme.color.textPrimary
                // Keep the kind glyph visible until the async preview is
                // ready, and permanently when the file has no thumbnail.
                visible: tile.thumbnail.length === 0
                         || thumbnail.status !== Image.Ready
            }

            // A real preview for local image files. The provider caches it in
            // the freedesktop thumbnail store and loads it off the UI thread.
            Image {
                id: thumbnail
                anchors.horizontalCenter: parent.horizontalCenter
                y: Theme.primitive.spacing.lg
                width: root.tileIconSize
                height: root.tileIconSize
                visible: tile.thumbnail.length > 0 && status === Image.Ready
                source: tile.thumbnail
                sourceSize.width: root.tileIconSize * 2
                sourceSize.height: root.tileIconSize * 2
                asynchronous: true
                cache: true
                smooth: true
                fillMode: Image.PreserveAspectFit
            }

            // The selected label's highlight hugs the text, macOS-style.
            Rectangle {
                id: labelPill
                visible: tile.isSelected && !tile.isRenaming
                anchors.horizontalCenter: parent.horizontalCenter
                y: labelText.y - Theme.primitive.spacing.xs
                width: Math.min(parent.width - Theme.primitive.spacing.sm,
                                labelText.contentWidth + 2 * Theme.primitive.spacing.sm)
                height: labelText.contentHeight + 2 * Theme.primitive.spacing.xs
                radius: Theme.primitive.radius.xs
                color: Theme.color.selection
            }

            // Desktop labels get a slight opposite-tone drop shadow so they
            // stay legible on photographic wallpaper without a plate.
            Text {
                id: labelShadow
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: root.tileIconSize + Theme.primitive.spacing.lg
                                              + Theme.primitive.spacing.sm + 1
                width: parent.width - Theme.primitive.spacing.md
                visible: root.desktopStyle && !tile.isSelected && !tile.isRenaming
                text: tile.name
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                color: root.desktopLabelShadowColor
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                id: labelText
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: root.tileIconSize + Theme.primitive.spacing.lg
                                              + Theme.primitive.spacing.sm
                width: parent.width - Theme.primitive.spacing.md
                visible: !tile.isRenaming
                text: tile.name
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                color: tile.isSelected
                       ? Theme.color.textPrimary
                       : (root.desktopStyle ? root.desktopLabelColor
                                            : Theme.color.textPrimary)
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            // Inline rename (T-10.4c). Return commits, Escape cancels.
            Item {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: root.tileIconSize + Theme.primitive.spacing.lg
                                              + Theme.primitive.spacing.sm
                width: parent.width - Theme.primitive.spacing.md
                height: 24
                visible: tile.isRenaming

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.primitive.radius.xs
                    color: Theme.color.controlFill
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.focusRing
                }

                TextInput {
                    id: editor
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.xs
                    anchors.rightMargin: Theme.primitive.spacing.xs
                    verticalAlignment: TextInput.AlignVCenter
                    horizontalAlignment: TextInput.AlignHCenter
                    clip: true
                    selectByMouse: true
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    text: tile.name
                    onVisibleChanged: {
                        if (visible) {
                            forceActiveFocus();
                            selectAll();
                        }
                    }
                    onAccepted: root.renameSubmitted(tile.nodeId, text)
                    Keys.onEscapePressed: (event) => {
                        root.renameCancelled();
                        event.accepted = true;
                    }
                }
            }

            HoverHandler {
                id: hover
            }

            MouseArea {
                id: tileMouse
                anchors.fill: parent
                enabled: !tile.isRenaming
                acceptedButtons: Qt.LeftButton | Qt.RightButton

                property real pressX: 0
                property real pressY: 0

                onPressed: (mouse) => {
                    if (mouse.button === Qt.LeftButton) {
                        tileMouse.pressX = mouse.x;
                        tileMouse.pressY = mouse.y;
                        tile.dragOffsetX = 0;
                        tile.dragOffsetY = 0;
                        tile.dragging = false;
                        tile.suppressClick = false;
                        // Finder selects on press, so a drag carries the
                        // selection with it.
                        root.selected(tile.nodeId, mouse.modifiers);
                    }
                }
                onPositionChanged: (mouse) => {
                    if (!root.freePlacement || !pressed
                            || !(mouse.buttons & Qt.LeftButton))
                        return;
                    if (!tile.dragging) {
                        if (Math.abs(mouse.x - tileMouse.pressX) < root.dragThreshold
                                && Math.abs(mouse.y - tileMouse.pressY) < root.dragThreshold)
                            return;
                        tile.dragging = true;
                    }
                    tile.dragOffsetX = mouse.x - tileMouse.pressX;
                    tile.dragOffsetY = mouse.y - tileMouse.pressY;
                }
                onReleased: (mouse) => {
                    if (mouse.button !== Qt.LeftButton || !tile.dragging)
                        return;
                    var base = tile.freeOffset;
                    root.setFreeOffset(tile.uri,
                                       (base ? base.x : 0) + tile.dragOffsetX,
                                       (base ? base.y : 0) + tile.dragOffsetY);
                    tile.dragging = false;
                    tile.dragOffsetX = 0;
                    tile.dragOffsetY = 0;
                    tile.suppressClick = true;
                }
                onCanceled: {
                    tile.dragging = false;
                    tile.dragOffsetX = 0;
                    tile.dragOffsetY = 0;
                }
                onDoubleClicked: root.activated(tile.uri, tile.isDir)
                onClicked: (mouse) => {
                    if (mouse.button === Qt.RightButton) {
                        if (!tile.isSelected)
                            root.selected(tile.nodeId, 0);
                        // Report the point in the view's coordinates, not the
                        // tile's, so the shell can place the menu correctly.
                        var p = root.mapFromItem(tile, mouse.x, mouse.y);
                        root.contextRequested(tile.nodeId, tile.uri, tile.isDir,
                                              p.x, p.y);
                    } else if (tile.suppressClick) {
                        // A completed free-placement drag is not a click.
                        tile.suppressClick = false;
                    }
                }
            }
        }
    }
}