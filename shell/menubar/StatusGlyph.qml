// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Original vector glyphs for menu-bar status items (T-09 FR-5). Like the
// design-system Icon, every mark is geometry we own — no bitmap assets.
// Drawn with the canvas 2D API so arcs (Wi-Fi, volume) and curves stay
// crisp at any status-item size.
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

    readonly property real stroke: Math.max(1.2, size * 0.09)

    onNameChanged: canvas.requestPaint()
    onColorChanged: canvas.requestPaint()
    onSizeChanged: canvas.requestPaint()
    onLevelChanged: canvas.requestPaint()
    Component.onCompleted: canvas.requestPaint()

    Canvas {
        id: canvas
        anchors.fill: parent
        // Paint synchronously so a one-shot offscreen grab (the shell's
        // shm render and the visual tests) always sees the glyph.
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

            switch (root.name) {
            case "wifi": {
                // Three arcs opening downward plus the origin dot.
                var cy = s * 0.82;
                var radii = [s * 0.18, s * 0.34, s * 0.50];
                for (var i = 0; i < radii.length; ++i) {
                    ctx.beginPath();
                    ctx.arc(c, cy, radii[i], Math.PI * 1.16, Math.PI * 1.84);
                    ctx.stroke();
                }
                ctx.beginPath();
                ctx.arc(c, cy, root.stroke * 0.9, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
            case "bluetooth": {
                ctx.beginPath();
                ctx.moveTo(c, s * 0.14);
                ctx.lineTo(c, s * 0.88);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(c, s * 0.14);
                ctx.lineTo(s * 0.72, s * 0.36);
                ctx.lineTo(c, s * 0.52);
                ctx.lineTo(s * 0.72, s * 0.68);
                ctx.lineTo(c, s * 0.88);
                ctx.stroke();
                break;
            }
            case "volume":
            case "volume-muted": {
                // Speaker cone.
                ctx.beginPath();
                ctx.moveTo(s * 0.16, s * 0.40);
                ctx.lineTo(s * 0.32, s * 0.40);
                ctx.lineTo(s * 0.50, s * 0.22);
                ctx.lineTo(s * 0.50, s * 0.78);
                ctx.lineTo(s * 0.32, s * 0.60);
                ctx.lineTo(s * 0.16, s * 0.60);
                ctx.closePath();
                ctx.fill();
                if (root.name === "volume") {
                    ctx.beginPath();
                    ctx.arc(s * 0.54, s * 0.50, s * 0.16, -Math.PI * 0.36, Math.PI * 0.36);
                    ctx.stroke();
                    ctx.beginPath();
                    ctx.arc(s * 0.54, s * 0.50, s * 0.30, -Math.PI * 0.32, Math.PI * 0.32);
                    ctx.stroke();
                } else {
                    ctx.beginPath();
                    ctx.moveTo(s * 0.62, s * 0.38);
                    ctx.lineTo(s * 0.86, s * 0.62);
                    ctx.moveTo(s * 0.86, s * 0.38);
                    ctx.lineTo(s * 0.62, s * 0.62);
                    ctx.stroke();
                }
                break;
            }
            case "battery": {
                ctx.beginPath();
                ctx.rect(s * 0.08, s * 0.30, s * 0.68, s * 0.40);
                ctx.stroke();
                ctx.beginPath();
                ctx.rect(s * 0.80, s * 0.42, s * 0.08, s * 0.16);
                ctx.fill();
                var fillW = s * 0.60 * Math.max(0, Math.min(1, root.level));
                if (fillW > 0) {
                    ctx.beginPath();
                    ctx.rect(s * 0.12, s * 0.34, fillW, s * 0.32);
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
                ctx.arc(s * 0.68, s * 0.36, s * 0.32, 0, Math.PI * 2);
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
                ctx.beginPath();
                ctx.moveTo(s * 0.14, s * 0.34);
                ctx.lineTo(s * 0.86, s * 0.34);
                ctx.moveTo(s * 0.14, s * 0.66);
                ctx.lineTo(s * 0.86, s * 0.66);
                ctx.stroke();
                ctx.beginPath();
                ctx.arc(s * 0.64, s * 0.34, s * 0.11, 0, Math.PI * 2);
                ctx.fill();
                ctx.beginPath();
                ctx.arc(s * 0.36, s * 0.66, s * 0.11, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
            case "mission-control": {
                var pad = s * 0.16;
                var gap = s * 0.10;
                var cell = (s - 2 * pad - gap) / 2;
                for (var row = 0; row < 2; ++row) {
                    for (var col = 0; col < 2; ++col) {
                        ctx.fillRect(pad + col * (cell + gap),
                                     pad + row * (cell + gap),
                                     cell, cell);
                    }
                }
                break;
            }
            }
        }
    }
}
