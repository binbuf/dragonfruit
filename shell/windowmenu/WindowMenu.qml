// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The SSD titlebar's window menu, rendered by the trusted shell (T329).
//
// The compositor owns the menu model, geometry, hit-testing, keyboard
// navigation, Escape, click-away, and the Move to Space submenu; this scene
// is a pure presentation of the `df_toplevel_manager.window_menu` snapshot
// (ADR 0336). Rows use the same design-system `MenuRow` anatomy and
// `component.contextMenu` tokens as every other menu, so labels, icons,
// highlight bars, and the submenu card cannot drift from `ContextMenu`.
//
// Geometry arrives in the surface's logical coordinates: `panelX/Y/W/H` is
// the main card and `submenuX/Y/W/H` the child card. The shell popup surface
// is the union of both, so a submenu that flips left is never clipped.
Item {
    id: root

    // `[{ index, label, icon }, ...]` for Move to Space / Minimize / Zoom /
    // Close, and the window output's Spaces for the submenu.
    property var rows: []
    property var spaces: []
    property int highlightedIndex: -1
    property int submenuHighlightedIndex: -1
    property bool submenuOpen: false
    property real panelX: 0
    property real panelY: 0
    property real panelWidth: 180
    property real panelHeight: 0
    property real submenuX: 0
    property real submenuY: 0
    property real submenuWidth: 180
    property real submenuHeight: 0
    // 0 = light, 1 = dark. The live Theme already follows the compositor's
    // scheme; the value is exposed for the protocol/tests and for a caller
    // that needs to pin a scene.
    property int scheme: 1

    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real radius: Material.menuRadius("menuDetached")
    readonly property color surfaceFill:
        Material.menuFill("menuDetached", true, 0, 0, true)
    readonly property color borderColor:
        Glass.increaseContrast ? Theme.color.textPrimary : Theme.color.border

    // The compositor's material regions for this one surface: the main card
    // and, while open, the detached child card. The shell controller sends
    // these before the image commit so the frosted backdrop follows the exact
    // painted cards instead of their union (T197/T329).
    readonly property var regions: {
        var out = [{
            key: "panel",
            preset: "menuDetached",
            x: root.panelX,
            y: root.panelY,
            w: root.panelWidth,
            h: root.panelHeight,
            tl: root.radius,
            tr: root.radius,
            br: root.radius,
            bl: root.radius,
            overscan: 20
        }];
        if (root.submenuOpen && root.submenuWidth > 0 && root.submenuHeight > 0) {
            out.push({
                key: "submenu",
                preset: "menuDetached",
                x: root.submenuX,
                y: root.submenuY,
                w: root.submenuWidth,
                h: root.submenuHeight,
                tl: root.radius,
                tr: root.radius,
                br: root.radius,
                bl: root.radius,
                overscan: 20
            });
        }
        return out;
    }

    function entryAt(index, list) {
        var row = list[index];
        return {
            index: index,
            key: "window-menu-" + index,
            label: row ? (row.label || "") : "",
            icon: row ? (row.icon || "") : "",
            type: index === 0 && list === root.rows ? "submenu" : "item",
            enabled: true,
            checked: false,
            shortcut: ""
        };
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: qsTr("Window menu")
    Accessible.description: qsTr("Move to Space, Minimize, Zoom, and Close")

    // The main card. The shadow is drawn by the compositor's material pass;
    // the scene owns the readable fill and border.
    Rectangle {
        id: panel
        objectName: "windowMenuPanel"
        x: root.panelX
        y: root.panelY
        width: root.panelWidth
        height: root.panelHeight
        radius: root.radius
        color: root.surfaceFill
        border.width: Theme.controls.window.borderWidth
        border.color: root.borderColor
        antialiasing: true
    }

    Column {
        id: rowColumn
        x: panel.x + root.padding
        y: panel.y + root.padding
        width: Math.max(0, panel.width - 2 * root.padding)
        spacing: 0

        Repeater {
            id: rowRepeater
            model: root.rows
            delegate: MenuRow {
                required property var modelData
                required property int index
                objectName: "windowMenuRow" + index
                entry: root.entryAt(index, root.rows)
                highlighted: root.highlightedIndex === index
                rowHeight: root.rowHeight
                padding: root.padding
                constrainedWidth: rowColumn.width
            }
        }
    }

    // The Move to Space child card, placed exactly where the compositor
    // flipped/clamped it. It is a separate card with its own radius.
    Rectangle {
        id: submenuPanel
        objectName: "windowMenuSubmenu"
        visible: root.submenuOpen && root.submenuWidth > 0 && root.submenuHeight > 0
        x: root.submenuX
        y: root.submenuY
        width: root.submenuWidth
        height: root.submenuHeight
        radius: root.radius
        color: root.surfaceFill
        border.width: Theme.controls.window.borderWidth
        border.color: root.borderColor
        antialiasing: true
    }

    Column {
        id: submenuColumn
        visible: submenuPanel.visible
        x: submenuPanel.x + root.padding
        y: submenuPanel.y + root.padding
        width: Math.max(0, submenuPanel.width - 2 * root.padding)
        spacing: 0

        Repeater {
            id: spaceRepeater
            model: root.spaces
            delegate: MenuRow {
                required property var modelData
                required property int index
                objectName: "windowMenuSpace" + index
                entry: root.entryAt(index, root.spaces)
                highlighted: root.submenuHighlightedIndex === index
                rowHeight: root.rowHeight
                padding: root.padding
                constrainedWidth: submenuColumn.width
            }
        }
    }
}
