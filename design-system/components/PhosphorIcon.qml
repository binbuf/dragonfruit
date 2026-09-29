// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit

// The Phosphor glyph primitive (T-19.1a). It renders one named glyph from the
// vendored Phosphor set, tinted to `color` and scaled to `size`, keeping the
// glyph's square aspect. It draws no container and no background: the icon is
// a resource, the styling is QML (see ADR 0163). `SettingsCategoryIcon`
// (T-19.1b) wraps it with the gradient tile; the menu bar and Dock use it bare.
//
// Why a Shape and not an Image/MultiEffect:
//   * Qt 6.11's MultiEffect colorization is GPU-only and silently no-ops on
//     the headless software backend the gallery and QML tests use — the same
//     limitation Shadow.qml documents.
//   * `MultiEffect` on a `VectorImage` rendered nothing at all under software.
//   * A ShapePath with a bound `fillColor` renders and tints under both the
//     software and the RHI scene graphs.
// The glyph's path data is generated from the vendored SVG at build time
// (`scripts/gen-phosphor-glyphs.py` -> `PhosphorGlyphs.qml`); the SVG itself
// stays the shipped resource at `qrc:/icons/phosphor/<file>` (`source` below).
Item {
    id: root

    // The Phosphor glyph name, e.g. "wifi-high". A name that is not vendored
    // resolves to nothing (and warns); it never silently substitutes a glyph.
    property string name: ""
    // The tint. Phosphor glyphs are monochrome paths with no baked color.
    property color color: Theme.color.textPrimary
    // The square edge length in logical pixels.
    property real size: 16
    // The Phosphor weight directory. The vendored default is "regular"; the
    // solid "fill" weight is the one the Settings gradient tile and the
    // near-white app marks use. Any other weight is not vendored and resolves
    // to nothing.
    property string weight: "regular"
    // Optional accessible name. Empty means presentational (AT-SPI ignored).
    property string accessibleName: ""

    implicitWidth: size
    implicitHeight: size

    // The upstream file for (name, weight). This is the shipped resource; the
    // registry below holds the same geometry in a software-tintable form.
    readonly property string source: {
        if (root.name.length === 0)
            return "";
        var suffix = (root.weight.length > 0 && root.weight !== "regular")
                ? "-" + root.weight : "";
        return "qrc:/icons/phosphor/" + root.name + suffix + ".svg";
    }
    // The PathSvg data for this glyph, or "" when (name, weight) is unknown.
    readonly property string glyphPath: PhosphorGlyphs.path(weight, name)
    // False when the name/weight is not vendored. Exposed so tests and callers
    // can tell an intentionally blank icon from a typo.
    readonly property bool resolved: glyphPath.length > 0

    Accessible.role: Accessible.Graphic
    Accessible.name: root.accessibleName
    Accessible.ignored: root.accessibleName.length === 0

    Shape {
        anchors.fill: parent
        visible: root.resolved
        // GeometryRenderer triangulates on the CPU and works under the
        // software backend; the CurveRenderer is GPU-only.
        preferredRendererType: Shape.GeometryRenderer
        transformOrigin: Item.TopLeft
        transform: Scale {
            xScale: root.size / PhosphorGlyphs.viewBox
            yScale: root.size / PhosphorGlyphs.viewBox
        }

        ShapePath {
            strokeColor: "transparent"
            fillColor: root.color
            fillRule: ShapePath.WindingFill
            PathSvg { path: root.glyphPath }
        }
    }

    // Never let a typo ship as a silent blank mark: it warns, and the
    // build-time gate (`scripts/check-phosphor-icons.py`) rejects it.
    Component.onCompleted: {
        if (root.name.length > 0 && !root.resolved)
            console.warn("PhosphorIcon: unknown glyph \"" + root.name
                         + "\" (weight \"" + root.weight + "\")");
    }
}