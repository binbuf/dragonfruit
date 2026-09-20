// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// Original vector glyphs drawn from the design-system palette. No bitmap
// assets: every mark here is geometry we own (see 14-risks.md, IP rule).
Item {
    id: root

    property string name: "check"
    property color color: Theme.color.textPrimary
    property real size: 16

    implicitWidth: size
    implicitHeight: size

    readonly property real stroke: Math.max(1.4, size * 0.14)

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
}
