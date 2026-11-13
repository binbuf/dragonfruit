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
    // This surface runs in its own process; it must consume appearance and
    // accessibility settings independently of the Files window.
    Binding {
        target: Theme; property: "dark"
        value: Settings.values["appearance.colorScheme"] === "dark" ? true
             : Settings.values["appearance.colorScheme"] === "light" ? false
             : Application.styleHints.colorScheme === Qt.Dark
    }
    Binding {
        target: Theme; property: "reducedMotion"
        value: Settings.values["accessibility.reduceMotion"] === true
    }
    Binding {
        target: Glass; property: "reduceTransparency"
        value: Settings.values["accessibility.reduceTransparency"] === true
    }
    Binding {
        target: Glass; property: "increaseContrast"
        value: Settings.values["accessibility.increaseContrast"] === true
    }
    Binding {
        target: Glass; property: "appChromeSupported"
        value: Settings.appChromeSupported
    }
    Binding {
        target: Glass; property: "rollout"
        value: Settings.glassRollout || Theme.glass.rollout.defaultMode
    }
    Binding {
        target: Glass; property: "slider"
        value: Settings.values["appearance.transparency"] !== undefined
             ? Number(Settings.values["appearance.transparency"])
             : Theme.glass.slider.defaultPosition
    }
    Binding {
        target: Theme; property: "accentOverride"
        value: Settings.values["appearance.accent"] || ""
    }
    Binding {
        target: Theme; property: "highlightOverride"
        value: Settings.values["appearance.highlight"] || ""
    }

    // Desktop labels sit directly on the wallpaper, so they use the same
    // sampled luminance/polarity pair as the menu bar instead of the scheme's
    // app text colour.
    property real backgroundLuma: Settings.wallpaperLuminance

    // The menu bar's compositor veil is visual-only below its 28 pt strip, but
    // it still paints over the desktop band. Desktop icons must start below
    // the full veil extent; otherwise their labels read as ghosts through the
    // transparent bar. With the bar background off there is no veil, so only
    // the strip needs to be cleared.
    readonly property int menuBarVisualInset:
        Settings.values["menu.showBackground"] !== false
            ? Theme.glass.presets.menuBarVeil.extent
            : Theme.controls.menuBar.height

    // T227: `desktop.showItems`. "On Desktop" shows the Files icons;
    // "In Stage Manager" hides them for the Stage Manager host (T242).
    readonly property bool itemsVisible:
        Settings.values["desktop.showItems"] !== "in-stage"

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
    // extends, otherwise the band replaces (an empty band clears). The live
    // path snapshots the pre-drag selection so Cmd/Shift previews toggle from
    // a stable base while the pointer is still moving.
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
            { label: qsTr("New Folder"), shortcut: "Shift+Super+N",
              action: "newFolder" },
            { label: qsTr("Clean Up"), action: "cleanUp" }
        ];
    }

    function nodeMenu(node) {
        return [
            { label: qsTr("Open"), action: "open",
              enabled: node !== null }
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

    // Capture/demo seam (T202): open the background menu without a synthetic
    // pointer so the menu-family capture is deterministic.
    function openBackgroundMenuFixture() {
        root.openBackgroundMenu(root.width / 2, root.height / 2);
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
        case "cleanUp":
            if (root.iconView)
                root.iconView.cleanUp();
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
        anchors.topMargin: root.menuBarVisualInset
        visible: root.itemsVisible
        directory: directory
        selectedId: root.selectedId
        selectedIds: root.selectedIds
        desktopStyle: true
        hoverHighlight: false
        backgroundLuma: root.backgroundLuma
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
        onMarqueeStarted: (modifiers) => root.beginMarquee(modifiers)
        onMarqueeUpdated: (nodeIds, modifiers) => root.updateMarquee(nodeIds, modifiers)
        onMarqueeSelected: (nodeIds, modifiers) => root.endMarquee(nodeIds, modifiers)
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

    ContextMenu {
        id: contextMenu
        accessibleName: qsTr("Desktop menu")
        materialBackdrop: "desktop"
        onTriggered: (index, item) => root.contextAction(item.action)
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Desktop")
}
