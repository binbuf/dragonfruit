// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files app shell (T-10.4a): the 52 pt unified window chrome (titlebar
// row, traffic lights, back/forward, view switch, and local search), the
// sidebar with the Favorites/Locations sections, and the footer path bar.
//
// The browsing model is `FilesBrowser`; the locations and breadcrumb come
// from the `Files` platform singleton, and the directory listing from
// `FilesDirectoryModel` (the files-core bridge, T-10.4b). Everything visual is
// a design-system component — this file owns only the information architecture
// and the navigation/selection state.
//
// T-10.4c adds multi-select (Cmd-click toggles, Shift-click ranges, Cmd+A),
// the Files-style context menus, and inline rename. Every mutation routes
// through the files-core operations worker, which paints optimistically and
// then confirms or snaps back — never `std::fs` in the UI.
//
// Keyboard: Up/Down/Home/End/Return on the sidebar; Alt+Left/Alt+Right step
// history; Ctrl+F focuses search; Escape clears it; Cmd+A selects all;
// Cmd+O/Cmd+Down opens the selection; Cmd+C/Cmd+X/Cmd+V copy/cut/paste;
// Cmd+Z undoes, Shift+Cmd+Z redoes (T331); Shift+Cmd+N makes a folder;
// Delete trashes; Return renames. Tabs (T333): Cmd+T opens a tab, Cmd+W closes
// the current tab (the last tab closes the window), Ctrl+Tab / Ctrl+Shift+Tab
// step tabs, and Cmd+1…Cmd+9 activate the nth tab (Cmd+9 the last). New Window
// is Cmd+N; New Folder stays Shift+Cmd+N, matching the design's File menu.
Item {
    id: root
    objectName: "filesShell"

    property string searchText: ""
    // Guards the sidebar selection sync so setting `currentIndex`
    // programmatically is not mistaken for a user navigation.
    property bool syncing: false
    // The primary selected node id (the last one clicked); both views bind to
    // it. The whole multi-selection is `selectedIds`.
    property real selectedId: 0
    // The selected node ids (T-10.4c). Owned here, so switching views cannot
    // lose the selection and every operation sees the same set.
    property var selectedIds: []
    // The node being renamed inline, or 0.
    property real renamingId: 0
    // The last plain-clicked id, the anchor a Shift-click ranges from.
    property real anchorId: 0
    // The node the open context menu targets, or null for the background.
    property var contextNode: null
    // T333: the tab whose context menu is open, or -1.
    property int menuTabIndex: -1
    // A transient inline notice (a snapped-back operation).
    property string notice: ""
    // The conflicting names shown by the paste conflict sheet, and the policy
    // its action buttons choose.
    property var pasteConflictNames: []
    property string pastePolicy: "keep-both"
    // Capture seams (siblings of `DF_FILES_START_VIEW`) applied once the
    // listing settles; nested synthetic pointer input cannot hold a modifier
    // or right-click a Qt client surface reliably.
    property string startMenu: Files.startMenu
    property bool startRename: Files.startRename
    property int startSelect: Files.startSelect
    property bool startEmptyTrash: Files.startEmptyTrash
    property bool captureApplied: false
    // The item to reveal once the location's listing settles (T-10.6c): the
    // Dock's Show in Files passes an executable, a Downloads row a file, and
    // `Files.revealUri` carries the item its parent folder holds. Both this
    // and `startUri` are overridable so the headless test can drive the
    // reveal without a second process.
    property string startUri: Files.startUri
    property string revealUri: Files.revealUri
    property bool revealApplied: false
    // T230d: Files's toolbar-hidden mode. Hiding the toolbar also hides the
    // sidebar; the content inset grows to the measured 64 pt and the status
    // bar spans the full window width. A mode change, not a visibility flag.
    property bool toolbarVisible: true
    // The Get Info auxiliary window is open. It follows the live selection.
    property bool getInfoOpen: false
    // Bumped on every model repaint so the inspector's data binding re-reads
    // the current row (rename, watcher update, optimistic edit).
    property int directoryRevision: 0
    // -- Tabs (T333) -------------------------------------------------------
    // The selection per tab id, so switching back restores the rows the user
    // had selected. Node ids are per-listing; a location change still clears
    // them (`navigate`), so a stored id that no longer exists is pruned by the
    // views rather than selected.
    property var tabSelections: ({})
    // The location of the tab that was active before a switch.
    property string tabUriBefore: ""
    // The highest tab count seen; the bar appears from the second tab.
    property int tabBarRevealedAt: 0
    // Shift+Cmd+T shows the bar with one tab (the capture seam shows it too).
    property bool tabBarForced: Files.startTabs > 0
    readonly property bool tabBarVisible: root.toolbarVisible
                                          && (root.tabBarForced
                                              || root.tabs.count > 1
                                              || root.tabBarRevealedAt > 1)
    // Capture seam: `DF_FILES_CAPTURE_TABS` pins the bar visible from the first
    // tab (the T333 one-tab still). Nothing opens tabs from it; the capture
    // builds them.
    // The history cursor, and therefore view/breadcrumb/title, changed with
    // the tab (or a back/forward in it, or the view kind of a location). A
    // plain counter bumped by every path that mutates the tab list without the
    // location being navigated again; `tabState`, `tabBar.tabs` and the view
    // switch re-read through it.
    property int tabRevision: 0

    property alias browser: browser
    property alias tabs: tabs
    property alias tabBar: tabBar
    property alias sidebar: sidebar
    property alias sidebarPane: sidebarPane
    // T230b: the unified toolbar is the titlebar row; keep the old alias so
    // existing chrome-material consumers read the same surface.
    property alias toolbar: titleBar
    property alias titleBar: titleBar
    property alias appWindow: appWindow
    property alias searchField: searchField
    property alias backButton: backButton
    property alias forwardButton: forwardButton
    property alias viewControl: viewControl
    property alias pathBar: pathBar
    property alias statusBar: statusBar
    property alias emptyState: emptyState
    property alias directory: directory
    property alias iconView: iconView
    property alias listView: listView
    property alias contextMenu: contextMenu
    property alias openWithDialog: openWithDialog
    property alias emptyTrashDialog: emptyTrashDialog
    property alias pasteConflictDialog: pasteConflictDialog
    property alias clipboard: clipboard
    property alias detailSheet: detailSheet
    property alias resizeEdges: resizeEdges
    property alias contentArea: contentArea
    property alias getInfoWindow: getInfoWindow
    property alias getInfoInspector: getInfoWindow.inspector
    property alias viewReasonTip: viewReasonTip

    signal closeRequested()
    signal minimizeRequested()
    signal zoomRequested()
    signal moveRequested(real x, real y)
    signal menuRequested(real x, real y)
    // A file the user asked to open. The window maps this onto the platform
    // `Files.open` handler; keeping it a signal lets the QML tests observe the
    // double-click/context-menu path without spawning a real application.
    signal openRequested(string uri)
    // T333: a second Files window on `uri` (empty: the current location). The
    // window maps this onto `Files.newWindow`; a signal keeps the QML tests
    // from spawning a process. `windowClosed` is emitted when a new window
    // anyway closes every tab, so the host window can go.
    signal newWindowRequested(string uri)
    signal windowClosed()
    // A hand-off with no location asks the window to come forward.
    signal raiseRequested()
    // A window-edge press. `edges` is a `Qt.Edges` flag; the host window
    // forwards it to `startSystemResize`.
    signal resizeRequested(int edges)

    // T333: the tab-derived state. The reading is the active tab's location,
    // history cursor and per-location view; `tabRevision` is bumped by the one
    // `FilesTabs` model change, so a tab switch or a back/forward re-reads all
    // of it without the location being navigated again.
    readonly property var tabState: {
        var revision = root.tabRevision;
        var tab = root.tabs.current;
        var rows = [];
        for (var i = 0; i < root.tabs.count; ++i)
            rows.push({ id: root.tabs.tabs[i].id, title: root.tabs.tabTitle(i) });
        return {
            uri: tab ? (tab.history[tab.historyIndex] || "") : "",
            canGoBack: root.tabs.canGoBack,
            canGoForward: root.tabs.canGoForward,
            view: root.browser.viewFor(tab ? tab.history[tab.historyIndex] : ""),
            tabRows: rows
        };
    }
    readonly property string currentUri: root.tabState.uri
    readonly property bool canGoBack: root.tabState.canGoBack
    readonly property bool canGoForward: root.tabState.canGoForward
    readonly property string currentView: root.tabState.view
    readonly property string locationTitle: Files.displayName(root.currentUri)
    readonly property var breadcrumb: Files.breadcrumb(root.currentUri)
    readonly property bool hasSearch: root.searchText.length > 0
    // Whether the window is at the Trash location, which owns Empty Trash.
    readonly property bool inTrash: root.currentUri === Files.trashUri
    // The active tab id and index (a tab is a browsing context; the selection
    // is saved per id, so this is the key the capture lines and tests read).
    readonly property int currentTabIndex: root.tabs.currentIndex
    readonly property int currentTabId: root.tabs.current ? root.tabs.current.id : -1
    // T230d: the measured Files content inset. Visible: the 52 pt unified
    // toolbar. Hidden: the 64 pt content start (the toolbar is a mode, and the
    // sidebar goes with it).
    readonly property real contentInset: root.toolbarVisible
        ? WindowStyle.height("unified")
        : Theme.controls.files.toolbarHiddenInset
    // The status bar's volume free space. A missing volume is an empty label,
    // never an invented number.
    readonly property string freeSpaceText: {
        var info = Files.volumeInfo(root.currentUri);
        return info.available ? info.freeText : "";
    }
    // T230d: the live Get Info model. The selection is the same shell-owned
    // set the views paint; a single id and a multi-selection share one
    // aggregation surface.
    readonly property var infoSelectionIds: root.selectionForInfo()
    readonly property var infoSummary: {
        var revision = root.directoryRevision;
        var ids = root.infoSelectionIds;
        if (!root.directory || ids.length === 0)
            return ({});
        if (ids.length === 1)
            return root.directory.nodeInfo(ids[0]);
        return root.directory.selectionInfo(ids);
    }
    readonly property var infoSections: root.buildInfoSections()
    readonly property bool infoMultiSelection: root.infoSelectionIds.length > 1
    readonly property string infoTitle: {
        var info = root.infoSummary;
        if (root.infoMultiSelection)
            return qsTr("%1 items").arg(root.infoSelectionIds.length);
        return info.name || "";
    }
    readonly property string infoKind: root.infoMultiSelection
        ? qsTr("%1 items selected").arg(root.infoSelectionIds.length)
        : (root.infoSummary.kindText || "")
    readonly property string infoPreviewSource:
        root.infoMultiSelection ? "" : (root.infoSummary.thumbnail || "")
    readonly property string infoPreviewIcon:
        root.infoMultiSelection ? "folder" : (root.infoSummary.icon || "file")
    readonly property string defaultHandlerText: {
        var info = root.infoSummary;
        if (root.infoMultiSelection)
            return qsTr("Multiple applications");
        if (!info.uri)
            return qsTr("Not available");
        var handler = Files.defaultHandler(info.uri);
        return handler.length > 0 ? handler : qsTr("Not available");
    }
    // T334: the one selected file (not a folder, not a multi-selection). The
    // File ▸ Open With submenu and the picker both use this target.
    readonly property string selectedFileUri: {
        var info = root.infoSummary;
        if (root.infoSelectionIds.length !== 1 || !info.uri || info.isDir)
            return "";
        return info.uri;
    }
    // The window's published menu model (T334). It is the same singleton the
    // menu-broker receives; there is no second list in this file.
    readonly property var menuModel: FilesMenu.publishedModel
    // The Open With picker's live state. `openWithUri` is the context row a
    // right-click targets, or the single selected file when the global menu
    // invokes the action.
    property string openWithUri: ""
    property string openWithName: ""
    property string openWithMime: ""
    property var openWithHandlers: []
    property string openWithSelectedId: ""
    property string openWithNotice: ""
    // Cmd is Super/Mod4 system-wide, but Qt desktop tests and muscle memory
    // both use Control; accept either as "command".
    readonly property int commandMask: Qt.ControlModifier | Qt.MetaModifier
    // T206: the app-local Theme/Glass mirror of the shared settingsd keys,
    // the same copyable pattern SettingsShell uses. Files is a separate
    // process, so the shell's ThemeBinding cannot reach this engine.
    readonly property string colorScheme: Settings.values["appearance.colorScheme"] || "auto"
    readonly property bool hostDark: Application.styleHints.colorScheme === Qt.Dark
    readonly property real transparency: {
        var value = Number(Settings.values["appearance.transparency"]);
        return isFinite(value) ? Math.max(0, Math.min(1, value))
                               : Theme.glass.slider.defaultPosition;
    }
    readonly property bool reduceTransparency:
        Settings.values["accessibility.reduceTransparency"] === true
    readonly property bool increaseContrast:
        Settings.values["accessibility.increaseContrast"] === true
    // The app-local rollout/capability view. Production follows the process
    // environment and platform bridge; tests and captures may override them
    // without changing the shared singleton's binding.
    property string glassRollout: Settings.glassRollout !== ""
        ? Settings.glassRollout : Theme.glass.rollout.defaultMode
    property bool appChromeSupported: Settings.appChromeSupported
    // Bound to the host Window's focus so the titlebar/controls show the
    // focused state; the app-local chrome material itself is focus-invariant.
    property bool windowActive: true

    // T330: the copy/cut marks and the mode a paste uses. It lives here, not
    // in the directory session, so a copy survives navigation.
    FilesClipboard { id: clipboard }

    // T333: the window's tabs. Each tab is a browsing context: its own
    // location, back/forward history, per-location view state (in `browser`)
    // and its own selection (recorded when the active tab changes). The
    // directory listing follows the tab's location; view kinds are the
    // window's one choice, kept per location so each tab remembers its own.
    FilesTabs { id: tabs }

    // -- Tab commands (T333) -------------------------------------------------

    // Record the active tab's selection under its id.
    function captureTabSelection() {
        var tab = root.tabs.current;
        if (!tab)
            return;
        var saved = {};
        for (var key in root.tabSelections)
            saved[key] = root.tabSelections[key];
        saved[tab.id] = root.selectedIds.slice();
        root.tabSelections = saved;
    }

    function tabUri(index) {
        return root.tabs.uriAt(index);
    }

    // Make `index` the visible tab. The selection and history of the tab that
    // was active are recorded first; the new tab's location becomes the
    // window's (title, contents, breadcrumb, status bar).
    function activateTab(index) {
        if (!root.tabs.validIndex(index))
            return false;
        if (index === root.tabs.currentIndex) {
            root.raiseRequested();
            return false;
        }
        root.tabUriBefore = root.currentUri;
        root.captureTabSelection();
        root.tabs.currentIndex = index;
        root.afterTabSwitch();
        return true;
    }

    // Everything that follows the model's current tab changing: the new tab's
    // location is the window's now, so take its selection and re-read the
    // history-derived controls.
    function afterTabSwitch() {
        var tab = root.tabs.current;
        var ids = tab ? (root.tabSelections[tab.id] || []) : [];
        root.selectedIds = ids.slice();
        root.selectedId = ids.length > 0 ? ids[ids.length - 1] : 0;
        root.anchorId = root.selectedId;
        root.cancelRename();
        root.syncSelection();
        root.resetBrowserDerived();
    }

    function resetBrowserDerived() {
        root.tabRevision += 1;
    }

    // Cmd+T / the tab bar's plus: a new tab on the current location, which is
    // the Files behaviour (a new tab starts where you are).
    function newTab(uri) {
        var target = uri !== undefined && uri !== null && uri.length > 0
                ? uri : root.currentUri;
        if (!target)
            target = Files.homeUri;
        // The new tab's selection starts empty; the current tab's is saved
        // under its id so switching back restores it.
        root.captureTabSelection();
        var at = root.tabs.openTab(target, Files.displayName(target));
        if (at < 0)
            return false;
        root.tabUriBefore = target;
        root.tabBarRevealedAt = Math.max(root.tabBarRevealedAt, root.tabs.count);
        var saved = {};
        for (var key in root.tabSelections)
            saved[key] = root.tabSelections[key];
        saved[root.tabs.current.id] = [];
        root.tabSelections = saved;
        root.afterTabSwitch();
        return true;
    }

    // Cmd+W: close the current tab. The last tab closes the window (the
    // decision in 09-files.md's "Closing the last tab" note): a window with no
    // browsing context has nothing to show, and Files never shows an empty
    // shell in place of one.
    function closeTab(index) {
        var at = (index === undefined || index === null) ? root.tabs.currentIndex
                                                         : index;
        if (!root.tabs.validIndex(at))
            return false;
        if (root.tabs.count <= 1) {
            // No browsing context is left, so the model empties before the
            // window goes; the shell never paints an empty tab strip.
            root.tabs.tabs = [];
            root.tabs.currentIndex = -1;
            root.resetBrowserDerived();
            root.closeRequested();
            root.windowClosed();
            return true;
        }
        var closing = root.tabs.tabs[at];
        var saved = {};
        for (var key in root.tabSelections)
            saved[key] = root.tabSelections[key];
        delete saved[closing.id];
        root.tabSelections = saved;
        root.tabUriBefore = root.currentUri;
        root.tabs.closeTab(at);
        root.afterTabSwitch();
        return true;
    }

    // Ctrl+Tab / Ctrl+Shift+Tab: step within the window's tabs.
    function stepTab(delta) {
        if (root.tabs.count < 2)
            return false;
        var count = root.tabs.count;
        var next = ((root.tabs.currentIndex + delta) % count + count) % count;
        return root.activateTab(next);
    }

    // Cmd+1…Cmd+9: the nth tab; Cmd+9 is always the last tab.
    function activateTabNumber(number) {
        if (number < 1 || root.tabs.count === 0)
            return false;
        var index = number === 9 ? root.tabs.count - 1
                                 : Math.min(number - 1, root.tabs.count - 1);
        return root.activateTab(index);
    }

    // Shift+Cmd+T: show the tab bar with one tab.
    function toggleTabBar() {
        root.tabBarForced = !root.tabBarForced;
        return root.tabBarForced;
    }

    // New Window / Open in New Window: a second process on `uri`, so each
    // window is its own crash domain (T333). The window maps the signal onto
    // `Files.newWindow`.
    function openNewWindow(uri) {
        var target = uri !== undefined && uri !== null && uri.length > 0
                ? uri : root.currentUri;
        root.newWindowRequested(target);
        return true;
    }

    // Open a folder of the current tab's listing in a new window.
    function openSelectionInNewWindow() {
        if (!root.contextNode)
            return false;
        return root.openNewWindow(root.contextNode.uri);
    }

    // The tab menu (`index` -1: the strip's "New Tab..." end). It is the
    // window's one context-menu surface, like the item and background menus.
    function openTabMenu(index, x, y) {
        root.contextNode = null;
        root.menuTabIndex = root.tabs.validIndex(index) ? index : -1;
        contextMenu.model = root.tabMenu(index);
        contextMenu.showAt(x, y);
    }

    function tabMenu(index) {
        var tab = root.tabs.validIndex(index) ? root.tabs.tabs[index] : null;
        var rows = [];
        if (tab) {
            rows.push({ label: root.tabs.count > 1 ? qsTr("Close Tab")
                                                   : qsTr("Close Window"),
                        shortcut: "Super+W", action: "closeTab" });
            rows.push({ label: qsTr("Duplicate to New Tab"),
                        action: "newTab" });
            rows.push({ label: qsTr("New Window"), shortcut: "Super+N",
                        action: "newWindow" });
            if (root.tabs.count > 1) {
                rows.push({ type: "separator" });
                rows.push({ label: qsTr("Close Other Tabs"),
                            action: "closeOtherTabs" });
                rows.push({ label: qsTr("Close Tabs to the Right"),
                            action: "closeTabsToRight",
                            enabled: index < root.tabs.count - 1 });
            }
        } else {
            rows.push({ label: qsTr("New Window"), shortcut: "Super+N",
                        action: "newWindow" });
            rows.push({ label: qsTr("New Tab"), shortcut: "Super+T",
                        action: "newTab" });
            rows.push({ type: "separator" });
            rows.push({ label: root.tabBarForced ? qsTr("Hide Tab Bar")
                                                 : qsTr("Show Tab Bar"),
                        action: "toggleTabBar" });
        }
        return rows;
    }

    // Close every tab but `keep` (the tab-bar context menu). The kept tab is
    // activated first so the selection save in `activateTab` does not record
    // the closed tabs' ids against the survivor.
    function closeOtherTabs(keep) {
        if (!root.tabs.validIndex(keep))
            return false;
        root.activateTab(keep);
        var list = root.tabs.tabs.slice();
        for (var i = list.length - 1; i >= 0; --i) {
            if (i !== keep)
                root.closeTab(i);
        }
        return true;
    }

    function closeTabsToRight(index) {
        if (!root.tabs.validIndex(index))
            return false;
        while (root.tabs.count > index + 1)
            root.closeTab(root.tabs.count - 1);
        return root.activateTab(index);
    }

    // Open `uri` in a new tab, in the background (`keepCurrent`) or not;
    // Open in New Tab uses the background form so the current listing stays.
    function openInNewTab(uri, keepCurrent) {
        if (!uri)
            return false;
        var current = root.tabs.currentIndex;
        if (!root.newTab(uri))
            return false;
        if (!keepCurrent)
            return true;
        var opened = root.tabs.currentIndex;
        // Move the new tab next to the current one and go back to it.
        var list = root.tabs.tabs.slice();
        var tab = list.splice(opened, 1)[0];
        list.splice(current + 1, 0, tab);
        // Keep the current tab active and current: the list index moves but
        // the active context must not.
        root.tabs.tabs = list;
        root.tabs.currentIndex = current;
        root.resetBrowserDerived();
        return true;
    }

    // The single-instance hand-off target (`org.dragonfruit.Files1`): open a
    // location as a new tab, or raise the window when the request names none.
    // Called from the entry point; returns whether the request was used.
    function activateLocation(uri) {
        if (!uri || uri.length === 0) {
            root.raiseRequested();
            return true;
        }
        var target = uri;
        if (!Files.isBrowsable(target)) {
            var resolved = Files.resolveOpenTarget(target);
            target = resolved.location;
        }
        if (!target)
            return false;
        if (root.tabs.count === 0) {
            root.tabs.openTab(target, Files.displayName(target));
            root.afterTabSwitch();
            return true;
        }
        var existing = -1;
        for (var i = 0; i < root.tabs.count; ++i) {
            if (root.tabs.uriAt(i) === target) {
                existing = i;
                break;
            }
        }
        if (existing >= 0)
            return root.activateTab(existing);
        return root.openInNewTab(target, false);
    }

    // The sidebar sections the design doc names: Favorites (real XDG user
    // directories), then Locations (Computer, mounted volumes, Trash last).
    readonly property var sidebarSections: {
        var favorites = [];
        var favs = Files.favorites;
        for (var i = 0; i < favs.length; ++i) {
            favorites.push({ label: favs[i].label, icon: favs[i].icon, uri: favs[i].uri });
        }
        var locations = [{ label: Files.userName, icon: "home",
                           uri: Files.homeUri },
                         { label: qsTr("Computer"), icon: "computer",
                           uri: Files.computerUri }];
        var vols = Files.volumes;
        for (var v = 0; v < vols.length; ++v) {
            locations.push({ label: vols[v].label, icon: vols[v].icon,
                             uri: vols[v].uri });
        }
        locations.push({ label: qsTr("Trash"), icon: "trash", uri: Files.trashUri });
        return [
            { title: qsTr("Favorites"), items: favorites },
            { title: qsTr("Locations"), items: locations }
        ];
    }

    // The location a sidebar entry points at. The Sidebar flattens sections
    // (adding header rows), so the source section/item is recovered by index.
    function uriForEntry(entry) {
        if (!entry || entry.type !== "item")
            return "";
        var section = root.sidebarSections[entry.section];
        if (!section)
            return "";
        var item = section.items[entry.itemIndex];
        return item ? item.uri : "";
    }

    function entryIndexForUri(uri) {
        var entries = sidebar.entries;
        for (var i = 0; i < entries.length; ++i) {
            if (root.uriForEntry(entries[i]) === uri)
                return i;
        }
        return -1;
    }

    function syncSelection() {
        root.syncing = true;
        sidebar.currentIndex = root.entryIndexForUri(root.currentUri);
        root.syncing = false;
    }

    // The window chords this window reserves (T333), for the shortcut
    // reference and any later binding that must not shadow them. New Window
    // takes `Super+N`; New Folder keeps `Shift+Super+N`.
    readonly property var tabChords: ["Super+T", "Super+W", "Ctrl+Tab",
                                      "Ctrl+Shift+Tab", "Super+1", "Super+9",
                                      "Super+N", "Shift+Super+N"]

    function focusSearch() {
        searchField.forceActiveFocus();
    }

    // -- Navigating the active tab (T333) ------------------------------------

    // Record a location in the active tab's history. A location change starts
    // a fresh selection (node ids are per-listing) and re-reads the tab-derived
    // state; the tab's own back/forward stack is the only thing that grows.
    function navigate(uri) {
        var title = Files.displayName(uri);
        if (!root.tabs.navigate(uri, title))
            return false;
        root.clearSelection();
        root.cancelRename();
        root.syncSelection();
        root.resetBrowserDerived();
        return true;
    }

    function back() {
        if (!root.tabs.back())
            return false;
        root.clearSelection();
        root.cancelRename();
        root.syncSelection();
        root.resetBrowserDerived();
        return true;
    }

    function forward() {
        if (!root.tabs.forward())
            return false;
        root.clearSelection();
        root.cancelRename();
        root.syncSelection();
        root.resetBrowserDerived();
        return true;
    }

    // The view switch: one window kind, remembered per location, so each tab
    // keeps the view its own location was last shown in.
    function setView(kind) {
        root.browser.setView(kind, root.currentUri);
        root.resetBrowserDerived();
    }

    // Selection and opening the two views share (T-10.4b/T-10.4c).
    function selectNode(nodeId, modifiers) {
        if (nodeId <= 0) {
            root.clearSelection();
            return;
        }
        var command = modifiers & root.commandMask;
        var shift = modifiers & Qt.ShiftModifier;
        if (command) {
            var ids = root.selectedIds.slice();
            var at = ids.indexOf(nodeId);
            if (at >= 0)
                ids.splice(at, 1);
            else
                ids.push(nodeId);
            root.selectedIds = ids;
            root.anchorId = nodeId;
        } else if (shift && root.anchorId > 0) {
            root.selectedIds = root.rangeTo(root.anchorId, nodeId);
        } else {
            root.selectedIds = [nodeId];
            root.anchorId = nodeId;
        }
        root.selectedId = root.selectedIds.length > 0
                ? root.selectedIds[root.selectedIds.length - 1] : 0;
    }

    // Rubber-band (marquee) selection (T-19.3): the view reports the enclosed
    // node ids; Cmd toggles them, Shift extends, otherwise the band replaces
    // the selection (an empty band clears it). The live path snapshots the
    // selection on `beginMarquee` so a Cmd/Shift drag toggles from a stable
    // base while the pointer is still moving, rather than flipping each tile
    // on every update.
    property var marqueeBaseIds: []
    property bool marqueeLive: false

    function beginMarquee(modifiers) {
        root.marqueeBaseIds = root.selectedIds.slice();
        root.marqueeLive = true;
    }

    function updateMarquee(nodeIds, modifiers) {
        var command = modifiers & root.commandMask;
        var shift = modifiers & Qt.ShiftModifier;
        var ids = (root.marqueeLive ? root.marqueeBaseIds
                                    : root.selectedIds).slice();
        if (command) {
            for (var i = 0; i < nodeIds.length; ++i) {
                var at = ids.indexOf(nodeIds[i]);
                if (at >= 0)
                    ids.splice(at, 1);
                else
                    ids.push(nodeIds[i]);
            }
        } else if (shift) {
            for (var j = 0; j < nodeIds.length; ++j) {
                if (ids.indexOf(nodeIds[j]) < 0)
                    ids.push(nodeIds[j]);
            }
        } else {
            ids = nodeIds.slice();
        }
        root.selectedIds = ids;
        if (!command && ids.length > 0)
            root.anchorId = ids[0];
        root.selectedId = ids.length > 0 ? ids[ids.length - 1] : 0;
    }

    function endMarquee(nodeIds, modifiers) {
        root.updateMarquee(nodeIds, modifiers);
        root.marqueeLive = false;
        root.marqueeBaseIds = [];
    }

    function selectMarquee(nodeIds, modifiers) {
        if (!root.marqueeLive)
            root.beginMarquee(modifiers);
        root.endMarquee(nodeIds, modifiers);
    }

    // The contiguous node ids from `fromId` to `toId` in the current model
    // order — a Files Shift-click range.
    function rangeTo(fromId, toId) {
        var from = directory.rowForNodeId(fromId);
        var to = directory.rowForNodeId(toId);
        if (from < 0 || to < 0)
            return [toId];
        var lo = Math.min(from, to);
        var hi = Math.max(from, to);
        var ids = [];
        for (var row = lo; row <= hi; ++row) {
            var id = directory.nodeIdAt(row);
            if (id > 0)
                ids.push(id);
        }
        return ids;
    }

    function clearSelection() {
        root.selectedIds = [];
        root.selectedId = 0;
        root.anchorId = 0;
    }

    function selectAll() {
        var ids = directory ? directory.allNodeIds() : [];
        root.selectedIds = ids;
        root.selectedId = ids.length > 0 ? ids[0] : 0;
        root.anchorId = root.selectedId;
    }

    // T230d: Get Info targets the whole selection; a lone primary id is the
    // fallback for a caller that only set `selectedId`.
    function selectionForInfo() {
        var ids = root.selectedIds.slice();
        if (ids.length === 0 && root.selectedId > 0)
            ids = [root.selectedId];
        return ids;
    }

    function openGetInfo() {
        if (root.selectionForInfo().length === 0)
            return false;
        root.getInfoOpen = true;
        return true;
    }

    function closeGetInfo() {
        root.getInfoOpen = false;
    }

    function toggleToolbar() {
        root.toolbarVisible = !root.toolbarVisible;
    }

    function setToolbarVisible(visible) {
        root.toolbarVisible = visible === true;
    }

    // Rename from the inspector. Only one item has a real backend; a
    // multi-selection says why instead of editing a fake aggregate name.
    function renameFromInfo(newName) {
        var ids = root.selectionForInfo();
        if (ids.length !== 1) {
            root.notice = qsTr("Rename one item at a time.");
            noticeTimer.restart();
            return false;
        }
        if (newName.length === 0 || !directory)
            return false;
        return directory.rename(ids[0], newName);
    }

    // The single scrolling column's data. Section ids are stable for tests and
    // for the "edit only a real backend" rule; rows carry an explicit reason
    // when a control is not backed yet.
    function buildInfoSections() {
        var revision = root.directoryRevision;
        var ids = root.infoSelectionIds;
        var info = root.infoSummary;
        var sections = [];
        if (!directory || ids.length === 0)
            return sections;
        var single = ids.length === 1;

        var general = [];
        if (single) {
            general.push({ label: qsTr("Name"), value: info.name || "" });
            general.push({ label: qsTr("Kind"), value: info.kindText || "" });
            general.push({ label: qsTr("Size"),
                           value: info.hasSize ? info.sizeText
                                               : qsTr("Not available") });
            general.push({ label: qsTr("Where"),
                           value: info.whereText || qsTr("Not available") });
        } else {
            general.push({ label: qsTr("Items"),
                           value: qsTr("%1 items selected").arg(info.count) });
            general.push({ label: qsTr("Kind"),
                           value: qsTr("%1 folders, %2 files")
                                  .arg(info.folderCount).arg(info.fileCount) });
            general.push({ label: qsTr("Size"),
                           value: info.hasTotalSize ? info.totalSizeText
                                                    : qsTr("Not available") });
        }
        sections.push({ id: "general", title: qsTr("General"), rows: general });

        var moreInfo = [];
        if (single) {
            moreInfo.push({ label: qsTr("Modified"),
                            value: info.hasModified ? info.modifiedText
                                                    : qsTr("Not available") });
            if ((info.symlinkTarget || "").length > 0) {
                moreInfo.push({ label: qsTr("Original"),
                                value: info.symlinkTarget });
            }
            moreInfo.push({ label: qsTr("Content created"),
                            value: qsTr("Not available"),
                            reason: qsTr("files-core exposes the modification time only.") });
        } else {
            moreInfo.push({ label: qsTr("Modified"),
                            value: qsTr("Multiple values") });
        }
        sections.push({ id: "moreInfo", title: qsTr("More Info"),
                        rows: moreInfo });

        var name = [];
        name.push({
            label: qsTr("Name"),
            value: single ? (info.name || "") : qsTr("%1 items").arg(info.count),
            editable: single && info.id > 0,
            reason: single ? "" : qsTr("Rename one item at a time.")
        });
        if (single) {
            name.push({ label: qsTr("Extension"),
                        value: info.extension || qsTr("None") });
        }
        name.push({ label: qsTr("Hide extension"),
                    value: qsTr("Not available"),
                    reason: qsTr("files-core does not store the hide-extension flag.") });
        sections.push({ id: "name", title: qsTr("Name & Extension"),
                        rows: name });

        sections.push({
            id: "comments",
            title: qsTr("Comments"),
            rows: [{
                label: qsTr("Comments"),
                value: qsTr("Not available"),
                reason: qsTr("The comments backend is not implemented.")
            }]
        });

        sections.push({
            id: "openWith",
            title: qsTr("Open with"),
            rows: [{
                label: qsTr("Open with"),
                value: root.defaultHandlerText,
                reason: single ? qsTr("Use File \u25b8 Open With to choose another application or change the default.")
                               : qsTr("Select one file to choose an application.")
            }]
        });

        sections.push({
            id: "preview",
            title: qsTr("Preview"),
            rows: [{
                label: single ? qsTr("Preview") : qsTr("Selection"),
                value: single ? (info.kindText || "") : qsTr("%1 items").arg(info.count)
            }]
        });

        sections.push({
            id: "sharing",
            title: qsTr("Sharing & Permissions"),
            rows: [
                { label: qsTr("Owner"), value: qsTr("Not available") },
                { label: qsTr("Group"), value: qsTr("Not available") },
                { label: qsTr("Permissions"), value: qsTr("Not available"),
                  reason: qsTr("files-core does not expose POSIX ownership yet.") }
            ]
        });
        return sections;
    }

    function activateNode(uri, isDir) {
        if (!uri)
            return;
        if (isDir)
            root.navigate(uri);
        else
            root.openRequested(uri);
    }

    // Open the primary selection (Cmd+O / Cmd+Down), using the same path as a
    // double click or the context menu's Open.
    function openSelection() {
        if (root.selectedId <= 0 || !directory)
            return;
        var row = directory.rowForNodeId(root.selectedId);
        if (row < 0)
            return;
        root.activateNode(directory.uriAt(row), directory.isDirAt(row));
    }

    // -- Open With (T334) ----------------------------------------------------

    // The file a right-click targets, or the single selected file when the
    // global menu (which has no context node) invokes the action.
    function openWithTargetUri() {
        if (root.contextNode && root.contextNode.uri && !root.contextNode.isDir)
            return root.contextNode.uri;
        return root.selectedFileUri;
    }

    // Open the picker for `uri`. An absent handler list still opens the dialog
    // with its visible message; a non-file target is refused with a notice.
    function openWithDialogFor(uri) {
        if (!uri) {
            root.notice = qsTr("Select one file to open with another application.");
            return false;
        }
        var handlers = Files.handlers(uri);
        root.openWithUri = uri;
        root.openWithName = Files.displayName(uri);
        root.openWithMime = Files.mimeType(uri);
        root.openWithHandlers = handlers;
        root.openWithSelectedId = "";
        for (var i = 0; i < handlers.length; ++i) {
            if (handlers[i].isDefault) {
                root.openWithSelectedId = handlers[i].id;
                break;
            }
        }
        if (root.openWithSelectedId.length === 0 && handlers.length > 0)
            root.openWithSelectedId = handlers[0].id;
        // Capture seam: pin a non-default handler for the live stills.
        var select = Files.startOpenWithSelect;
        if (select.length > 0) {
            var found = false;
            for (var s = 0; s < handlers.length; ++s) {
                if (handlers[s].id === select) {
                    root.openWithSelectedId = handlers[s].id;
                    found = true;
                    break;
                }
            }
            var index = parseInt(select, 10);
            if (!found && !isNaN(index) && index >= 0 && index < handlers.length)
                root.openWithSelectedId = handlers[index].id;
        }
        root.openWithNotice = "";
        openWithDialog.show();
        return true;
    }

    function openWithForTarget() {
        return root.openWithDialogFor(root.openWithTargetUri());
    }

    function openWithSelected(id) {
        if (!root.openWithUri || !id)
            return false;
        if (!Files.openWith(root.openWithUri, id)) {
            root.openWithNotice = qsTr("Could not open “%1” with the chosen application.")
                    .arg(root.openWithName);
            return false;
        }
        openWithDialog.hide();
        return true;
    }

    // The context/global menu's handler row. The target is the right-clicked
    // row when one is open, otherwise the current single selection.
    function openWithContext(id) {
        var uri = root.openWithTargetUri();
        if (!uri || !id)
            return false;
        if (!Files.openWith(uri, id)) {
            root.notice = qsTr("Could not open “%1” with the chosen application.")
                    .arg(Files.displayName(uri));
            return false;
        }
        return true;
    }

    // The picker's explicit "Set as Default" path. It writes the MIME default
    // through `xdg-mime`/`gio` (FilesBridge) and refreshes the live rows so the
    // default marker moves immediately.
    function setDefaultOpenHandler(id) {
        if (!root.openWithUri || !id)
            return false;
        if (!Files.setDefaultHandler(root.openWithUri, id)) {
            root.openWithNotice = qsTr("Could not set “%1” as the default application.")
                    .arg(root.openWithName);
            return false;
        }
        root.openWithHandlers = Files.handlers(root.openWithUri);
        root.openWithSelectedId = id;
        root.openWithNotice = qsTr("“%1” is now the default application.")
                .arg(Files.displayName(root.openWithUri));
        FilesMenu.revision += 1;
        return true;
    }

    // The File menu's "Choose a Default Application…" row: open the same
    // picker in default-selection mode. The Set as Default button is the
    // write path; the dialog is one surface, not a second chooser.
    function chooseDefaultApplication() {
        return root.openWithDialogFor(root.openWithTargetUri());
    }

    // The published menu model's action dispatcher. It is the one local
    // endpoint for both a global-menu click/accelerator and a test.
    function dispatchMenuAction(action) {
        if (!action)
            return false;
        if (action.indexOf("openWith:") === 0)
            return root.openWithContext(action.substring(9));
        switch (action) {
        case "newWindow": root.openNewWindow(); return true;
        case "newTab": root.newTab(); return true;
        case "newFolder": root.makeFolder(); return true;
        case "open": root.openSelection(); return true;
        case "chooseDefaultApp": return root.chooseDefaultApplication();
        case "getInfo": return root.openGetInfo();
        case "rename":
            if (root.selectedId > 0) {
                root.beginRename(root.selectedId);
                return true;
            }
            return false;
        case "trash":
            root.trashSelection(); return true;
        case "putBack": root.putBackSelection(); return true;
        case "undo": return root.undo();
        case "redo": return root.redo();
        case "cut": root.cutSelection(); return true;
        case "copy": root.copySelection(); return true;
        case "paste": root.paste(); return true;
        case "selectAll": root.selectAll(); return true;
        case "view.icon": root.setView("icon"); return true;
        case "view.list": root.setView("list"); return true;
        case "toggleToolbar": root.toggleToolbar(); return true;
        case "toggleTabBar": root.toggleTabBar(); return true;
        case "back": return root.back();
        case "forward": return root.forward();
        case "go.home": return root.navigate(Files.homeUri);
        case "go.computer": return root.navigate(Files.computerUri);
        case "go.trash": return root.navigate(Files.trashUri);
        case "go.desktop": return root.navigate(root.favoriteUri("desktop"));
        case "go.documents": return root.navigate(root.favoriteUri("documents"));
        case "go.downloads": return root.navigate(root.favoriteUri("downloads"));
        case "close": root.closeRequested(); return true;
        case "minimize": root.minimizeRequested(); return true;
        case "zoom": root.zoomRequested(); return true;
        case "bring-all-to-front": root.raiseRequested(); return true;
        }
        return false;
    }

    function favoriteUri(id) {
        var favorites = Files.favorites;
        for (var i = 0; i < favorites.length; ++i) {
            if (favorites[i].id === id)
                return favorites[i].uri;
        }
        return "";
    }

    // Inline rename (T-10.4c). Exactly one row at a time.
    function beginRename(nodeId) {
        if (nodeId > 0)
            root.renamingId = nodeId;
    }

    function commitRename(nodeId, newName) {
        root.renamingId = 0;
        if (nodeId > 0 && newName.length > 0)
            directory.rename(nodeId, newName);
    }

    function cancelRename() {
        root.renamingId = 0;
    }

    // Move every selected row to Trash, optimistically (T-10.4c).
    function trashSelection() {
        var ids = root.selectedIds.slice();
        if (ids.length === 0 && root.selectedId > 0)
            ids = [root.selectedId];
        for (var i = 0; i < ids.length; ++i)
            directory.trash(ids[i]);
        root.clearSelection();
    }

    function makeFolder() {
        directory.newFolder(root.currentUri);
    }

    // -- Undo / redo (T331) --------------------------------------------------

    // The stacks live in the files-core session (one folder visit): a
    // location change clears them, and every completed rename, new folder,
    // paste (move or copy), or Move to Trash pushes one bounded step.
    readonly property bool canUndo: directory ? directory.canUndo : false
    readonly property bool canRedo: directory ? directory.canRedo : false

    function undo() {
        if (!directory)
            return false;
        if (!directory.undo()) {
            // Nothing to undo, or a replay is already running: the model's
            // message says which; show it only when there is one.
            if (directory.undoMessage.length > 0)
                root.showUndoMessage(directory.undoMessage);
            return false;
        }
        return true;
    }

    function redo() {
        if (!directory)
            return false;
        if (!directory.redo()) {
            if (directory.undoMessage.length > 0)
                root.showUndoMessage(directory.undoMessage);
            return false;
        }
        return true;
    }

    // A refusal keeps the current state intact and says why.
    function showUndoMessage(message) {
        if (message.length === 0)
            return;
        root.notice = message;
        noticeTimer.restart();
    }

    // Put Back a Trash row (T331; also under Cmd+Z, through the recorded
    // trash undo). The Trash listing's node ids are put-back ids recorded by
    // the trash source, so the session can restore the exact store item.
    function putBackSelection() {
        if (!root.inTrash || !directory)
            return false;
        var ids = root.selectedIds.slice();
        if (ids.length === 0 && root.selectedId > 0)
            ids = [root.selectedId];
        var restored = 0;
        for (var i = 0; i < ids.length; ++i) {
            if (directory.putBack(ids[i]))
                ++restored;
        }
        if (restored > 0) {
            root.clearSelection();
            root.notice = restored === 1 ? qsTr("Put Back 1 item.")
                                         : qsTr("Put Back %1 items.").arg(restored);
            noticeTimer.restart();
        } else if (directory.lastError.length > 0) {
            root.showUndoMessage(directory.lastError);
        }
        return restored > 0;
    }

    // -- Copy / cut / paste (T330) ------------------------------------------

    // The marked items, in selection order. A copy never changes the listing;
    // a cut only marks, exactly as the Files design requires.
    function selectionClipboardItems() {
        var ids = root.selectedIds.slice();
        if (ids.length === 0 && root.selectedId > 0)
            ids = [root.selectedId];
        var items = [];
        for (var i = 0; i < ids.length; ++i) {
            if (!directory)
                continue;
            var info = directory.nodeInfo(ids[i]);
            if (!info || !info.uri)
                continue;
            items.push({ uri: info.uri, name: info.name || "",
                         isDir: info.isDir === true });
        }
        return items;
    }

    function copySelection() {
        var items = root.selectionClipboardItems();
        if (items.length === 0)
            return false;
        clipboard.copy(items);
        root.notice = items.length === 1
                ? qsTr("Copied “%1”").arg(items[0].name)
                : qsTr("Copied %1 items").arg(items.length);
        noticeTimer.restart();
        return true;
    }

    function cutSelection() {
        var items = root.selectionClipboardItems();
        if (items.length === 0)
            return false;
        clipboard.cut(items);
        root.notice = items.length === 1
                ? qsTr("Cut “%1”").arg(items[0].name)
                : qsTr("Cut %1 items").arg(items.length);
        noticeTimer.restart();
        return true;
    }

    // The names the clipboard would collide with in the current folder. The
    // pre-flight is intentionally model-only: the optimistic path only paints
    // the current directory, and a conflict outside it is resolved by the
    // worker's policy.
    function pasteConflicts() {
        var out = [];
        if (!directory || clipboard.count === 0)
            return out;
        var names = directory.names();
        for (var i = 0; i < clipboard.items.length; ++i) {
            var name = clipboard.items[i].name;
            if (names.indexOf(name) >= 0)
                out.push(name);
        }
        return out;
    }

    // Paste the clipboard into the current folder. A conflict pauses on the
    // one conflict sheet (Replace / Skip / Keep Both / Cancel); a clean paste
    // uses Keep Both, which is a no-op policy when nothing conflicts.
    function paste() {
        if (clipboard.empty)
            return false;
        if (root.inTrash) {
            root.notice = qsTr("Paste is not available in the Trash.");
            noticeTimer.restart();
            return false;
        }
        var conflicts = root.pasteConflicts();
        if (conflicts.length > 0) {
            root.pasteConflictNames = conflicts;
            pasteConflictDialog.show();
            return true;
        }
        return root.performPaste("keep-both");
    }

    // Queue one files-core paste per marked item. The clipboard is cleared for
    // a cut immediately: the marks are consumed by the paste, and a failed
    // item remains on disk for the user to retry.
    function performPaste(policy) {
        if (clipboard.empty || !directory)
            return false;
        var target = root.currentUri;
        var mode = clipboard.mode;
        var items = clipboard.items.slice();
        var queued = 0;
        for (var i = 0; i < items.length; ++i) {
            if (directory.pasteItem(target, items[i].uri, items[i].name,
                                    items[i].isDir === true, mode, policy))
                ++queued;
        }
        if (queued === 0)
            return false;
        if (mode === "cut")
            clipboard.clear();
        return true;
    }

    function pasteFinished(copied, skipped, failed) {
        var message = "";
        if (failed > 0)
            message = failed === 1 ? qsTr("1 item could not be pasted.")
                                   : qsTr("%1 items could not be pasted.").arg(failed);
        else if (skipped > 0)
            message = skipped === 1 ? qsTr("Skipped 1 existing item.")
                                    : qsTr("Skipped %1 existing items.").arg(skipped);
        else if (copied > 0)
            message = copied === 1 ? qsTr("Pasted 1 item.")
                                   : qsTr("Pasted %1 items.").arg(copied);
        if (message.length > 0) {
            root.notice = message;
            noticeTimer.restart();
        }
    }

    // Empty Trash (T-10.6b): always a confirmation, per 09-files.md#trash. The
    // action itself routes through files-core, so the Dock and Files share the
    // one result.
    function confirmEmptyTrash() {
        if (!root.inTrash || directory.count === 0)
            return;
        emptyTrashDialog.show();
    }

    function emptyTrashNow() {
        if (root.inTrash) {
            directory.emptyTrash();
            root.clearSelection();
        }
    }

    // The File ▸ Open With rows for one node. The same bridge list feeds the
    // picker, so the submenu never invents application names.
    function openWithMenu(node) {
        var rows = [];
        var handlers = node && node.uri ? Files.handlers(node.uri) : [];
        for (var i = 0; i < handlers.length; ++i) {
            var handler = handlers[i];
            rows.push({
                label: handler.isDefault
                       ? qsTr("%1 (Default)").arg(handler.name) : handler.name,
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
        rows.push({ type: "separator" });
        rows.push({ label: qsTr("Choose a Default Application\u2026"),
                    action: "chooseDefaultApp" });
        return rows;
    }

    // The Files rule: the menu depends on what is under the pointer.
    function nodeMenu(node) {
        if (root.inTrash && node !== null) {
            // A Trash row's verb is Put Back; it is the same undo the Edit
            // menu offers, scoped to the selected items.
            var trashRows = [
                { label: qsTr("Open"), action: "open", enabled: node !== null },
                { type: "separator" },
                { label: qsTr("Put Back"), shortcut: "Cmd+Z",
                  action: "putBack", enabled: node !== null },
                { label: qsTr("Get Info"), shortcut: "Super+I",
                  action: "getInfo", enabled: node !== null }
            ];
            return trashRows;
        }
        if (root.selectedIds.length > 1) {
            return [
                { label: qsTr("Get Info"), shortcut: "Super+I",
                  action: "getInfo" },
                { label: qsTr("Copy"), shortcut: "Super+C", action: "copy" },
                { label: qsTr("Cut"), shortcut: "Super+X", action: "cut" },
                { label: qsTr("Move to Trash"), shortcut: "Delete",
                  action: "trash" }
            ];
        }
        var entries = [
            { label: qsTr("Open"), action: "open", enabled: node !== null }
        ];
        // T334: Open With is a live submenu of the installed handlers, the
        // default row checked. The last row opens the picker for the
        // "Choose a default application" path.
        if (node && !node.isDir) {
            entries.push({ label: qsTr("Open With"), type: "submenu",
                           submenu: root.openWithMenu(node) });
        }
        entries.push({ label: qsTr("Get Info"), shortcut: "Super+I",
                       action: "getInfo", enabled: node !== null });
        // T333: a folder opens in its own tab or its own window; the window
        // is a second process (crash-independent), the tab shares this one.
        if (node && node.isDir) {
            entries.push({ label: qsTr("Open in New Tab"),
                           action: "openInNewTab" });
            entries.push({ label: qsTr("Open in New Window"),
                           action: "openInNewWindow" });
        }
        entries.push({ type: "separator" });
        entries.push({ label: qsTr("Copy"), shortcut: "Super+C",
                       action: "copy", enabled: node !== null });
        entries.push({ label: qsTr("Cut"), shortcut: "Super+X",
                       action: "cut", enabled: node !== null });
        entries.push({ type: "separator" });
        entries.push({ label: qsTr("Rename"), shortcut: "Return",
                       action: "rename" });
        entries.push({ label: qsTr("Move to Trash"), shortcut: "Delete",
                       action: "trash" });
        return entries;
    }

    // The Edit menu rows (T331). Undo/Redo name their next step so the row
    // says what it will do; disabled is a real disabled row, not a dead one.
    function undoMenuRows() {
        var undoLabel = directory ? directory.undoLabel : "";
        var redoLabel = directory ? directory.redoLabel : "";
        return [
            { label: undoLabel.length > 0
                    ? qsTr("Undo %1").arg(undoLabel) : qsTr("Undo"),
              shortcut: "Super+Z", action: "undo", enabled: root.canUndo },
            { label: redoLabel.length > 0
                    ? qsTr("Redo %1").arg(redoLabel) : qsTr("Redo"),
              shortcut: "Shift+Super+Z", action: "redo", enabled: root.canRedo }
        ];
    }

    // T331: the Edit menu's Edit section, shown as the small menu the
    // window's context-menu surface draws at the toolbar's Edit position.
    // The native window menu for Files is not published (G-04), so this is
    // the menu the user opens for Undo/Redo beside Cmd+Z.
    function openEditMenu(x, y) {
        root.contextNode = null;
        root.menuTabIndex = -1;
        contextMenu.model = root.editMenuRows();
        contextMenu.showAt(x, y);
        return true;
    }

    function editMenuRows() {
        return root.undoMenuRows().concat([
            { type: "separator" },
            { label: qsTr("Cut"), shortcut: "Super+X", action: "cut",
              enabled: root.selectionClipboardItems().length > 0 },
            { label: qsTr("Copy"), shortcut: "Super+C", action: "copy",
              enabled: root.selectionClipboardItems().length > 0 },
            { label: qsTr("Paste Item"), shortcut: "Super+V", action: "paste",
              enabled: root.inTrash ? false : clipboard.count > 0 },
            { type: "separator" },
            { label: qsTr("Select All"), shortcut: "Super+A",
              action: "selectAll" }
        ]);
    }

    // The toolbar's Edit target: the edit menu opens under the search field's
    // left edge, the measured position of the window's Edit control.
    function openEditMenuAtToolbar() {
        var point = searchField.mapToItem(root, 0, searchField.height);
        root.openEditMenu(Math.max(0, point.x), point.y);
    }

    function startMenuRows() {
        return [
            { label: root.toolbarVisible ? qsTr("Hide Toolbar")
                                         : qsTr("Show Toolbar"),
              shortcut: "Ctrl+Alt+T", action: "toggleToolbar" },
            { label: root.tabBarVisible ? qsTr("Hide Tab Bar")
                                        : qsTr("Show Tab Bar"),
              action: "toggleTabBar" }
        ];
    }

    function backgroundMenu() {
        if (root.startMenu === "edit")
            return root.editMenuRows();
        // T333: the File section leads with New Tab / New Window. Shift+Cmd+N
        // is New Window (the design's File menu row); New Folder keeps its
        // T-10.4c chord on Shift+Cmd+N.
        var fileRows = [
            { label: qsTr("New Tab"), shortcut: "Super+T", action: "newTab" },
            { label: qsTr("New Window"), shortcut: "Super+N",
              action: "newWindow" },
            { type: "separator" }
        ];
        var editRows = root.undoMenuRows();
        if (root.inTrash) {
            return fileRows.concat(editRows, [
                { type: "separator" },
                { label: qsTr("Empty Trash"), shortcut: "Shift+Super+Delete",
                  action: "emptyTrash", enabled: directory.count > 0 },
                { type: "separator" }
            ], root.startMenuRows(), [
                { label: qsTr("Select All"), shortcut: "Super+A",
                  action: "selectAll" }
            ]);
        }
        var rows = fileRows.concat(editRows, [
            { label: qsTr("Cut"), shortcut: "Super+X", action: "cut",
              enabled: root.selectionClipboardItems().length > 0 },
            { label: qsTr("Copy"), shortcut: "Super+C", action: "copy",
              enabled: root.selectionClipboardItems().length > 0 },
            { label: qsTr("Paste Item"), shortcut: "Super+V",
              action: "paste", enabled: clipboard.count > 0 },
            { type: "separator" },
            { label: qsTr("New Folder"), shortcut: "Shift+Super+N",
              action: "newFolder" }
        ]);
        // Free placement is an icon-view rule; a list has no icon slots to
        // clean up, so the row would be a dead affordance there.
        if (root.currentView !== "list")
            rows.push({ label: qsTr("Clean Up"), action: "cleanUp" });
        rows.push({ type: "separator" });
        rows = rows.concat(root.startMenuRows());
        rows.push({ label: qsTr("Select All"), shortcut: "Super+A",
                    action: "selectAll" });
        return rows;
    }

    function openNodeMenu(nodeId, uri, isDir, x, y) {
        root.contextNode = { id: nodeId, uri: uri, isDir: isDir };
        contextMenu.model = root.nodeMenu(root.contextNode);
        contextMenu.showAt(x, y);
    }

    function openBackgroundMenu(x, y) {
        root.contextNode = null;
        root.menuTabIndex = -1;
        contextMenu.model = root.backgroundMenu();
        contextMenu.showAt(x, y);
    }

    function contextAction(action) {
        if (action && action.indexOf("openWith:") === 0) {
            root.openWithContext(action.substring(9));
            return;
        }
        switch (action) {
        case "open":
            if (root.contextNode)
                root.activateNode(root.contextNode.uri, root.contextNode.isDir);
            break;
        case "chooseDefaultApp":
            root.chooseDefaultApplication();
            break;
        case "rename":
            if (root.contextNode)
                root.beginRename(root.contextNode.id);
            break;
        case "copy":
            root.copySelection();
            break;
        case "cut":
            root.cutSelection();
            break;
        case "paste":
            root.paste();
            break;
        case "trash":
            root.trashSelection();
            break;
        case "newFolder":
            root.makeFolder();
            break;
        case "cleanUp":
            if (root.iconView)
                root.iconView.cleanUp();
            break;
        case "selectAll":
            root.selectAll();
            break;
        case "getInfo":
            root.openGetInfo();
            break;
        case "toggleToolbar":
            root.toggleToolbar();
            break;
        case "emptyTrash":
            root.confirmEmptyTrash();
            break;
        case "undo":
            root.undo();
            break;
        case "redo":
            root.redo();
            break;
        case "putBack":
            root.putBackSelection();
            break;
        case "openInNewTab":
            if (root.contextNode)
                root.openInNewTab(root.contextNode.uri, true);
            break;
        case "openInNewWindow":
            root.openSelectionInNewWindow();
            break;
        case "newTab":
            if (root.menuTabIndex >= 0 && root.tabs.validIndex(root.menuTabIndex))
                root.openInNewTab(root.tabs.uriAt(root.menuTabIndex), true);
            else
                root.newTab();
            root.menuTabIndex = -1;
            break;
        case "closeTab":
            root.closeTab(root.menuTabIndex >= 0 ? root.menuTabIndex : undefined);
            break;
        case "closeOtherTabs":
            root.closeOtherTabs(root.menuTabIndex);
            break;
        case "closeTabsToRight":
            root.closeTabsToRight(root.menuTabIndex);
            break;
        case "newWindow":
            root.openNewWindow();
            break;
        case "toggleTabBar":
            root.toggleTabBar();
            break;
        }
    }

    // Apply a capture seam once, as soon as the first listing settles.
    function applyCaptureSeam() {
        if (root.captureApplied || !directory)
            return;
        if (directory.state !== "complete" || directory.count === 0)
            return;
        root.captureApplied = true;
        var ids = directory.allNodeIds();
        if (root.startSelect >= 1) {
            var chosen = ids.slice(0, root.startSelect);
            root.selectedIds = chosen;
            root.selectedId = chosen[chosen.length - 1];
            root.anchorId = chosen[0];
        } else if (root.startRename) {
            root.selectedIds = [ids[0]];
            root.selectedId = ids[0];
            root.anchorId = ids[0];
            root.beginRename(ids[0]);
        } else if (root.startMenu === "item") {
            root.selectedIds = [ids[0]];
            root.selectedId = ids[0];
            root.openNodeMenu(ids[0], directory.uriAt(0), directory.isDirAt(0),
                              24, 140);
        } else if (root.startMenu === "background") {
            root.openBackgroundMenu(320, 220);
        } else if (root.startMenu === "edit") {
            root.openBackgroundMenu(320, 220);
        } else if (root.startMenu === "openWith") {
            // T334 capture: select the first file row and open the picker
            // (the nested synthetic harness cannot reliably right-click a Qt
            // client surface and walk a submenu).
            var fileId = 0;
            for (var f = 0; f < ids.length; ++f) {
                if (!directory.isDirAt(f)) {
                    fileId = ids[f];
                    break;
                }
            }
            if (fileId > 0) {
                root.selectedIds = [fileId];
                root.selectedId = fileId;
                root.openWithDialogFor(directory.uriAt(directory.rowForNodeId(fileId)));
            }
        } else if (root.startEmptyTrash) {
            root.confirmEmptyTrash();
        }
    }

    // Select the reveal target once its folder's listing has settled, so the
    // Dock's Show in Files / a Downloads-stack open lands on the exact row
    // (T-10.6c). Scrolls to it so an off-screen reveal is still visible.
    function applyReveal() {
        if (root.revealApplied || root.revealUri.length === 0 || !directory)
            return;
        if (directory.state !== "complete" || directory.count === 0)
            return;
        root.revealApplied = true;
        var ids = directory.allNodeIds();
        for (var i = 0; i < ids.length; ++i) {
            if (directory.uriAt(i) === root.revealUri) {
                root.selectedIds = [ids[i]];
                root.selectedId = ids[i];
                root.anchorId = ids[i];
                if (root.currentView === "list")
                    listView.scrollToNode(ids[i]);
                else
                    iconView.scrollToNode(ids[i]);
                break;
            }
        }
    }

    Component.onCompleted: {
        var start = Files.homeUri;
        // `DF_FILES_START_URI` opens a specific location for captures and
        // scripted checks; an unusable URI falls back to Home.
        if (root.startUri.length > 0 && Files.isBrowsable(root.startUri))
            start = root.startUri;
        root.tabs.openTab(start, Files.displayName(start));
        root.tabBarRevealedAt = root.tabs.count;
        root.syncSelection();
    }

    onSearchTextChanged: {
        if (searchField.text !== root.searchText)
            searchField.text = root.searchText;
    }

    Keys.onPressed: (event) => root.processShortcut(event)

    // The shell's command keys. Folders-scoped: the view calls this for a key
    // it did not use, and the shell's own Keys handler calls it directly.
    function processShortcut(event) {
        if (event.accepted)
            return;

        var command = event.modifiers & root.commandMask;
        switch (event.key) {
        case Qt.Key_Left:
            if (event.modifiers & Qt.AltModifier) {
                root.back();
                event.accepted = true;
            }
            break;
        case Qt.Key_Right:
            if (event.modifiers & Qt.AltModifier) {
                root.forward();
                event.accepted = true;
            }
            break;
        case Qt.Key_F:
            if (command) {
                root.focusSearch();
                event.accepted = true;
            }
            break;
        case Qt.Key_BracketLeft:
            if (event.modifiers & Qt.ControlModifier) {
                root.back();
                event.accepted = true;
            }
            break;
        case Qt.Key_BracketRight:
            if (event.modifiers & Qt.ControlModifier) {
                root.forward();
                event.accepted = true;
            }
            break;
        case Qt.Key_1:
        case Qt.Key_2:
        case Qt.Key_3:
        case Qt.Key_4:
        case Qt.Key_5:
        case Qt.Key_6:
        case Qt.Key_7:
        case Qt.Key_8:
        case Qt.Key_9:
            if (command && !(event.modifiers & Qt.ShiftModifier)) {
                root.activateTabNumber(event.key - Qt.Key_0);
                event.accepted = true;
            }
            break;
        case Qt.Key_A:
            if (command) {
                root.selectAll();
                event.accepted = true;
            }
            break;
        case Qt.Key_Z:
            if (command && !root.textEntryActive) {
                if (event.modifiers & Qt.ShiftModifier)
                    root.redo();
                else
                    root.undo();
                event.accepted = true;
            }
            break;
        case Qt.Key_N:
            if (command && (event.modifiers & Qt.ShiftModifier)) {
                // New Folder keeps its T-10.4c chord; the design's File menu
                // gives New Window the unshifted Cmd+N.
                root.makeFolder();
                event.accepted = true;
            } else if (command) {
                root.openNewWindow();
                event.accepted = true;
            }
            break;
        case Qt.Key_C:
            if (command) {
                root.copySelection();
                event.accepted = true;
            }
            break;
        case Qt.Key_X:
            if (command) {
                root.cutSelection();
                event.accepted = true;
            }
            break;
        case Qt.Key_V:
            if (command) {
                root.paste();
                event.accepted = true;
            }
            break;
        case Qt.Key_O:
            if (command) {
                root.openSelection();
                event.accepted = true;
            }
            break;
        case Qt.Key_I:
            if (command) {
                root.openGetInfo();
                event.accepted = true;
            }
            break;
        case Qt.Key_Tab:
            // Ctrl+Tab / Ctrl+Shift+Tab is the defined tab switch chord. It
            // stays off Super/Meta so the compositor's own window switcher is
            // never shadowed.
            if (event.modifiers & Qt.ControlModifier) {
                root.stepTab((event.modifiers & Qt.ShiftModifier) ? -1 : 1);
                event.accepted = true;
            }
            break;
        case Qt.Key_T:
            if (command && (event.modifiers & Qt.AltModifier)) {
                root.toggleToolbar();
                event.accepted = true;
            } else if (command) {
                if (event.modifiers & Qt.ShiftModifier)
                    root.toggleTabBar();
                else
                    root.newTab();
                event.accepted = true;
            }
            break;
        case Qt.Key_W:
            if (command) {
                root.closeTab();
                event.accepted = true;
            }
            break;
        case Qt.Key_Down:
            if (command) {
                root.openSelection();
                event.accepted = true;
            }
            break;
        case Qt.Key_Return:
        case Qt.Key_Enter:
            if (root.renamingId === 0 && root.selectedIds.length === 1) {
                root.beginRename(root.selectedId);
                event.accepted = true;
            }
            break;
        case Qt.Key_Delete:
        case Qt.Key_Backspace:
            if (command && (event.modifiers & Qt.ShiftModifier) && root.inTrash) {
                root.confirmEmptyTrash();
                event.accepted = true;
            } else if (root.selectedIds.length > 0) {
                root.trashSelection();
                event.accepted = true;
            }
            break;
        case Qt.Key_Escape:
            if (root.renamingId !== 0)
                root.cancelRename();
            else
                root.clearSelection();
            event.accepted = true;
            break;
        }
    }

    Binding {
        target: Theme
        property: "dark"
        value: root.colorScheme === "dark" ? true
             : root.colorScheme === "light" ? false : root.hostDark
    }
    Binding {
        target: Theme
        property: "accentOverride"
        value: Settings.values["appearance.accent"] || ""
    }
    Binding {
        target: Theme
        property: "highlightOverride"
        value: Settings.values["appearance.highlight"] || ""
    }
    Binding {
        target: Theme
        property: "scrollBars"
        value: Settings.values["appearance.scrollBars"] || "automatic"
    }
    Binding {
        target: Theme
        property: "scrollBarClick"
        value: Settings.values["appearance.scrollBarClick"] || "jump-to-next-page"
    }
    Binding {
        target: Theme
        property: "folderColor"
        value: Settings.values["appearance.folderColor"] || ""
    }
    Binding {
        target: Theme
        property: "reducedMotion"
        value: Settings.values["accessibility.reduceMotion"] === true
    }
    Binding {
        target: Glass
        property: "wallpaperTint"
        value: Settings.values["appearance.wallpaperTint"] !== false
    }
    Binding {
        target: Glass
        property: "rollout"
        value: root.glassRollout
    }
    Binding {
        target: Glass
        property: "slider"
        value: root.transparency
    }
    Binding {
        target: Glass
        property: "reduceTransparency"
        value: root.reduceTransparency
    }
    Binding {
        target: Glass
        property: "increaseContrast"
        value: root.increaseContrast
    }
    Binding {
        target: Glass
        property: "appChromeSupported"
        value: root.appChromeSupported
    }

    // T334: the window's one published menu model reads this live state. It is
    // the same `FilesMenu` object the broker receives, not a second list.
    Binding {
        target: FilesMenu
        property: "currentUri"
        value: root.currentUri
    }
    Binding {
        target: FilesMenu
        property: "currentView"
        value: root.currentView
    }
    Binding {
        target: FilesMenu
        property: "canGoBack"
        value: root.canGoBack
    }
    Binding {
        target: FilesMenu
        property: "canGoForward"
        value: root.canGoForward
    }
    Binding {
        target: FilesMenu
        property: "canUndo"
        value: root.canUndo
    }
    Binding {
        target: FilesMenu
        property: "canRedo"
        value: root.canRedo
    }
    Binding {
        target: FilesMenu
        property: "undoLabel"
        value: directory ? directory.undoLabel : ""
    }
    Binding {
        target: FilesMenu
        property: "redoLabel"
        value: directory ? directory.redoLabel : ""
    }
    Binding {
        target: FilesMenu
        property: "hasSelection"
        value: root.selectedIds.length > 0
    }
    Binding {
        target: FilesMenu
        property: "selectedCount"
        value: root.selectedIds.length
    }
    Binding {
        target: FilesMenu
        property: "selectionIsDir"
        value: root.infoSummary.isDir === true
    }
    Binding {
        target: FilesMenu
        property: "selectionUri"
        value: root.selectedFileUri
    }
    Binding {
        target: FilesMenu
        property: "canPaste"
        value: clipboard.count > 0
    }
    Binding {
        target: FilesMenu
        property: "inTrash"
        value: root.inTrash
    }
    Binding {
        target: FilesMenu
        property: "toolbarVisible"
        value: root.toolbarVisible
    }
    Binding {
        target: FilesMenu
        property: "tabBarVisible"
        value: root.tabBarVisible
    }

    // T333: the per-location view kinds (uri -> "icon" | "list"). Tabs read
    // this through `tabState`, so each tab remembers its own view.
    FilesBrowser {
        id: browser
        // `DF_FILES_START_VIEW` lets the capture harness open in either view;
        // the toolbar switch still owns the live choice.
        defaultView: Files.startView.length > 0 ? Files.startView : "icon"
    }

    AppWindow {
        id: appWindow
        anchors.fill: parent
        title: root.locationTitle
        active: root.windowActive
        // T230b: Files is a 52 pt unified-toolbar window. The style is
        // declared once here; the titlebar inherits it.
        windowStyle: "unified"
        // T206: the sidebar/toolbar carry the measured app-local alpha; the
        // content area and path bar stay opaque.
        chromeTransparent: true

        titleBarData: TitleBar {
            id: titleBar
            title: root.locationTitle
            active: root.windowActive
            // T230d: toolbar-hidden mode keeps a drag/traffic-light strip and
            // grows the chrome to the measured 64 pt content inset; the
            // toolbar controls and sidebar are removed together.
            extraToolbarRow: root.toolbarVisible
                ? 0
                : Theme.controls.files.toolbarHiddenInset
                  - WindowStyle.height("unified")
            fillColor: Material.appChromeFill("filesToolbar",
                                              Theme.color.chrome)
            borderColor: Material.appChromeBorder("filesToolbar",
                                                  Theme.color.border)
            leadingData: Row {
                visible: root.toolbarVisible
                spacing: Theme.controls.toolbar.spacing
                Button {
                    id: backButton
                    icon: "chevron-left"
                    variant: "ghost"
                    accessibleName: qsTr("Back")
                    enabled: root.canGoBack
                    onClicked: root.back()
                }
                Button {
                    id: forwardButton
                    icon: "chevron-right"
                    variant: "ghost"
                    accessibleName: qsTr("Forward")
                    enabled: root.canGoForward
                    onClicked: root.forward()
                }
                SegmentedControl {
                    id: viewControl
                    // Files's measured 164×52 pt group with four 40×36 pt
                    // segments. Icon and List are live; Column and Gallery
                    // are explicit disabled states until T244a lands them.
                    groupWidth: Theme.controls.files.viewGroupWidth
                    groupHeight: Theme.controls.files.viewGroupHeight
                    segmentWidth: Theme.controls.files.viewSegmentWidth
                    segmentHeight: Theme.controls.files.viewSegmentHeight
                    paddingX: Theme.controls.files.viewGroupPaddingX
                    paddingY: Theme.controls.files.viewGroupPaddingY
                    model: [
                        { label: qsTr("Icon View"), icon: "icon-view",
                          enabled: true },
                        { label: qsTr("List View"), icon: "list-view",
                          enabled: true },
                        { label: qsTr("Column View"), icon: "column-view",
                          enabled: false,
                          reason: qsTr("Column view arrives with T244a.") },
                        { label: qsTr("Gallery View"), icon: "gallery-view",
                          enabled: false,
                          reason: qsTr("Gallery view arrives with T244a.") }
                    ]
                    onActivated: (index) => root.setView(index === 1
                                                        ? "list" : "icon")
                    onRejected: (index, reason) => {
                        root.notice = reason;
                        noticeTimer.restart();
                    }
                }
            }
            trailingData: Row {
                spacing: 0
                // T331: the Edit menu's entry point (Undo/Redo/Cut/Copy/Paste/
                // Select All). The text row names the next undo step, so the
                // menu's promise is visible before it is opened.
                Button {
                    id: editMenuButton
                    objectName: "editMenuButton"
                    text: root.canUndo ? qsTr("Undo %1").arg(directory.undoLabel)
                                       : qsTr("Edit")
                    variant: "ghost"
                    accessibleName: qsTr("Edit menu")
                    onClicked: root.openEditMenuAtToolbar()
                }
                SearchField {
                id: searchField
                visible: root.toolbarVisible
                width: 220
                placeholderText: qsTr("Search")
                onTextChanged: root.searchText = searchField.text
                onCleared: root.searchText = ""
                }
            }
            onCloseRequested: root.closeRequested()
            onMinimizeRequested: root.minimizeRequested()
            onZoomRequested: root.zoomRequested()
            onMoveRequested: (x, y) => root.moveRequested(x, y)
            onMenuRequested: (x, y) => root.menuRequested(x, y)
        }

        Row {
            anchors.fill: parent
            spacing: 0

            // Sidebar: the Favorites and Locations source lists.
            ChromeSheet {
                id: sidebarPane
                // Files's measured 198 pt source-list width (T230d).
                width: Theme.controls.files.sidebarWidth
                height: parent.height
                visible: root.toolbarVisible
                color: Material.appChromeFill("filesSidebar",
                                             Theme.color.surfaceMuted)
                bottomLeftRadius: Theme.controls.window.radius

                Sidebar {
                    id: sidebar
                    anchors.fill: parent
                    anchors.margins: Theme.controls.sidebar.padding
                    sections: root.sidebarSections
                    // The outer ChromeSheet owns the app-local material.
                    fillColor: "transparent"

                    onActivated: (index, item) => {
                        var uri = root.uriForEntry(item);
                        if (uri)
                            root.navigate(uri);
                    }
                    onCurrentIndexChanged: {
                        // A click/Return moves the index and then emits
                        // `activated`; `activating` marks that move so the
                        // same location is not navigated twice. Arrow keys
                        // still follow the selection live.
                        if (root.syncing || sidebar.activating)
                            return;
                        var uri = root.uriForEntry(sidebar.entries[sidebar.currentIndex]);
                        if (uri)
                            root.navigate(uri);
                    }
                }
            }

            // Detail: the location's content, path bar and status bar (all
            // below the unified chrome).
            Rectangle {
                width: parent.width - (sidebarPane.visible ? sidebarPane.width : 0)
                height: parent.height
                color: "transparent"

                // The titlebar is the app-local unified toolbar (52 pt); the
                // listing behind it and the path bar below stay opaque. The
                // sheet owns the bottom-right window corner.
                ChromeSheet {
                    id: detailSheet
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    color: Theme.color.surface
                    bottomRightRadius: Theme.controls.window.radius
                }

                Column {
                    anchors.fill: parent
                    spacing: 0

                    // T333: the measured 702×28 pt tab bar, below the unified
                    // toolbar and above the path bar. It spans the detail
                    // column only (the sidebar starts at the toolbar bottom);
                    // it appears from the second tab, or when shifted open.
                    FilesTabBar {
                        id: tabBar
                        width: parent.width
                        visible: root.tabBarVisible
                        height: visible ? implicitHeight : 0
                        // `{ id, title }` rows; `tabState` re-reads the model
                        // on every tab-list change, so the labels track the
                        // locations as the user navigates.
                        tabs: root.tabState.tabRows
                        currentIndex: root.tabs.currentIndex
                        onTabSelected: (index) => root.activateTab(index)
                        onTabCloseRequested: (index) => root.closeTab(index)
                        onNewTabRequested: root.newTab()
                        onContextRequested: (index, x, y) => {
                            var p = tabBar.mapToItem(root, x, y);
                            root.openTabMenu(index, p.x, p.y);
                        }
                    }

                    // The location content: the files-core listing rendered
                    // by the icon or list view (T-10.4b). Both views stay
                    // instantiated and share the selection, so the view
                    // switch cannot lose it. The empty/error state sits above
                    // them once the listing settles.
                    Item {
                        id: contentArea
                        width: parent.width
                        height: Math.max(0, parent.height - tabBar.height
                                            - pathBar.height
                                            - statusBar.height)

                        FilesDirectoryModel {
                            id: directory
                            location: root.currentUri
                        }

                        FilesIconView {
                            id: iconView
                            anchors.fill: parent
                            visible: root.currentView !== "list"
                            directory: directory
                            selectedId: root.selectedId
                            selectedIds: root.selectedIds
                            renamingId: root.renamingId
                            onSelected: (nodeId, modifiers) => root.selectNode(nodeId, modifiers)
                            onActivated: (uri, isDir) => root.activateNode(uri, isDir)
                            onContextRequested: (nodeId, uri, isDir, x, y) => {
                                var p = iconView.mapToItem(root, x, y);
                                root.openNodeMenu(nodeId, uri, isDir, p.x, p.y);
                            }
                            onBackgroundContextRequested: (x, y) => {
                                var p = iconView.mapToItem(root, x, y);
                                root.openBackgroundMenu(p.x, p.y);
                            }
                            onMarqueeStarted: (modifiers) => root.beginMarquee(modifiers)
                            onMarqueeUpdated: (nodeIds, modifiers) => root.updateMarquee(nodeIds, modifiers)
                            onMarqueeSelected: (nodeIds, modifiers) => root.endMarquee(nodeIds, modifiers)
                            onRenameSubmitted: (nodeId, name) => root.commitRename(nodeId, name)
                            onRenameCancelled: root.cancelRename()
                            Keys.onPressed: (event) => root.forwardKey(event)
                        }

                        FilesListView {
                            id: listView
                            anchors.fill: parent
                            visible: root.currentView === "list"
                            directory: directory
                            selectedId: root.selectedId
                            selectedIds: root.selectedIds
                            renamingId: root.renamingId
                            onSelected: (nodeId, modifiers) => root.selectNode(nodeId, modifiers)
                            onActivated: (uri, isDir) => root.activateNode(uri, isDir)
                            onContextRequested: (nodeId, uri, isDir, x, y) => {
                                var p = listView.mapToItem(root, x, y);
                                root.openNodeMenu(nodeId, uri, isDir, p.x, p.y);
                            }
                            onBackgroundContextRequested: (x, y) => {
                                var p = listView.mapToItem(root, x, y);
                                root.openBackgroundMenu(p.x, p.y);
                            }
                            onRenameSubmitted: (nodeId, name) => root.commitRename(nodeId, name)
                            onRenameCancelled: root.cancelRename()
                            Keys.onPressed: (event) => root.forwardKey(event)
                        }

                        Column {
                            id: emptyState
                            anchors.centerIn: parent
                            width: Math.min(parent.width - 2 * Theme.primitive.spacing.xl, 360)
                            spacing: Theme.primitive.spacing.sm
                            visible: root.hasSearch
                                     || (directory.count === 0
                                         && directory.state !== "streaming")

                            Icon {
                                anchors.horizontalCenter: parent.horizontalCenter
                                name: root.hasSearch ? "search"
                                                     : (directory.state === "error"
                                                        ? "folder" : "folder")
                                size: Theme.primitive.spacing.xxl
                                color: Theme.color.textTertiary
                            }

                            Text {
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                text: root.hasSearch
                                      ? qsTr("Search \u201c%1\u201d in %2")
                                            .arg(root.searchText).arg(root.locationTitle)
                                      : (directory.state === "error"
                                         ? qsTr("Can\u2019t open %1").arg(root.locationTitle)
                                         : root.locationTitle)
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeLg
                                font.weight: Theme.primitive.font.weightMedium
                                elide: Text.ElideRight
                            }

                            Text {
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                text: root.hasSearch
                                      ? qsTr("Search results are coming later.")
                                      : (directory.state === "error"
                                         ? directory.errorMessage
                                         : qsTr("This folder is empty."))
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                wrapMode: Text.WordWrap
                            }
                        }

                        // The inline notice for a snapped-back operation.
                        Rectangle {
                            id: noticeBanner
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.bottom: parent.bottom
                            anchors.bottomMargin: Theme.primitive.spacing.md
                            width: noticeText.implicitWidth + 2 * Theme.primitive.spacing.md
                            height: noticeText.implicitHeight + 2 * Theme.primitive.spacing.sm
                            radius: Theme.primitive.radius.sm
                            color: Theme.color.surfaceElevated
                            border.width: Theme.controls.window.borderWidth
                            border.color: Theme.color.border
                            visible: root.notice.length > 0

                            Text {
                                id: noticeText
                                anchors.centerIn: parent
                                text: root.notice
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                        }
                    }

                    PathBar {
                        id: pathBar
                        width: parent.width
                        model: root.breadcrumb
                        bottomRightRadius: Theme.controls.window.radius
                        onNavigated: (uri) => root.navigate(uri)
                    }

                    // T230d: item count, selection count and the volume's
                    // free space. A 28 pt content-layer bar; when the toolbar
                    // is hidden it spans the full window width because the
                    // sidebar is gone.
                    StatusBar {
                        id: statusBar
                        width: parent.width
                        itemCount: directory ? directory.count : 0
                        selectionCount: root.selectedIds.length
                        freeSpaceText: root.freeSpaceText
                        pasteText: directory ? directory.pasteProgressText : ""
                    }
                }
            }
        }
    }

    // Clicking anywhere outside the menu dismisses it; the menu itself sits
    // above this layer, so its rows still receive the click.
    MouseArea {
        anchors.fill: parent
        z: contextMenu.z - 1
        visible: contextMenu.open
        acceptedButtons: Qt.AllButtons
        onPressed: contextMenu.hide()
    }

    // The one context menu (T-10.4c). Its entries come from the node or
    // background model; every action routes back through this shell.
    ContextMenu {
        id: contextMenu
        accessibleName: qsTr("Context menu")
        onTriggered: (index, item) => root.contextAction(item.action)
    }

    // T334: the Open With picker. The File menu's "Choose a Default
    // Application…" row opens the same dialog; its Set as Default button is
    // the write path.
    OpenWithDialog {
        id: openWithDialog
        fileName: root.openWithName
        mimeName: root.openWithMime
        handlers: root.openWithHandlers
        selectedId: root.openWithSelectedId
        notice: root.openWithNotice
        onOpenRequested: (id) => root.openWithSelected(id)
        onDefaultRequested: (id) => root.setDefaultOpenHandler(id)
    }

    // T230d: a disabled view segment explains itself on hover; a click also
    // raises the inline notice. No segment is a dead glyph.
    Tooltip {
        id: viewReasonTip
        anchorRect: {
            var p = viewControl.mapToItem(root, 0, 0);
            return { x: p.x, y: p.y, w: viewControl.width, h: viewControl.height };
        }
        placement: "below"
        bounds: root
        text: {
            var index = viewControl.hoveredIndex;
            if (index < 0 || index >= viewControl.entries.length)
                return "";
            var entry = viewControl.entries[index];
            return entry && !entry.enabled ? entry.reason : "";
        }
        open: text.length > 0
    }

    // T230d: the Get Info auxiliary window. It is a separate 328×1034 pt
    // window with a 32 pt title bar; its data follows the live selection.
    GetInfoWindow {
        id: getInfoWindow
        visible: root.getInfoOpen
        itemName: root.infoTitle
        itemKind: root.infoKind
        previewSource: root.infoPreviewSource
        previewIcon: root.infoPreviewIcon
        multiSelection: root.infoMultiSelection
        selectionCount: root.infoSelectionIds.length
        sections: root.infoSections
        onCloseRequested: root.getInfoOpen = false
        onRenameRequested: (name) => root.renameFromInfo(name)
    }

    // A failed operation records a message; show it briefly, then fade.
    Connections {
        target: directory
        function onLastErrorChanged() {
            if (directory.lastError.length > 0) {
                root.notice = directory.lastError;
                noticeTimer.restart();
            }
        }
        function onStateChanged() {
            root.applyCaptureSeam();
            root.applyReveal();
        }
        function onCountChanged() {
            root.directoryRevision += 1;
            root.applyCaptureSeam();
            root.applyReveal();
        }
        function onDataChanged() {
            root.directoryRevision += 1;
        }
        function onPasteFinished(copied, skipped, failed) {
            root.pasteFinished(copied, skipped, failed);
        }
    }

    Timer {
        id: noticeTimer
        interval: 4000
        onTriggered: root.notice = ""
    }

    // Keep the view switch in step with the location's persisted view state;
    // the control writes on activation, this re-reads it on navigation.
    Binding {
        target: viewControl
        property: "currentIndex"
        value: root.currentView === "list" ? 1 : 0
    }

    // T330: one conflict sheet for the whole paste. Replace / Skip / Keep Both
    // are the three policies files-core implements; "Apply to all" is the
    // sheet's scope (one decision for the batch), matching the design's
    // "one sheet, not a stack of modals".
    Dialog {
        id: pasteConflictDialog
        title: qsTr("An item already exists")
        message: {
            var names = root.pasteConflictNames;
            if (names.length === 1)
                return qsTr("“%1” already exists in this folder. "
                            + "Choose how to paste it.").arg(names[0]);
            return qsTr("%1 items already exist in this folder. "
                        + "Choose how to paste them.").arg(names.length);
        }
        onAccepted: root.performPaste(root.pastePolicy)
        onRejected: root.pasteConflictNames = []

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                id: pasteCancel
                text: qsTr("Cancel")
                variant: "ghost"
                onClicked: pasteConflictDialog.reject()
            }
            Button {
                id: pasteSkip
                text: qsTr("Skip")
                variant: "secondary"
                onClicked: {
                    root.pastePolicy = "skip";
                    pasteConflictDialog.accept();
                }
            }
            Button {
                id: pasteReplace
                text: qsTr("Replace")
                variant: "secondary"
                onClicked: {
                    root.pastePolicy = "replace";
                    pasteConflictDialog.accept();
                }
            }
            Button {
                id: pasteKeepBoth
                text: qsTr("Keep Both")
                variant: "primary"
                onClicked: {
                    root.pastePolicy = "keep-both";
                    pasteConflictDialog.accept();
                }
            }
        }
    }

    // Empty Trash is a confirmation, never a one-click destructive action
    // (09-files.md#trash). Accepting routes through `directory.emptyTrash`.
    Dialog {
        id: emptyTrashDialog
        title: qsTr("Empty Trash?")
        message: qsTr("The items in the Trash will be deleted immediately. "
                      + "This cannot be undone.")
        onAccepted: root.emptyTrashNow()

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                id: emptyTrashCancel
                text: qsTr("Cancel")
                variant: "ghost"
                onClicked: emptyTrashDialog.reject()
            }
            Button {
                id: emptyTrashConfirm
                text: qsTr("Empty Trash")
                variant: "primary"
                onClicked: emptyTrashDialog.accept()
            }
        }
    }

    // T331: the shortcut chords a focused text field owns. While one runs,
    // Cmd+Z edits the field's text, not the folder.
    readonly property bool textEntryActive: root.renamingId !== 0
            || (searchField && searchField.activeFocus)

    // Forward a key the views did not use to the shell's command handler, so
    // Cmd+Z / Cmd+C / … work while the content has the key focus (the views
    // own arrow keys and typing; they leave commands unconsumed).
    function forwardKey(event) {
        if (!event.accepted)
            root.processShortcut(event);
    }

    // The frameless window owns its edge input; the shared grips report the
    // matching `Qt.Edges` and the host forwards them to `startSystemResize`.
    WindowResizeGrips {
        id: resizeEdges
        anchors.fill: parent
        onResizeRequested: (edges) => root.resizeRequested(edges)
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Files")
}
