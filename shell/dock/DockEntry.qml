// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One Dock entry (T-10): the artwork, its running indicator, and the
// interaction (hover, press, left-click activation, right-click menu).
//
// The Dock owns layout and magnification; it sizes this item per frame by
// setting `iconSize` and the x/y position, so the entry itself is a pure
// presentation unit and is fully testable headless.
Item {
    id: root

    // --- Injected data --------------------------------------------------
    property var entry: ({})
    property real iconSize: Theme.controls.dock.iconSize
    // Which edge the running indicator sits on: "bottom" | "left" | "right".
    property string indicatorEdge: "bottom"
    property bool showIndicator: true

    // --- Interaction state ----------------------------------------------
    readonly property bool hovered: hoverHandler.hovered
    property bool pressed: false
    property bool dragging: false

    readonly property string kind: entry.kind !== undefined ? entry.kind : "app"
    readonly property bool isDivider: kind === "divider"
    readonly property bool isTrash: kind === "trash"
    readonly property bool running: entry.running === true
    readonly property bool attention: entry.attention === true
    readonly property string name: entry.name !== undefined ? entry.name : ""
    readonly property string appId: entry.appId !== undefined ? entry.appId : ""
    readonly property bool trashFull: entry.trashFull === true
    readonly property bool launching: entry.launch === "launching"

    readonly property bool verticalIndicator:
        indicatorEdge === "left" || indicatorEdge === "right"
    readonly property real indicatorSize: Theme.controls.dock.indicatorSize
    readonly property real indicatorSpace:
        showIndicator && running ? Theme.controls.dock.indicatorGap + indicatorSize : 0

    signal activated(var entry)
    signal contextMenuRequested(var entry, real globalX, real globalY)

    implicitWidth: verticalIndicator ? iconSize + indicatorSpace : iconSize
    implicitHeight: verticalIndicator ? iconSize : iconSize + indicatorSpace

    // The artwork is inset from the indicator edge so the dot has room.
    readonly property real artworkX: indicatorEdge === "left" ? indicatorSpace : 0
    readonly property real artworkY: indicatorEdge === "bottom" ? 0 : 0

    // Scale only the artwork on press so the indicator stays put; the
    // design-system pressed state (motion.hover).
    readonly property real pressedScale: pressed && !dragging ? 0.9 : 1.0

    Accessible.role: isDivider ? Accessible.Separator : Accessible.ListItem
    Accessible.name: isDivider ? qsTr("Dock separator")
                     : isTrash ? qsTr("Trash")
                     : name + (running ? qsTr(", running") : qsTr(", not running"))
    Accessible.focusable: !isDivider

    // Divider between the app and minimized/Trash regions. It is the drag
    // handle and the Control-click target for the Dock options menu.
    Rectangle {
        objectName: "divider"
        visible: root.isDivider
        x: (root.width - width) / 2
        y: root.height * 0.15
        width: 1
        height: root.height * 0.7
        color: Theme.color.separator
    }

    // Hover highlight behind the artwork.
    Rectangle {
        objectName: "hoverHighlight"
        visible: root.hovered && !root.dragging && !root.isDivider
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: Theme.controls.dock.radius
        color: Theme.color.controlFill
        opacity: 0.6
    }

    DockGlyph {
        id: glyph
        objectName: "glyph"
        visible: !root.isDivider
        kind: root.isTrash ? "trash" : "app"
        name: root.name
        appId: root.appId
        trashFull: root.trashFull
        size: root.iconSize
        x: root.artworkX
        y: root.artworkY
        scale: root.pressedScale
        transformOrigin: Item.Center
        opacity: root.launching ? 0.6 : 1.0

        Behavior on scale {
            NumberAnimation {
                duration: Theme.motion.hover.duration
                easing.type: Easing.OutCubic
            }
        }
        Behavior on opacity {
            NumberAnimation { duration: Theme.motion.focus.duration }
        }
    }

    // Running indicator on the dock-edge side (one per app entry, never per
    // window). Hidden when `showIndicators` is off.
    Rectangle {
        objectName: "indicator"
        visible: root.showIndicator && root.running
        width: root.indicatorSize
        height: root.indicatorSize
        radius: root.indicatorSize / 2
        color: Theme.color.textPrimary
        opacity: root.attention ? 1.0 : 0.85
        x: root.indicatorEdge === "left" ? 0
           : root.indicatorEdge === "right" ? root.width - width
           : (root.width - width) / 2
        y: root.indicatorEdge === "bottom" ? root.height - height
           : (root.height - height) / 2

        Behavior on opacity {
            NumberAnimation { duration: Theme.motion.focus.duration }
        }
    }

    HoverHandler {
        id: hoverHandler
    }

    TapHandler {
        acceptedButtons: Qt.LeftButton
        onPressedChanged: root.pressed = pressed
        onTapped: root.activated(root.entry)
    }

    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: (eventPoint) => {
            var global = root.mapToItem(null, eventPoint.position.x,
                                        eventPoint.position.y);
            root.contextMenuRequested(root.entry, global.x, global.y);
        }
    }
}
