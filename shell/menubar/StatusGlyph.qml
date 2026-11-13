// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Menu-bar status marks. Every state renders as a plain monochrome Tabler
// outline glyph through `TablerIcon` — no tile, no gradient, no container.
// The public API is unchanged from the original-geometry version (`name`,
// `color`, `size`, `level`, `backgroundColor`), so `StatusItem`, `WifiMenu`,
// `VolumeMenu`, and `BatteryMenu` need no changes and the slot footprint
// (`implicitWidth`/`implicitHeight` == `size`) is identical, so the bar layout
// does not shift.
//
// State -> Tabler glyph:
//   wifi / wifi-secure      -> wifi
//   wifi-off / wifi-disabled-> wifi-off       (dimmed to 0.4 as before)
//   wifi-connecting         -> wifi           (dimmed to 0.5; static, so it is
//                                              reduced-motion safe)
//   wifi-error              -> wifi-off
//   bluetooth               -> bluetooth
//   volume                  -> volume
//   volume-muted            -> volume-off
//   battery                 -> battery        (outline + a token level fill
//                                              overlay inside the cell, so the
//                                              level stays continuous)
//   battery-charging        -> battery-charging
//   focus                   -> moon
//   accessibility           -> accessible
//   search                  -> search
//   notifications           -> bell
//   overflow                -> chevron-down
//   application             -> app-window (a tray item without a themed icon)
//   control-center          -> adjustments-horizontal
//   mission-control         -> layout-dashboard
//
// The Dragonfruit system-menu logo is untouched (`DragonfruitLogo.qml`).
Item {
    id: root

    property string name: "wifi"
    property color color: Theme.color.textPrimary
    // Optional wallpaper contrast shadow, forwarded to the active TablerIcon.
    // The menu bar supplies the opposite-tone pair; other callers leave it
    // transparent.
    property color shadowColor: "transparent"
    property point shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)
    // Retained for API compatibility with the original geometry (the Focus
    // crescent used it for its bite). Tabler's moon is a self-contained path,
    // so the mark is drawn without a background cut-out.
    property color backgroundColor: Theme.color.chrome
    property real size: Theme.controls.menuBar.iconSize
    // Battery level, 0..1.
    property real level: 0.8

    implicitWidth: size
    implicitHeight: size

    readonly property real clampedLevel: Math.max(0, Math.min(1, level))

    // The Tabler glyph for the current state. Kept in one place so the mapping
    // is legible; an unknown state resolves to "" and renders blank (the same
    // contract as TablerIcon's unknown name).
    readonly property string glyphName: {
        switch (root.name) {
        case "wifi":
        case "wifi-secure":
        case "wifi-connecting":
            return "wifi";
        case "wifi-off":
        case "wifi-disabled":
        case "wifi-error":
            return "wifi-off";
        case "bluetooth":
            return "bluetooth";
        case "volume":
            return "volume";
        case "volume-muted":
            return "volume-off";
        case "battery":
            return "battery";
        case "battery-charging":
            return "battery-charging";
        case "focus":
            return "moon";
        case "accessibility":
            return "accessible";
        case "search":
            return "search";
        case "notifications":
            return "bell";
        case "overflow":
            return "chevron-down";
        case "application":
            return "app-window";
        case "control-center":
            return "adjustments-horizontal";
        case "mission-control":
            return "layout-dashboard";
        }
        return "";
    }

    // Off/disabled dims to 0.4 (as the original geometry did); connecting is
    // half-lit, replacing the original pulse with a legible static state.
    readonly property real markOpacity: (root.name === "wifi-off"
                                          || root.name === "wifi-disabled") ? 0.4
                                       : (root.name === "wifi-connecting") ? 0.5
                                       : 1.0

    // Tabler's battery viewBox is 24; the outline's cell interior spans
    // roughly x 6.8..16.8 and y 9.5..14.5, so the continuous level fill
    // follows that interior (a plain token fill overlay inside the outline).
    readonly property real batteryCellX: 6.8
    readonly property real batteryCellY: 9.5
    readonly property real batteryCellWidth: 10.0
    readonly property real batteryCellHeight: 5.0
    readonly property real batteryCellRadius: 1.0

    // The marks. One literal TablerIcon per glyph (so the vendored-name gate
    // `scripts/check-tabler-icons.py` sees every name); only the active one is
    // visible. Battery is composed from an outline plus a fill below.
    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "wifi"
        name: "wifi"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "wifi-off"
        name: "wifi-off"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "bluetooth"
        name: "bluetooth"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "volume"
        name: "volume"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "volume-off"
        name: "volume-off"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "moon"
        name: "moon"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "accessible"
        name: "accessible"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "search"
        name: "search"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "app-window"
        name: "app-window"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "bell"
        name: "bell"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "chevron-down"
        name: "chevron-down"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "adjustments-horizontal"
        name: "adjustments-horizontal"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "layout-dashboard"
        name: "layout-dashboard"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    // Continuous battery: the Tabler outline with a token level fill overlay
    // inside the cell. The fill is a plain Rectangle (the Shape renderer's
    // ancestor clipping is not applied to a child Shape under the software
    // scene graph the tests use), so it clips by its own geometry.
    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "battery"
        name: "battery"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }

    Rectangle {
        visible: root.glyphName === "battery"
        x: root.size * root.batteryCellX / TablerGlyphs.viewBox
        y: root.size * root.batteryCellY / TablerGlyphs.viewBox
        width: root.size * root.batteryCellWidth * root.clampedLevel
               / TablerGlyphs.viewBox
        height: root.size * root.batteryCellHeight / TablerGlyphs.viewBox
        radius: root.size * root.batteryCellRadius / TablerGlyphs.viewBox
        color: root.color
        opacity: root.markOpacity
        antialiasing: true
    }

    TablerIcon {
        anchors.fill: parent
        visible: root.glyphName === "battery-charging"
        name: "battery-charging"
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
        size: root.size
        strokeWidth: 2.25
        opacity: root.markOpacity
    }
}
