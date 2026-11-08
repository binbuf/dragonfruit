// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One row of a menu. Both the top bar's dropdowns (`MenuBarMenu`) and pointer
// context menus (`ContextMenu`) use this component, so the row anatomy cannot
// drift:
//
//   [check/icon gutter] label (flex, elides) [shortcut hint] [chevron gutter]
//
// The check and chevron gutters are reserved on every row even when empty, so
// labels and shortcut hints line up across rows and between menus. The
// shortcut hint is the one `ShortcutHint` renderer.
Item {
    id: root

    required property var entry
    property bool highlighted: false
    property int rowHeight: Theme.controls.contextMenu.rowHeight
    property int padding: Theme.controls.contextMenu.padding
    property int labelFontSize: Theme.controls.button.fontSize
    property int checkSize: Theme.controls.button.fontSize
    property int shortcutGap: Theme.controls.contextMenu.shortcutGap

    signal rowHovered(int index)
    signal rowActivated(int index)

    readonly property bool isSeparator: root.entry.type === "separator"
    readonly property bool hasSubmenu: root.entry.type === "submenu"
    readonly property bool disabled: root.entry.enabled === false
    readonly property string disabledReason: root.entry.disabledReason || ""

    // The row contributes its natural width to the Column first, then
    // stretches to the resolved menu width so every highlight bar spans the
    // full panel. `Math.max` keeps the initial zero-width pass from collapsing
    // the menu to its minimum width.
    width: Math.max(implicitWidth, parent ? parent.width : 0)
    height: root.isSeparator ? root.padding : root.rowHeight
    implicitWidth: root.isSeparator ? 0
                   : root.padding * 2
                     + checkGlyph.width
                     + Theme.primitive.spacing.sm
                     + labelMeasure.implicitWidth
                     + (shortcutHint.hasContent
                        ? root.shortcutGap + shortcutHint.implicitWidth : 0)
                     + Theme.primitive.spacing.sm
                     + chevron.width
    opacity: root.entry.enabled !== false ? 1.0 : 0.4

    Rectangle {
        visible: root.isSeparator
        anchors.verticalCenter: parent.verticalCenter
        x: root.padding
        width: parent.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
    }

    Rectangle {
        visible: !root.isSeparator
        anchors.fill: parent
        radius: Theme.controls.focusRing.radius
        color: root.highlighted ? Theme.color.accent : "transparent"
        antialiasing: true
    }

    TablerIcon {
        id: checkGlyph
        visible: !root.isSeparator
        name: "check"
        size: root.checkSize
        color: root.highlighted ? Theme.color.accentContent : Theme.color.accent
        opacity: root.entry.checked === true ? 1 : 0
        x: root.padding
        anchors.verticalCenter: parent.verticalCenter
    }

    // The hidden measure keeps the row's implicit width independent of the
    // anchored label's constrained width, so a menu grows to fit its longest
    // label instead of eliding it at the minimum width.
    Text {
        id: labelMeasure
        visible: false
        text: root.entry.label || ""
        font.pixelSize: root.labelFontSize
    }

    Text {
        id: labelText
        visible: !root.isSeparator
        text: root.entry.label || ""
        color: root.highlighted ? Theme.color.accentContent : Theme.color.textPrimary
        font.pixelSize: root.labelFontSize
        anchors.left: checkGlyph.right
        anchors.leftMargin: Theme.primitive.spacing.sm
        anchors.right: shortcutHint.hasContent ? shortcutHint.left : chevron.left
        anchors.rightMargin: shortcutHint.hasContent ? root.shortcutGap
                                                     : Theme.primitive.spacing.sm
        anchors.verticalCenter: parent.verticalCenter
        elide: Text.ElideRight
        maximumLineCount: 1
    }

    ShortcutHint {
        id: shortcutHint
        shortcut: root.entry.shortcut || ""
        modifierColor: root.highlighted ? Theme.color.accentContent
                                        : Theme.color.textTertiary
        keyColor: root.highlighted ? Theme.color.accentContent
                                   : Theme.color.textSecondary
        anchors.right: chevron.left
        anchors.rightMargin: Theme.primitive.spacing.sm
        anchors.verticalCenter: parent.verticalCenter
    }

    TablerIcon {
        id: chevron
        visible: !root.isSeparator
        name: "chevron-right"
        size: root.checkSize
        color: root.highlighted ? Theme.color.accentContent : Theme.color.textTertiary
        opacity: root.hasSubmenu ? 1 : 0
        anchors.right: parent.right
        anchors.rightMargin: root.padding
        anchors.verticalCenter: parent.verticalCenter
    }

    HoverHandler {
        onHoveredChanged: {
            if (hovered && !root.isSeparator && root.entry.enabled !== false)
                root.rowHovered(root.entry.index);
        }
    }

    TapHandler {
        onTapped: root.rowActivated(root.entry.index)
    }

    Accessible.role: Accessible.MenuItem
    Accessible.name: root.entry.label || ""
    // A disabled row's reason is the useful description; an enabled row keeps
    // its chord hint. Checked state is exposed separately below.
    Accessible.description: root.disabled && root.disabledReason.length > 0
                            ? root.disabledReason
                            : (root.entry.shortcut || "")
    Accessible.checkable: root.entry.checkable === true
    Accessible.checked: root.entry.checked === true
    Accessible.ignored: root.isSeparator
    Accessible.onPressAction: root.rowActivated(root.entry.index)
}
