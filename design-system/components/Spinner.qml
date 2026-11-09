// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit

// A small circular progress indicator (ADR 0240). The Wallpaper pane uses it
// for a tile that is downloading/applying and for the catalogue's refreshing
// state. It is intentionally a supporting primitive: callers own the label and
// the surrounding scrim.
//
// `progress` is -1 for an indeterminate spinner; 0..1 draws a determinate arc
// and stops the rotation. Under `Theme.reducedMotion` the arc is static either
// way, so the state stays legible without motion.
Item {
    id: root

    property real size: Theme.controls.spinner.size
    property real strokeWidth: Theme.controls.spinner.strokeWidth
    property int speed: Theme.controls.spinner.speed
    property bool running: true
    // -1 = indeterminate; 0..1 = determinate progress.
    property real progress: -1
    property string accessibleName: ""

    readonly property bool spinning:
        root.running && root.progress < 0 && !Theme.reducedMotion
    readonly property real sweepAngle:
        root.progress >= 0
        ? Math.max(1, Math.min(360, root.progress * 360))
        : 270

    implicitWidth: root.size
    implicitHeight: root.size

    Accessible.role: Accessible.ProgressBar
    Accessible.name: root.accessibleName
    Accessible.ignored: root.accessibleName.length === 0

    Rectangle {
        anchors.fill: parent
        radius: width / 2
        color: "transparent"
        border.width: root.strokeWidth
        border.color: Theme.color.border
        opacity: Theme.controls.spinner.trackOpacity
        antialiasing: true
    }

    Shape {
        anchors.fill: parent

        ShapePath {
            strokeColor: Theme.color.accent
            strokeWidth: root.strokeWidth
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.size / 2
                centerY: root.size / 2
                radiusX: root.size / 2 - root.strokeWidth / 2
                radiusY: root.size / 2 - root.strokeWidth / 2
                startAngle: -90
                sweepAngle: root.sweepAngle
            }
        }
    }

    RotationAnimator on rotation {
        from: 0
        to: 360
        duration: root.speed
        loops: Animation.Infinite
        // `spinning` already folds in reduced motion; repeat the guard here so
        // the animation sweep can see it on the animation block itself.
        running: root.spinning && !Theme.reducedMotion
    }
}
