// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes

// The Dragonfruit brand mark: the Swept Trio silhouette from
// `assets/icons/brand/dragonfruit.svg`. The geometry comes from the generated
// `BrandGlyphs` registry (`make check-brand` keeps it in sync with the SVG),
// so editing the source SVG and re-running the generator is the only way to
// change the mark. The filled geometry stays legible at menu-bar size and
// inherits the palette for light and dark chrome.
Item {
    id: root

    property color color: Theme.color.textPrimary
    property real size: Theme.controls.menuBar.iconSize
    // Optional wallpaper contrast shadow: the same silhouette translated by
    // `shadowOffset` and painted behind the mark. The menu bar supplies the
    // opposite-tone pair; other callers leave it transparent.
    property color shadowColor: "transparent"
    property point shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)

    // Preserve the square source view box at any `size`.
    implicitWidth: size
    implicitHeight: size

    Shape {
        x: root.shadowOffset.x
        y: root.shadowOffset.y
        width: BrandGlyphs.dragonfruitViewBoxWidth
        height: BrandGlyphs.dragonfruitViewBoxHeight
        visible: root.shadowColor.a > 0
        scale: root.size / BrandGlyphs.dragonfruitViewBoxWidth
        transformOrigin: Item.TopLeft
        antialiasing: true
        preferredRendererType: Shape.GeometryRenderer

        ShapePath {
            fillColor: root.shadowColor
            strokeColor: "transparent"
            PathSvg {
                path: BrandGlyphs.dragonfruitPath
            }
        }
    }

    Shape {
        id: shape
        width: BrandGlyphs.dragonfruitViewBoxWidth
        height: BrandGlyphs.dragonfruitViewBoxHeight
        // Uniform scale so the source view box fits `root.size`.
        scale: root.size / BrandGlyphs.dragonfruitViewBoxWidth
        transformOrigin: Item.TopLeft
        antialiasing: true
        // The geometry renderer is supported by every scene-graph backend
        // (including the software backend the headless tests use).
        preferredRendererType: Shape.GeometryRenderer

        ShapePath {
            fillColor: root.color
            strokeColor: "transparent"
            PathSvg {
                path: BrandGlyphs.dragonfruitPath
            }
        }
    }
}
