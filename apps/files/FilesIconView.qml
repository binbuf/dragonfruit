// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files icon view (T-10.4b/T-10.4c): a centered grid of icon tiles (glyph
// over a two-line label) over the files-core listing. Selection is the set of
// node ids the shell owns, so it survives the view switch (T-10.4c multi-
// select); opening is a double click; right-click opens the context menu;
// Return/`renamingId` swaps the label for an inline editor.
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

    signal selected(real nodeId, int modifiers)
    signal activated(string uri, bool isDir)
    signal contextRequested(real nodeId, string uri, bool isDir, real x, real y)
    signal backgroundContextRequested(real x, real y)
    signal renameSubmitted(real nodeId, string name)
    signal renameCancelled()
    // Rubber-band selection (T-19.3): the node ids enclosed by the marquee,
    // with the live modifier state so the shell can toggle (Cmd) or extend
    // (Shift) instead of replacing.
    signal marqueeSelected(var nodeIds, int modifiers)

    readonly property int tileIconSize: 48
    readonly property int cellWidth: 116
    readonly property int cellHeight: 104
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
            }
            root.bandRect = Qt.rect(Math.min(root.bandStartX, mouse.x),
                                    Math.min(root.bandStartY, mouse.y),
                                    Math.abs(mouse.x - root.bandStartX),
                                    Math.abs(mouse.y - root.bandStartY));
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

            width: grid.cellWidth
            height: grid.cellHeight

            readonly property bool isSelected: root.isSelected(tile.nodeId)
            readonly property bool isRenaming: root.renamingId === tile.nodeId

            Rectangle {
                anchors.centerIn: parent
                width: Math.min(parent.width - Theme.primitive.spacing.sm,
                                root.cellWidth - Theme.primitive.spacing.sm)
                height: parent.height - Theme.primitive.spacing.sm
                radius: Theme.primitive.radius.md
                color: tile.isSelected ? Theme.color.accentMuted
                                       : (hover.hovered ? Theme.color.controlHover
                                                        : "transparent")
            }

            Icon {
                anchors.horizontalCenter: parent.horizontalCenter
                y: Theme.primitive.spacing.lg
                name: tile.icon
                size: root.tileIconSize
                color: tile.isSelected ? Theme.color.accent
                                       : Theme.color.textPrimary
            }

            Text {
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
                color: tile.isSelected ? Theme.color.accent : Theme.color.textPrimary
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
                anchors.fill: parent
                enabled: !tile.isRenaming
                acceptedButtons: Qt.LeftButton | Qt.RightButton
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
                    } else {
                        root.selected(tile.nodeId, mouse.modifiers);
                    }
                }
            }
        }
    }
}