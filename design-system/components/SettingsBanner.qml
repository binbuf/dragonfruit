// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A titled banner for a service-owned Settings surface (T286). The banner
// names the delegate and offers its one action; it is not a fake row. Pane
// tasks set `title`, `text`, and optionally `actionText`.
Rectangle {
    id: root

    property string title: ""
    property string text: ""
    property string actionText: ""
    property alias titleText: titleLabel
    property alias bodyText: bodyLabel
    property alias actionButton: actionButton

    signal actionClicked()

    implicitWidth: 320
    implicitHeight: layout.implicitHeight
                     + 2 * Theme.primitive.spacing.md
    radius: Theme.controls.settingsGroup.radius
    color: Theme.color.surfaceElevated
    border.width: Theme.controls.window.borderWidth
    border.color: Glass.increaseContrast ? Theme.color.textPrimary
                                         : Theme.color.border
    antialiasing: true

    Column {
        id: layout
        x: Theme.controls.settingsRow.paddingH
        y: Theme.primitive.spacing.md
        width: Math.max(0, root.width - 2 * Theme.controls.settingsRow.paddingH)
        spacing: Theme.primitive.spacing.xs

        Text {
            id: titleLabel
            width: parent.width
            visible: root.title.length > 0
            text: root.title
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.weight: Theme.primitive.font.weightSemibold
            elide: Text.ElideRight
            horizontalAlignment: root.LayoutMirroring.enabled
                                 ? Text.AlignRight : Text.AlignLeft
        }

        Text {
            id: bodyLabel
            width: parent.width
            visible: root.text.length > 0
            text: root.text
            color: Glass.increaseContrast ? Theme.color.textPrimary
                                          : Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
            horizontalAlignment: root.LayoutMirroring.enabled
                                 ? Text.AlignRight : Text.AlignLeft
        }

        Button {
            id: actionButton
            visible: root.actionText.length > 0
            text: root.actionText
            variant: "secondary"
            onClicked: root.actionClicked()
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.title
    Accessible.description: root.text
}
