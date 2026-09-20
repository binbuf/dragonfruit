// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One label + control row inside a SettingsGroup. The row owns the label
// column, the control slot, and the separator; it never draws the control.
// AT-SPI sees a labelled grouping so a screen reader reads the control with
// its setting name.
Item {
    id: root

    property string label: ""
    property string description: ""
    property alias controlData: controlSlot.data
    property alias control: controlSlot
    property bool showSeparator: true

    implicitWidth: 480
    implicitHeight: Math.max(Theme.controls.settingsRow.height,
                             labelColumn.implicitHeight + 2 * Theme.primitive.spacing.sm)

    Item {
        id: controlSlot
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.settingsRow.paddingH
        anchors.verticalCenter: parent.verticalCenter
        width: childrenRect.width
        height: childrenRect.height
    }

    Column {
        id: labelColumn
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.settingsRow.paddingH
        anchors.right: controlSlot.left
        anchors.rightMargin: Theme.controls.settingsRow.controlGap
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xxs

        Text {
            width: parent.width
            text: root.label
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            elide: Text.ElideRight
        }
        Text {
            width: parent.width
            visible: root.description.length > 0
            text: root.description
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
    }

    Rectangle {
        visible: root.showSeparator
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.settingsRow.paddingH
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.settingsRow.paddingH
        anchors.bottom: parent.bottom
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.label
    Accessible.description: root.description
}
