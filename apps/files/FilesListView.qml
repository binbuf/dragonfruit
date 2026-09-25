// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files list view (T-10.4b): `Name` first, then Date Modified / Size /
// Kind with sortable headers, over the files-core listing. Selection is the
// node id the shell owns, so it survives the view switch. Disclosure
// triangles, inline rename, and column resizing are T-10.4c.
Item {
    id: root

    property var directory: null
    property real selectedId: 0

    signal selected(real nodeId)
    signal activated(string uri, bool isDir)

    readonly property int rowHeight: 28
    readonly property int modifiedWidth: 170
    readonly property int sizeWidth: 90
    readonly property int kindWidth: 110
    readonly property int gutter: Theme.primitive.spacing.lg
    readonly property int nameWidth: Math.max(160, list.width
                                              - root.modifiedWidth - root.sizeWidth
                                              - root.kindWidth - 2 * root.gutter)

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

                readonly property bool isSelected: root.selectedId === row.nodeId

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
                            text: row.name
                            elide: Text.ElideMiddle
                            color: row.isSelected ? Theme.color.accent
                                                  : Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeSm
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
                    onPressed: root.selected(row.nodeId)
                    onDoubleClicked: root.activated(row.uri, row.isDir)
                }
            }
        }
    }
}