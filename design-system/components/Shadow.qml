// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A software-renderer-friendly drop shadow. Qt's shader effects (MultiEffect,
// RectangularShadow) are GPU-only and silently no-op on the headless software
// backend, which is exactly where the visual-regression suite runs, so the
// design system ships a layered approximation instead. T-13's compositor
// renderer owns real blurred shadows; this is the app-side equivalent.
//
// Geometry comes from the shared elevation tokens (`component.elevation.*`),
// the same source the compositor's shadow pass reads, so the two cannot
// drift (FR-2). Pick the surface's `level`; the individual properties remain
// overridable for one-off call sites.
Item {
    id: root

    // The design-system elevation level this shadow is drawn at. A floating
    // window is "high"; popovers/overlays are "overlay".
    property string level: "high"
    readonly property var elevation: Theme.controls.elevation[level]

    property real radius: Theme.controls.window.radius
    property real blur: elevation.blur
    property real shadowOpacity: Theme.material.shadowOpacity
    property color shadowColor: Theme.color.shadowColor
    property point offset: Qt.point(0, elevation.offsetY)
    property int layers: elevation.layers

    Repeater {
        model: root.layers
        delegate: Rectangle {
            required property int index
            readonly property real spread: (root.layers - index) / root.layers
            x: root.offset.x - root.blur * spread
            y: root.offset.y - root.blur * spread
            width: root.width + 2 * root.blur * spread
            height: root.height + 2 * root.blur * spread
            radius: root.radius + root.blur * spread
            color: root.shadowColor
            opacity: root.shadowOpacity / root.layers
            antialiasing: true
        }
    }
}
