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

    // Pane and Files glyphs painted on the Canvas below. The shell/sidebar
    // and first-party-app vocabulary.
    readonly property var paintedGlyphs: ["appearance", "wallpaper", "dock",
                                          "displays", "home", "documents",
                                          "downloads", "music", "movies",
                                          "trash", "computer", "volume",
                                          "folder", "file", "icon-view",
                                          "list-view", "wifi", "bluetooth",
                                          "brightness", "focus"]
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
            case "focus": {
                // Crescent moon: a filled disc with the upper-right bitten out
                // (compositing to transparent so it reads on any tile fill).
                ctx.beginPath();
                ctx.arc(c, c, s * 0.36, 0, Math.PI * 2);
                ctx.fill();
                ctx.globalCompositeOperation = "destination-out";
                ctx.beginPath();
                ctx.arc(s * 0.66, s * 0.34, s * 0.33, 0, Math.PI * 2);
                ctx.fill();
                ctx.globalCompositeOperation = "source-over";
                break;
            }
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
            case "wifi": {
                // Three arcs opening downward with a small dot at the origin.
                var cy = s * 0.82;
                var radii = [s * 0.18, s * 0.34, s * 0.50];
                for (var r = 0; r < radii.length; ++r) {
                    ctx.beginPath();
                    ctx.arc(c, cy, radii[r], Math.PI * 1.15, Math.PI * 1.85);
                    ctx.stroke();
                }
                ctx.beginPath();
                ctx.arc(c, cy, root.stroke * 0.8, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
            case "bluetooth": {
                // A vertical spine with the two-triangle bowtie.
                ctx.beginPath();
                ctx.moveTo(c, s * 0.12);
                ctx.lineTo(c, s * 0.88);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(c, s * 0.12);
                ctx.lineTo(s * 0.70, s * 0.33);
                ctx.lineTo(c, s * 0.50);
                ctx.lineTo(s * 0.70, s * 0.67);
                ctx.lineTo(c, s * 0.88);
                ctx.stroke();
                break;
            }
            case "brightness": {
                // A sun: a center disc with eight rays.
                ctx.beginPath();
                ctx.arc(c, c, s * 0.22, 0, Math.PI * 2);
                ctx.stroke();
                for (var k = 0; k < 8; ++k) {
                    var a = k * Math.PI / 4;
                    ctx.beginPath();
                    ctx.moveTo(c + Math.cos(a) * s * 0.32, c + Math.sin(a) * s * 0.32);
                    ctx.lineTo(c + Math.cos(a) * s * 0.44, c + Math.sin(a) * s * 0.44);
                    ctx.stroke();
                }
                break;
            }
            case "home": {
                // A house: pitched roof over a body with a door.
                ctx.beginPath();
                ctx.moveTo(s * 0.50, s * 0.14);
                ctx.lineTo(s * 0.13, s * 0.46);
                ctx.lineTo(s * 0.22, s * 0.46);
                ctx.lineTo(s * 0.22, s * 0.85);
                ctx.lineTo(s * 0.42, s * 0.85);
                ctx.lineTo(s * 0.42, s * 0.62);
                ctx.lineTo(s * 0.58, s * 0.62);
                ctx.lineTo(s * 0.58, s * 0.85);
                ctx.lineTo(s * 0.78, s * 0.85);
                ctx.lineTo(s * 0.78, s * 0.46);
                ctx.lineTo(s * 0.87, s * 0.46);
                ctx.closePath();
                ctx.stroke();
                break;
            }
            case "documents": {
                // A page with a folded corner and three text rules.
                ctx.beginPath();
                ctx.moveTo(s * 0.26, s * 0.14);
                ctx.lineTo(s * 0.60, s * 0.14);
                ctx.lineTo(s * 0.76, s * 0.30);
                ctx.lineTo(s * 0.76, s * 0.86);
                ctx.lineTo(s * 0.26, s * 0.86);
                ctx.closePath();
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.60, s * 0.14);
                ctx.lineTo(s * 0.60, s * 0.30);
                ctx.lineTo(s * 0.76, s * 0.30);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.36, s * 0.50);
                ctx.lineTo(s * 0.66, s * 0.50);
                ctx.moveTo(s * 0.36, s * 0.62);
                ctx.lineTo(s * 0.66, s * 0.62);
                ctx.moveTo(s * 0.36, s * 0.74);
                ctx.lineTo(s * 0.56, s * 0.74);
                ctx.stroke();
                break;
            }
            case "downloads": {
                // A tray with a downward arrow.
                ctx.beginPath();
                ctx.moveTo(s * 0.18, s * 0.62);
                ctx.lineTo(s * 0.18, s * 0.84);
                ctx.lineTo(s * 0.82, s * 0.84);
                ctx.lineTo(s * 0.82, s * 0.62);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(c, s * 0.16);
                ctx.lineTo(c, s * 0.60);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.35, s * 0.45);
                ctx.lineTo(c, s * 0.60);
                ctx.lineTo(s * 0.65, s * 0.45);
                ctx.stroke();
                break;
            }
            case "music": {
                // A beamed eighth note.
                ctx.beginPath();
                ctx.moveTo(s * 0.64, s * 0.16);
                ctx.lineTo(s * 0.64, s * 0.66);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.64, s * 0.16);
                ctx.lineTo(s * 0.84, s * 0.24);
                ctx.lineTo(s * 0.84, s * 0.38);
                ctx.lineTo(s * 0.64, s * 0.30);
                ctx.closePath();
                ctx.fill();
                ctx.beginPath();
                ctx.arc(s * 0.50, s * 0.70, s * 0.13, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
            case "movies": {
                // A screen with a play triangle.
                roundedRect(s * 0.13, s * 0.24, s * 0.74, s * 0.52, s * 0.12);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.44, s * 0.36);
                ctx.lineTo(s * 0.44, s * 0.64);
                ctx.lineTo(s * 0.66, s * 0.50);
                ctx.closePath();
                ctx.fill();
                break;
            }
            case "trash": {
                // A waste bin: lid, handle, tapered body, two slats.
                ctx.beginPath();
                ctx.moveTo(s * 0.15, s * 0.28);
                ctx.lineTo(s * 0.85, s * 0.28);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.38, s * 0.28);
                ctx.lineTo(s * 0.38, s * 0.17);
                ctx.lineTo(s * 0.62, s * 0.17);
                ctx.lineTo(s * 0.62, s * 0.28);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.24, s * 0.28);
                ctx.lineTo(s * 0.30, s * 0.87);
                ctx.lineTo(s * 0.70, s * 0.87);
                ctx.lineTo(s * 0.76, s * 0.28);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.42, s * 0.40);
                ctx.lineTo(s * 0.44, s * 0.76);
                ctx.moveTo(s * 0.58, s * 0.40);
                ctx.lineTo(s * 0.56, s * 0.76);
                ctx.stroke();
                break;
            }
            case "computer": {
                // A display on a stand (the Computer location).
                roundedRect(s * 0.12, s * 0.20, s * 0.76, s * 0.50, s * 0.10);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(c, s * 0.70);
                ctx.lineTo(c, s * 0.83);
                ctx.moveTo(s * 0.34, s * 0.86);
                ctx.lineTo(s * 0.66, s * 0.86);
                ctx.stroke();
                break;
            }
            case "volume": {
                // A drive slab with a level indicator and a small light.
                roundedRect(s * 0.12, s * 0.32, s * 0.76, s * 0.36, s * 0.10);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.22, s * 0.50);
                ctx.lineTo(s * 0.50, s * 0.50);
                ctx.stroke();
                ctx.beginPath();
                ctx.arc(s * 0.71, s * 0.50, s * 0.04, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
            case "folder": {
                // A folder: a back tab and a front body.
                ctx.beginPath();
                ctx.moveTo(s * 0.14, s * 0.30);
                ctx.lineTo(s * 0.42, s * 0.30);
                ctx.lineTo(s * 0.50, s * 0.40);
                ctx.lineTo(s * 0.86, s * 0.40);
                ctx.lineTo(s * 0.86, s * 0.82);
                ctx.lineTo(s * 0.14, s * 0.82);
                ctx.closePath();
                ctx.stroke();
                break;
            }
            case "file": {
                // A page with a folded corner (a generic document).
                ctx.beginPath();
                ctx.moveTo(s * 0.24, s * 0.14);
                ctx.lineTo(s * 0.58, s * 0.14);
                ctx.lineTo(s * 0.76, s * 0.32);
                ctx.lineTo(s * 0.76, s * 0.86);
                ctx.lineTo(s * 0.24, s * 0.86);
                ctx.closePath();
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(s * 0.58, s * 0.14);
                ctx.lineTo(s * 0.58, s * 0.32);
                ctx.lineTo(s * 0.76, s * 0.32);
                ctx.stroke();
                break;
            }
            case "icon-view": {
                // A 2x2 grid of rounded tiles (the toolbar view switch).
                var tileSize = s * 0.28;
                var tileGap = s * 0.12;
                var gridOrigin = s * 0.16;
                for (var gx = 0; gx < 2; ++gx) {
                    for (var gy = 0; gy < 2; ++gy) {
                        roundedRect(gridOrigin + gx * (tileSize + tileGap),
                                    gridOrigin + gy * (tileSize + tileGap),
                                    tileSize, tileSize, s * 0.05);
                        ctx.fill();
                    }
                }
                break;
            }
            case "list-view": {
                // Three stacked rows (the toolbar view switch).
                var rowHeight = s * 0.13;
                var rowGap = s * 0.10;
                var rowOrigin = s * 0.17;
                for (var lr = 0; lr < 3; ++lr) {
                    roundedRect(s * 0.14, rowOrigin + lr * (rowHeight + rowGap),
                                s * 0.72, rowHeight, rowHeight / 2);
                    ctx.fill();
                }
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
