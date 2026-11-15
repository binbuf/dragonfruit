// SPDX-License-Identifier: MIT
import QtQuick

// The Settings pane catalog (T-09.1a).
//
// One ordered list mirroring the macOS System Settings information
// architecture exactly (see docs/reference/System_Preferences.md): id, title,
// the design-system `Icon` glyph name, and the sidebar group. A pane also
// carries a header-card description, but only panes whose macOS capture shows
// a header card render it (`headerPanes`); the Wave-1 settings-list panes start
// directly at their first group.
//
// `shipped` is the no-half-panes gate: the sidebar lists a pane only when its
// content has landed and every control on it is functional. A pane can also be
// `hidden` (Storage now lives inside General) or a navigation link (`linkTo`)
// whose optional host service gates its visibility (VPN). Both still keep
// their id and deep link; `resolve()` turns the link into the pane that opens.
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

    // The sidebar group order (T263). One ordered source list, grouped; the
    // catalog below carries the matching `group` field. The titles are the
    // macOS Tahoe group names.
    readonly property var groups: [
        { id: "connectivity", title: qsTr("Connectivity") },
        { id: "general", title: qsTr("General") },
        { id: "personalization", title: qsTr("Personalization") },
        { id: "desktop", title: qsTr("Desktop") },
        { id: "accounts-devices", title: qsTr("Accounts & Devices") },
        { id: "input", title: qsTr("Input") },
        { id: "system", title: qsTr("System") }
    ]

    function groupTitle(id) {
        for (var i = 0; i < root.groups.length; ++i) {
            if (root.groups[i].id === id)
                return root.groups[i].title;
        }
        return "";
    }

    // Turn the flat visible pane list into the sidebar's titled sections. The
    // order inside each section is the catalog order, so there is never a
    // second list to keep in sync.
    function grouped(panes) {
        var out = [];
        for (var g = 0; g < root.groups.length; ++g) {
            var group = root.groups[g];
            var items = panes.filter(function(pane) {
                return pane.group === group.id;
            });
            if (items.length > 0)
                out.push({ title: group.title, items: items });
        }
        return out;
    }

    // The category-tile table (T-19.1b): the one place a pane's existing
    // design-system `icon` identity maps to a Phosphor glyph and the gradient
    // the `SettingsCategoryIcon` container paints. The container geometry is
    // identical for every category (only the hue changes), so this is the only
    // place category colour lives; the literal hues are named constants here,
    // not baked into any SVG. Every shipped icon has a mapping (T263 removed
    // the shared General/Biometrics, Displays/Screen Saver, and
    // Network/Online Accounts identities).
    readonly property var categoryStyles: ({
        "wifi":          { source: "wifi-high",     gradientStart: "#3f8cff", gradientEnd: "#1f5fd6" },
        "bluetooth":     { source: "bluetooth",     gradientStart: "#4a9bff", gradientEnd: "#2158d0" },
        "network":       { source: "globe",         gradientStart: "#3aa0ff", gradientEnd: "#1667c8" },
        "vpn":           { source: "lock",          gradientStart: "#5d8ef0", gradientEnd: "#2b4fa8" },
        "battery":       { source: "battery-full",  gradientStart: "#34c759", gradientEnd: "#14883a" },
        "storage":       { source: "hard-drives",   gradientStart: "#7d8fa6", gradientEnd: "#4a5b70" },
        "general":       { source: "gear",          gradientStart: "#8e8e93", gradientEnd: "#58585c" },
        "accessibility": { source: "person",        gradientStart: "#2f7bff", gradientEnd: "#1450c4" },
        "appearance":    { source: "swatches",      gradientStart: "#4a4a55", gradientEnd: "#1c1c24" },
        "dock":          { source: "desktop",       gradientStart: "#5a5a66", gradientEnd: "#26262e" },
        "overview":      { source: "squares-four",  gradientStart: "#17b3a3", gradientEnd: "#0b7a6f" },
        "displays":      { source: "monitor",       gradientStart: "#2e8dff", gradientEnd: "#1555c0" },
        "screen-saver":  { source: "monitor-play",  gradientStart: "#3f8cff", gradientEnd: "#1f5fd6" },
        "menu-bar":      { source: "list",          gradientStart: "#8a8f98", gradientEnd: "#4f545c" },
        "search":        { source: "magnifying-glass", gradientStart: "#3f8cff", gradientEnd: "#1f5fd6" },
        "wallpaper":     { source: "image",         gradientStart: "#46b6e6", gradientEnd: "#1f74b8" },
        "bell":          { source: "bell",          gradientStart: "#ff5a5f", gradientEnd: "#c62431" },
        "volume":        { source: "speaker-high",  gradientStart: "#ff5f6d", gradientEnd: "#c62a52" },
        "focus":         { source: "moon",          gradientStart: "#a06bff", gradientEnd: "#6236c4" },
        "lock":          { source: "lock",          gradientStart: "#5d8ef0", gradientEnd: "#2b4fa8" },
        "privacy":       { source: "shield-check",  gradientStart: "#3f7bff", gradientEnd: "#1a4fb8" },
        "biometrics":    { source: "user",          gradientStart: "#b06bff", gradientEnd: "#6a36c4" },
        "users":         { source: "users",         gradientStart: "#6b7bff", gradientEnd: "#3a3fc0" },
        "online-accounts": { source: "circles-three", gradientStart: "#4ab6e6", gradientEnd: "#1f74b8" },
        "keyboard":      { source: "keyboard",      gradientStart: "#8a8f98", gradientEnd: "#4f545c" },
        "mouse":         { source: "mouse",         gradientStart: "#8a8f98", gradientEnd: "#4f545c" },
        "trackpad":      { source: "hand-pointing", gradientStart: "#8a8f98", gradientEnd: "#4f545c" },
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
    // `SettingsMap.panes` one-for-one: id, label, icon, group, shipped flag,
    // owner, deep link, and absence kind. The map is the source of truth; the
    // map test fails on any drift and on a visible row with no owner.
    //
    // T263 corrections: the order is the grouped sidebar order; `storage` is
    // kept as a shipped General sub-pane but hidden from the sidebar and
    // aliased to `general`; `online-accounts` is always present; and `vpn` is
    // a navigation link to Network that appears only when a VPN profile
    // exists (T272 turns it into a real pane).
    readonly property var catalog: [
        { id: "wifi", title: qsTr("Wi-Fi"), icon: "wifi", group: "connectivity",
          description: "", shipped: true,
          owner: "NetworkManager", deepLink: "settings://pane/wifi",
          absence: "disabled-with-reason" },
        { id: "bluetooth", title: qsTr("Bluetooth"), icon: "bluetooth", group: "connectivity",
          description: "", shipped: true,
          owner: "BlueZ", deepLink: "settings://pane/bluetooth",
          absence: "disabled-with-reason" },
        { id: "network", title: qsTr("Network"), icon: "network", group: "connectivity",
          description: "", shipped: true,
          owner: "NetworkManager + firewall adapter",
          deepLink: "settings://pane/network",
          absence: "disabled-with-reason" },
        { id: "vpn", title: qsTr("VPN"), icon: "vpn", group: "connectivity",
          description: "", shipped: false, optional: "vpn",
          linkTo: "network", anchor: "VPN",
          owner: "NetworkManager VPN", deepLink: "settings://pane/vpn",
          absence: "disabled-with-reason" },
        { id: "general", title: qsTr("General"), icon: "general", group: "general",
          description: qsTr("Manage your overall setup and preferences, such "
                            + "as software updates, your device details, and "
                            + "more."),
          shipped: true,
          owner: "settingsd + host/distro adapters",
          deepLink: "settings://pane/general",
          absence: "live-on-defaults" },
        { id: "storage", title: qsTr("Storage"), icon: "storage", group: "general",
          description: "", shipped: true, hidden: true,
          aliasOf: "general", subPane: "storage",
          owner: "UDisks2 + filesystem usage",
          deepLink: "settings://pane/storage",
          absence: "disabled-with-reason" },
        { id: "appearance", title: qsTr("Appearance"), icon: "appearance", group: "personalization",
          description: "", shipped: true,
          owner: "settingsd + compositor",
          deepLink: "settings://pane/appearance",
          absence: "live-on-defaults" },
        { id: "accessibility", title: qsTr("Accessibility"), icon: "accessibility", group: "personalization",
          description: qsTr("Personalize your system in ways that work best for "
                            + "you with accessibility features for vision, "
                            + "hearing, motor, speech, and cognition."),
          shipped: true,
          owner: "settingsd + compositor + AT-SPI",
          deepLink: "settings://pane/accessibility",
          absence: "live-on-defaults" },
        { id: "control-center", title: qsTr("Quick Settings"), icon: "control-center", group: "personalization",
          description: "", shipped: true,
          owner: "settingsd + shell + adapters",
          deepLink: "settings://pane/control-center",
          absence: "live-on-defaults" },
        { id: "search", title: qsTr("Search"), icon: "search", group: "personalization",
          description: "", shipped: true,
          owner: "search service + app-index + settingsd",
          deepLink: "settings://pane/search",
          absence: "live-on-defaults" },
        { id: "privacy", title: qsTr("Privacy & Security"), icon: "privacy", group: "personalization",
          description: qsTr("Control which apps can access your data, location, "
                            + "camera, and microphone, and manage safety "
                            + "protections."),
          shipped: true,
          owner: "portals + polkit + disk encryption + firewall adapter",
          deepLink: "settings://pane/privacy",
          absence: "disabled-with-reason" },
        { id: "desktop-dock", title: qsTr("Desktop & Dock"), icon: "dock", group: "desktop",
          description: "", shipped: true,
          owner: "settingsd -> compositor/shell",
          deepLink: "settings://pane/desktop-dock",
          absence: "live-on-defaults" },
        { id: "mission-control", title: qsTr("Overview"), icon: "overview", group: "desktop",
          description: "", shipped: true,
          owner: "settingsd + compositor",
          deepLink: "settings://pane/mission-control",
          absence: "live-on-defaults" },
        { id: "displays", title: qsTr("Displays"), icon: "displays", group: "desktop",
          description: "", shipped: true,
          owner: "compositor + colord/display adapter",
          deepLink: "settings://pane/displays",
          absence: "live-on-defaults" },
        { id: "wallpaper", title: qsTr("Wallpaper"), icon: "wallpaper", group: "desktop",
          description: "", shipped: true,
          owner: "settingsd + wallpaperd + compositor",
          deepLink: "settings://pane/wallpaper",
          absence: "live-on-defaults" },
        { id: "screen-saver", title: qsTr("Screen Saver"), icon: "screen-saver", group: "desktop",
          description: "", shipped: true,
          owner: "shell/compositor + settingsd",
          deepLink: "settings://pane/screen-saver",
          absence: "live-on-defaults" },
        { id: "notifications", title: qsTr("Notifications"), icon: "bell", group: "desktop",
          description: qsTr("Customize when and how notifications appear, if "
                            + "they play a sound, and which apps can send them."),
          shipped: true,
          owner: "notification service + settingsd",
          deepLink: "settings://pane/notifications",
          absence: "live-on-defaults" },
        { id: "sound", title: qsTr("Sound"), icon: "volume", group: "desktop",
          description: "", shipped: true,
          owner: "PipeWire/WirePlumber + settingsd",
          deepLink: "settings://pane/sound",
          absence: "live-on-defaults" },
        { id: "focus", title: qsTr("Focus"), icon: "focus", group: "desktop",
          description: "", shipped: true,
          owner: "notification service + settingsd",
          deepLink: "settings://pane/focus",
          absence: "disabled-with-reason" },
        { id: "lock-screen", title: qsTr("Lock Screen"), icon: "lock", group: "desktop",
          description: "", shipped: true,
          owner: "settingsd + compositor/lock + logind",
          deepLink: "settings://pane/lock-screen",
          absence: "live-on-defaults" },
        { id: "users-groups", title: qsTr("Users & Groups"), icon: "users", group: "accounts-devices",
          description: "", shipped: true,
          owner: "accountsservice + distro adapter + polkit",
          deepLink: "settings://pane/users-groups",
          absence: "read-only" },
        { id: "biometrics", title: qsTr("Biometrics & Password"), icon: "biometrics", group: "accounts-devices",
          description: "", shipped: true,
          owner: "fprintd + PAM + distro adapter",
          deepLink: "settings://pane/biometrics",
          absence: "disabled-with-reason" },
        { id: "online-accounts", title: qsTr("Online Accounts"), icon: "online-accounts", group: "accounts-devices",
          description: "", shipped: true,
          owner: "Online Accounts service",
          deepLink: "settings://pane/online-accounts",
          absence: "delegated" },
        { id: "game-controllers", title: qsTr("Game Controllers"), icon: "input", group: "accounts-devices",
          description: "", shipped: true,
          owner: "udev/SDL", deepLink: "settings://pane/game-controllers",
          absence: "disabled-with-reason" },
        { id: "printers", title: qsTr("Printers & Scanners"), icon: "printer", group: "accounts-devices",
          description: "", shipped: true,
          owner: "CUPS + SANE", deepLink: "settings://pane/printers",
          absence: "disabled-with-reason" },
        { id: "keyboard", title: qsTr("Keyboard"), icon: "keyboard", group: "input",
          description: "", shipped: true,
          owner: "settingsd + compositor input + ibus/fcitx5",
          deepLink: "settings://pane/keyboard",
          absence: "live-on-defaults" },
        { id: "mouse", title: qsTr("Mouse"), icon: "mouse", group: "input",
          description: "", shipped: true,
          owner: "settingsd + compositor input",
          deepLink: "settings://pane/mouse",
          absence: "live-on-defaults" },
        { id: "trackpad", title: qsTr("Trackpad"), icon: "trackpad", group: "input",
          description: "", shipped: true,
          owner: "settingsd + compositor input (libinput)",
          deepLink: "settings://pane/trackpad",
          absence: "live-on-defaults" },
        { id: "battery", title: qsTr("Battery"), icon: "battery", group: "system",
          description: "", shipped: true,
          owner: "UPower + power-profiles-daemon",
          deepLink: "settings://pane/battery",
          absence: "disabled-with-reason" },
        { id: "menu-bar", title: qsTr("Menu Bar"), icon: "menu-bar", group: "system",
          description: "", shipped: true,
          owner: "settingsd -> shell", deepLink: "settings://pane/menu-bar",
          absence: "live-on-defaults" }
    ]

    // Whether an optional catalog entry is visible on this host. VPN is the
    // only optional entry today: it appears in Connectivity only when
    // NetworkManager reports at least one VPN profile; otherwise Network -> VPN
    // remains the only entry point. The view is a parameter so the rule is
    // testable without a live adapter.
    function optionalVisible(pane, vpnView) {
        if (pane === null || pane.optional === undefined)
            return true;
        if (pane.optional === "vpn") {
            if (vpnView === undefined || vpnView === null
                    || vpnView.connections === undefined)
                return false;
            return vpnView.connections.length > 0;
        }
        return true;
    }

    // The visible sidebar entries: shipped panes (Storage is hidden) plus the
    // optional navigation links whose host state is present. An optional pane
    // is never hidden just because its host is absent; Online Accounts keeps
    // its absent-service state (T279).
    readonly property var shippedPanes: root.catalog.filter(function(pane) {
        if (pane.hidden === true)
            return false;
        if (pane.shipped === true || pane.linkTo !== undefined)
            return root.optionalVisible(pane, Settings.vpn);
        return false;
    })

    // Resolve a pane id or link id to the pane the shell opens. Storage opens
    // General with its Storage sub-pane; VPN opens Network at its VPN group.
    function resolve(id) {
        var pane = root.paneById(id);
        if (pane === null)
            return null;
        if (pane.aliasOf !== undefined)
            return { paneId: pane.aliasOf, subPaneId: pane.subPane || "", anchor: "" };
        if (pane.linkTo !== undefined)
            return { paneId: pane.linkTo, subPaneId: "", anchor: pane.anchor || "" };
        return { paneId: id, subPaneId: "", anchor: "" };
    }

    // Whether a pane id (or alias) lands on a visible pane. A navigation link
    // with an optional host (VPN) must itself be visible; a hidden alias
    // (Storage) follows its target pane.
    function isVisible(id) {
        var pane = root.paneById(id);
        if (pane !== null && pane.linkTo !== undefined)
            return root.indexOf(id) >= 0;
        var target = root.resolve(id);
        return target !== null && root.indexOf(target.paneId) >= 0;
    }

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
