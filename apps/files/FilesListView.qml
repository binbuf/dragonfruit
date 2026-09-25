// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files list view (T-10.4b/T-10.4c): `Name` first, then Date Modified /
// Size / Kind with sortable headers, over the files-core listing. Selection is
// the set of node ids the shell owns, so it survives the view switch
// (T-10.4c multi-select); right-click opens the context menu; Return/`renamingId`
// swaps the name for an inline editor.
Item {
    id: root

    property var directory: null
    property real selectedId: 0
    property var selectedIds: []
    property real renamingId: 0

    signal selected(real nodeId, int modifiers)
    signal activated(string uri, bool isDir)
    signal contextRequested(real nodeId, string uri, bool isDir, real x, real y)
    signal backgroundContextRequested(real x, real y)
    signal renameSubmitted(real nodeId, string name)
    signal renameCancelled()

    // The virtualizing view, exposed for the windowed-rendering check (T-10.5).
    property alias listView: list
    readonly property int rowHeight: 28
    readonly property int modifiedWidth: 170
    readonly property int sizeWidth: 90
    readonly property int kindWidth: 110
    readonly property int gutter: Theme.primitive.spacing.lg
    readonly property int nameWidth: Math.max(160, list.width
                                              - root.modifiedWidth - root.sizeWidth
                                              - root.kindWidth - 2 * root.gutter)

    function isSelected(nodeId) {
        return root.selectedIds.indexOf(nodeId) >= 0;
    }

    // Keep a programmatic selection (the T-10.6c reveal) on screen.
    function scrollToNode(nodeId) {
        var row = root.directory ? root.directory.rowForNodeId(nodeId) : -1;
        if (row >= 0)
            list.positionViewAtIndex(row, ListView.Center);
    }

    // Empty-space right-click under the table.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.RightButton
        onClicked: (mouse) => root.backgroundContextRequested(mouse.x, mouse.y)
    }

    component HeaderCell: Item {
        id: cell
        property string label: ""
        property string sortKey: ""
        property int cellWidth: root.nameWidth
        width: cell.cellWidth
        height: 28

        readonly property bool active: root.directory
                                       && root.directory.sortKey === cell.sortKey

        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: Theme.primitive.radius.xs
            color: headerHover.hovered ? Theme.color.controlHover : "transparent"
        }

        Text {
            id: headerText
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.primitive.spacing.sm
            text: cell.label
            color: cell.active ? Theme.color.textPrimary : Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            font.weight: cell.active ? Theme.primitive.font.weightMedium
                                     : Theme.primitive.font.weightRegular
        }

        Icon {
            visible: cell.active
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.primitive.spacing.sm
                   + headerText.contentWidth + Theme.primitive.spacing.xs
            name: "chevron-down"
            size: 12
            rotation: root.directory && root.directory.sortAscending ? 0 : 180
            color: Theme.color.textSecondary
        }

        HoverHandler {
            id: headerHover
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                if (root.directory)
                    root.directory.sortBy(cell.sortKey);
            }
        }
    }

    Column {
        anchors.fill: parent

        // Fixed column header; the rows scroll under it.
        Rectangle {
            id: header
            width: parent.width
            height: 32
            color: Theme.color.surface

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 1
                color: Theme.color.separator
            }

            Row {
                anchors.fill: parent
                anchors.leftMargin: root.gutter
                anchors.rightMargin: root.gutter
                spacing: 0

                HeaderCell {
                    label: qsTr("Name")
                    sortKey: "name"
                    cellWidth: root.nameWidth
                }
                HeaderCell {
                    label: qsTr("Date Modified")
                    sortKey: "modified"
                    cellWidth: root.modifiedWidth
                }
                HeaderCell {
                    label: qsTr("Size")
                    sortKey: "size"
                    cellWidth: root.sizeWidth
                }
                HeaderCell {
                    label: qsTr("Kind")
                    sortKey: "kind"
                    cellWidth: root.kindWidth
                }
            }
        }

        ListView {
            id: list
            width: parent.width
            height: parent.height - header.height
            clip: true
            model: root.directory
            focus: true

            Accessible.role: Accessible.Table
            Accessible.name: qsTr("List view")

            delegate: Item {
                id: row
                required property int nodeId
                required property string name
                required property string uri
                required property bool isDir
                required property string icon
                required property string sizeText
                required property string modifiedText
                required property string kindText

                width: list.width
                height: root.rowHeight

                readonly property bool isSelected: root.isSelected(row.nodeId)
                readonly property bool isRenaming: root.renamingId === row.nodeId

                Rectangle {
                    anchors.fill: parent
                    color: row.isSelected
                           ? Theme.color.accentMuted
                           : (rowHover.hovered ? Theme.color.controlHover : "transparent")
                }

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: root.gutter
                    anchors.rightMargin: root.gutter
                    spacing: 0

                    Item {
                        width: root.nameWidth
                        height: parent.height

                        Icon {
                            id: rowIcon
                            anchors.verticalCenter: parent.verticalCenter
                            name: row.icon
                            size: 16
                            color: row.isSelected ? Theme.color.accent
                                                  : Theme.color.textSecondary
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: rowIcon.right
                            anchors.leftMargin: Theme.primitive.spacing.sm
                            anchors.right: parent.right
                            visible: !row.isRenaming
                            text: row.name
                            elide: Text.ElideMiddle
                            color: row.isSelected ? Theme.color.accent
                                                  : Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeSm
                        }

                        // Inline rename (T-10.4c): Return commits, Escape cancels.
                        Rectangle {
                            visible: row.isRenaming
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: rowIcon.right
                            anchors.leftMargin: Theme.primitive.spacing.xs
                            anchors.right: parent.right
                            height: root.rowHeight - Theme.primitive.spacing.xs
                            radius: Theme.primitive.radius.xs
                            color: Theme.color.controlFill
                            border.width: Theme.controls.window.borderWidth
                            border.color: Theme.color.focusRing

                            TextInput {
                                id: rowEditor
                                anchors.fill: parent
                                anchors.leftMargin: Theme.primitive.spacing.xs
                                anchors.rightMargin: Theme.primitive.spacing.xs
                                verticalAlignment: TextInput.AlignVCenter
                                clip: true
                                selectByMouse: true
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                text: row.name
                                onVisibleChanged: {
                                    if (visible) {
                                        forceActiveFocus();
                                        selectAll();
                                    }
                                }
                                onAccepted: root.renameSubmitted(row.nodeId, text)
                                Keys.onEscapePressed: (event) => {
                                    root.renameCancelled();
                                    event.accepted = true;
                                }
                            }
                        }
                    }

                    Text {
                        width: root.modifiedWidth
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        leftPadding: Theme.primitive.spacing.sm
                        text: row.modifiedText
                        elide: Text.ElideRight
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Text {
                        width: root.sizeWidth
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        horizontalAlignment: Text.AlignRight
                        rightPadding: Theme.primitive.spacing.sm
                        text: row.sizeText
                        elide: Text.ElideRight
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Text {
                        width: root.kindWidth
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        leftPadding: Theme.primitive.spacing.sm
                        text: row.kindText
                        elide: Text.ElideRight
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }

                HoverHandler {
                    id: rowHover
                }

                MouseArea {
                    anchors.fill: parent
                    enabled: !row.isRenaming
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onDoubleClicked: root.activated(row.uri, row.isDir)
                    onClicked: (mouse) => {
                        if (mouse.button === Qt.RightButton) {
                            if (!row.isSelected)
                                root.selected(row.nodeId, 0);
                            // Report the point in the view's coordinates, not
                            // the row's, so the shell places the menu right.
                            var p = root.mapFromItem(row, mouse.x, mouse.y);
                            root.contextRequested(row.nodeId, row.uri, row.isDir,
                                                  p.x, p.y);
                        } else {
                            root.selected(row.nodeId, mouse.modifiers);
                        }
                    }
                }
            }
        }
    }
}