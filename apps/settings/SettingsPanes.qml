// SPDX-License-Identifier: MIT
import QtQuick

// The Settings pane catalog (T-09.1a).
//
// One ordered list mirroring the macOS System Settings information
// architecture exactly (see docs/reference/System_Preferences.md): id, title,
// and the design-system `Icon` glyph name. A pane also carries a header-card
// description, but only panes whose macOS capture shows a header card render
// it (`headerPanes`); the Wave-1 settings-list panes start directly at their
// first group (Desktop & Dock, Displays, Wallpaper captures; Appearance is
// uncaptured and follows the same pattern).
//
// `shipped` is the no-half-panes gate: the sidebar lists a pane only when its
// content has landed and every control on it is functional. T-09.1a ships the
// shell and the four Wave-1 pane rows; T-09.2…T-09.5 flip their pane's
// `shipped` to true as the content lands (and give it a real body component in
// SettingsShell), so a row never appears before it works.
pragma Singleton

QtObject {
    id: root

    // Panes whose macOS reference has a header card (large icon/title plus a
    // description). From the captures: General, Accessibility, Notifications,
    // and Privacy & Security. Everything else opens at its first group.
    readonly property var headerPanes: [
        "general", "accessibility", "notifications", "privacy"
    ]

    function hasHeader(pane) {
        return pane !== null && root.headerPanes.indexOf(pane.id) >= 0;
    }

    // The category-tile table (T-19.1b): the one place a pane's existing
    // design-system `icon` identity maps to a Phosphor glyph and the gradient
    // the `SettingsCategoryIcon` container paints. The container geometry is
    // identical for every category (only the hue changes), so this is the only
    // place category colour lives; the literal hues are named constants here,
    // not baked into any SVG. A pane whose icon is absent (Trackpad has no
    // vendored Phosphor mark) falls back to the Tabler-backed `Icon.qml` glyph.
    readonly property var categoryStyles: ({
        "wifi":          { source: "wifi-high",     gradientStart: "#3f8cff", gradientEnd: "#1f5fd6" },
        "bluetooth":     { source: "bluetooth",     gradientStart: "#4a9bff", gradientEnd: "#2158d0" },
        "network":       { source: "globe",         gradientStart: "#3aa0ff", gradientEnd: "#1667c8" },
        "battery":       { source: "battery-full",  gradientStart: "#34c759", gradientEnd: "#14883a" },
        "storage":       { source: "hard-drives",   gradientStart: "#7d8fa6", gradientEnd: "#4a5b70" },
        "general":       { source: "gear",          gradientStart: "#8e8e93", gradientEnd: "#58585c" },
        "accessibility": { source: "person",        gradientStart: "#2f7bff", gradientEnd: "#1450c4" },
        "appearance":    { source: "swatches",      gradientStart: "#4a4a55", gradientEnd: "#1c1c24" },
        "dock":          { source: "desktop",       gradientStart: "#5a5a66", gradientEnd: "#26262e" },
        "overview":      { source: "squares-four",  gradientStart: "#17b3a3", gradientEnd: "#0b7a6f" },
        "displays":      { source: "monitor",       gradientStart: "#2e8dff", gradientEnd: "#1555c0" },
        "menu-bar":      { source: "list",          gradientStart: "#8a8f98", gradientEnd: "#4f545c" },
        "search":        { source: "magnifying-glass", gradientStart: "#3f8cff", gradientEnd: "#1f5fd6" },
        "wallpaper":     { source: "image",         gradientStart: "#46b6e6", gradientEnd: "#1f74b8" },
        "bell":          { source: "bell",          gradientStart: "#ff5a5f", gradientEnd: "#c62431" },
        "volume":        { source: "speaker-high",  gradientStart: "#ff5f6d", gradientEnd: "#c62a52" },
        "focus":         { source: "moon",          gradientStart: "#a06bff", gradientEnd: "#6236c4" },
        "lock":          { source: "lock",          gradientStart: "#5d8ef0", gradientEnd: "#2b4fa8" },
        "privacy":       { source: "shield-check",  gradientStart: "#3f7bff", gradientEnd: "#1a4fb8" },
        "users":         { source: "users",         gradientStart: "#6b7bff", gradientEnd: "#3a3fc0" },
        "keyboard":      { source: "keyboard",      gradientStart: "#8a8f98", gradientEnd: "#4f545c" },
        "mouse":         { source: "mouse",         gradientStart: "#8a8f98", gradientEnd: "#4f545c" },
        "printer":       { source: "printer",       gradientStart: "#5c8fd6", gradientEnd: "#2d559c" },
        "control-center": { source: "sliders-horizontal", gradientStart: "#6b7bff", gradientEnd: "#3a3fc0" },
        "input":         { source: "squares-four",  gradientStart: "#8a8f98", gradientEnd: "#4f545c" }
    })

    // The `{ source, gradientStart, gradientEnd }` tile style for a pane, keyed
    // by its existing `icon` identity; null means the caller keeps `Icon.qml`.
    function categoryStyle(pane) {
        if (pane === null)
            return null;
        var style = root.categoryStyles[pane.icon];
        return style === undefined ? null : style;
    }

    // The one ordered pane catalog (T222). Every entry mirrors
    // `SettingsMap.panes` one-for-one: id, label, icon, shipped flag, owner,
    // deep link, and absence kind. The map is the source of truth; the map
    // test fails on any drift and on a visible row with no owner. `storage`,
    // `mission-control`, and `menu-bar` are existing shipped panes the
    // reference groups inside General/Desktop & Dock/Quick Settings; they keep
    // their ids and top-level placement (T222 design note). The new panes use
    // the map names, with the T220 generic user-facing titles.
    readonly property var catalog: [
        { id: "wifi", title: qsTr("Wi-Fi"), icon: "wifi",
          description: "", shipped: false,
          owner: "NetworkManager", deepLink: "settings://pane/wifi",
          absence: "disabled-with-reason" },
        { id: "bluetooth", title: qsTr("Bluetooth"), icon: "bluetooth",
          description: "", shipped: true,
          owner: "BlueZ", deepLink: "settings://pane/bluetooth",
          absence: "disabled-with-reason" },
        { id: "network", title: qsTr("Network"), icon: "network",
          description: "", shipped: true,
          owner: "NetworkManager + firewall adapter",
          deepLink: "settings://pane/network",
          absence: "disabled-with-reason" },
        { id: "battery", title: qsTr("Battery"), icon: "battery",
          description: "", shipped: true,
          owner: "UPower + power-profiles-daemon",
          deepLink: "settings://pane/battery",
          absence: "disabled-with-reason" },
        { id: "general", title: qsTr("General"), icon: "general",
          description: qsTr("Manage your overall setup and preferences, such "
                            + "as software updates, your device details, and "
                            + "more."),
          shipped: true,
          owner: "settingsd + host/distro adapters",
          deepLink: "settings://pane/general",
          absence: "live-on-defaults" },
        { id: "storage", title: qsTr("Storage"), icon: "storage",
          description: "", shipped: true,
          owner: "UDisks2 + filesystem usage",
          deepLink: "settings://pane/storage",
          absence: "disabled-with-reason" },
        { id: "appearance", title: qsTr("Appearance"), icon: "appearance",
          description: "", shipped: true,
          owner: "settingsd + compositor",
          deepLink: "settings://pane/appearance",
          absence: "live-on-defaults" },
        { id: "accessibility", title: qsTr("Accessibility"), icon: "accessibility",
          description: qsTr("Personalize your system in ways that work best for "
                            + "you with accessibility features for vision, "
                            + "hearing, motor, speech, and cognition."),
          shipped: true,
          owner: "settingsd + compositor + AT-SPI",
          deepLink: "settings://pane/accessibility",
          absence: "live-on-defaults" },
        { id: "control-center", title: qsTr("Quick Settings"), icon: "control-center",
          description: "", shipped: false,
          owner: "settingsd + shell + adapters",
          deepLink: "settings://pane/control-center",
          absence: "live-on-defaults" },
        { id: "menu-bar", title: qsTr("Menu Bar"), icon: "menu-bar",
          description: "", shipped: true,
          owner: "settingsd -> shell", deepLink: "settings://pane/menu-bar",
          absence: "live-on-defaults" },
        { id: "search", title: qsTr("Search"), icon: "search",
          description: "", shipped: false,
          owner: "search service + app-index + settingsd",
          deepLink: "settings://pane/search",
          absence: "live-on-defaults" },
        { id: "privacy", title: qsTr("Privacy & Security"), icon: "privacy",
          description: qsTr("Control which apps can access your data, location, "
                            + "camera, and microphone, and manage safety "
                            + "protections."),
          shipped: true,
          owner: "portals + polkit + disk encryption + firewall adapter",
          deepLink: "settings://pane/privacy",
          absence: "disabled-with-reason" },
        { id: "desktop-dock", title: qsTr("Desktop & Dock"), icon: "dock",
          description: "", shipped: true,
          owner: "settingsd -> compositor/shell",
          deepLink: "settings://pane/desktop-dock",
          absence: "live-on-defaults" },
        { id: "mission-control", title: qsTr("Overview"), icon: "overview",
          description: "", shipped: true,
          owner: "settingsd + compositor",
          deepLink: "settings://pane/mission-control",
          absence: "live-on-defaults" },
        { id: "displays", title: qsTr("Displays"), icon: "displays",
          description: "", shipped: true,
          owner: "compositor + colord/display adapter",
          deepLink: "settings://pane/displays",
          absence: "live-on-defaults" },
        { id: "wallpaper", title: qsTr("Wallpaper"), icon: "wallpaper",
          description: "", shipped: true,
          owner: "settingsd + wallpaperd + compositor",
          deepLink: "settings://pane/wallpaper",
          absence: "live-on-defaults" },
        { id: "screen-saver", title: qsTr("Screen Saver"), icon: "displays",
          description: "", shipped: false,
          owner: "shell/compositor + settingsd",
          deepLink: "settings://pane/screen-saver",
          absence: "live-on-defaults" },
        { id: "notifications", title: qsTr("Notifications"), icon: "bell",
          description: qsTr("Customize when and how notifications appear, if "
                            + "they play a sound, and which apps can send them."),
          shipped: true,
          owner: "notification service + settingsd",
          deepLink: "settings://pane/notifications",
          absence: "live-on-defaults" },
        { id: "sound", title: qsTr("Sound"), icon: "volume",
          description: "", shipped: true,
          owner: "PipeWire/WirePlumber + settingsd",
          deepLink: "settings://pane/sound",
          absence: "live-on-defaults" },
        { id: "focus", title: qsTr("Focus"), icon: "focus",
          description: "", shipped: true,
          owner: "notification service + settingsd",
          deepLink: "settings://pane/focus",
          absence: "disabled-with-reason" },
        { id: "lock-screen", title: qsTr("Lock Screen"), icon: "lock",
          description: "", shipped: true,
          owner: "settingsd + compositor/lock + logind",
          deepLink: "settings://pane/lock-screen",
          absence: "live-on-defaults" },
        { id: "biometrics", title: qsTr("Biometrics & Password"), icon: "general",
          description: "", shipped: false,
          owner: "fprintd + PAM + distro adapter",
          deepLink: "settings://pane/biometrics",
          absence: "disabled-with-reason" },
        { id: "users-groups", title: qsTr("Users & Groups"), icon: "users",
          description: "", shipped: true,
          owner: "accountsservice + distro adapter + polkit",
          deepLink: "settings://pane/users-groups",
          absence: "read-only" },
        { id: "online-accounts", title: qsTr("Online Accounts"), icon: "network",
          description: "", shipped: false,
          owner: "Online Accounts service",
          deepLink: "settings://pane/online-accounts",
          absence: "delegated" },
        { id: "printers", title: qsTr("Printers & Scanners"), icon: "printer",
          description: "", shipped: true,
          owner: "CUPS + SANE", deepLink: "settings://pane/printers",
          absence: "disabled-with-reason" },
        { id: "keyboard", title: qsTr("Keyboard"), icon: "keyboard",
          description: "", shipped: true,
          owner: "settingsd + compositor input + ibus/fcitx5",
          deepLink: "settings://pane/keyboard",
          absence: "live-on-defaults" },
        { id: "mouse", title: qsTr("Mouse"), icon: "mouse",
          description: "", shipped: true,
          owner: "settingsd + compositor input",
          deepLink: "settings://pane/mouse",
          absence: "live-on-defaults" },
        { id: "trackpad", title: qsTr("Trackpad"), icon: "trackpad",
          description: "", shipped: true,
          owner: "settingsd + compositor input (libinput)",
          deepLink: "settings://pane/trackpad",
          absence: "live-on-defaults" },
        { id: "game-controllers", title: qsTr("Game Controllers"), icon: "input",
          description: "", shipped: false,
          owner: "udev/SDL", deepLink: "settings://pane/game-controllers",
          absence: "disabled-with-reason" }
    ]

    // Panes whose content has landed (the sidebar list).
    readonly property var shippedPanes: root.catalog.filter(function(pane) {
        return pane.shipped === true;
    })

    // Case-insensitive local search over the shipped pane titles, ids, and
    // descriptions.
    function filter(query) {
        var q = (query || "").trim().toLowerCase();
        if (q.length === 0)
            return root.shippedPanes;
        return root.shippedPanes.filter(function(pane) {
            return pane.title.toLowerCase().indexOf(q) >= 0
                    || pane.id.indexOf(q) >= 0
                    || pane.description.toLowerCase().indexOf(q) >= 0;
        });
    }

    function paneById(id) {
        for (var i = 0; i < root.catalog.length; ++i) {
            if (root.catalog[i].id === id)
                return root.catalog[i];
        }
        return null;
    }

    function indexOf(id) {
        for (var i = 0; i < root.shippedPanes.length; ++i) {
            if (root.shippedPanes[i].id === id)
                return i;
        }
        return -1;
    }
}