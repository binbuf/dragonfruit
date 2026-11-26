// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// A big-list row (T286): 52 pt, a 29 pt category tile, a label, an optional
// second line, and a trailing chevron. It is a variant of the shared
// `SettingsRow` anatomy (same tile, focus ring, AT-SPI, and stationary-click
// path), used by `SettingsBigList`.
SettingsRow {
    id: root

    property var category: null
    property string categoryGlyph: root.category ? root.category.source : ""
    property string secondaryText: ""

    rowHeight: Theme.controls.settingsRow.bigHeight
    fixedRowHeight: true
    description: root.secondaryText
    disclosure: true
    leadingTile: root.category
    leadingTileSize: Theme.controls.settingsRow.leadingTile
    labelIndent: root.category !== null
                 ? Theme.controls.settingsRow.paddingH
                   + Theme.controls.settingsRow.leadingTile
                   + Theme.primitive.spacing.md
                 : -1
}
