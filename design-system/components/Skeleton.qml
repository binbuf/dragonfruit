// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A loading placeholder (T-18.2): a grey rounded rectangle with a slow,
// left-to-right animated highlight (a "shimmer"). The Wallpaper pane renders a
// row of these while the content provider's first catalogue is still
// downloading, so an empty Featured row reads as "loading", never as broken.
//
// Reduced motion is a first-class variant: under `Theme.reducedMotion` the
// highlight stops and rests centered, so the placeholder is a static grey
// rectangle (no animation is started at all — `shimmering` is false).
Rectangle {
    id: root

    // The owner sets this false to hold the placeholder static even when the
    // scheme is not in reduced motion (e.g. a filled row that still reserves
    // space). `shimmering` is the resolved state the tests assert.
    property bool active: true
    // The accessible name announced for this placeholder; empty means purely
    // presentational (ignored by AT-SPI). The Wallpaper pane passes one shared
    // name across the row ("Downloading wallpapers").
    property string accessibleName: ""

    readonly property bool shimmering: root.active && !Theme.reducedMotion
    // 0 at the leading edge, 1 at the trailing edge. Driven by the animation
    // below; exposed so a test can assert the highlight moves (or does not).
    property real progress: 0.0
    // The highlight band's live x, for introspection and reduced-motion checks.
    readonly property real highlightX: band.x

    implicitWidth: Theme.controls.skeleton.width
    implicitHeight: Theme.controls.skeleton.height
    radius: Theme.controls.skeleton.radius
    color: Theme.color.skeletonBase
    clip: true
    antialiasing: true

    Accessible.role: Accessible.Graphic
    Accessible.name: root.accessibleName
    Accessible.ignored: root.accessibleName.length === 0

    // The moving highlight. It spans `highlightRatio` of the width and travels
    // from fully off the leading edge to fully off the trailing edge, wrapped
    // by the infinite loop. Under reduced motion the animation is not running
    // and the band rests centered.
    Rectangle {
        id: band
        width: Math.max(1, root.width * Theme.controls.skeleton.highlightRatio)
        height: root.height
        x: root.shimmering
           ? -band.width + root.progress * (root.width + band.width)
           : (root.width - band.width) / 2

        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: "transparent" }
            GradientStop { position: 0.4; color: Theme.color.skeletonHighlight }
            GradientStop { position: 0.6; color: Theme.color.skeletonHighlight }
            GradientStop { position: 1.0; color: "transparent" }
        }
    }

    // The animation drives `root.progress`; `band.x` follows it. Keeping the
    // animation on the root (not the band) is what lets the reduced-motion
    // variant leave `progress` at zero while still resting the band centered.
    NumberAnimation on progress {
        running: root.shimmering
        loops: Animation.Infinite
        from: 0.0
        to: 1.0
        duration: Theme.motion.skeleton.duration
        easing.type: Easing.InOutSine
    }
}