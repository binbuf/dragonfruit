// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The System Settings category tile (T-19.1b): a rounded, gradient-backed
// container with an inner top highlight, a token drop shadow, and a centered
// near-white Phosphor glyph. The container is the differentiator, not the
// glyph — every category shares the same rounded geometry and elevation and
// only the gradient changes, so the gradient is never baked into an SVG.
//
// This is the *only* surface that gets the container. Everywhere else a
// migrated Phosphor mark is a bare `PhosphorIcon` sized for its place (ADR
// 0163). The category -> (glyph, gradient) table lives with the Settings pane
// catalog (`SettingsPanes.categoryStyle`, apps/settings), keyed by the pane's
// existing `icon` identity; callers pass the resolved values here and
// `gradientStart`/`gradientEnd` fall back to a token gradient when used bare.
//
// Software-renderer friendly by construction: `Rectangle.gradient` and the
// layered `Shadow` both draw under the headless software scene graph the
// gallery and QML tests use, and the glyph is a tintable `ShapePath` (T-19.1a).
Item {
    id: root

    // The Phosphor glyph name, drawn in the solid `fill` weight. Empty draws
    // the container with no glyph.
    property string source: ""
    // The vertical gradient, top to bottom. Literal category hues are named
    // constants in the one app-side mapping table; the bare defaults are tokens.
    property color gradientStart: Theme.primitive.color.violet500
    property color gradientEnd: Theme.primitive.color.violet700
    // The glyph tint. Near-white by default so one glyph colour reads on every
    // category hue.
    property color symbolColor: Theme.primitive.color.neutral50
    // The square edge length in logical pixels.
    property real size: Theme.controls.settingsCategory.size
    // Corner radius. Derived from `size` through the token ratio so the small
    // sidebar tile and the large hero tile keep the same optical roundness;
    // callers may override.
    property real radius: Math.round(root.size * Theme.controls.settingsCategory.radiusRatio)
    // Opt-in coloured halo behind the tile, tinted to `gradientStart`.
    property bool glow: false

    implicitWidth: size
    implicitHeight: size

    // The drop shadow, declared before the tile so it sits behind it. Elevation
    // geometry comes from the shared `Shadow`/elevation tokens.
    Shadow {
        anchors.fill: tile
        level: "low"
        radius: root.radius + Theme.primitive.spacing.xxs
    }

    // Optional category-hued glow. The same layered approximation as the
    // shadow, but wider and coloured from the tile's own gradient.
    Shadow {
        anchors.fill: tile
        visible: root.glow
        blur: Theme.primitive.elevation.high
        radius: root.radius + Theme.primitive.spacing.xs
        shadowColor: root.gradientStart
        shadowOpacity: Theme.controls.settingsCategory.glowOpacity
    }

    Rectangle {
        id: tile
        width: root.size
        height: root.size
        radius: root.radius
        antialiasing: true
        gradient: Gradient {
            GradientStop { position: 0.0; color: root.gradientStart }
            GradientStop { position: 1.0; color: root.gradientEnd }
        }

        // The top inner highlight: a soft white wash that fades out by mid-tile.
        // It carries the tile radius so its top corners follow the container
        // instead of poking past the rounded edge.
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: Math.round(parent.height * Theme.controls.settingsCategory.highlightRatio)
            radius: root.radius
            antialiasing: true
            gradient: Gradient {
                GradientStop {
                    position: 0.0
                    color: Qt.rgba(1, 1, 1,
                                   Theme.controls.settingsCategory.highlightOpacity)
                }
                GradientStop { position: 1.0; color: Qt.rgba(1, 1, 1, 0) }
            }
        }

        PhosphorIcon {
            anchors.centerIn: parent
            visible: root.source.length > 0
            name: root.source
            weight: "fill"
            size: Math.round(root.size * Theme.controls.settingsCategory.iconRatio)
            color: root.symbolColor
        }
    }

    // Presentational: the pane title/row label already carries the accessible
    // name, so the tile itself is not a separate accessibility node.
    Accessible.role: Accessible.Graphic
    Accessible.ignored: true
}