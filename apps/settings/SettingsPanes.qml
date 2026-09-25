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

    readonly property var catalog: [
        { id: "wifi", title: qsTr("Wi-Fi"), icon: "wifi",
          description: "", shipped: false },
        { id: "bluetooth", title: qsTr("Bluetooth"), icon: "bluetooth",
          description: "", shipped: false },
        { id: "network", title: qsTr("Network"), icon: "network",
          description: "", shipped: false },
        { id: "battery", title: qsTr("Battery"), icon: "battery",
          description: "", shipped: false },
        { id: "general", title: qsTr("General"), icon: "general",
          description: "", shipped: false },
        { id: "accessibility", title: qsTr("Accessibility"), icon: "accessibility",
          description: "", shipped: false },
        { id: "appearance", title: qsTr("Appearance"), icon: "appearance",
          description: "", shipped: true },
        { id: "desktop-dock", title: qsTr("Desktop & Dock"), icon: "dock",
          description: "", shipped: true },
        { id: "displays", title: qsTr("Displays"), icon: "displays",
          description: "", shipped: true },
        { id: "menu-bar", title: qsTr("Menu Bar"), icon: "general",
          description: "", shipped: false },
        { id: "spotlight", title: qsTr("Spotlight"), icon: "search",
          description: "", shipped: false },
        { id: "wallpaper", title: qsTr("Wallpaper"), icon: "wallpaper",
          description: "", shipped: true },
        { id: "notifications", title: qsTr("Notifications"), icon: "general",
          description: "", shipped: false },
        { id: "sound", title: qsTr("Sound"), icon: "general",
          description: "", shipped: false },
        { id: "focus", title: qsTr("Focus"), icon: "general",
          description: "", shipped: false },
        { id: "screen-time", title: qsTr("Screen Time"), icon: "general",
          description: "", shipped: false },
        { id: "lock-screen", title: qsTr("Lock Screen"), icon: "general",
          description: "", shipped: false },
        { id: "privacy", title: qsTr("Privacy & Security"), icon: "general",
          description: "", shipped: false },
        { id: "biometrics", title: qsTr("Biometrics & Password"), icon: "general",
          description: "", shipped: false },
        { id: "users-groups", title: qsTr("Users & Groups"), icon: "general",
          description: "", shipped: false },
        { id: "internet-accounts", title: qsTr("Internet Accounts"), icon: "general",
          description: "", shipped: false },
        { id: "keyboard", title: qsTr("Keyboard"), icon: "general",
          description: "", shipped: false },
        { id: "mouse", title: qsTr("Mouse"), icon: "general",
          description: "", shipped: false },
        { id: "trackpad", title: qsTr("Trackpad"), icon: "general",
          description: "", shipped: false },
        { id: "printers", title: qsTr("Printers & Scanners"), icon: "general",
          description: "", shipped: false }
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