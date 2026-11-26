// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// A card of big rows (T286): the General, Accessibility, Privacy & Security,
// and Focus list shape. The card has no inner padding; the rows own their
// 12 pt horizontal inset, and the last row suppresses its separator through
// the shared `SettingsGroup` rule.
Column {
    id: root

    property bool reserveBottomMargin: false
    default property alias contentData: rows.data
    property alias rows: rows

    width: parent ? parent.width : implicitWidth
    spacing: 0

    Item {
        id: body
        width: root.width
        height: rows.implicitHeight

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.settingsGroup.radius
            color: Theme.color.surfaceElevated
            border.width: Theme.controls.window.borderWidth
            border.color: Glass.increaseContrast ? Theme.color.textPrimary
                                                 : Theme.color.border
            antialiasing: true
        }

        Column {
            id: rows
            property bool settingsGroupRows: true
            width: body.width
            spacing: Theme.controls.settingsGroup.rowGap
        }
    }

    Item {
        width: parent.width
        height: root.reserveBottomMargin
                ? Theme.controls.settingsGroup.marginBottom : 0
    }

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Settings list")
}
