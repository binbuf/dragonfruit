// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files status bar (T230d): a 28 pt content-layer bar at the bottom edge.
// It reports the item count, the selection count and the current volume's
// free space. It is content, not window chrome, so it does not increase the
// 52 pt toolbar height; when the toolbar is hidden the sidebar disappears and
// this bar spans the full window width.
Item {
    id: root

    property int itemCount: 0
    property int selectionCount: 0
    property string freeSpaceText: ""

    readonly property string itemText:
        root.itemCount === 1 ? qsTr("1 item")
                             : qsTr("%1 items").arg(root.itemCount)
    readonly property string selectionText:
        qsTr("%1 selected").arg(root.selectionCount)

    implicitHeight: Theme.controls.accessoryBar.status

    Rectangle {
        anchors.fill: parent
        color: Theme.color.surface
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.controls.window.borderWidth
        color: Theme.color.border
    }

    Text {
        id: itemLabel
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.toolbar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        text: root.itemText
        color: Theme.color.textSecondary
        font.pixelSize: Theme.primitive.font.sizeSm
        elide: Text.ElideRight
    }

    Text {
        id: selectionLabel
        anchors.left: itemLabel.right
        anchors.leftMargin: Theme.primitive.spacing.md
        anchors.verticalCenter: parent.verticalCenter
        visible: root.selectionCount > 0
        text: root.selectionText
        color: Theme.color.textPrimary
        font.pixelSize: Theme.primitive.font.sizeSm
        elide: Text.ElideRight
    }

    Text {
        id: freeSpaceLabel
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.toolbar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        text: root.freeSpaceText
        color: Theme.color.textSecondary
        font.pixelSize: Theme.primitive.font.sizeSm
        elide: Text.ElideLeft
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Status bar")
}
