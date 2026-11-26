// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// A Startup Disk row (T286): the disk name, its capacity, a usage bar, and an
// optional action. The pane supplies the real sampler values.
SettingsRow {
    id: root

    property string diskName: ""
    property string capacityText: ""
    property real usedFraction: 0

    label: root.diskName
    description: root.capacityText
    rowHeight: Theme.controls.settingsRow.iconHeight

    Column {
        width: Math.min(root.width * 0.4, 160)
        spacing: Theme.primitive.spacing.xs
        anchors.verticalCenter: parent.verticalCenter

        Item {
            width: parent.width
            height: Theme.primitive.spacing.xs

            Rectangle {
                anchors.fill: parent
                radius: height / 2
                color: Theme.color.controlFill
                antialiasing: true
            }

            Rectangle {
                width: parent.width
                       * Math.max(0, Math.min(1, root.usedFraction))
                height: parent.height
                radius: height / 2
                color: Theme.color.accent
                antialiasing: true
            }
        }

        Text {
            width: parent.width
            text: root.capacityText
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            elide: Text.ElideRight
        }
    }
}
