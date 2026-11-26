// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick

// The Settings app's published native menu model (T-09.6a).
//
// This singleton is the *single source* of the app's menus. It is written in
// the design system's normalized entry shape (the same shape the
// `MenuBarMenu` component renders and re-emits as `menuModel`), so the shell's
// menu bar consumes it without translation: the model is injected as
// `applicationMenuItems` (the fixed application menu) and `appMenuModel` (the
// app's own top-level menus) and round-trips to the exact rows the user sees.
//
// Until the menu-broker (T-14.2a) resolves the focused app's model over the
// native publication channel, the shell keeps using the fixed application
// menu, so this model is deliberately fixed (no live enable/disable state) —
// the scope note for T-09.6a. The shape is the contract T-14 consumes.
//
// Entry shape (per design-system `MenuBarMenu`):
//   { label, shortcut, type: "item"|"separator"|"submenu",
//     enabled, checked, checkable, keepOpen, submenu: [...] }
// An entry may carry an `action` string; `activate()` is the dispatch seam.
//
// `publishedModel` is JSON-serializable and free of QML objects, so it can be
// carried over an IPC boundary unchanged.
pragma Singleton

QtObject {
    id: root

    readonly property string appName: qsTr("Settings")

    // Live navigation state, bound by `SettingsShell`. The View menu's
    // Back/Forward rows and the checked pane row read these; the model stays
    // one JSON-serializable object, not a second menu list.
    property string currentPaneId: ""
    property bool canGoBack: false
    property bool canGoForward: false
    // The Help rows disable with a reason when the map document is absent.
    readonly property bool helpAvailable: Settings.mapDocUrl.toString().length > 0
    readonly property string helpUnavailableReason:
        qsTr("The business-logic map is not installed.")

    // The fixed application menu (About/Settings/Hide/Hide Others/Show All/
    // Quit). The shell own this menu until T-14.2a takes it over, but the app
    // publishes the same items so `publishedModel` is the complete menu.
    readonly property var applicationMenuItems: [
        { label: qsTr("About %1").arg(root.appName), action: "about" },
        { label: qsTr("Settings\u2026"), shortcut: "Super+,", action: "settings" },
        { type: "separator" },
        { label: qsTr("Hide %1").arg(root.appName), shortcut: "Super+H", action: "hide" },
        { label: qsTr("Hide Others"), shortcut: "Super+Alt+H", action: "hide-others" },
        { label: qsTr("Show All"), action: "show-all" },
        { type: "separator" },
        { label: qsTr("Quit %1").arg(root.appName), shortcut: "Super+Q", action: "quit" }
    ]

    // The View -> Settings Pane submenu is generated from the one catalog:
    // every shipped pane, in sidebar order, with the current pane checked.
    readonly property var paneItems: SettingsPanes.shippedPanes.map(function(pane) {
        return {
            label: pane.title,
            action: "pane." + pane.id,
            checked: pane.id === root.currentPaneId,
            checkable: true
        };
    })

    // The app's own top-level menus, left to right after the fixed two.
    readonly property var menus: [
        {
            title: qsTr("File"),
            items: [
                { label: qsTr("Close Window"), shortcut: "Super+W", action: "close" }
            ]
        },
        {
            title: qsTr("Edit"),
            items: [
                { label: qsTr("Undo"), shortcut: "Super+Z", action: "edit.undo" },
                { label: qsTr("Redo"), shortcut: "Super+Shift+Z", action: "edit.redo" },
                { type: "separator" },
                { label: qsTr("Cut"), shortcut: "Super+X", action: "edit.cut" },
                { label: qsTr("Copy"), shortcut: "Super+C", action: "edit.copy" },
                { label: qsTr("Paste"), shortcut: "Super+V", action: "edit.paste" },
                { label: qsTr("Select All"), shortcut: "Super+A", action: "edit.select-all" }
            ]
        },
        {
            title: qsTr("View"),
            items: [
                { label: qsTr("Back"), shortcut: "Alt+Left", action: "back",
                  enabled: root.canGoBack },
                { label: qsTr("Forward"), shortcut: "Alt+Right", action: "forward",
                  enabled: root.canGoForward },
                { type: "separator" },
                { label: qsTr("Enter Full Screen"), shortcut: "Ctrl+Super+F",
                  action: "fullscreen" },
                { type: "separator" },
                { label: qsTr("Settings Pane"), type: "submenu",
                  submenu: root.paneItems }
            ]
        },
        {
            title: qsTr("Window"),
            items: [
                { label: qsTr("Minimize"), shortcut: "Super+M", action: "minimize" },
                { label: qsTr("Zoom"), action: "zoom" },
                { type: "separator" },
                { label: qsTr("Bring All to Front"), action: "bring-all-to-front" }
            ]
        },
        {
            title: qsTr("Help"),
            items: [
                { label: qsTr("%1 Help").arg(root.appName), action: "help",
                  enabled: root.helpAvailable },
                { type: "separator" },
                { label: qsTr("Pane & Row Owners…"), action: "help.debug" },
                { label: qsTr("Business-Logic Map"), action: "help.map",
                  enabled: root.helpAvailable }
            ]
        }
    ]

    // The complete, JSON-serializable publication. This is what the
    // menu-broker (T-14.2a) resolves for the focused Settings window.
    readonly property var publishedModel: {
        return {
            appName: root.appName,
            applicationMenuItems: root.applicationMenuItems,
            menus: root.menus
        };
    }

    // T230: publish the native model to the menu-broker under the canonical
    // desktop id the shell resolves focus to. `MenuPublisher` is a context
    // property in the app executable; the QML test has no broker and skips it.
    function publish() {
        if (typeof MenuPublisher !== "undefined")
            MenuPublisher.publish("org.dragonfruit.Settings.desktop",
                                  JSON.stringify(root.publishedModel));
    }

    Component.onCompleted: root.publish()
    // The View submenu's checked pane and Back/Forward rows are live state;
    // republish when they change so the global menu bar stays truthful.
    onMenusChanged: root.publish()
    Component.onDestruction: {
        if (typeof MenuPublisher !== "undefined")
            MenuPublisher.withdraw("org.dragonfruit.Settings.desktop");
    }

    // Raised when a published item is activated. The app wires its own window
    // actions to this (T-09.6a); the broker routes the same action back to the
    // app once it owns dispatch (T-14.2b).
    signal activated(string action, var item)

    readonly property var editActions: [
        "edit.undo", "edit.redo", "edit.cut", "edit.copy", "edit.paste",
        "edit.select-all"
    ]
    readonly property var windowActions: [
        "about", "settings", "hide", "hide-others", "show-all", "quit",
        "close", "minimize", "zoom", "fullscreen", "back", "forward",
        "bring-all-to-front", "help", "help.debug", "help.map"
    ]

    // Whether a published action names something the app can dispatch. An
    // unknown action is rejected by `activate()` instead of being broadcast.
    function isKnownAction(action) {
        if (!action || action.length === 0)
            return false;
        if (root.windowActions.indexOf(action) >= 0)
            return true;
        if (root.editActions.indexOf(action) >= 0)
            return true;
        if (action.indexOf("pane.") === 0)
            return SettingsPanes.indexOf(action.substring(5)) >= 0;
        return false;
    }

    // Dispatch one row. Returns whether the action was handled locally; the
    // broker treats an unhandled action as inert rather than guessing.
    function activate(action, item) {
        if (!root.isKnownAction(action))
            return false;
        root.activated(action, item !== undefined ? item : ({}));
        return true;
    }

    // The action for a row, following the `MenuBarMenu` `triggered` contract
    // (menuIndex into the app's own menus, itemIndex into that menu's items).
    function actionFor(menuIndex, itemIndex) {
        if (menuIndex < 0 || menuIndex >= root.menus.length)
            return "";
        var items = root.menus[menuIndex].items || [];
        if (itemIndex < 0 || itemIndex >= items.length)
            return "";
        return items[itemIndex].action || "";
    }
}