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
    // A lifted (dragged) entry scales up and casts a shadow (T-10 section 12).
    property bool lifted: false
    // Keyboard navigation focus (T-10 section 20): draws the design-system
    // FocusRing around the artwork.
    property bool keyboardFocused: false
    // The entry is the target of an external drag (T-10 section 12): draws a
    // drop highlight around the artwork.
    property bool externalDropTarget: false

    readonly property string kind: entry.kind !== undefined ? entry.kind : "app"
    readonly property bool isDivider: kind === "divider"
    readonly property bool isTrash: kind === "trash"
    // The Downloads stack (T-10 section 17): a folder entry with a count and a
    // new-items badge; clicking opens its popover rather than launching.
    readonly property bool isStack: kind === "stack"
    readonly property int stackCount: entry.stackCount !== undefined ? entry.stackCount : 0
    readonly property int badge: entry.badge !== undefined ? entry.badge : 0
    // A placeholder gap opened by an application-alias external drop; it is
    // layout only and never interactive.
    readonly property bool isExternal: kind === "external"
    readonly property bool running: entry.running === true
    readonly property bool attention: entry.attention === true
    readonly property string name: entry.name !== undefined ? entry.name : ""
    readonly property string appId: entry.appId !== undefined ? entry.appId : ""
    readonly property bool trashFull: entry.trashFull === true
    readonly property bool launching: entry.launch === "launching"
    readonly property bool failed: entry.launch === "failed"
    readonly property bool missing: entry.missing === true
    // The shell-driven hop phase (0..1), -1 when the entry is not bouncing
    // (T-10 section 8.1). The Dock owns the translation; this is only for
    // the reduced-motion pulse.
    readonly property real bounce: entry.bounce !== undefined ? entry.bounce : -1

    readonly property bool verticalIndicator:
        indicatorEdge === "left" || indicatorEdge === "right"
    readonly property real indicatorSize: Theme.controls.dock.indicatorSize
    readonly property real indicatorSpace:
        showIndicator && running ? Theme.controls.dock.indicatorGap + indicatorSize : 0

    signal activated(var entry)
    signal contextMenuRequested(var entry, real globalX, real globalY)
    // Drag rearrangement (T-10 section 12): scene coordinates so the Dock can
    // map them into its own axis space.
    signal dragBegan(var entry, real sceneX, real sceneY)
    signal dragMoved(var entry, real sceneX, real sceneY)
    signal dragEnded(var entry, real sceneX, real sceneY)

    implicitWidth: verticalIndicator ? iconSize + indicatorSpace : iconSize
    implicitHeight: verticalIndicator ? iconSize : iconSize + indicatorSpace

    // The artwork is inset from the indicator edge so the dot has room.
    readonly property real artworkX: indicatorEdge === "left" ? indicatorSpace : 0
    readonly property real artworkY: indicatorEdge === "bottom" ? 0 : 0

    // Scale only the artwork on press so the indicator stays put; the
    // design-system pressed state (motion.hover).
    readonly property real pressedScale: pressed && !dragging ? 0.9 : 1.0
    readonly property real liftScale: lifted ? 1.12 : 1.0
    // Under reduced motion the bounce translation is removed and the state
    // change stays legible as a subtle scale pulse (T-10 section 20).
    readonly property real pulseScale:
        Theme.reducedMotion && bounce >= 0
            ? 1 + 0.05 * Math.sin(Math.PI * bounce) : 1.0

    // The accessibility state, carrying launch and identity failures (T-10
    // section 20).
    readonly property string stateLabel: {
        if (isTrash)
            return "";
        if (isStack) {
            var items = stackCount === 1 ? qsTr(", 1 item") : qsTr(", %1 items").arg(stackCount);
            if (badge > 0)
                items += qsTr(", %1 new").arg(badge);
            return items;
        }
        var label = missing ? qsTr(", not found")
                  : launching ? qsTr(", launching")
                  : failed ? qsTr(", failed to launch")
                  : running ? qsTr(", running") : qsTr(", not running");
        if (attention && !missing && !launching)
            label += qsTr(", needs attention");
        return label;
    }

    Accessible.role: isDivider ? Accessible.Separator : Accessible.ListItem
    Accessible.name: isDivider ? qsTr("Dock separator")
                     : isExternal ? qsTr("Drop here")
                     : isTrash ? qsTr("Trash")
                     : isStack ? qsTr("Downloads") + stateLabel
                     : name + stateLabel
    Accessible.focusable: !isDivider && !isExternal

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
        visible: root.hovered && !root.dragging && !root.isDivider && !root.isExternal
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: Theme.controls.dock.radius
        color: Theme.color.controlFill
        opacity: 0.6
    }

    // A placeholder gap opened by an application-alias external drop: a
    // translucent slot the dragged app will occupy (T-10 section 12).
    Rectangle {
        objectName: "externalPlaceholder"
        visible: root.isExternal
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: Theme.controls.dock.radius
        color: Theme.color.controlFill
        opacity: 0.35
        border.width: 1
        border.color: Theme.color.border
    }

    // The entry under an external drag is highlighted as the drop target
    // (T-10 section 12).
    Rectangle {
        objectName: "externalDropHighlight"
        visible: root.externalDropTarget && !root.isDivider && !root.isExternal
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: Theme.controls.dock.radius
        color: Theme.color.accent
        opacity: 0.25
    }

    // A lifted entry casts a shadow to read as picked up.
    Shadow {
        objectName: "liftShadow"
        visible: root.lifted
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: Theme.controls.dock.radius
        blur: Theme.controls.popover.shadowBlur
        z: -1
    }

    // Keyboard navigation focus ring (T-10 section 20), drawn around the
    // artwork only (the running indicator is not part of the target).
    FocusRing {
        objectName: "keyboardFocusRing"
        target: glyph
        cornerRadius: Theme.controls.dock.radius
        shown: root.keyboardFocused && !root.isDivider && !root.isExternal
    }

    DockGlyph {
        id: glyph
        objectName: "glyph"
        visible: !root.isDivider && !root.isExternal
        kind: root.isTrash ? "trash" : root.isStack ? "stack" : "app"
        name: root.name
        appId: root.appId
        trashFull: root.trashFull
        size: root.iconSize
        x: root.artworkX
        y: root.artworkY
        scale: root.pressedScale * root.pulseScale * root.liftScale
        transformOrigin: Item.Center
        opacity: root.launching ? 0.6 : root.missing ? 0.45 : 1.0

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

    // A launch failure or an unresolved pinned identity is a state, not a
    // crash (T-10 sections 8.5, 4.2): a small badge marks it and the
    // accessible name says why.
    Rectangle {
        objectName: "statusBadge"
        visible: !root.isDivider && !root.isExternal && (root.failed || root.missing)
        width: root.indicatorSize
        height: root.indicatorSize
        radius: root.indicatorSize / 2
        color: root.failed ? Theme.color.danger : Theme.color.warning
        border.width: 1
        border.color: Theme.color.chrome
        x: root.artworkX + root.iconSize - width
        y: root.artworkY
    }

    // The Downloads stack's new-items badge (T-10 section 17). It shows the
    // number added since the stack was last opened; the accessible name also
    // carries the count and the new count.
    Rectangle {
        objectName: "stackBadge"
        visible: root.isStack && root.badge > 0
        width: Math.max(root.indicatorSize, badgeText.implicitWidth + 6)
        height: root.indicatorSize
        radius: height / 2
        color: Theme.color.accent
        border.width: 1
        border.color: Theme.color.chrome
        x: root.artworkX + root.iconSize - width
        y: root.artworkY
        Text {
            id: badgeText
            anchors.centerIn: parent
            text: root.badge > 9 ? qsTr("9+") : String(root.badge)
            color: Theme.color.accentContent
            font.pixelSize: Math.max(9, Math.round(root.indicatorSize * 0.7))
        }
    }

    // Running indicator on the dock-edge side (one per app entry, never per
    // window). Hidden when `showIndicators` is off.
    Rectangle {
        objectName: "indicator"
        visible: root.showIndicator && root.running && !root.isExternal
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
        enabled: !root.isExternal
        onPressedChanged: root.pressed = pressed
        onTapped: root.activated(root.entry)
    }

    TapHandler {
        acceptedButtons: Qt.RightButton
        enabled: !root.isExternal
        onTapped: (eventPoint) => {
            var global = root.mapToItem(null, eventPoint.position.x,
                                        eventPoint.position.y);
            root.contextMenuRequested(root.entry, global.x, global.y);
        }
    }

    // Press-and-hold then move lifts the entry into a rearrangement (T-10
    // section 12). The Dock owns the reorder model; this only reports the
    // gesture and the pointer's scene position.
    property point dragScenePos: Qt.point(0, 0)

    DragHandler {
        id: dragHandler
        objectName: "dragHandler"
        acceptedButtons: Qt.LeftButton
        enabled: !root.isDivider && !root.isTrash && !root.isStack && !root.isExternal
                 && root.kind !== "minimized"
        dragThreshold: 8

        onActiveChanged: {
            var p = centroid.scenePosition;
            if (active) {
                root.dragScenePos = p;
                root.dragBegan(root.entry, p.x, p.y);
            } else {
                root.dragEnded(root.entry, root.dragScenePos.x, root.dragScenePos.y);
            }
        }
        onCentroidChanged: {
            var p = centroid.scenePosition;
            root.dragScenePos = p;
            if (active)
                root.dragMoved(root.entry, p.x, p.y);
        }
    }
}
