// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit

// A passive hover label (T-14.7i, ADR 0093): a pointer-anchored capsule with a
// single elided line of text, first used by the Dock's name labels. It is
// deliberately unlike Popup/Popover: it never takes active focus, never accepts
// keys, and blocks no pointer events to the control beneath it. The owner owns
// the dwell timer (the `dwell` token is exposed here so every consumer shares
// one delay) and commits the capsule through its own overlay path.
Item {
    id: root

    // When true the capsule is shown; the owner drives this from its dwell
    // timer. `anchorItem` is the control the label describes: the capsule
    // follows its geometry frame by frame, so a Dock label tracks a magnified
    // entry instead of freezing at the hover origin.
    property bool open: false
    property Item anchorItem: null
    property string text: ""
    // Where the capsule sits relative to the anchor: "above" | "below" |
    // "left" | "right". A bottom Dock uses "above"; a left/right Dock uses the
    // interior side so the label never crosses the screen edge.
    property string placement: "above"
    // The recommended hover delay before opening. The owner owns the timer;
    // exposing the token here (overridable) keeps every consumer on one value.
    property int dwell: Theme.controls.tooltip.dwell
    // Optional surface to clamp inside (usually the Dock). When null the
    // capsule is positioned freely and the owner must guarantee headroom.
    property Item bounds: null
    // Whether the capsule carries a pointer tail aimed at the anchor. The Dock
    // name labels opt in; the tail is the reference hover label's pointer
    // (T-14.7aa). The tail's perpendicular extension is part of the label's
    // footprint: the capsule clears the anchor by `tailHeight + offset`, so
    // the tip always lands `offset` from the anchor.
    property bool tailVisible: false

    readonly property int horizontalPadding: Theme.controls.tooltip.paddingH
    readonly property int verticalPadding: Theme.controls.tooltip.paddingV
    readonly property int offset: Theme.controls.tooltip.offset
    readonly property int tailWidth: Theme.controls.tooltip.tailWidth
    readonly property int tailHeight: Theme.controls.tooltip.tailHeight
    // The pill radius: half the capsule height, however the token rounds it.
    readonly property real capsuleRadius:
        Math.min(height / 2, Theme.controls.tooltip.radius)
    readonly property real tailGap: tailVisible ? tailHeight : 0
    // True when the single line did not fit and is elided (test/introspection).
    readonly property bool textElided: label.truncated

    signal opened()
    signal closed()

    // Presentational only: no focus, no keys, no pointer blocking.
    focus: false
    activeFocusOnTab: false
    enabled: false
    Accessible.ignored: true

    visible: opacity > 0
    z: 2500

    implicitWidth: Math.min(Theme.controls.tooltip.maxWidth,
                            label.implicitWidth + 2 * horizontalPadding)
    implicitHeight: label.implicitHeight + 2 * verticalPadding
    width: implicitWidth
    height: implicitHeight

    // The anchor must share the Tooltip's parent (both the Dock's entry delegate
// and the gallery's demo anchor do). Reading the geometry directly — instead
// of mapping through a function — is what makes the capsule follow a magnified
// entry: `mapToItem` alone does not track the anchor's live geometry.
    readonly property real anchorX: anchorItem ? anchorItem.x : 0
    readonly property real anchorY: anchorItem ? anchorItem.y : 0
    readonly property real anchorWidth: anchorItem ? anchorItem.width : 0
    readonly property real anchorHeight: anchorItem ? anchorItem.height : 0

    readonly property real preferredX:
        placement === "left" ? anchorX - width - offset - tailGap
        : placement === "right" ? anchorX + anchorWidth + offset + tailGap
        : anchorX + anchorWidth / 2 - width / 2
    readonly property real preferredY:
        placement === "above" ? anchorY - height - offset - tailGap
        : placement === "below" ? anchorY + anchorHeight + offset + tailGap
        : anchorY + anchorHeight / 2 - height / 2

    // Clamp inside `bounds` so a label at either end of the Dock is never
    // clipped; "above" intentionally reads into the reserved headroom (a
    // negative y is inside the Dock's pre-sized buffer), so only the inward
    // edge is clamped there.
    x: {
        var px = preferredX;
        if (bounds && bounds.width > 0)
            px = Math.max(0, Math.min(bounds.width - width, px));
        return px;
    }
    y: {
        var py = preferredY;
        if (bounds && bounds.height > 0 && placement !== "above")
            py = Math.max(0, Math.min(bounds.height - height, py));
        return py;
    }

    scale: open ? 1.0 : 0.96
    transformOrigin: Item.Center
    opacity: open ? 1.0 : 0.0

    onOpenChanged: {
        if (open)
            root.opened();
        else
            root.closed();
    }

    Behavior on opacity {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration
                                : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve
                                          : Theme.motion.popupClose.curve
        }
    }
    Behavior on scale {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration
                                : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve
                                          : Theme.motion.popupClose.curve
        }
    }

    // The capsule fill (a pill, like the reference hover label).
    Rectangle {
        objectName: "tooltipCapsule"
        anchors.fill: parent
        radius: root.capsuleRadius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    // A one-pixel inner rim: the reference capsule's top edge catches the
    // light, so the pill reads as translucent material rather than a flat
    // card. In the light scheme it is invisible over the white fill, exactly
    // like a bright edge on white paper.
    Rectangle {
        objectName: "tooltipRim"
        anchors.fill: parent
        anchors.margins: 1
        radius: Math.max(0, root.capsuleRadius - 1)
        color: "transparent"
        border.width: 1
        border.color: Theme.controls.tooltip.rimColor
        opacity: Theme.controls.tooltip.rimOpacity
        antialiasing: true
    }

    // The tail: a filled triangle aimed at the anchor, overlapping the
    // capsule's border by one pixel so the fill merges and the border does not
    // run across its base. The centre is clamped past the pill's end arcs, so
    // the base always sits on the capsule's flat edge.
    readonly property real tailAlong:
        Math.max(root.capsuleRadius + root.tailWidth / 2,
                 Math.min(root.width - root.capsuleRadius - root.tailWidth / 2,
                          root.anchorX + root.anchorWidth / 2 - root.x))
    readonly property real tailCross:
        Math.max(root.capsuleRadius + root.tailWidth / 2,
                 Math.min(root.height - root.capsuleRadius - root.tailWidth / 2,
                          root.anchorY + root.anchorHeight / 2 - root.y))
    Shape {
        objectName: "tooltipTail"
        visible: root.tailVisible
        preferredRendererType: Shape.GeometryRenderer
        antialiasing: true
        x: root.placement === "left" ? root.width - 1
         : root.placement === "right" ? 1 - root.tailHeight
         : root.tailAlong - root.tailWidth / 2
        y: root.placement === "above" ? root.height - 1
         : root.placement === "below" ? 1 - root.tailHeight
         : root.tailCross - root.tailWidth / 2
        width: root.placement === "above" || root.placement === "below"
               ? root.tailWidth : root.tailHeight
        height: root.placement === "above" || root.placement === "below"
                ? root.tailHeight : root.tailWidth
        ShapePath {
            objectName: "tooltipTailPath"
            strokeColor: "transparent"
            fillColor: Theme.color.surfaceElevated
            PathSvg {
                path: root.placement === "above"
                      ? "M 0 0 L " + root.tailWidth + " 0 L "
                        + (root.tailWidth / 2) + " " + root.tailHeight + " Z"
                      : root.placement === "below"
                      ? "M 0 " + root.tailHeight + " L " + root.tailWidth + " "
                        + root.tailHeight + " L " + (root.tailWidth / 2) + " 0 Z"
                      : root.placement === "left"
                      ? "M 0 0 L 0 " + root.tailWidth + " L " + root.tailHeight
                        + " " + (root.tailWidth / 2) + " Z"
                      : "M " + root.tailHeight + " 0 L " + root.tailHeight + " "
                        + root.tailWidth + " L 0 " + (root.tailWidth / 2) + " Z"
            }
        }
    }

    Text {
        id: label
        objectName: "tooltipLabel"
        anchors.centerIn: parent
        width: root.width - 2 * root.horizontalPadding
        text: root.text
        color: Theme.color.textPrimary
        font.pixelSize: Theme.controls.tooltip.fontSize
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        maximumLineCount: 1
    }
}