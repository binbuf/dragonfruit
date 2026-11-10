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
// the Finder-style context menus, and inline rename. Every mutation routes
// through the files-core operations worker, which paints optimistically and
// then confirms or snaps back — never `std::fs` in the UI.
//
// Keyboard: Up/Down/Home/End/Return on the sidebar; Alt+Left/Alt+Right step
// history; Ctrl+F focuses search; Escape clears it; Cmd+A selects all;
// Cmd+O/Cmd+Down opens the selection; Shift+Cmd+N makes a folder; Delete
// trashes; Return renames.
Item {
    id: root

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
    // A transient inline notice (a snapped-back operation).
    property string notice: ""
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
    // T230d: Finder's toolbar-hidden mode. Hiding the toolbar also hides the
    // sidebar; the content inset grows to the measured 64 pt and the status
    // bar spans the full window width. A mode change, not a visibility flag.
    property bool toolbarVisible: true
    // The Get Info auxiliary window is open. It follows the live selection.
    property bool getInfoOpen: false
    // Bumped on every model repaint so the inspector's data binding re-reads
    // the current row (rename, watcher update, optimistic edit).
    property int directoryRevision: 0

    property alias browser: browser
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
    property alias emptyTrashDialog: emptyTrashDialog
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
    // A window-edge press. `edges` is a `Qt.Edges` flag; the host window
    // forwards it to `startSystemResize`.
    signal resizeRequested(int edges)

    readonly property string locationTitle: Files.displayName(browser.currentUri)
    readonly property var breadcrumb: Files.breadcrumb(browser.currentUri)
    readonly property bool hasSearch: root.searchText.length > 0
    // Whether the window is at the Trash location, which owns Empty Trash.
    readonly property bool inTrash: browser.currentUri === Files.trashUri
    // T230d: the measured Finder content inset. Visible: the 52 pt unified
    // toolbar. Hidden: the 64 pt content start (the toolbar is a mode, and the
    // sidebar goes with it).
    readonly property real contentInset: root.toolbarVisible
        ? WindowStyle.height("unified")
        : Theme.controls.files.toolbarHiddenInset
    // The status bar's volume free space. A missing volume is an empty label,
    // never an invented number.
    readonly property string freeSpaceText: {
        var info = Files.volumeInfo(root.browser.currentUri);
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
        sidebar.currentIndex = root.entryIndexForUri(browser.currentUri);
        root.syncing = false;
    }

    function focusSearch() {
        searchField.forceActiveFocus();
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
    // order — a Finder Shift-click range.
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
                reason: qsTr("Choosing a default application is not implemented.")
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
            browser.navigate(uri);
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
        directory.newFolder(browser.currentUri);
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

    // The Finder rule: the menu depends on what is under the pointer.
    function nodeMenu(node) {
        if (root.selectedIds.length > 1) {
            return [
                { label: qsTr("Get Info"), shortcut: "Super+I",
                  action: "getInfo" },
                { label: qsTr("Move to Trash"), shortcut: "Delete",
                  action: "trash" }
            ];
        }
        var entries = [
            { label: qsTr("Open"), action: "open", enabled: node !== null },
            { label: qsTr("Get Info"), shortcut: "Super+I",
              action: "getInfo", enabled: node !== null }
        ];
        if (node && node.isDir) {
            entries.push({ label: qsTr("Open in New Window"), action: "noop",
                           enabled: false });
        }
        entries.push({ type: "separator" });
        entries.push({ label: qsTr("Rename"), shortcut: "Return",
                       action: "rename" });
        entries.push({ label: qsTr("Move to Trash"), shortcut: "Delete",
                       action: "trash" });
        return entries;
    }

    function backgroundMenu() {
        var toolbarRow = { label: root.toolbarVisible ? qsTr("Hide Toolbar")
                                                       : qsTr("Show Toolbar"),
                           shortcut: "Ctrl+Alt+T",
                           action: "toggleToolbar" };
        if (root.inTrash) {
            return [
                { label: qsTr("Empty Trash"), shortcut: "Shift+Super+Delete",
                  action: "emptyTrash", enabled: directory.count > 0 },
                { type: "separator" },
                toolbarRow,
                { label: qsTr("Select All"), shortcut: "Super+A",
                  action: "selectAll" }
            ];
        }
        var rows = [
            { label: qsTr("New Folder"), shortcut: "Shift+Super+N",
              action: "newFolder" }
        ];
        // Free placement is an icon-view rule; a list has no icon slots to
        // clean up, so the row would be a dead affordance there.
        if (browser.currentView !== "list")
            rows.push({ label: qsTr("Clean Up"), action: "cleanUp" });
        rows.push({ type: "separator" });
        rows.push(toolbarRow);
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
        contextMenu.model = root.backgroundMenu();
        contextMenu.showAt(x, y);
    }

    function contextAction(action) {
        switch (action) {
        case "open":
            if (root.contextNode)
                root.activateNode(root.contextNode.uri, root.contextNode.isDir);
            break;
        case "rename":
            if (root.contextNode)
                root.beginRename(root.contextNode.id);
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
        if (root.startSelect > 1) {
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
                if (browser.currentView === "list")
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
        browser.reset(start);
        root.syncSelection();
    }

    onSearchTextChanged: {
        if (searchField.text !== root.searchText)
            searchField.text = root.searchText;
    }

    Keys.onPressed: (event) => {
        var command = event.modifiers & root.commandMask;
        switch (event.key) {
        case Qt.Key_Left:
            if (event.modifiers & Qt.AltModifier) {
                browser.back();
                event.accepted = true;
            }
            break;
        case Qt.Key_Right:
            if (event.modifiers & Qt.AltModifier) {
                browser.forward();
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
                browser.back();
                event.accepted = true;
            }
            break;
        case Qt.Key_BracketRight:
            if (event.modifiers & Qt.ControlModifier) {
                browser.forward();
                event.accepted = true;
            }
            break;
        case Qt.Key_A:
            if (command) {
                root.selectAll();
                event.accepted = true;
            }
            break;
        case Qt.Key_N:
            if (command && (event.modifiers & Qt.ShiftModifier)) {
                root.makeFolder();
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
        case Qt.Key_T:
            if (command && (event.modifiers & Qt.AltModifier)) {
                root.toggleToolbar();
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

    FilesBrowser {
        id: browser
        // `DF_FILES_START_VIEW` lets the capture harness open in either view;
        // the toolbar switch still owns the live choice.
        defaultView: Files.startView.length > 0 ? Files.startView : "icon"
        onCurrentUriChanged: {
            // Node ids are per-listing; a selection from another folder is
            // meaningless here.
            root.clearSelection();
            root.cancelRename();
            root.syncSelection();
        }
        onNavigated: root.syncSelection()
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
                    enabled: browser.canGoBack
                    onClicked: browser.back()
                }
                Button {
                    id: forwardButton
                    icon: "chevron-right"
                    variant: "ghost"
                    accessibleName: qsTr("Forward")
                    enabled: browser.canGoForward
                    onClicked: browser.forward()
                }
                SegmentedControl {
                    id: viewControl
                    // Finder's measured 164×52 pt group with four 40×36 pt
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
                    onActivated: (index) => browser.setView(index === 1
                                                            ? "list" : "icon")
                    onRejected: (index, reason) => {
                        root.notice = reason;
                        noticeTimer.restart();
                    }
                }
            }
            trailingData: SearchField {
                id: searchField
                visible: root.toolbarVisible
                width: 220
                placeholderText: qsTr("Search")
                onTextChanged: root.searchText = searchField.text
                onCleared: root.searchText = ""
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
                // Finder's measured 198 pt source-list width (T230d).
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
                            browser.navigate(uri);
                    }
                    onCurrentIndexChanged: {
                        if (root.syncing)
                            return;
                        var uri = root.uriForEntry(sidebar.entries[sidebar.currentIndex]);
                        if (uri)
                            browser.navigate(uri);
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

                    // The location content: the files-core listing rendered
                    // by the icon or list view (T-10.4b). Both views stay
                    // instantiated and share the selection, so the view
                    // switch cannot lose it. The empty/error state sits above
                    // them once the listing settles.
                    Item {
                        id: contentArea
                        width: parent.width
                        height: Math.max(0, parent.height - pathBar.height
                                            - statusBar.height)

                        FilesDirectoryModel {
                            id: directory
                            location: browser.currentUri
                        }

                        FilesIconView {
                            id: iconView
                            anchors.fill: parent
                            visible: browser.currentView !== "list"
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
                        }

                        FilesListView {
                            id: listView
                            anchors.fill: parent
                            visible: browser.currentView === "list"
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
                        onNavigated: (uri) => browser.navigate(uri)
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
        value: browser.currentView === "list" ? 1 : 0
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
