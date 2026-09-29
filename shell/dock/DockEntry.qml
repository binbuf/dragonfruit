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
    // The anchored tile of the magnified profile (T-14.7aa). macOS conveys the
    // hover state with the zoom itself: the tile keeps no static highlight
    // wash, and it lifts off the plate with a soft shadow.
    property bool zoomed: false
    // Keyboard navigation focus (T-10 section 20): draws the design-system
    // FocusRing around the artwork.
    property bool keyboardFocused: false
    // The keyboard reorder chord (T-14.7t), injected axis-aware by the Dock for
    // pinned entries only (empty for everything else). It is the accessible
    // description's only extra text, so a screen reader names the affordance.
    property string reorderHint: ""
    // The entry is the target of an external drag (T-10 section 12): draws a
    // drop highlight around the artwork.
    property bool externalDropTarget: false
    // Tap/drag arbitration (T-14.7g): a press that moves less than this many
    // device-independent pixels activates; a larger move lifts the entry into
    // a rearrangement. Both handlers share the value so a near-stationary
    // click — magnified, or entered through the transparent band — can never
    // fall between the two thresholds.
    readonly property real dragSlop: 8

    readonly property string kind: entry.kind !== undefined ? entry.kind : "app"
    readonly property bool isDivider: kind === "divider"
    readonly property bool isTrash: kind === "trash"
    // The Downloads stack (T-10 section 17): a folder entry with a count and a
    // new-items badge; clicking opens its popover rather than launching.
    readonly property bool isStack: kind === "stack"
    // The terminal overflow cell (T-14.7q): a synthetic entry carrying the
    // running groups that did not fit. It is never dragged, pinned, or grouped.
    readonly property bool isOverflow: kind === "overflow"
    readonly property int overflowCount:
        entry.hiddenCount !== undefined ? entry.hiddenCount : 0
    readonly property int stackCount: entry.stackCount !== undefined ? entry.stackCount : 0
    readonly property int badge: entry.badge !== undefined ? entry.badge : 0
    // The number of windows the app is running (T-14.7o). The pure projection
    // owns it (`windowCount`); the fallback derives it from `windowList` for
    // direct callers (the QML tests, a fixture) that only set the list.
    readonly property int windowCount: entry.windowCount !== undefined
        ? entry.windowCount
        : (entry.windowList !== undefined && entry.windowList !== null
           ? entry.windowList.length : 0)
    // A per-window minimized row is one window, never the app's whole group, so
    // it never carries the window-count badge even though it holds the app's
    // full list for its menu.
    readonly property bool isMinimized: kind === "minimized"
    // A placeholder gap opened by an application-alias external drop; it is
    // layout only and never interactive.
    readonly property bool isExternal: kind === "external"
    // A dropped folder's ghost/reflow gap (T-14.7k) draws the folder stack
    // silhouette, exactly like a pinned folder stack.
    readonly property bool isExternalFolder: isExternal && entry.externalFolder === true
    // The identity carried by an external drag (T-14.7f): the app-index
    // name/icon for an app alias, so the gap shows the real tile instead of a
    // generic square.
    readonly property bool externalHasIdentity:
        isExternal && (name.length > 0 || iconPath.length > 0)
    // A duplicate app-alias drop pulsed the already-pinned entry (T-14.7f):
    // a brief highlight makes the no-op visible.
    property bool duplicateFlash: false
    readonly property bool running: entry.running === true
    readonly property string name: entry.name !== undefined ? entry.name : ""
    readonly property string appId: entry.appId !== undefined ? entry.appId : ""
    // The app-index record id ("org.dragonfruit.Files.desktop"); the first-party
    // bundled-tile map (T-19.1d) is keyed by this.
    readonly property string desktopId: entry.desktopId !== undefined ? entry.desktopId : ""
    // Themed icon file resolved by app-index (T-14.1a); empty until the
    // service is running or when no theme provides the name.
    readonly property string iconPath: entry.iconPath !== undefined ? entry.iconPath : ""
    readonly property bool trashFull: entry.trashFull === true
    // The Trash backend is unreachable (T-10 section 16 lifecycle): dim the
    // entry and say so, but never block the session.
    readonly property bool trashUnavailable: isTrash && entry.available === false
    readonly property bool launching: entry.launch === "launching"
    readonly property bool failed: entry.launch === "failed"
    readonly property bool missing: entry.missing === true
    // The shell-driven hop phase (0..1), -1 when the entry is not bouncing
    // (T-10 section 8.1). Injected by the Dock from its `bouncePhases` map so
    // a bounce never rebuilds the entry model (T-14.7c); an entry-embedded
    // `bounce` still wins for direct callers.
    property real bouncePhase: -1
    property bool bounceAttention: false
    readonly property bool attention: entry.attention === true || bounceAttention
    readonly property real bounce:
        entry.bounce !== undefined ? entry.bounce : bouncePhase

    readonly property bool verticalIndicator:
        indicatorEdge === "left" || indicatorEdge === "right"
    readonly property real indicatorSize: Theme.controls.dock.indicatorSize
    // The rounded-square (squircle) tile radius for the artwork slot (T-14.7j).
    // Every state surface shares it: the placeholder, the drop/duplicate
    // highlights, the lift shadow, and the keyboard focus ring.
    readonly property real tileRadius:
        iconSize * Theme.controls.dock.icon.radiusRatio
    // The hover highlight is a slightly rounder wash behind the artwork.
    readonly property real hoverRadius:
        iconSize * Theme.controls.dock.hover.radiusRatio
    // Reserved for every entry (not just running ones) so the Dock keeps all
    // artwork on one baseline; the dot itself still only shows when running.
    readonly property real indicatorSpace:
        showIndicator ? Theme.controls.dock.indicatorGap + indicatorSize : 0

    signal activated(var entry)
    signal contextMenuRequested(var entry, real globalX, real globalY)
    // Drag rearrangement (T-10 section 12): scene coordinates so the Dock can
    // map them into its own axis space.
    signal dragBegan(var entry, real sceneX, real sceneY)
    signal dragMoved(var entry, real sceneX, real sceneY)
    signal dragEnded(var entry, real sceneX, real sceneY)
    // The pointer entered/left this entry's hover region (T-14.7i). The Dock
    // owns the dwell timer and the single Tooltip; this only reports the edge.
    signal hoverBegan(Item entryItem)
    signal hoverEnded(Item entryItem)
    // The divider resize handle (T-10 section 5): scene coordinates so the
    // Dock maps them into its axis space.
    signal dividerResizeBegan(real sceneX, real sceneY)
    signal dividerResizeMoved(real sceneX, real sceneY)
    signal dividerResizeEnded()

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
            return trashUnavailable ? qsTr(", unavailable") : "";
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

    // The hover name label shown by the design-system Tooltip (T-14.7i): the
    // entry name plus its state. It is the one label source ADR 0092 approved;
    // no text is ever drawn inside the artwork.
    readonly property string tooltipLabel: {
        if (isDivider || isExternal)
            return "";
        if (isTrash) {
            var trashName = name.length > 0 ? name : qsTr("Trash");
            return trashName + (trashUnavailable ? qsTr(" — unavailable")
                                : trashFull ? qsTr(" — full") : qsTr(" — empty"));
        }
        if (isStack) {
            var stackName = name.length > 0 ? name : qsTr("Downloads");
            var countLabel = stackCount === 1 ? qsTr("1 item")
                                              : qsTr("%1 items").arg(stackCount);
            return stackName + qsTr(" — %1").arg(countLabel);
        }
        if (isOverflow) {
            var groupsLabel = overflowCount === 1 ? qsTr("1 more window group")
                                                  : qsTr("%1 more window groups").arg(overflowCount);
            return groupsLabel;
        }
        var windows = entry.windowList !== undefined && entry.windowList !== null
                      ? entry.windowList.length : 0;
        if (windows > 1)
            return name + qsTr(" — %1 windows").arg(windows);
        return name;
    }

    // The state badge (failure/missing/Trash-unavailable) wins over every
    // other badge (T-14.7o precedence: status > app badge > window count).
    readonly property bool showStatusBadge:
        !isDivider && !isExternal && (failed || missing || trashUnavailable)
    // The window-count badge (T-14.7o): a grouped app (2+ windows) with no
    // status and no app-provided count shows how many windows it has. It never
    // shows on folders/stacks or Trash, and a status or app badge suppresses it
    // so there is exactly one badge per entry.
    readonly property bool showWindowBadge:
        !isDivider && !isExternal && !isTrash && !isStack && !isMinimized
        && !showStatusBadge && badge <= 0
        && (isOverflow ? overflowCount >= 1 : windowCount >= 2)
    // The badge geometry/typography is token-driven and scales with the tile so
    // it stays legible at the minimum and maximum icon sizes (T-14.7o).
    readonly property real windowBadgeSize: {
        var wb = Theme.controls.dock.windowBadge;
        return Math.max(wb.sizeMin,
                        Math.min(wb.sizeMax,
                                 Math.round(root.iconSize * wb.sizeRatio)));
    }
    readonly property string windowBadgeLabel: {
        var count = root.isOverflow ? root.overflowCount : root.windowCount;
        return count > 9 ? qsTr("9+") : String(count);
    }
    // An app demanding attention tints the badge; otherwise the accent marks a
    // grouped app (the reference behavior, our own tokens).
    readonly property color windowBadgeColor:
        attention ? Theme.color.danger : Theme.color.accent

    Accessible.role: isDivider ? Accessible.Separator : Accessible.ListItem
    Accessible.name: isDivider ? qsTr("Dock separator")
                     : isExternal ? (name.length > 0 ? name : qsTr("Drop here"))
                     : isOverflow ? qsTr("%1 more window groups").arg(overflowCount)
                     : isTrash ? qsTr("Trash") + stateLabel
                     : isStack ? (name.length > 0 ? name : qsTr("Downloads")) + stateLabel
                     : name + stateLabel
    Accessible.focusable: !isDivider && !isExternal
    // The reorder chord (T-14.7t) is exposed as the description so it joins the
    // entry's accessibility surface when it is movable (pinned); every other
    // entry keeps an empty description.
    Accessible.description: root.reorderHint

    // Divider between regions (T-14.7v). Every region boundary draws the same
    // hairline; only the app | right-region divider carries the drag resize
    // handle (T-10 section 5), so a second boundary never adds a second
    // handle. A bottom Dock's rule is vertical (across the plate); a
    // left/right Dock's is horizontal.
    readonly property bool resizeHandle: entry.resizeHandle !== false
    readonly property bool ruleIsVertical: !root.verticalIndicator
    Rectangle {
        objectName: "divider"
        visible: root.isDivider
        width: root.ruleIsVertical
               ? Theme.controls.dock.divider.width
               : root.width * Theme.controls.dock.divider.heightRatio
        height: root.ruleIsVertical
                ? root.height * Theme.controls.dock.divider.heightRatio
                : Theme.controls.dock.divider.width
        x: (root.width - width) / 2
        y: (root.height - height) / 2
        color: Theme.color.dockDivider
        opacity: Theme.controls.dock.divider.opacity
    }

    // The divider is only 1 px wide, so the drag handle is a wider invisible
    // hit target centred on it (T-10 section 5). The layout slot stays 1 px;
    // only the pointer target grows. Only the resize-handle divider mounts it.
    Item {
        objectName: "dividerHit"
        visible: root.isDivider && root.resizeHandle
        anchors.centerIn: parent
        width: root.ruleIsVertical ? Math.max(16, root.iconSize * 0.4) : root.width
        height: root.ruleIsVertical ? root.height : Math.max(16, root.iconSize * 0.4)

        DragHandler {
            id: dividerDragHandler
            objectName: "dividerDragHandler"
            acceptedButtons: Qt.LeftButton
            dragThreshold: 4
            onActiveChanged: {
                var p = centroid.scenePosition;
                if (active)
                    root.dividerResizeBegan(p.x, p.y);
                else
                    root.dividerResizeEnded();
            }
            onCentroidChanged: {
                if (!active)
                    return;
                var p = centroid.scenePosition;
                root.dividerResizeMoved(p.x, p.y);
            }
        }
    }

    // Hover highlight behind the artwork. It yields to the zoom: the magnified
    // anchored tile is the hover state, so it draws no wash (T-14.7aa).
    Rectangle {
        objectName: "hoverHighlight"
        visible: root.hovered && !root.dragging && !root.zoomed
                 && !root.isDivider && !root.isExternal
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: root.hoverRadius
        color: Theme.color.dockHoverFill
        opacity: Theme.controls.dock.hover.fillOpacity
    }

    // A placeholder gap opened by an application-alias external drop: a
    // translucent slot the dragged app will occupy (T-10 section 12). When
    // the identity is known (T-14.7f) it fades behind the real ghost glyph.
    Rectangle {
        objectName: "externalPlaceholder"
        visible: root.isExternal
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: root.tileRadius
        color: Theme.color.controlFill
        opacity: root.externalHasIdentity ? 0.18 : 0.35
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
        radius: root.tileRadius
        color: Theme.color.accent
        opacity: 0.25
    }

    // A duplicate app-alias drop pulses the already-pinned entry so the no-op
    // is visible (T-14.7f).
    Rectangle {
        objectName: "duplicateFlash"
        visible: root.duplicateFlash && !root.isDivider && !root.isExternal
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: root.tileRadius
        color: Theme.color.accent
        opacity: 0.35
    }

    // A lifted entry casts a shadow to read as picked up.
    Shadow {
        objectName: "liftShadow"
        visible: root.lifted
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: root.tileRadius
        blur: Theme.controls.popover.shadowBlur
        z: -1
    }

    // The zoomed (anchored) tile lifts off the plate with a soft, small
    // shadow — the reference hover state's only extra treatment beyond the
    // scale (T-14.7aa). It is confined to the one anchored tile.
    Shadow {
        objectName: "zoomShadow"
        visible: root.zoomed
        x: root.artworkX
        y: root.artworkY
        width: root.iconSize
        height: root.iconSize
        radius: root.tileRadius
        level: "low"
        blur: Theme.controls.dock.hover.shadowBlur
        shadowOpacity: Theme.controls.dock.hover.shadowOpacity
        z: -1
    }

    // Keyboard navigation focus ring (T-10 section 20), drawn around the
    // artwork only (the running indicator is not part of the target).
    FocusRing {
        objectName: "keyboardFocusRing"
        target: glyph
        cornerRadius: root.tileRadius
        shown: root.keyboardFocused && !root.isDivider && !root.isExternal
    }

    DockGlyph {
        id: glyph
        objectName: "glyph"
        visible: !root.isDivider && (!root.isExternal || root.externalHasIdentity)
        kind: root.isTrash ? "trash"
              : root.isOverflow ? "overflow"
              : (root.isStack || root.isExternalFolder) ? "stack" : "app"
        name: root.name
        appId: root.appId
        desktopId: root.desktopId
        iconPath: root.iconPath
        trashFull: root.trashFull
        size: root.iconSize
        x: root.artworkX
        y: root.artworkY
        scale: root.pressedScale * root.pulseScale * root.liftScale
        transformOrigin: Item.Center
        opacity: root.launching ? 0.6
                 : root.missing ? 0.45
                 : root.trashUnavailable ? 0.4 : 1.0

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

    // The dragged app's real name under the identity ghost (T-14.7f). It is
    // presentation only and never interactive (the gap is skipped by the
    // Dock's hit testing).
    Text {
        objectName: "externalGhostName"
        visible: root.isExternal && root.name.length > 0
        text: root.name
        color: Theme.color.textPrimary
        font.pixelSize: Math.max(9, Math.round(root.iconSize * 0.26))
        width: Math.max(root.iconSize, 160)
        x: root.artworkX + (root.iconSize - width) / 2
        y: root.artworkY - height - 2
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        z: 1
    }

    // A launch failure, an unresolved pinned identity, or an unreachable
    // Trash backend is a state, not a crash (T-10 sections 8.5, 4.2, 16): a
    // small badge marks it and the accessible name says why.
    Rectangle {
        objectName: "statusBadge"
        visible: root.showStatusBadge
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

    // The grouped-app window-count badge (T-14.7o): one numeral (capped at
    // `9+`) at the tile's top-right corner when the app has two or more
    // windows. It is presentational only; the accessible name and the tooltip
    // already carry the count, so it announces nothing on its own. It fades in
    // with `motion.focus` (instant under reduced motion) and is suppressed by a
    // status or app badge.
    Rectangle {
        objectName: "windowBadge"
        visible: opacity > 0
        opacity: root.showWindowBadge ? 1.0 : 0.0
        width: Math.max(root.windowBadgeSize,
                        windowBadgeText.implicitWidth
                        + 2 * Theme.controls.dock.windowBadge.paddingH)
        height: root.windowBadgeSize
        radius: height / 2
        color: root.windowBadgeColor
        border.width: Theme.controls.dock.windowBadge.borderWidth
        border.color: Theme.color.chrome
        x: root.artworkX + root.iconSize - width
           - Theme.controls.dock.windowBadge.inset
        y: root.artworkY + Theme.controls.dock.windowBadge.inset
        z: 1
        Text {
            id: windowBadgeText
            objectName: "windowBadgeText"
            anchors.centerIn: parent
            text: root.windowBadgeLabel
            color: Theme.color.accentContent
            font.pixelSize: Math.max(
                Theme.controls.dock.windowBadge.fontMin,
                Math.round(root.windowBadgeSize
                           * Theme.controls.dock.windowBadge.fontRatio))
            font.weight: Theme.primitive.font.weightSemibold
        }

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.motion.focus.duration
                easing.type: Easing.OutCubic
            }
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
        color: Theme.color.dockIndicator
        opacity: root.attention ? 1.0 : Theme.controls.dock.indicator.opacity
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
        onHoveredChanged: {
            if (hovered)
                root.hoverBegan(root);
            else
                root.hoverEnded(root);
        }
    }

    TapHandler {
        acceptedButtons: Qt.LeftButton
        // The generic tap handles app/temporary/minimized/trash entries. A
        // folder stack has its own handler below (single vs double click).
        enabled: !root.isExternal && !root.isStack
        // A click activates while the pointer stays within `dragSlop`; the
        // DragHandler below takes over at the same threshold, so a slop-drag
        // lifts instead (T-14.7g). Both handlers share the value so a
        // near-stationary click — magnified, or entered through the band —
        // cannot fall between them.
        gesturePolicy: TapHandler.DragThreshold
        dragThreshold: root.dragSlop
        onPressedChanged: root.pressed = pressed
        onTapped: root.activated(root.entry)
    }

    // A folder stack uses the same tap signal as every entry; the Dock resolves
    // a single click (open popover) vs a double click (open in Files) from the
    // timing, because the entry delegate can be recreated when the stack's
    // badge clears (T-14.7h). A user folder pin also lifts into a drag-out
    // removal; the built-in Downloads member does not.
    readonly property bool canRemoveStack: isStack && entry.canRemove === true
    TapHandler {
        objectName: "stackTapHandler"
        acceptedButtons: Qt.LeftButton
        enabled: root.isStack && !root.isExternal
        gesturePolicy: TapHandler.DragThreshold
        dragThreshold: root.dragSlop
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
    // gesture and the pointer's scene position. It shares `dragSlop` with the
    // left TapHandler so a stationary click always taps and only a real move
    // lifts (T-14.7g).
    property point dragScenePos: Qt.point(0, 0)

    DragHandler {
        id: dragHandler
        objectName: "dragHandler"
        acceptedButtons: Qt.LeftButton
        enabled: !root.isDivider && !root.isTrash && !root.isExternal
                 && !root.isOverflow
                 && root.kind !== "minimized"
                 && (!root.isStack || root.canRemoveStack)
        dragThreshold: root.dragSlop

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
