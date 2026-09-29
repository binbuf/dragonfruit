// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files-owned desktop surface (T-19.3): `~/Desktop` rendered as an icon
// view over the compositor's `background` layer, below windows and above the
// wallpaper. It is the same core, semantics, and selection model as a Files
// window — `FilesDirectoryModel` + `FilesIconView` — but it is its own process
// and its own crash domain.
//
// This first slice is render + select + open: click selects, Cmd/Shift extend,
// a rubber-band drag over the background selects the enclosed tiles, and a
// double-click opens. The only mutation is the background `New Folder`, routed
// through the files-core operations engine (never the filesystem directly).
FocusScope {
    id: root

    // The `file://` directory the desktop lists, set by `FilesDesktop` from
    // `FilesArguments.filesDesktopDirectory()` (xdg-user-dirs).
    property string location: ""

    // Selection is the Files shell's set of stable node ids (never paths).
    property real selectedId: 0
    property var selectedIds: []
    property real anchorId: 0
    // The node a node-menu acts on, or null for the background.
    property var contextNode: null

    // The desktop wants to open `uri` (directory → a Files window; file → the
    // default handler). The process that owns launching wires this.
    signal openRequested(string uri, bool isDir)

    property alias directory: directory
    property alias iconView: iconView
    property alias contextMenu: contextMenu

    // Cmd is Super/Mod4 system-wide; Qt desktop tests use Control. Accept
    // either, exactly like the Files window.
    readonly property int commandMask: Qt.ControlModifier | Qt.MetaModifier

    function isSelected(nodeId) {
        return root.selectedIds.indexOf(nodeId) >= 0;
    }

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

    // Rubber band: the view reports the enclosed ids; Cmd toggles, Shift
    // extends, otherwise the band replaces (an empty band clears).
    function selectMarquee(nodeIds, modifiers) {
        var command = modifiers & root.commandMask;
        var shift = modifiers & Qt.ShiftModifier;
        var ids = root.selectedIds.slice();
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

    // The minimal desktop background menu (T-19.3): open the folder in a Files
    // window, or make a folder through the ops engine. Richer desktop actions
    // are a later slice of legacy T-19.
    function backgroundMenu() {
        return [
            { label: qsTr("Open in Files"), action: "openInFiles" },
            { type: "separator" },
            { label: qsTr("New Folder"), shortcut: qsTr("Shift+Cmd+N"),
              action: "newFolder" }
        ];
    }

    function nodeMenu(node) {
        return [
            { label: qsTr("Open"), action: "open",
              enabled: node ? node.isDir : false }
        ];
    }

    function openBackgroundMenu(x, y) {
        root.contextNode = null;
        contextMenu.model = root.backgroundMenu();
        contextMenu.showAt(x, y);
    }

    function openNodeMenu(nodeId, uri, isDir, x, y) {
        root.contextNode = { id: nodeId, uri: uri, isDir: isDir };
        contextMenu.model = root.nodeMenu(root.contextNode);
        contextMenu.showAt(x, y);
    }

    function contextAction(action) {
        switch (action) {
        case "openInFiles":
            root.openRequested(root.location, true);
            break;
        case "open":
            if (root.contextNode)
                root.openRequested(root.contextNode.uri, root.contextNode.isDir);
            break;
        case "newFolder":
            if (directory)
                directory.newFolder(root.location);
            break;
        case "selectAll":
            root.selectAll();
            break;
        }
    }

    Keys.onPressed: (event) => {
        var command = event.modifiers & root.commandMask;
        switch (event.key) {
        case Qt.Key_A:
            if (command) {
                root.selectAll();
                event.accepted = true;
            }
            break;
        case Qt.Key_N:
            if (command && (event.modifiers & Qt.ShiftModifier)) {
                if (directory)
                    directory.newFolder(root.location);
                event.accepted = true;
            }
            break;
        case Qt.Key_Delete:
        case Qt.Key_Backspace:
            if (root.selectedIds.length > 0) {
                var ids = root.selectedIds.slice();
                for (var i = 0; i < ids.length; ++i)
                    directory.trash(ids[i]);
                root.clearSelection();
                event.accepted = true;
            }
            break;
        case Qt.Key_Escape:
            root.clearSelection();
            event.accepted = true;
            break;
        }
    }

    FilesDirectoryModel {
        id: directory
        location: root.location
    }

    FilesIconView {
        id: iconView
        anchors.fill: parent
        directory: directory
        selectedId: root.selectedId
        selectedIds: root.selectedIds
        onSelected: (nodeId, modifiers) => root.selectNode(nodeId, modifiers)
        onActivated: (uri, isDir) => root.openRequested(uri, isDir)
        onContextRequested: (nodeId, uri, isDir, x, y) => {
            var p = iconView.mapToItem(root, x, y);
            root.openNodeMenu(nodeId, uri, isDir, p.x, p.y);
        }
        onBackgroundContextRequested: (x, y) => {
            var p = iconView.mapToItem(root, x, y);
            root.openBackgroundMenu(p.x, p.y);
        }
        onMarqueeSelected: (nodeIds, modifiers) => root.selectMarquee(nodeIds, modifiers)
    }

    ContextMenu {
        id: contextMenu
        accessibleName: qsTr("Desktop menu")
        onTriggered: (index, item) => root.contextAction(item.action)
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Desktop")
}