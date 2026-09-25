// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings detail-pane header card (T-09.1a): the pane's large icon, its
// title, and the one-line description, exactly as the macOS reference shows.
// It is shell chrome — pane bodies are supplied by T-09.2…T-09.5 — so it takes
// the catalog entry and never derives anything itself.
Item {
    id: root

    property var pane: null

    implicitHeight: layout.implicitHeight
    height: layout.implicitHeight

    Row {
        id: layout
        width: parent.width
        spacing: Theme.primitive.spacing.lg

        Icon {
            id: glyph
            name: root.pane ? root.pane.icon : ""
            size: Theme.primitive.spacing.xxl
            color: Theme.color.accent
            anchors.verticalCenter: parent.verticalCenter
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