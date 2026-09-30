// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Menu-bar status marks (T-19.1c). Every state renders as a plain monochrome
// Phosphor glyph through `PhosphorIcon` — no tile, no gradient, no container.
// The public API is unchanged from the original-geometry version (`name`,
// `color`, `size`, `level`, `backgroundColor`), so `StatusItem`, `WifiMenu`,
// `VolumeMenu`, and `BatteryMenu` need no changes and the slot footprint
// (`implicitWidth`/`implicitHeight` == `size`) is identical, so the bar layout
// does not shift.
//
// State -> Phosphor glyph (weight chosen for the mark's optical weight):
//   wifi / wifi-secure      -> wifi-high       (fill; no per-network strength
//                                                is carried by the mark)
//   wifi-off / wifi-disabled-> wifi-slash      (fill; dimmed to 0.4 as before)
//   wifi-connecting         -> wifi-high       (fill; dimmed to 0.5; static,
//                                                so it is reduced-motion safe)
//   wifi-error              -> wifi-x          (fill)
//   bluetooth               -> bluetooth       (fill)
//   volume                  -> speaker-high    (fill)
//   volume-muted            -> speaker-x       (fill)
//   battery                 -> battery-empty   (regular outline) + a token
//                                                level fill overlay inside the
//                                                cell, so the level is
//                                                continuous
//   battery-charging        -> battery-charging(regular outline + bolt; no
//                                                level fill, as before)
//   focus                   -> moon            (fill)
//   accessibility           -> person          (fill)
//   control-center          -> sliders-horizontal (fill)
//   mission-control         -> squares-four    (fill)
//
// The Dragonfruit system-menu logo is untouched (`DragonfruitLogo.qml`).
Item {
    id: root

    property string name: "wifi"
    property color color: Theme.color.textPrimary
    // Retained for API compatibility with the original geometry (the Focus
    // crescent used it for its bite). Phosphor's moon is a self-contained
    // path, so the mark is drawn without a background cut-out.
    property color backgroundColor: Theme.color.chrome
    property real size: Theme.controls.menuBar.iconSize
    // Battery level, 0..1.
    property real level: 0.8

    implicitWidth: size
    implicitHeight: size

    readonly property real clampedLevel: Math.max(0, Math.min(1, level))

    // The Phosphor glyph for the current state. Kept in one place so the
    // mapping is legible; an unknown state resolves to "" and renders blank
    // (the same contract as PhosphorIcon's unknown name).
    readonly property string glyphName: {
        switch (root.name) {
        case "wifi":
        case "wifi-secure":
        case "wifi-connecting":
            return "wifi-high";
        case "wifi-off":
        case "wifi-disabled":
            return "wifi-slash";
        case "wifi-error":
            return "wifi-x";
        case "bluetooth":
            return "bluetooth";
        case "volume":
            return "speaker-high";
        case "volume-muted":
            return "speaker-x";
        case "battery":
            return "battery-empty";
        case "battery-charging":
            return "battery-charging";
        case "focus":
            return "moon";
        case "accessibility":
            return "person";
        case "control-center":
            return "sliders-horizontal";
        case "mission-control":
            return "squares-four";
        }
        return "";
    }

    // Off/disabled dims to 0.4 (as the original geometry did); connecting is
    // half-lit, replacing the original pulse with a legible static state.
    readonly property real markOpacity: (root.name === "wifi-off"
                                          || root.name === "wifi-disabled") ? 0.4
                                       : (root.name === "wifi-connecting") ? 0.5
                                       : 1.0

    // Phosphor's battery viewBox is 256; `battery-full`'s solid interior spans
    // x 40..192 and y 88..168, so the continuous level fill follows that
    // interior (a plain token fill overlay inside the Phosphor outline).
    readonly property real batteryCellX: 40
    readonly property real batteryCellY: 88
    readonly property real batteryCellWidth: 152
    readonly property real batteryCellHeight: 80
    readonly property real batteryCellRadius: 8

    // The marks. One literal PhosphorIcon per glyph (so the vendored-name gate
    // `scripts/check-phosphor-icons.py` sees every name); only the active one
    // is visible. Battery is composed from an outline plus a fill below.
    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "wifi-high"
        name: "wifi-high"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "wifi-slash"
        name: "wifi-slash"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "wifi-x"
        name: "wifi-x"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "bluetooth"
        name: "bluetooth"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "speaker-high"
        name: "speaker-high"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "speaker-x"
        name: "speaker-x"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "moon"
        name: "moon"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "person"
        name: "person"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "sliders-horizontal"
        name: "sliders-horizontal"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "squares-four"
        name: "squares-four"
        color: root.color
        size: root.size
        weight: "fill"
        opacity: root.markOpacity
    }

    // Continuous battery: the Phosphor outline with a token level fill
    // overlay inside the cell. The fill is a plain Rectangle (the Shape
    // renderer's ancestor clipping is not applied to a child Shape under the
    // software scene graph the tests use), so it clips by its own geometry.
    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "battery-empty"
        name: "battery-empty"
        color: root.color
        size: root.size
        opacity: root.markOpacity
    }

    Rectangle {
        visible: root.glyphName === "battery-empty"
        x: root.size * root.batteryCellX / PhosphorGlyphs.viewBox
        y: root.size * root.batteryCellY / PhosphorGlyphs.viewBox
        width: root.size * root.batteryCellWidth * root.clampedLevel
               / PhosphorGlyphs.viewBox
        height: root.size * root.batteryCellHeight / PhosphorGlyphs.viewBox
        radius: root.size * root.batteryCellRadius / PhosphorGlyphs.viewBox
        color: root.color
        opacity: root.markOpacity
        antialiasing: true
    }

    PhosphorIcon {
        anchors.fill: parent
        visible: root.glyphName === "battery-charging"
        name: "battery-charging"
        color: root.color
        size: root.size
        opacity: root.markOpacity
    }
}