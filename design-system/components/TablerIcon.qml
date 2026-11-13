// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit

// The Tabler glyph primitive. It renders one named glyph from the vendored
// Tabler set, tinted to `color` and scaled to `size`, keeping the glyph's
// square aspect. It draws no container and no background: the icon is a
// resource, the styling is QML (see ADR 0185). `Icon` is the compatibility
// facade over this component; new code should use `TablerIcon` directly.
//
// Why a Shape and not an Image/MultiEffect:
//   * Qt 6.11's MultiEffect colorization is GPU-only and silently no-ops on
//     the headless software backend the gallery and QML tests use — the same
//     limitation Shadow.qml documents.
//   * `MultiEffect` on a `VectorImage` rendered nothing at all under software.
//   * A ShapePath with bound `strokeColor`/`fillColor` renders and tints under
//     both the software and the RHI scene graphs.
// The glyph's path data is generated from the vendored SVG at build time
// (`scripts/gen-tabler-glyphs.py` -> `TablerGlyphs.qml`); the SVG itself
// stays the shipped resource at `qrc:/icons/tabler/<file>` (`source` below).
Item {
    id: root

    // The Tabler glyph name, e.g. "wifi". A name that is not vendored
    // resolves to nothing (and warns); it never silently substitutes a glyph.
    property string name: ""
    // The tint. Tabler glyphs are monochrome paths with no baked color.
    property color color: Theme.color.textPrimary
    // The square edge length in logical pixels.
    property real size: 16
    // "outline" (the default; stroked) or "filled" (solid). The vendored
    // filled set is deliberately small: the Dock launcher and app artwork.
    property string weight: "outline"
    // The outline stroke width in the upstream 24-unit viewBox. The default
    // matches Tabler's own 2 px stroke; callers at very small sizes can raise
    // it to keep the mark legible.
    property real strokeWidth: 2
    // Optional accessible name. Empty means presentational (AT-SPI ignored).
    property string accessibleName: ""

    // Optional wallpaper contrast shadow: the same path translated by
    // `shadowOffset` and painted behind the glyph. The menu bar uses this so
    // the status marks survive a busy backdrop; the Dock and app icons leave
    // it transparent.
    property color shadowColor: "transparent"
    property point shadowOffset: Qt.point(0, 0)

    implicitWidth: size
    implicitHeight: size

    readonly property bool filled: root.weight === "filled"

    // The upstream file for (name, weight). This is the shipped resource; the
    // registry below holds the same geometry in a software-tintable form.
    readonly property string source: {
        if (root.name.length === 0)
            return "";
        if (root.weight !== "outline" && root.weight !== "filled")
            return "";
        var suffix = root.filled ? "-filled" : "";
        return "qrc:/icons/tabler/" + root.name + suffix + ".svg";
    }
    // The primary PathSvg data (stroke for outline, fill for filled).
    readonly property string glyphPath: TablerGlyphs.path(root.weight, root.name)
    // The optional filled overlay for an outline glyph (the `accessible` head
    // dot). Empty for filled glyphs, which already return their solid path.
    readonly property string glyphFillPath: TablerGlyphs.fillPath(root.weight, root.name)
    // False when the name/weight is not vendored. Exposed so tests and callers
    // can tell an intentionally blank icon from a typo.
    readonly property bool resolved: root.glyphPath.length > 0
                                     || root.glyphFillPath.length > 0

    Accessible.role: Accessible.Graphic
    Accessible.name: root.accessibleName
    Accessible.ignored: root.accessibleName.length === 0

    Shape {
        x: root.shadowOffset.x
        y: root.shadowOffset.y
        width: root.size
        height: root.size
        visible: root.resolved && root.shadowColor.a > 0
        preferredRendererType: Shape.GeometryRenderer
        transformOrigin: Item.TopLeft
        transform: Scale {
            xScale: root.size / TablerGlyphs.viewBox
            yScale: root.size / TablerGlyphs.viewBox
        }

        ShapePath {
            strokeColor: root.filled ? "transparent" : root.shadowColor
            strokeWidth: root.filled ? 0 : root.strokeWidth
            fillColor: root.filled ? root.shadowColor : "transparent"
            fillRule: ShapePath.WindingFill
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.glyphPath }
        }

        ShapePath {
            strokeColor: "transparent"
            strokeWidth: 0
            fillColor: root.shadowColor
            fillRule: ShapePath.WindingFill
            PathSvg { path: root.glyphFillPath }
        }
    }

    Shape {
        anchors.fill: parent
        visible: root.resolved
        // GeometryRenderer triangulates on the CPU and works under the
        // software backend; the CurveRenderer is GPU-only.
        preferredRendererType: Shape.GeometryRenderer
        transformOrigin: Item.TopLeft
        transform: Scale {
            xScale: root.size / TablerGlyphs.viewBox
            yScale: root.size / TablerGlyphs.viewBox
        }

        // The stroked outline (or the solid filled path).
        ShapePath {
            strokeColor: root.filled ? "transparent" : root.color
            strokeWidth: root.filled ? 0 : root.strokeWidth
            fillColor: root.filled ? root.color : "transparent"
            fillRule: ShapePath.WindingFill
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.glyphPath }
        }

        // The rare filled dot inside an outline glyph.
        ShapePath {
            strokeColor: "transparent"
            strokeWidth: 0
            fillColor: root.color
            fillRule: ShapePath.WindingFill
            PathSvg { path: root.glyphFillPath }
        }
    }

    // Never let a typo ship as a silent blank mark: it warns, and the
    // build-time gate (`scripts/check-tabler-icons.py`) rejects it.
    Component.onCompleted: {
        if (root.name.length > 0 && !root.resolved)
            console.warn("TablerIcon: unknown glyph \"" + root.name
                         + "\" (weight \"" + root.weight + "\")");
    }
}
