// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit

// A single-fill sheet with independent corner radii (T206). First-party app
// chrome needs one or two rounded window corners without the other pair:
// the detail sheet is square at the top, the sidebar is square except for its
// bottom-left window corner, and the path bar is square except for its
// bottom-right corner. A normal Rectangle can only round all four, and
// overlaying a second translucent rectangle would stack alpha in the corners.
// One ShapePath keeps the fill a single composite, so the measured near-opaque
// app alpha is applied exactly once.
Shape {
    id: root

    property color color: Theme.color.surface
    property real topLeftRadius: 0
    property real topRightRadius: 0
    property real bottomLeftRadius: 0
    property real bottomRightRadius: 0

    antialiasing: true
    layer.enabled: false

    ShapePath {
        fillColor: root.color
        strokeWidth: 0

        startX: root.topLeftRadius
        startY: 0

        PathLine {
            x: root.width - root.topRightRadius
            y: 0
        }
        PathArc {
            x: root.width
            y: root.topRightRadius
            radiusX: root.topRightRadius
            radiusY: root.topRightRadius
            direction: PathArc.Clockwise
        }
        PathLine {
            x: root.width
            y: root.height - root.bottomRightRadius
        }
        PathArc {
            x: root.width - root.bottomRightRadius
            y: root.height
            radiusX: root.bottomRightRadius
            radiusY: root.bottomRightRadius
            direction: PathArc.Clockwise
        }
        PathLine {
            x: root.bottomLeftRadius
            y: root.height
        }
        PathArc {
            x: 0
            y: root.height - root.bottomLeftRadius
            radiusX: root.bottomLeftRadius
            radiusY: root.bottomLeftRadius
            direction: PathArc.Clockwise
        }
        PathLine {
            x: 0
            y: root.topLeftRadius
        }
        PathArc {
            x: root.topLeftRadius
            y: 0
            radiusX: root.topLeftRadius
            radiusY: root.topLeftRadius
            direction: PathArc.Clockwise
        }
    }
}
