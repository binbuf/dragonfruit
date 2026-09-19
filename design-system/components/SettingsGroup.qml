// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// A titled card of SettingsRows (or any content) on an elevated surface. The
// group owns the card, the optional section header, and the trailing gap; the
// rows are supplied through the default property so apps never draw the card.
Column {
    id: root

    property string title: ""
    property bool reserveBottomMargin: true
    default property alias contentData: rows.data
    property alias rows: rows

    spacing: 0

    Text {
        visible: root.title.length > 0
        width: parent.width
        bottomPadding: Theme.primitive.spacing.sm
        text: root.title
        color: Theme.color.textSecondary
        font.pixelSize: Theme.primitive.font.sizeSm
        font.weight: Theme.primitive.font.weightMedium
    }

    Item {
        id: body
        width: root.width
        height: rows.implicitHeight + 2 * Theme.controls.settingsGroup.padding

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.settingsGroup.radius
            color: Theme.color.surfaceElevated
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        Column {
            id: rows
            x: Theme.controls.settingsGroup.padding
            y: Theme.controls.settingsGroup.padding
            width: body.width - 2 * Theme.controls.settingsGroup.padding
            spacing: Theme.controls.settingsGroup.rowGap
        }
    }

    Item {
        width: parent.width
        height: root.reserveBottomMargin ? Theme.controls.settingsGroup.marginBottom : 0
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.title
}
