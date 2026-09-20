// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// One menu-bar status-item slot (T-09 FR-5). First-party items and (later,
// T-30) StatusNotifier items both render through this slot, so sizing,
// hover, and dark/light treatment are identical everywhere.
//
// Graceful degradation (FR-4): `available: false` hides the item entirely
// when its backing daemon is absent; `enabled: false` keeps it visible but
// dimmed when the adapter is present but the feature is off.
Item {
    id: root

    property string itemId: ""
    property string icon: ""
    property string label: ""
    property string accessibleName: ""
    property bool available: true
    property bool selected: false
    property color tint: Theme.color.textPrimary
    property color backgroundColor: Theme.color.chrome

    signal activated(string itemId)

    visible: available
    enabled: available
    implicitWidth: available ? slotContent.implicitWidth + 2 * Theme.controls.menuBar.statusItemPaddingH : 0
    implicitHeight: Theme.controls.menuBar.height
    opacity: enabled ? 1.0 : 0.4

    Rectangle {
        id: slotBackground
        anchors.centerIn: parent
        width: root.width
        height: Theme.controls.menuBar.height - 2 * Theme.primitive.spacing.xs
        radius: Theme.controls.menuBar.hoverRadius
        color: root.selected ? Theme.color.accentMuted
                             : (slotHover.hovered ? Theme.color.controlHover : "transparent")
        antialiasing: true
    }

    Row {
        id: slotContent
        anchors.centerIn: parent
        spacing: root.label.length > 0 ? Theme.controls.menuBar.labelGap : 0

        StatusGlyph {
            id: glyph
            visible: root.icon.length > 0
            name: root.icon
            color: root.selected ? Theme.color.accent : root.tint
            backgroundColor: root.backgroundColor
            size: Theme.controls.menuBar.iconSize
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: labelText
            visible: root.label.length > 0
            text: root.label
            color: root.selected ? Theme.color.accent : root.tint
            font.pixelSize: Theme.controls.menuBar.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    HoverHandler { id: slotHover }

    TapHandler {
        onTapped: root.activated(root.itemId)
    }

    Accessible.role: Accessible.Button
    Accessible.name: root.accessibleName.length > 0 ? root.accessibleName
                                                    : (root.label.length > 0 ? root.label : root.itemId)
    Accessible.focusable: true
    Accessible.onPressAction: root.activated(root.itemId)
}
