// SPDX-License-Identifier: MIT
import QtQuick

// The OS menu glyph: the single tintable mark the menu bar renders at its far
// left. `glyph` selects the source artwork:
//
//   * "dragonfruit" — the Dragonfruit brand mark (`DragonfruitLogo.qml`);
//   * "tux"         — the alternate penguin mark (`TuxLogo.qml`).
//
// Both children share one contract (`color`, `size`, `shadowColor`,
// `shadowOffset`) so the menu bar applies the same foreground, wallpaper
// contrast shadow, and hover/open plate to either choice. The slot stays
// square for both marks; Tux centres its narrower silhouette inside it.
Item {
    id: root

    property string glyph: "dragonfruit"
    property color color: Theme.color.textPrimary
    property real size: Theme.controls.menuBar.iconSize
    property color shadowColor: "transparent"
    property point shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)

    readonly property bool tux: root.glyph === "tux"

    implicitWidth: size
    implicitHeight: size

    DragonfruitLogo {
        visible: !root.tux
        anchors.centerIn: parent
        size: root.size
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
    }

    TuxLogo {
        visible: root.tux
        anchors.centerIn: parent
        size: root.size
        color: root.color
        shadowColor: root.shadowColor
        shadowOffset: root.shadowOffset
    }
}
