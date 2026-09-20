// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A software-renderer-friendly drop shadow. Qt's shader effects (MultiEffect,
// RectangularShadow) are GPU-only and silently no-op on the headless software
// backend, which is exactly where the visual-regression suite runs, so the
// design system ships a layered approximation instead. T-13's compositor
// renderer owns real blurred shadows; this is the app-side equivalent.
Item {
    id: root

    property real radius: Theme.controls.window.radius
    property real blur: Theme.controls.window.shadowBlur
    property real shadowOpacity: Theme.material.shadowOpacity
    property color shadowColor: Theme.color.shadowColor
    property point offset: Qt.point(0, Theme.controls.shadow.offsetY)
    property int layers: Theme.controls.shadow.layers

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
