// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Original vector glyphs for menu-bar status items (T-09 FR-5). Every mark
// is geometry we own — no bitmap assets and no copied proprietary artwork
// (see docs/design/14-risks.md). The visual language is tuned to read as a
// macOS-style menu bar: monochrome, one uniform optical stroke with rounded
// caps/joins, even optical sizing, and a filled level element where macOS
// uses one (Wi-Fi origin dot, battery fill).
Item {
    id: root

    property string name: "wifi"
    property color color: Theme.color.textPrimary
    property color backgroundColor: Theme.color.chrome
    property real size: Theme.controls.menuBar.iconSize
    // Battery level, 0..1.
    property real level: 0.8

    implicitWidth: size
    implicitHeight: size

    // One optical stroke weight at every size.
    readonly property real stroke: Math.max(1.3, size * 0.10)

    onNameChanged: canvas.requestPaint()
    onColorChanged: canvas.requestPaint()
    onSizeChanged: canvas.requestPaint()
    onLevelChanged: canvas.requestPaint()
    // The first request may fire before the item is in an exposed window and
    // be dropped; re-request when the item joins a window and becomes
    // visible, so the shell's one-shot `grabWindow` always sees the glyph.
    onWindowChanged: if (root.window) canvas.requestPaint()
    onVisibleChanged: if (root.visible) canvas.requestPaint()
    Component.onCompleted: canvas.requestPaint()

    Canvas {
        id: canvas
        anchors.fill: parent
        // Paint synchronously so a one-shot offscreen grab (the shell's shm
        // render and the visual tests) always sees the glyph.
        renderStrategy: Canvas.Immediate

        // Canvas 2D has no roundRect on every Qt build; draw one from arcs.
        function roundedRect(ctx, x, y, w, h, r) {
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

            switch (root.name) {
            case "wifi": {
                // Three thin arcs opening downward with a small filled dot at
                // the origin (the macOS silhouette).
                var cy = s * 0.80;
                var radii = [s * 0.20, s * 0.36, s * 0.52];
                for (var i = 0; i < radii.length; ++i) {
                    ctx.beginPath();
                    ctx.arc(c, cy, radii[i], Math.PI * 1.15, Math.PI * 1.85);
                    ctx.stroke();
                }
                ctx.beginPath();
                ctx.arc(c, cy, root.stroke * 0.85, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
            case "bluetooth": {
                // The rune: a vertical spine with the two-triangle bowtie
                // meeting it at the top, centre, and bottom.
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
            case "volume":
            case "volume-muted": {
                // Speaker body + cone, filled like macOS's volume glyph.
                ctx.beginPath();
                ctx.moveTo(s * 0.14, s * 0.38);
                ctx.lineTo(s * 0.32, s * 0.38);
                ctx.lineTo(s * 0.52, s * 0.20);
                ctx.lineTo(s * 0.52, s * 0.80);
                ctx.lineTo(s * 0.32, s * 0.62);
                ctx.lineTo(s * 0.14, s * 0.62);
                ctx.closePath();
                ctx.fill();
                if (root.name === "volume") {
                    ctx.beginPath();
                    ctx.arc(s * 0.56, s * 0.50, s * 0.15, -Math.PI * 0.38, Math.PI * 0.38);
                    ctx.stroke();
                    ctx.beginPath();
                    ctx.arc(s * 0.56, s * 0.50, s * 0.30, -Math.PI * 0.34, Math.PI * 0.34);
                    ctx.stroke();
                } else {
                    ctx.beginPath();
                    ctx.moveTo(s * 0.64, s * 0.40);
                    ctx.lineTo(s * 0.86, s * 0.60);
                    ctx.moveTo(s * 0.86, s * 0.40);
                    ctx.lineTo(s * 0.64, s * 0.60);
                    ctx.stroke();
                }
                break;
            }
            case "battery": {
                // Outline cell + nub + a rounded level fill (macOS battery).
                roundedRect(ctx, s * 0.08, s * 0.30, s * 0.66, s * 0.40, s * 0.12);
                ctx.stroke();
                roundedRect(ctx, s * 0.78, s * 0.42, s * 0.09, s * 0.16, s * 0.035);
                ctx.fill();
                var fillW = s * 0.60 * Math.max(0, Math.min(1, root.level));
                if (fillW > s * 0.02) {
                    roundedRect(ctx, s * 0.11, s * 0.33, fillW, s * 0.34, s * 0.09);
                    ctx.fill();
                }
                break;
            }
            case "focus": {
                // Crescent moon: a filled disc with a background-coloured
                // bite taken out of the upper right.
                ctx.beginPath();
                ctx.arc(s * 0.46, s * 0.52, s * 0.36, 0, Math.PI * 2);
                ctx.fill();
                ctx.fillStyle = root.backgroundColor;
                ctx.beginPath();
                ctx.arc(s * 0.70, s * 0.34, s * 0.33, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
            case "accessibility": {
                ctx.beginPath();
                ctx.arc(c, s * 0.20, s * 0.11, 0, Math.PI * 2);
                ctx.fill();
                ctx.beginPath();
                ctx.moveTo(s * 0.18, s * 0.44);
                ctx.lineTo(s * 0.82, s * 0.44);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(c, s * 0.34);
                ctx.lineTo(c, s * 0.62);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(c, s * 0.62);
                ctx.lineTo(s * 0.28, s * 0.88);
                ctx.moveTo(c, s * 0.62);
                ctx.lineTo(s * 0.72, s * 0.88);
                ctx.stroke();
                break;
            }
            case "control-center": {
                // Two toggle pills with knobs (the macOS Control Center
                // silhouette): top knob left, bottom knob right.
                roundedRect(ctx, s * 0.10, s * 0.22, s * 0.80, s * 0.24, s * 0.12);
                ctx.stroke();
                ctx.beginPath();
                ctx.arc(s * 0.30, s * 0.34, s * 0.085, 0, Math.PI * 2);
                ctx.fill();
                roundedRect(ctx, s * 0.10, s * 0.54, s * 0.80, s * 0.24, s * 0.12);
                ctx.stroke();
                ctx.beginPath();
                ctx.arc(s * 0.70, s * 0.66, s * 0.085, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
            case "mission-control": {
                // A window-thumbnail grid (Mission Control is not a macOS
                // menu-bar item; this is our own mark).
                var pad = s * 0.16;
                var gap = s * 0.10;
                var cell = (s - 2 * pad - gap) / 2;
                for (var row = 0; row < 2; ++row) {
                    for (var col = 0; col < 2; ++col) {
                        roundedRect(ctx, pad + col * (cell + gap),
                                    pad + row * (cell + gap), cell, cell, s * 0.04);
                        ctx.fill();
                    }
                }
                break;
            }
            }
        }
    }
}
