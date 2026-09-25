// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files app shell (T-10.4a): the window chrome (titlebar + traffic
// lights), the sidebar with the Favorites/Locations sections, the toolbar
// (back/forward, the view switch, local search), and the footer path bar.
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
// Shift+Cmd+N makes a folder; Delete trashes; Return renames.
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
    property bool captureApplied: false

    property alias browser: browser
    property alias sidebar: sidebar
    property alias toolbar: toolbar
    property alias titleBar: titleBar
    property alias appWindow: appWindow
    property alias searchField: searchField
    property alias backButton: backButton
    property alias forwardButton: forwardButton
    property alias viewControl: viewControl
    property alias pathBar: pathBar
    property alias emptyState: emptyState
    property alias directory: directory
    property alias iconView: iconView
    property alias listView: listView
    property alias contextMenu: contextMenu

    signal closeRequested()
    signal minimizeRequested()
    signal zoomRequested()
    signal moveRequested(real x, real y)
    signal menuRequested(real x, real y)

    readonly property string locationTitle: Files.displayName(browser.currentUri)
    readonly property var breadcrumb: Files.breadcrumb(browser.currentUri)
    readonly property bool hasSearch: root.searchText.length > 0
    // Cmd is Super/Mod4 system-wide, but Qt desktop tests and muscle memory
    // both use Control; accept either as "command".
    readonly property int commandMask: Qt.ControlModifier | Qt.MetaModifier

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

    function activateNode(uri, isDir) {
        if (isDir && uri)
            browser.navigate(uri);
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

    // The Finder rule: the menu depends on what is under the pointer.
    function nodeMenu(node) {
        if (root.selectedIds.length > 1) {
            return [
                { label: qsTr("Move to Trash"), shortcut: qsTr("Delete"),
                  action: "trash" }
            ];
        }
        var entries = [
            { label: qsTr("Open"), action: "open", enabled: node ? node.isDir : false }
        ];
        if (node && node.isDir) {
            entries.push({ label: qsTr("Open in New Window"), action: "noop",
                           enabled: false });
        }
        entries.push({ type: "separator" });
        entries.push({ label: qsTr("Rename"), shortcut: qsTr("Return"),
                       action: "rename" });
        entries.push({ label: qsTr("Move to Trash"), shortcut: qsTr("Delete"),
                       action: "trash" });
        return entries;
    }

    function backgroundMenu() {
        return [
            { label: qsTr("New Folder"), shortcut: qsTr("Shift+Cmd+N"),
              action: "newFolder" },
            { type: "separator" },
            { label: qsTr("Select All"), shortcut: qsTr("Cmd+A"),
              action: "selectAll" }
        ];
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
        case "selectAll":
            root.selectAll();
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
        }
    }

    Component.onCompleted: {
        var start = Files.homeUri;
        // `DF_FILES_START_URI` opens a specific location for captures and
        // scripted checks; an unusable URI falls back to Home.
        if (Files.startUri.length > 0 && Files.isBrowsable(Files.startUri))
            start = Files.startUri;
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
        case Qt.Key_Return:
        case Qt.Key_Enter:
            if (root.renamingId === 0 && root.selectedIds.length === 1) {
                root.beginRename(root.selectedId);
                event.accepted = true;
            }
            break;
        case Qt.Key_Delete:
        case Qt.Key_Backspace:
            if (root.selectedIds.length > 0) {
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
        active: true

        titleBarData: TitleBar {
            id: titleBar
            title: root.locationTitle
            active: true
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
            Rectangle {
                id: sidebarPane
                width: 240
                height: parent.height
                color: Theme.color.surfaceMuted

                Sidebar {
                    id: sidebar
                    anchors.fill: parent
                    anchors.margins: Theme.controls.sidebar.padding
                    sections: root.sidebarSections

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

            // Detail: toolbar, the location's content, and the path bar.
            Rectangle {
                width: parent.width - sidebarPane.width
                height: parent.height
                color: Theme.color.surface

                Column {
                    anchors.fill: parent
                    spacing: 0

                    Toolbar {
                        id: toolbar
                        width: parent.width

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
                            model: [
                                { label: qsTr("Icon View"), icon: "icon-view" },
                                { label: qsTr("List View"), icon: "list-view" }
                            ]
                            onActivated: (index) => browser.setView(index === 1
                                                                    ? "list" : "icon")
                        }

                        trailingData: SearchField {
                            id: searchField
                            width: 220
                            placeholderText: qsTr("Search")
                            onTextChanged: root.searchText = searchField.text
                            onCleared: root.searchText = ""
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
                        height: Math.max(0, parent.height - toolbar.height - pathBar.height)

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
                        onNavigated: (uri) => browser.navigate(uri)
                    }
                }
            }
        }
    }

    // The one context menu (T-10.4c). Its entries come from the node or
    // background model; every action routes back through this shell.
    ContextMenu {
        id: contextMenu
        accessibleName: qsTr("Context menu")
        onTriggered: (index, item) => root.contextAction(item.action)
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
        function onStateChanged() { root.applyCaptureSeam(); }
        function onCountChanged() { root.applyCaptureSeam(); }
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

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Files")
}