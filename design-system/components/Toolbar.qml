// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// A window toolbar: a chrome strip with a leading content slot (navigation
// buttons, a search field, the title) and a trailing slot (actions). Apps
// place design-system controls in the slots; the bar itself only owns the
// surface and spacing tokens.
Item {
    id: root

    property string title: ""
    default property alias contentData: leadingRow.data
    property alias leading: leadingRow
    property alias trailingData: trailingRow.data
    property alias trailing: trailingRow

    implicitWidth: 480
    implicitHeight: Theme.controls.toolbar.height

    Rectangle {
        anchors.fill: parent
        color: Theme.color.chrome
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.controls.window.borderWidth
        color: Theme.color.border
    }

    Text {
        visible: root.title.length > 0
        anchors.centerIn: parent
        width: parent.width - 2 * Theme.controls.toolbar.paddingH
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        text: root.title
        color: Theme.color.textPrimary
        font.pixelSize: Theme.controls.button.fontSize
        font.weight: Theme.controls.button.fontWeight
    }

    Row {
        id: leadingRow
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.toolbar.paddingH
        anchors.right: trailingRow.left
        anchors.rightMargin: Theme.controls.toolbar.spacing
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.controls.toolbar.spacing
    }

    Row {
        id: trailingRow
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.toolbar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.controls.toolbar.spacing
    }

    Accessible.role: Accessible.ToolBar
    Accessible.name: root.title
}
