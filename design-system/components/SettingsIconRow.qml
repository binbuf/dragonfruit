// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The 53 pt icon row (T286): a 29 pt leading category tile beside a label and
// an optional description. It is the same SettingsRow anatomy, not a second
// widget; a pane sets `category` (a resolved `SettingsCategoryIcon` style).
SettingsRow {
    id: root

    property var category: null
    property string categoryGlyph: root.category ? root.category.source : ""

    rowHeight: Theme.controls.settingsRow.iconHeight
    leadingTile: root.category
    leadingTileSize: Theme.controls.settingsRow.leadingTile
}
