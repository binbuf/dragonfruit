// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings detail-pane header card (T-09.1a): the pane's icon, its
// title, and its one-line description. Only panes whose macOS reference has a
// header card render it (see `SettingsPanes.headerPanes`); the macOS
// treatments vary by pane (a left-aligned row for Accessibility, a centered
// hero for General, an icon tile), so this is the shared left-aligned shape
// and the shell hides it where the reference has none. It is shell chrome —
// pane bodies are supplied by T-09.2…T-09.5 — so it takes the catalog entry
// and never derives anything itself.
Item {
    id: root

    property var pane: null

    // The pane's category-tile style (T-19.1b), or null when the icon has no
    // Phosphor mapping and the original `Icon` glyph is kept.
    readonly property var category: SettingsPanes.categoryStyle(root.pane)
    // The resolved Phosphor name, kept out of the `SettingsCategoryIcon` block
    // so the static glyph-reference scan only ever sees a bound value.
    readonly property string categoryGlyph: root.category ? root.category.source : ""

    implicitHeight: layout.implicitHeight
    height: layout.implicitHeight

    Row {
        id: layout
        width: parent.width
        spacing: Theme.primitive.spacing.lg

        Item {
            id: glyph
            readonly property bool tiled: root.category !== null
            readonly property real glyphSize: tiled
                    ? Theme.primitive.spacing.xxxl : Theme.primitive.spacing.xxl
            width: glyphSize
            height: glyphSize
            anchors.verticalCenter: parent.verticalCenter

            SettingsCategoryIcon {
                anchors.fill: parent
                visible: glyph.tiled
                source: root.categoryGlyph
                gradientStart: root.category ? root.category.gradientStart : Theme.color.accent
                gradientEnd: root.category ? root.category.gradientEnd : Theme.color.accent
            }

            Icon {
                anchors.centerIn: parent
                visible: !glyph.tiled
                name: root.pane ? root.pane.icon : ""
                size: Theme.primitive.spacing.xxl
                color: Theme.color.accent
            }
        }

        Column {
            width: Math.max(0, layout.width - glyph.width - layout.spacing)
            spacing: Theme.primitive.spacing.xxs

            Text {
                width: parent.width
                text: root.pane ? root.pane.title : ""
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeXxl
                font.weight: Theme.primitive.font.weightBold
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                visible: root.pane && root.pane.description.length > 0
                text: root.pane ? root.pane.description : ""
                color: Theme.color.textSecondary
                font.pixelSize: Theme.controls.button.fontSize
                wrapMode: Text.WordWrap
            }
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.pane ? root.pane.title : ""
    Accessible.description: root.pane ? root.pane.description : ""
}