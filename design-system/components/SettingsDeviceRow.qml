// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// A device row (T286): the Sound output/input and Bluetooth device shape. It
// is the shared `SettingsRow` anatomy with a 29 pt glyph tile and a status
// line; the pane supplies the live device state.
SettingsRow {
    id: root

    property string deviceGlyph: ""
    property string statusText: ""
    property bool deviceActive: true
    property var category: null

    description: root.statusText
    rowHeight: Theme.controls.settingsRow.iconHeight
    leadingTile: root.category
    leadingTileSize: Theme.controls.settingsRow.leadingTile
    labelIndent: Theme.controls.settingsRow.paddingH
                 + Theme.controls.settingsRow.leadingTile
                 + Theme.primitive.spacing.md

    leadingData: Item {
        width: Theme.controls.settingsRow.leadingTile
        height: width

        Rectangle {
            anchors.fill: parent
            radius: Theme.primitive.radius.sm
            color: Theme.color.controlFill
            antialiasing: true
        }

        Icon {
            anchors.centerIn: parent
            name: root.deviceGlyph
            size: Theme.controls.button.iconSize
            color: root.deviceActive ? Theme.color.textPrimary
                                     : Theme.color.textTertiary
        }
    }
}
