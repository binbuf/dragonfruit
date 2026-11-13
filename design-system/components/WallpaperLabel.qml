// SPDX-License-Identifier: MIT
pragma ComponentBehavior: Bound
import QtQuick
import Dragonfruit

// Text that sits directly on the wallpaper: the menu-bar titles, the status
// clock and the desktop icon labels. A plain `Text` has no shadow, and the
// platform text renderer cannot draw one, so this item layers a small number
// of offset copies behind the glyphs and adds a one-pixel edge outline. The
// copies follow `Shadow.qml`'s software-renderer-friendly approach: Qt's
// `MultiEffect` shadow is GPU-only and silently no-ops on the headless
// software backend the tests and gallery use.
//
// The caller owns the polarity. Pass `shadowColor` and `shadowOpacity` from
// the `Theme.controls.wallpaperText` pair that matches the text colour (dark
// shadow under white text, light shadow under black text). Turn the treatment
// off when the text sits on an opaque plate (an open menu capsule, a hover
// wash or a selection pill) so it does not paint a halo over the plate.
Item {
    id: root

    property string text: ""
    property color color: Theme.controls.menuBar.foregroundOnDark
    // The opposite-tone shadow/outline colour. Defaults to the white-text
    // pair; callers using the black-text pair pass `shadowColorOnLight`.
    property color shadowColor: Theme.controls.wallpaperText.shadowColorOnDark
    property bool shadowEnabled: true
    property int shadowLayers: Theme.controls.wallpaperText.shadowLayers
    property real shadowOpacity: Theme.controls.wallpaperText.shadowOpacityOnDark
    property point shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)
    property real outlineOpacity: Theme.controls.wallpaperText.outlineOpacity
    // The accessibility "Increase contrast" override raises both the shadow
    // and the edge so the glyph keeps a hard local contrast field.
    property bool increaseContrast: Glass.increaseContrast

    // The Text properties callers actually use. Aliases keep this a drop-in
    // replacement for the bare `Text` at every call site.
    property alias font: label.font
    property alias horizontalAlignment: label.horizontalAlignment
    property alias verticalAlignment: label.verticalAlignment
    property alias wrapMode: label.wrapMode
    property alias elide: label.elide
    property alias maximumLineCount: label.maximumLineCount
    property alias lineHeight: label.lineHeight
    property alias lineHeightMode: label.lineHeightMode
    property alias textFormat: label.textFormat
    readonly property alias contentWidth: label.contentWidth
    readonly property alias contentHeight: label.contentHeight
    readonly property alias lineCount: label.lineCount
    readonly property alias truncated: label.truncated

    readonly property real effectiveShadowOpacity:
        shadowEnabled && shadowColor.a > 0
        ? (increaseContrast
           ? Theme.controls.wallpaperText.increaseContrastShadowOpacity
           : shadowOpacity)
        : 0
    readonly property real effectiveOutlineOpacity:
        shadowEnabled && shadowColor.a > 0
        ? (increaseContrast
           ? Theme.controls.wallpaperText.increaseContrastOutlineOpacity
           : outlineOpacity)
        : 0

    implicitWidth: label.implicitWidth
    implicitHeight: label.implicitHeight
    width: implicitWidth
    height: implicitHeight

    // The soft part: a few copies stepped downward. Each copy is weaker than
    // the one above it, so the shadow fades into the wallpaper instead of
    // ending in a hard second edge.
    Repeater {
        model: root.effectiveShadowOpacity > 0 ? Math.max(1, root.shadowLayers) : 0
        delegate: Text {
            required property int index
            x: root.shadowOffset.x
            y: root.shadowOffset.y + index
            width: root.width
            text: root.text
            color: root.shadowColor
            opacity: root.effectiveShadowOpacity
                     * (1.0 - 0.55 * index / Math.max(1, root.shadowLayers))
            font: root.font
            horizontalAlignment: root.horizontalAlignment
            verticalAlignment: root.verticalAlignment
            wrapMode: root.wrapMode
            elide: root.elide
            maximumLineCount: root.maximumLineCount
            lineHeight: root.lineHeight
            lineHeightMode: root.lineHeightMode
            textFormat: root.textFormat
        }
    }

    // The hard part: the glyph itself with a one-pixel outline. The outline
    // guarantees an edge even where the soft copies are too faint, which is
    // what keeps a 12 pt label readable on a photographic wallpaper.
    Text {
        id: label
        width: root.width
        text: root.text
        color: root.color
        style: root.effectiveOutlineOpacity > 0 ? Text.Outline : Text.Normal
        styleColor: Qt.rgba(root.shadowColor.r, root.shadowColor.g,
                            root.shadowColor.b, root.effectiveOutlineOpacity)
    }
}
