// SPDX-License-Identifier: GPL-3.0-only
import QtQuick

// T268: a Loader with a per-pane cache.
//
// The plain `Loader` destroys its item on every `sourceComponent` change, so
// returning to a pane re-ran `Component.onCompleted`: every adapter refresh,
// every wallpaper preload, and every General sub-pane reset. This component
// keeps the instantiated item alive (parented to a hidden holder) and reparents
// it back on return, so side effects run once per process. A pane that needs
// fresh data asks for it explicitly through its own `refresh*()` entry point.
//
// The surface matches the Loader subset the shell uses: `item`, `status`
// (the `Loader.*` enum values), and the `loaded()` signal (so `onLoaded:`
// keeps working). `paneId` is the cache key; `componentFor(paneId)` resolves
// the body component.
Item {
    id: root

    property string paneId: ""
    // A function: paneId -> Component. Set by the shell to its one catalog.
    property var componentFor: null
    // The currently shown body, or null while none is selected.
    property Item item: null
    // `Loader.Null` / `Loader.Loading` / `Loader.Ready` / `Loader.Error`.
    property int status: Loader.Null
    // True when the last `loaded()` came from the cache rather than a first
    // construction; the shell's trace records it.
    property bool restoredFromCache: false
    // paneId -> Item. Not a QML property map: it is private bookkeeping and
    // never bound.
    property var cache: ({})
    property string shownPaneId: ""

    signal loaded()

    // Like Loader, follow the shown body's implicit size so the detail
    // Column lays the pane out at its content height.
    implicitWidth: item !== null ? item.implicitWidth : 0
    implicitHeight: item !== null ? item.implicitHeight : 0

    Item {
        id: host
        anchors.fill: parent
    }

    // Cached bodies stay instantiated and keep their bindings, but are out of
    // the render tree. Width follows the loader so a reparent does not reflow
    // every row twice.
    Item {
        id: cacheHolder
        width: root.width
        height: root.height
        visible: false
    }

    onPaneIdChanged: root.showPane()
    Component.onCompleted: root.showPane()

    function showPane() {
        if (root.paneId.length === 0) {
            root.item = null;
            root.status = Loader.Null;
            root.shownPaneId = "";
            return;
        }
        if (root.item !== null && root.shownPaneId === root.paneId)
            return;

        var cached = root.cache[root.paneId];
        if (cached !== undefined && cached !== null) {
            if (root.item !== null && root.item !== cached) {
                root.item.visible = false;
                root.item.parent = cacheHolder;
            }
            cached.parent = host;
            cached.visible = true;
            root.item = cached;
            root.status = Loader.Ready;
            root.restoredFromCache = true;
            root.shownPaneId = root.paneId;
            root.loaded();
            return;
        }

        // Park the outgoing body instead of destroying it.
        if (root.item !== null) {
            root.item.visible = false;
            root.item.parent = cacheHolder;
        }
        root.item = null;
        root.status = Loader.Loading;

        var component = root.componentFor !== null
                ? root.componentFor(root.paneId) : null;
        if (component === null || component === undefined) {
            root.status = Loader.Error;
            root.shownPaneId = root.paneId;
            return;
        }
        var created = component.createObject(host);
        if (created === null) {
            root.status = Loader.Error;
            root.shownPaneId = root.paneId;
            return;
        }
        root.cache[root.paneId] = created;
        root.item = created;
        root.status = Loader.Ready;
        root.restoredFromCache = false;
        root.shownPaneId = root.paneId;
        root.loaded();
    }
}
