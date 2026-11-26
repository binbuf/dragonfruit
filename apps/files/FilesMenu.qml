// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick

// The Files app's published native menu model (T334).
//
// This singleton is the one source of the window's menus. `FilesShell` binds
// its live selection/history state here and exposes `FilesMenu.publishedModel`
// as the window's `menuModel`; the menu-broker publishes that same object, so
// the global bar and any local consumer cannot drift into a second copy.
//
// The shape is the design-system entry shape (ADR 0041):
//   { label, shortcut, type: "item"|"separator"|"submenu",
//     enabled, checked, checkable, keepOpen, disabledReason, submenu: [...] }
// Every row either names a real action or is disabled with a reason.
pragma Singleton

QtObject {
    id: root

    readonly property string appName: qsTr("Files")

    // Live state, bound by `FilesShell`. The model stays one JSON-serializable
    // object, never a second menu list.
    property string currentUri: ""
    property string currentView: "icon"
    property bool canGoBack: false
    property bool canGoForward: false
    property bool canUndo: false
    property bool canRedo: false
    property string undoLabel: ""
    property string redoLabel: ""
    property bool hasSelection: false
    property int selectedCount: 0
    property bool selectionIsDir: false
    property string selectionUri: ""
    property bool canPaste: false
    property bool inTrash: false
    property bool toolbarVisible: true
    property bool tabBarVisible: false
    // Bumped after a default-application write so the Open With submenu
    // re-reads the bridge's (now cleared) handler cache.
    property int revision: 0

    // The fixed application menu (About/Settings/Hide/Hide Others/Show All/
    // Quit). The shell and broker own these lifecycle verbs; the app publishes
    // the rows so the menu is complete and every action is named.
    readonly property var applicationMenuItems: [
        { label: qsTr("About %1").arg(root.appName), action: "about" },
        { label: qsTr("Settings\u2026"), shortcut: "Super+,", action: "settings" },
        { type: "separator" },
        { label: qsTr("Hide %1").arg(root.appName), shortcut: "Super+H",
          action: "hide" },
        { label: qsTr("Hide Others"), shortcut: "Super+Alt+H",
          action: "hide-others" },
        { label: qsTr("Show All"), action: "show-all" },
        { type: "separator" },
        { label: qsTr("Quit %1").arg(root.appName), shortcut: "Super+Q",
          action: "quit" }
    ]

    function favoriteUri(id) {
        var favorites = Files.favorites;
        for (var i = 0; i < favorites.length; ++i) {
            if (favorites[i].id === id)
                return favorites[i].uri;
        }
        return "";
    }

    // The File ▸ Open With submenu is built from the same bridge call the
    // picker uses. The default row is checked; a missing handler is a visible
    // disabled row, never a silent no-op.
    readonly property var openWithItems: {
        var revision = root.revision;
        var rows = [];
        var canChoose = root.selectionUri.length > 0 && !root.selectionIsDir;
        if (!canChoose) {
            rows.push({
                label: qsTr("No File Selected"),
                enabled: false,
                disabledReason: qsTr("Select one file to open with another application.")
            });
        } else {
            var handlers = Files.handlers(root.selectionUri);
            for (var i = 0; i < handlers.length; ++i) {
                var handler = handlers[i];
                rows.push({
                    label: handler.isDefault
                           ? qsTr("%1 (Default)").arg(handler.name)
                           : handler.name,
                    action: "openWith:" + handler.id,
                    checked: handler.isDefault === true,
                    checkable: true
                });
            }
            if (rows.length === 0) {
                rows.push({
                    label: qsTr("No Applications Available"),
                    enabled: false,
                    disabledReason: qsTr("No installed application handles this file type.")
                });
            }
        }
        rows.push({ type: "separator" });
        rows.push({
            label: qsTr("Choose a Default Application\u2026"),
            action: "chooseDefaultApp",
            enabled: canChoose,
            disabledReason: canChoose ? "" : qsTr("Select one file first.")
        });
        return rows;
    }

    readonly property var menus: [
        {
            title: qsTr("File"),
            items: [
                { label: qsTr("New Window"), shortcut: "Super+N",
                  action: "newWindow" },
                { label: qsTr("New Tab"), shortcut: "Super+T", action: "newTab" },
                { label: qsTr("New Folder"), shortcut: "Shift+Super+N",
                  action: "newFolder", enabled: !root.inTrash },
                { type: "separator" },
                { label: qsTr("Open"), shortcut: "Super+O", action: "open",
                  enabled: root.hasSelection },
                { label: qsTr("Open With"), type: "submenu",
                  submenu: root.openWithItems },
                { label: qsTr("Get Info"), shortcut: "Super+I",
                  action: "getInfo", enabled: root.hasSelection },
                { type: "separator" },
                { label: qsTr("Rename"), shortcut: "Return", action: "rename",
                  enabled: root.selectedCount === 1 && !root.inTrash },
                {
                    label: root.inTrash ? qsTr("Put Back")
                                        : qsTr("Move to Trash"),
                    shortcut: root.inTrash ? "Super+Z" : "Super+Delete",
                    action: root.inTrash ? "putBack" : "trash",
                    enabled: root.hasSelection
                },
                { type: "separator" },
                { label: qsTr("Close Window"), shortcut: "Super+W",
                  action: "close" }
            ]
        },
        {
            title: qsTr("Edit"),
            items: [
                {
                    label: root.undoLabel.length > 0
                           ? qsTr("Undo %1").arg(root.undoLabel) : qsTr("Undo"),
                    shortcut: "Super+Z", action: "undo", enabled: root.canUndo
                },
                {
                    label: root.redoLabel.length > 0
                           ? qsTr("Redo %1").arg(root.redoLabel) : qsTr("Redo"),
                    shortcut: "Shift+Super+Z", action: "redo",
                    enabled: root.canRedo
                },
                { type: "separator" },
                { label: qsTr("Cut"), shortcut: "Super+X", action: "cut",
                  enabled: root.hasSelection },
                { label: qsTr("Copy"), shortcut: "Super+C", action: "copy",
                  enabled: root.hasSelection },
                { label: qsTr("Paste Item"), shortcut: "Super+V",
                  action: "paste", enabled: root.canPaste && !root.inTrash },
                { type: "separator" },
                { label: qsTr("Select All"), shortcut: "Super+A",
                  action: "selectAll" }
            ]
        },
        {
            title: qsTr("View"),
            items: [
                { label: qsTr("as Icons"), shortcut: "Super+1",
                  action: "view.icon", checked: root.currentView !== "list",
                  checkable: true },
                { label: qsTr("as List"), shortcut: "Super+2",
                  action: "view.list", checked: root.currentView === "list",
                  checkable: true },
                { label: qsTr("as Columns"), shortcut: "Super+3",
                  action: "view.columns", enabled: false,
                  disabledReason: qsTr("Column view is not implemented yet.") },
                { label: qsTr("as Gallery"), shortcut: "Super+4",
                  action: "view.gallery", enabled: false,
                  disabledReason: qsTr("Gallery view is not implemented yet.") },
                { type: "separator" },
                { label: root.toolbarVisible ? qsTr("Hide Toolbar")
                                             : qsTr("Show Toolbar"),
                  shortcut: "Ctrl+Alt+T", action: "toggleToolbar" },
                { label: root.tabBarVisible ? qsTr("Hide Tab Bar")
                                            : qsTr("Show Tab Bar"),
                  action: "toggleTabBar" }
            ]
        },
        {
            title: qsTr("Go"),
            items: [
                { label: qsTr("Back"), shortcut: "Super+[", action: "back",
                  enabled: root.canGoBack },
                { label: qsTr("Forward"), shortcut: "Super+]",
                  action: "forward", enabled: root.canGoForward },
                { type: "separator" },
                { label: qsTr("Computer"), shortcut: "Shift+Super+C",
                  action: "go.computer" },
                { label: qsTr("Home"), shortcut: "Shift+Super+H",
                  action: "go.home" },
                { label: qsTr("Desktop"), shortcut: "Shift+Super+D",
                  action: "go.desktop",
                  enabled: root.favoriteUri("desktop").length > 0 },
                { label: qsTr("Documents"), shortcut: "Shift+Super+O",
                  action: "go.documents",
                  enabled: root.favoriteUri("documents").length > 0 },
                { label: qsTr("Downloads"), shortcut: "Alt+Super+L",
                  action: "go.downloads",
                  enabled: root.favoriteUri("downloads").length > 0 },
                { type: "separator" },
                { label: qsTr("Trash"), action: "go.trash" }
            ]
        },
        {
            title: qsTr("Window"),
            items: [
                { label: qsTr("Minimize"), shortcut: "Super+M",
                  action: "minimize" },
                { label: qsTr("Zoom"), action: "zoom" },
                { type: "separator" },
                { label: root.tabBarVisible ? qsTr("Hide Tab Bar")
                                            : qsTr("Show Tab Bar"),
                  action: "toggleTabBar" },
                { label: qsTr("Bring All to Front"),
                  action: "bring-all-to-front" }
            ]
        },
        {
            title: qsTr("Help"),
            items: [
                { label: qsTr("Files Help"), action: "help", enabled: false,
                  disabledReason: qsTr("The Files help document is not installed.") }
            ]
        }
    ]

    // The complete, JSON-serializable publication. `FilesShell.menuModel` is
    // this exact object; the menu-broker resolves it for the focused Files
    // window (native tier).
    readonly property var publishedModel: {
        return {
            appName: root.appName,
            applicationMenuItems: root.applicationMenuItems,
            menus: root.menus
        };
    }

    // T334: publish under the canonical desktop id the shell's focus
    // resolution uses. `MenuPublisher` is a context property in the app
    // executable; the QML test has no broker and skips it.
    function publish() {
        if (typeof MenuPublisher !== "undefined")
            MenuPublisher.publish("org.dragonfruit.Files.desktop",
                                  JSON.stringify(root.publishedModel));
    }

    Component.onCompleted: root.publish()
    // Selection, history, undo and view state change the live rows; republish
    // so the global menu stays truthful instead of showing stale enable flags.
    onMenusChanged: root.publish()
    Component.onDestruction: {
        if (typeof MenuPublisher !== "undefined")
            MenuPublisher.withdraw("org.dragonfruit.Files.desktop");
    }
}
