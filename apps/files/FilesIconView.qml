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

    readonly property int tileIconSize: 48
    readonly property int cellWidth: 116
    readonly property int cellHeight: 104
    // The virtualizing view, exposed for the windowed-rendering check (T-10.5).
    property alias gridView: grid

    function isSelected(nodeId) {
        return root.selectedIds.indexOf(nodeId) >= 0;
    }

    // Empty-space right-click. Sits under the grid so a tile's own MouseArea
    // wins wherever a tile is painted.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.RightButton
        onClicked: (mouse) => root.backgroundContextRequested(mouse.x, mouse.y)
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