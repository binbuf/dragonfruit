// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes

// The alternate OS-menu mark: the penguin silhouette from
// `assets/icons/brand/tux.svg`. The geometry comes from the generated
// `BrandGlyphs` registry (`make check-brand` keeps it in sync with the SVG),
// so editing the source SVG and re-running the generator is the only way to
// change the mark.
//
// The menu bar is a monochrome, tintable surface, so every source path is
// painted in `color` (the source's internal detail paths flatten into the same
// foreground). The source's view box is preserved; the penguin is narrower
// than the square menu-bar slot and is centred inside it.
//
// Like `DragonfruitLogo`, the optional wallpaper-contrast shadow paints the
// same silhouette translated by `shadowOffset` behind the mark. MenuBarMenu
// supplies the opposite-tone pair, and the bar's hover/open wash sits behind
// both marks.
Item {
    id: root

    property color color: Theme.color.textPrimary
    property real size: Theme.controls.menuBar.iconSize
    // Optional wallpaper contrast shadow, the same contract as
    // `DragonfruitLogo` and `StatusGlyph`.
    property color shadowColor: "transparent"
    property point shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)

    readonly property real viewWidth: BrandGlyphs.tuxViewBoxWidth
    readonly property real viewHeight: BrandGlyphs.tuxViewBoxHeight
    readonly property real artWidth: root.size * root.viewWidth / root.viewHeight
    readonly property real artScale: root.size / root.viewHeight

    // Preserve the square menu-bar slot; the penguin is narrower than tall.
    implicitWidth: size
    implicitHeight: size

    // One Shape per source path so overlapping detail paths stay solid instead
    // of relying on fill-rule winding. The shadow is the same geometry.
    Item {
        x: (root.size - root.artWidth) / 2 + root.shadowOffset.x
        y: root.shadowOffset.y
        width: root.artWidth
        height: root.size
        visible: root.shadowColor.a > 0

        Repeater {
            model: BrandGlyphs.tuxPaths

            delegate: Shape {
                required property var modelData

                width: root.viewWidth
                height: root.viewHeight
                scale: root.artScale
                transformOrigin: Item.TopLeft
                antialiasing: true
                preferredRendererType: Shape.GeometryRenderer

                ShapePath {
                    fillColor: root.shadowColor
                    strokeColor: "transparent"
                    PathSvg { path: modelData }
                }
            }
        }
    }

    Item {
        x: (root.size - root.artWidth) / 2
        y: 0
        width: root.artWidth
        height: root.size

        Repeater {
            model: BrandGlyphs.tuxPaths

            delegate: Shape {
                required property var modelData

                width: root.viewWidth
                height: root.viewHeight
                scale: root.artScale
                transformOrigin: Item.TopLeft
                antialiasing: true
                preferredRendererType: Shape.GeometryRenderer

                ShapePath {
                    fillColor: root.color
                    strokeColor: "transparent"
                    PathSvg { path: modelData }
                }
            }
        }
    }
}
