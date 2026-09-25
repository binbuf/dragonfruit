// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files icon view (T-10.4b): a centered grid of icon tiles (glyph over a
// two-line label) over the files-core listing. Selection is the node id the
// shell owns, so it survives the view switch; opening is a double click.
// Rubber-band selection and icon-size gestures are T-10.4c.
FocusScope {
    id: root

    // The `FilesDirectoryModel` (files-core facade). The view renders it and
    // never touches the filesystem.
    property var directory: null
    // The node id the shell has selected.
    property real selectedId: 0

    signal selected(real nodeId)
    signal activated(string uri, bool isDir)

    readonly property int tileIconSize: 48
    readonly property int cellWidth: 116
    readonly property int cellHeight: 104

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

            readonly property bool isSelected: root.selectedId === tile.nodeId

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
                text: tile.name
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                color: tile.isSelected ? Theme.color.accent : Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            HoverHandler {
                id: hover
            }

            MouseArea {
                anchors.fill: parent
                onPressed: root.selected(tile.nodeId)
                onDoubleClicked: root.activated(tile.uri, tile.isDir)
            }
        }
    }
}