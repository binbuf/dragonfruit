// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Original vector glyphs drawn from the design-system palette. No bitmap
// assets: every mark here is geometry we own (see 14-risks.md, IP rule).
//
// Two drawing paths: straight-stroke glyphs are built from rotated bars, and
// the richer pane glyphs (Settings sidebar) are painted on a Canvas. Adding a
// glyph means adding it to exactly one of the two; the pane names below are
// the shared vocabulary first-party apps use for their sidebar rows.
Item {
    id: root

    property string name: "check"
    property color color: Theme.color.textPrimary
    property real size: 16

    implicitWidth: size
    implicitHeight: size

    readonly property real stroke: Math.max(1.4, size * 0.14)

    // Pane glyphs painted on the Canvas below. The shell/sidebar vocabulary.
    readonly property var paintedGlyphs: ["appearance", "wallpaper", "dock",
                                          "displays"]
    readonly property bool painted: root.paintedGlyphs.indexOf(root.name) >= 0

    readonly property var bars: {
        var s = size;
        var t = stroke;
        switch (name) {
        case "close":
            return [
                { x: s * 0.5, y: s * 0.5, w: s * 0.60, h: t, a: 45 },
                { x: s * 0.5, y: s * 0.5, w: s * 0.60, h: t, a: -45 }
            ];
        case "minimize":
            return [{ x: s * 0.5, y: s * 0.5, w: s * 0.60, h: t, a: 0 }];
        case "zoom":
            return [
                { x: s * 0.5, y: s * 0.5, w: s * 0.60, h: t, a: 0 },
                { x: s * 0.5, y: s * 0.5, w: t, h: s * 0.60, a: 0 }
            ];
        case "check":
            return [
                { x: s * 0.30, y: s * 0.54, w: s * 0.30, h: t, a: 45 },
                { x: s * 0.58, y: s * 0.44, w: s * 0.52, h: t, a: -45 }
            ];
        case "chevron-down":
            return [
                { x: s * 0.40, y: s * 0.52, w: s * 0.40, h: t, a: 45 },
                { x: s * 0.60, y: s * 0.52, w: s * 0.40, h: t, a: -45 }
            ];
        case "chevron-right":
            return [
                { x: s * 0.52, y: s * 0.40, w: s * 0.40, h: t, a: 45 },
                { x: s * 0.52, y: s * 0.60, w: s * 0.40, h: t, a: -45 }
            ];
        case "chevron-left":
            return [
                { x: s * 0.48, y: s * 0.40, w: s * 0.40, h: t, a: -45 },
                { x: s * 0.48, y: s * 0.60, w: s * 0.40, h: t, a: 45 }
            ];
        case "search":
            return [
                { x: s * 0.67, y: s * 0.67, w: s * 0.32, h: t, a: 45 }
            ];
        default:
            return [];
        }
    }

    Repeater {
        model: root.bars
        delegate: Rectangle {
            required property var modelData
            width: modelData.w
            height: modelData.h
            radius: height / 2
            color: root.color
            antialiasing: true
            x: modelData.x - width / 2
            y: modelData.y - height / 2
            rotation: modelData.a
            transformOrigin: Item.Center
        }
    }

    // A ring for the search glyph; kept separate so the bars stay simple.
    Rectangle {
        visible: root.name === "search"
        width: root.size * 0.44
        height: width
        radius: width / 2
        x: root.size * 0.20
        y: root.size * 0.20
        color: "transparent"
        border.width: root.stroke
        border.color: root.color
        antialiasing: true
    }

    // Pane glyphs. One optical stroke weight, original geometry (no assets).
    // Repaint on the same triggers as the menu-bar status glyph so a one-shot
    // offscreen grab always sees the mark.
    Canvas {
        id: canvas
        anchors.fill: parent
        visible: root.painted
        renderStrategy: Canvas.Immediate

        onPaint: {
            var ctx = getContext("2d");
            var s = root.size;
            var c = s / 2;
            ctx.reset();
            ctx.lineWidth = root.stroke;
            ctx.lineCap = "round";
            ctx.lineJoin = "round";
            ctx.strokeStyle = root.color;
            ctx.fillStyle = root.color;

            function roundedRect(x, y, w, h, r) {
                r = Math.min(r, w / 2, h / 2);
                ctx.beginPath();
                ctx.moveTo(x + r, y);
                ctx.lineTo(x + w - r, y);
                ctx.arcTo(x + w, y, x + w, y + r, r);
                ctx.lineTo(x + w, y + h - r);
                ctx.arcTo(x + w, y + h, x + w - r, y + h, r);
                ctx.lineTo(x + r, y + h);
                ctx.arcTo(x, y + h, x, y + h - r, r);
                ctx.lineTo(x, y + r);
                ctx.arcTo(x, y, x + r, y, r);
                ctx.closePath();
            }

            switch (root.name) {
            case "appearance": {
                // A full ring with the right half filled (light/dark contrast).
                ctx.beginPath();
                ctx.arc(c, c, s * 0.36, 0, Math.PI * 2);
                ctx.stroke();
                ctx.beginPath();
                ctx.arc(c, c, s * 0.36, -Math.PI / 2, Math.PI / 2, false);
                ctx.closePath();
                ctx.fill();
                break;
            }
            case "wallpaper": {
                // A picture frame: sun over a pair of hills, clipped to it.
                ctx.save();
                roundedRect(s * 0.12, s * 0.18, s * 0.76, s * 0.64, s * 0.10);
                ctx.clip();
                ctx.beginPath();
                ctx.arc(s * 0.36, s * 0.38, s * 0.10, 0, Math.PI * 2);
                ctx.fill();
                ctx.beginPath();
                ctx.moveTo(s * 0.14, s * 0.82);
                ctx.lineTo(s * 0.44, s * 0.52);
                ctx.lineTo(s * 0.60, s * 0.68);
                ctx.lineTo(s * 0.72, s * 0.56);
                ctx.lineTo(s * 0.88, s * 0.82);
                ctx.closePath();
                ctx.fill();
                ctx.restore();
                roundedRect(s * 0.12, s * 0.18, s * 0.76, s * 0.64, s * 0.10);
                ctx.stroke();
                break;
            }
            case "dock": {
                // The Dock bar with three running-app tiles resting above it.
                roundedRect(s * 0.10, s * 0.60, s * 0.80, s * 0.26, s * 0.12);
                ctx.stroke();
                var tile = s * 0.16;
                for (var i = 0; i < 3; ++i) {
                    roundedRect(s * 0.24 + i * (tile + s * 0.06),
                                s * 0.30, tile, tile, s * 0.05);
                    ctx.fill();
                }
                break;
            }
            case "displays": {
                // A display outline on a stand.
                roundedRect(s * 0.10, s * 0.16, s * 0.80, s * 0.52, s * 0.08);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(c, s * 0.68);
                ctx.lineTo(c, s * 0.82);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.34, s * 0.86);
                ctx.lineTo(s * 0.66, s * 0.86);
                ctx.stroke();
                break;
            }
            }
        }
    }

    onNameChanged: canvas.requestPaint()
    onColorChanged: canvas.requestPaint()
    onSizeChanged: canvas.requestPaint()
    Component.onCompleted: canvas.requestPaint()
    onWindowChanged: if (root.window) canvas.requestPaint()
    onVisibleChanged: if (root.visible) canvas.requestPaint()
}
