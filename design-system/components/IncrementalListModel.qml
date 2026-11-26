// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick

// A `ListModel` that reconciles an owner's array by a stable key instead of
// resetting itself on every update.
//
// The shell publishes whole snapshots for folder stacks and the application
// picker. Handing that array straight to a `ListView` (or a `Repeater`)
// recreates every delegate whenever the array's identity changes, even when
// one file was renamed or one app was pinned. This model keeps the rows that
// still exist, updates only the roles that changed, and emits the matching
// `dataChanged`/`rowsInserted`/`rowsRemoved`/`rowsMoved` signals, so a list
// view reuses the delegates it already has.
//
// `keyRole` must identify a row across snapshots (a folder item's `path`, a
// picker row's `desktopId`). `roleNames` is the complete role set the owner
// may publish; it is used to give the model a stable schema even when the
// first snapshot omits a field.
ListModel {
    id: root

    // The owner's ordered snapshot. Assigning a new array starts a reconcile;
    // unchanged rows keep their delegates.
    property var source: []
    // The role that identifies a row across snapshots.
    property string keyRole: "id"
    // Every role the owner may publish, in addition to the keys found in the
    // first row. The order is not significant.
    property var roleNames: []
    // While false, source changes are ignored. A closed popover uses this to
    // avoid building a large model it cannot show yet; opening calls `sync()`.
    property bool live: true

    property bool _ready: false
    property bool _syncing: false

    onSourceChanged: {
        if (root._ready && root.live)
            root.sync();
    }
    onKeyRoleChanged: if (root._ready && root.live) root.sync();
    onRoleNamesChanged: if (root._ready && root.live) root.sync();
    onLiveChanged: if (root.live && root._ready) root.sync();

    Component.onCompleted: {
        root._ready = true;
        if (root.live)
            root.sync();
    }

    function _rowKey(row) {
        if (row === undefined || row === null)
            return "";
        var key = row[root.keyRole];
        return key === undefined || key === null ? "" : String(key);
    }

    function _normalized(row) {
        var out = {};
        // Seed the configured schema first so the first insert fixes every
        // role the owner may use later.
        for (var i = 0; i < root.roleNames.length; ++i)
            out[root.roleNames[i]] = row !== undefined && row !== null
                                     ? row[root.roleNames[i]] : undefined;
        if (row !== undefined && row !== null) {
            for (var key in row) {
                if (out[key] === undefined)
                    out[key] = row[key];
            }
        }
        return out;
    }

    // Reconcile the model with `source` in place. Removals run from the end so
    // the surviving rows keep their indices; then each target row is inserted,
    // moved, or updated in order. No `modelReset` is emitted.
    function sync() {
        if (root._syncing)
            return;
        root._syncing = true;
        try {
            root._sync();
        } finally {
            root._syncing = false;
        }
    }

    function _sync() {
        var src = root.source !== undefined && root.source !== null
                  ? root.source : [];
        var i;

        // Drop rows whose identity disappeared from the snapshot.
        var keep = {};
        for (i = 0; i < src.length; ++i)
            keep[root._rowKey(src[i])] = true;
        for (i = root.count - 1; i >= 0; --i) {
            if (keep[root._rowKey(root.get(i))] !== true)
                root.remove(i);
        }

        // Index the surviving rows once. The map is maintained through the
        // moves and inserts below, so this stays linear in the row count.
        var indexByKey = {};
        for (i = 0; i < root.count; ++i)
            indexByKey[root._rowKey(root.get(i))] = i;

        for (i = 0; i < src.length; ++i) {
            var target = src[i];
            var key = root._rowKey(target);
            if (indexByKey[key] !== undefined) {
                var current = indexByKey[key];
                if (current !== i) {
                    root.move(current, i, 1);
                    // The move shifts every row between the old and new
                    // position; refresh that whole window once.
                    var lo = Math.min(current, i);
                    var hi = Math.max(current, i);
                    for (var m = lo; m <= hi; ++m)
                        indexByKey[root._rowKey(root.get(m))] = m;
                    indexByKey[key] = i;
                }
                // Update only the roles that actually changed. A `dataChanged`
                // keeps the delegate and its hover/press state.
                var stored = root.get(i);
                for (var role in target) {
                    if (stored[role] !== target[role])
                        root.setProperty(i, role, target[role]);
                }
            } else {
                root.insert(i, root._normalized(target));
                indexByKey[key] = i;
                for (var p = i + 1; p < root.count; ++p)
                    indexByKey[root._rowKey(root.get(p))] = p;
            }
        }
    }
}
