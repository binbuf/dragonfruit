// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes

// The Dragonfruit brand mark: the Swept Trio silhouette from `dragonfruit.svg`.
// Keep this 24 x 24 path in sync with the source SVG. The filled geometry stays
// legible at menu-bar size and inherits the palette for light and dark chrome.
Item {
    id: root

    property color color: Theme.color.textPrimary
    property real size: Theme.controls.menuBar.iconSize

    // Preserve the square source view box at any `size`.
    implicitWidth: size
    implicitHeight: size

    Shape {
        id: shape
        width: 24
        height: 24
        // Uniform scale so the 24 x 24 art fits `root.size`.
        scale: root.size / 24
        transformOrigin: Item.TopLeft
        antialiasing: true
        // The geometry renderer is supported by every scene-graph backend
        // (including the software backend the headless tests use).
        preferredRendererType: Shape.GeometryRenderer

        ShapePath {
            fillColor: root.color
            strokeColor: "transparent"
            PathSvg {
                path: "M12 22C6.8 22 3.4 18.5 3.4 14.1C3.4 10.6 5.2 8.5 5 4.4C7.6 5 9.3 6.5 10.1 8.4C10.1 5.6 11 3.1 13.1 1.5C13 4.4 14 6.8 15.7 8.4C17.4 7.2 19.1 6.6 21 6.5C20.3 9.5 20.6 11.8 20.6 14.2C20.6 18.8 17 22 12 22Z"
            }
        }
    }
}
