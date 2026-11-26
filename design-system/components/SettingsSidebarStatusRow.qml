// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The Settings sidebar's update status row (T286): a category tile, the
// provider's status label, a stable secondary line, and an optional badge.
// The label and badge come from the update provider; the row opens
// General -> Software Update.
Item {
    id: root

    property string label: ""
    property string statusLabel: ""
    property string icon: ""
    property string badge: ""
    property var category: null
    property bool selected: false
    property bool rtl: false
    property alias labelText: labelText
    property alias statusText: statusText
    property alias badgeRect: badgeRect

    implicitWidth: 200
    implicitHeight: Theme.controls.settingsSidebar.rowHeight

    readonly property string categoryGlyph: root.category ? root.category.source : ""
    readonly property real glyphSlot: root.category !== null
                                      ? Theme.controls.sidebar.categoryIconSize
                                      : Theme.controls.sidebar.iconSize

    Item {
        id: glyph
        width: root.glyphSlot
        height: width
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.sidebar.padding
        anchors.verticalCenter: parent.verticalCenter

        SettingsCategoryIcon {
            anchors.fill: parent
            visible: root.category !== null
            source: root.categoryGlyph
            gradientStart: root.category ? root.category.gradientStart
                                         : Theme.color.accent
            gradientEnd: root.category ? root.category.gradientEnd
                                       : Theme.color.accent
        }

        Icon {
            anchors.centerIn: parent
            visible: root.category === null
            name: root.icon
            size: Theme.controls.sidebar.iconSize
            color: root.selected ? Theme.color.textPrimary
                                 : Theme.color.textSecondary
        }
    }

    Column {
        id: labelColumn
        anchors.left: glyph.right
        anchors.leftMargin: Theme.controls.sidebar.padding
        anchors.right: badgeRect.visible ? badgeRect.left : parent.right
        anchors.rightMargin: Theme.controls.sidebar.padding
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xxs

        Text {
            id: labelText
            width: parent.width
            text: root.label
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.weight: root.selected ? Theme.primitive.font.weightSemibold
                                       : Theme.primitive.font.weightRegular
            elide: Text.ElideRight
            horizontalAlignment: root.rtl ? Text.AlignRight : Text.AlignLeft
        }

        Text {
            id: statusText
            width: parent.width
            visible: root.statusLabel.length > 0
            text: root.statusLabel
            color: Glass.increaseContrast ? Theme.color.textPrimary
                                          : Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeXs
            elide: Text.ElideRight
            horizontalAlignment: root.rtl ? Text.AlignRight : Text.AlignLeft
        }
    }

    Rectangle {
        id: badgeRect
        visible: root.badge.length > 0
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.sidebar.padding
        anchors.verticalCenter: parent.verticalCenter
        width: badgeLabel.implicitWidth + 2 * Theme.primitive.spacing.xs
        height: badgeLabel.implicitHeight + Theme.primitive.spacing.xxs
        radius: height / 2
        color: Theme.color.controlActive

        Text {
            id: badgeLabel
            anchors.centerIn: parent
            text: root.badge
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeXs
        }
    }

    Accessible.role: Accessible.ListItem
    Accessible.name: root.statusLabel.length > 0
                     ? root.label + ", " + root.statusLabel : root.label
    Accessible.description: qsTr("Software update status")
    Accessible.selected: root.selected
    Accessible.selectable: true
    Accessible.focusable: false
}
