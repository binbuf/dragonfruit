// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The inherited original-geometry icon vocabulary, now backed by Tabler
// (ADR 0185). This component keeps the public API every Files/Settings/shell
// call site already uses (`name`, `color`, `size`) and maps each legacy name
// to a vendored Tabler outline glyph. New code should use `TablerIcon`
// directly; this facade exists so the migration did not touch every caller.
//
// The map is the single source of the compatibility vocabulary. A name that
// is not here resolves to nothing (and warns); `scripts/check-tabler-icons.py`
// fails the build for a literal name that is not mapped or whose Tabler target
// is not vendored. Dynamic names from the services/Files models are covered by
// the same map and by `TablerGlyphs.has()` in the design-system tests.
Item {
    id: root

    property string name: "check"
    property color color: Theme.color.textPrimary
    property real size: 16
    // Passed through to the Tabler outline; the default is Tabler's own
    // 2-unit stroke in the 24-unit viewBox. Tiny chrome marks (traffic lights)
    // raise this to stay legible at 6 px.
    property real strokeWidth: 2

    implicitWidth: size
    implicitHeight: size

    // Colored place artwork from the vendored WhiteSur set. These are complete
    // SVG resources, so they intentionally ignore `color`; the monochrome
    // interface vocabulary below stays on Tabler. Keep this map small and
    // explicit: a name here is the only way a legacy `Icon` call site gets a
    // non-tintable image.
    readonly property var placeIcons: ({
        "folder": "folder",
        "documents": "folder-documents",
        "downloads": "folder-download",
        "wallpaper": "folder-images",
        "music": "folder-music",
        "movies": "folder-videos",
        "public": "folder-public",
        "templates": "folder-templates",
        "home": "user-home",
        "displays": "user-desktop",
        "trash": "user-trash",
        "computer": "computer",
        "drive-harddisk": "drive-harddisk",
        "drive-removable-media": "drive-removable-media"
    })

    // Legacy name -> vendored Tabler outline name.
    readonly property var aliases: ({
        "close": "x",
        "minimize": "minus",
        "restore": "arrow-up",
        "zoom": "maximize",
        "check": "check",
        "chevron-down": "chevron-down",
        "chevron-right": "chevron-right",
        "chevron-left": "chevron-left",
        "search": "search",
        "appearance": "circle-half-2",
        "wallpaper": "photo",
        "dock": "layout-bottombar",
        "displays": "device-desktop",
        "home": "home",
        "documents": "file-text",
        "downloads": "download",
        "music": "music",
        "movies": "movie",
        "trash": "trash",
        "computer": "device-desktop",
        "volume": "volume",
        "volume-muted": "volume-off",
        "folder": "folder",
        "file": "file",
        "icon-view": "layout-grid",
        "list-view": "list",
        "column-view": "layout-bottombar",
        "gallery-view": "photo",
        "wifi": "wifi",
        "wifi-secure": "wifi",
        "wifi-disabled": "wifi-off",
        "wifi-off": "wifi-off",
        "wifi-connecting": "wifi",
        "wifi-error": "wifi-off",
        "bluetooth": "bluetooth",
        "bluetooth-disabled": "bluetooth-off",
        "brightness": "sun",
        "focus": "moon",
        "storage": "server",
        "keyboard": "keyboard",
        "mouse": "mouse",
        "trackpad": "pointer",
        "touch": "hand-finger",
        "tablet": "device-tablet",
        "input": "devices",
        "overview": "layout-dashboard",
        "battery": "battery",
        "power-saver": "leaf",
        "power-balanced": "scale",
        "power-performance": "bolt",
        "info": "info-circle",
        "bell": "bell",
        "lock": "lock",
        "menu-bar": "layout-navbar",
        "general": "settings",
        "software-update": "refresh",
        "users": "users",
        "printer": "printer",
        "scanner": "scan",
        "privacy": "shield-check",
        "accessibility": "accessible",
        "network": "world",
        "vpn": "shield-lock",
        "vpn-off": "shield-off",
        "application": "app-window",
        "drive-optical": "disc",
        "drive-removable-media": "usb",
        "drive-harddisk": "server",
        "control-center": "adjustments-horizontal",
        "mission-control": "layout-dashboard"
    })

    // The WhiteSur resource for the current legacy name, or "" when this is a
    // monochrome Tabler glyph. The Trash picks the dark artwork on a light
    // scheme and the light artwork on a dark scheme, matching WhiteSur's pair.
    readonly property string placeSource: {
        if (root.name === "trash")
            return "qrc:/icons/whitesur/" + (Theme.dark
                ? "user-trash.svg" : "user-trash-dark.svg");
        return root.placeIcons[root.name] !== undefined
            ? "qrc:/icons/whitesur/" + root.placeIcons[root.name] + ".svg" : "";
    }
    readonly property bool isPlaceIcon: root.placeSource.length > 0

    // The Tabler name for the current legacy name, or "" for an unknown one.
    readonly property string tablerName: root.aliases[root.name] !== undefined
                                         ? root.aliases[root.name] : ""
    readonly property bool resolved: root.isPlaceIcon
                                     || (root.tablerName.length > 0
                                         && TablerGlyphs.has("outline", root.tablerName))

    Image {
        anchors.fill: parent
        visible: root.isPlaceIcon
        source: root.placeSource
        sourceSize.width: Math.ceil(root.size * 2)
        sourceSize.height: Math.ceil(root.size * 2)
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        cache: true
        smooth: true
    }

    TablerIcon {
        anchors.fill: parent
        visible: !root.isPlaceIcon
        name: root.tablerName
        color: root.color
        size: root.size
        strokeWidth: root.strokeWidth
    }

    Component.onCompleted: {
        if (root.name.length > 0 && !root.resolved)
            console.warn("Icon: unknown legacy glyph \"" + root.name + "\"");
    }
}
