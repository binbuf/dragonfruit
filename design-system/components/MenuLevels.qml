// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The shared menu-level state for one menu tree at arbitrary supported depth.
//
// `ContextMenu` and `MenuBarMenu` both own one of these instead of keeping a
// second copy of the open-submenu and highlight rules. A menu is a chain of
// levels: level 0 is the root list, level 1 is the submenu opened from the
// root, and so on. `openPath` holds the stable row key that opened each level;
// `highlightKeys` holds the stable row key selected on every visible level.
//
// Stable keys follow a row across live model updates, and `path` on every
// normalized entry records its ancestor keys. Numeric indices remain available
// for rendering and for the existing component API, but they are never the
// identity that a press, a keyboard move, or a late model update follows.
QtObject {
    id: root

    // The raw root list (`{ label, type, enabled, submenu, ... }`).
    property var model: []
    // Whether the owning menu is logically open. Hover dwell and type-ahead
    // do nothing while it is closed.
    property bool menuOpen: false
    // Stable row keys of the open submenu chain, one per open submenu.
    property var openPath: []
    // Stable row keys selected on each visible level (length openPath+1).
    property var highlightKeys: []
    // Last numeric position per level. Kept so a row that disappears moves the
    // selection to the nearest surviving enabled row, exactly like the
    // single-level context-menu reconciliation did.
    property var highlightIndices: []

    property string typeAheadBuffer: ""
    // The row that armed the submenu dwell. A row that disappears before the
    // timer fires must not open its replacement.
    property int pendingLevel: -1
    property int pendingIndex: -1
    property string pendingKey: ""
    // The level whose pointer just left a row. A zero-delay timer clears the
    // stale highlight unless another row (or a submenu) claims the pointer
    // first; that is what stops the last hovered row staying lit after the
    // pointer leaves the menu.
    property int hoverLeaveLevel: -1

    signal activated(var item, var entry, var ancestors)
    signal stateChanged()
    signal submenuChanged(int level, bool opened)

    // The raw list behind each level. A level exists only while every key on
    // the path resolves; a removed ancestor truncates the chain.
    readonly property var levelModels: {
        var out = [root.model || []];
        for (var level = 0; level < root.openPath.length; ++level) {
            var entries = root.normalizeList(out[level]);
            var at = root.indexOfKey(entries, root.openPath[level]);
            var opener = entries[at];
            if (!opener || !opener.enabled || opener.type !== "submenu")
                break;
            out.push(opener.submenu);
        }
        return out;
    }
    // Normalized entries per visible level (same shape as the old `entries`).
    // Every entry carries its stable key, its level, and the ancestor path of
    // stable keys that opened it, so a press or a late update can name the
    // exact command rather than a numeric row.
    readonly property var levelEntries: {
        var models = root.levelModels;
        var out = [];
        var ancestors = [];
        for (var level = 0; level < models.length; ++level) {
            var entries = root.normalizeList(models[level]);
            for (var i = 0; i < entries.length; ++i) {
                entries[i].level = level;
                entries[i].path = ancestors.concat([entries[i].key]);
            }
            out.push(entries);
            if (level < root.openPath.length)
                ancestors = ancestors.concat([root.openPath[level]]);
        }
        return out;
    }
    readonly property int levelCount: root.levelEntries.length
    // The deepest visible level: where keyboard navigation and type-ahead act.
    readonly property int deepestLevel: root.levelCount - 1

    readonly property int dwellInterval: dwellTimer.interval

    // --- Identity ---------------------------------------------------------

    function stableKey(item, index) {
        var m = item || {};
        if (m.key !== undefined && m.key !== null && String(m.key) !== "")
            return String(m.key);
        if (m.id !== undefined && m.id !== null && String(m.id) !== "")
            return String(m.id);
        if (m.action !== undefined && m.action !== null && String(m.action) !== "") {
            var parts = [String(m.action)];
            var payload = m.payload;
            if (payload && typeof payload === "object") {
                var names = ["windowId", "desktopId", "appId", "path",
                             "target", "position"];
                for (var i = 0; i < names.length; ++i) {
                    if (payload[names[i]] !== undefined
                            && payload[names[i]] !== null)
                        parts.push(names[i] + "=" + String(payload[names[i]]));
                }
            }
            return parts.join("|");
        }
        if (m.type === "separator")
            return "separator";
        return String(m.type || "item") + "|" + String(m.label || "");
    }

    // Normalize one raw list. Duplicate stable keys get an occurrence suffix so
    // every row on a level has a unique identity.
    function normalizeList(list) {
        var out = [];
        if (!list)
            return out;
        var seen = ({});
        for (var i = 0; i < list.length; ++i) {
            var m = list[i] || {};
            var base = root.stableKey(m, i);
            var key = base;
            var occurrence = 0;
            while (seen[key] === true) {
                occurrence += 1;
                key = base + "#" + occurrence;
            }
            seen[key] = true;
            out.push({
                index: i,
                key: key,
                type: m.type === "separator" ? "separator"
                      : (m.type === "submenu" ? "submenu" : "item"),
                label: m.label || "",
                shortcut: m.shortcut || "",
                enabled: m.enabled !== false,
                disabledReason: m.disabledReason || "",
                checked: m.checked === true,
                checkable: m.checkable === true || m.checked === true,
                keepOpen: m.keepOpen === true,
                submenu: m.submenu || m.items || []
            });
        }
        return out;
    }

    function keyAt(list, index) {
        return index >= 0 && index < list.length ? list[index].key : "";
    }

    function indexOfKey(list, key) {
        if (key === "" || !list)
            return -1;
        for (var i = 0; i < list.length; ++i) {
            if (list[i].key === key)
                return i;
        }
        return -1;
    }

    function isSelectable(entry) {
        return entry !== undefined && entry.type !== "separator"
                && entry.enabled !== false;
    }

    function nearestSelectable(list, preferred) {
        if (!list || list.length === 0)
            return -1;
        var start = Math.max(0, Math.min(list.length - 1, preferred));
        for (var distance = 0; distance < list.length; ++distance) {
            var after = start + distance;
            if (after < list.length && root.isSelectable(list[after]))
                return after;
            var before = start - distance;
            if (distance > 0 && before >= 0 && root.isSelectable(list[before]))
                return before;
        }
        return -1;
    }

    // The stable ancestor path of a row on `level`, ending with its own key.
    function pathAt(level, index) {
        var list = root.levelEntries[level] || [];
        var entry = index >= 0 && index < list.length ? list[index] : null;
        if (entry && entry.path)
            return entry.path;
        var key = root.keyAt(list, index);
        return root.openPath.slice(0, level).concat(key !== "" ? [key] : []);
    }

    // --- Derived positions ------------------------------------------------

    function openIndexAt(level) {
        if (level < 0 || level >= root.openPath.length)
            return -1;
        return root.indexOfKey(root.levelEntries[level] || [],
                               root.openPath[level]);
    }

    function highlightIndexAt(level) {
        if (level < 0 || level >= root.highlightKeys.length)
            return -1;
        return root.indexOfKey(root.levelEntries[level] || [],
                               root.highlightKeys[level]);
    }

    // --- Selection and open state ----------------------------------------

    function setHighlight(level, index) {
        if (level < 0)
            return;
        var list = root.levelEntries[level] || [];
        var keys = root.highlightKeys.slice();
        var indices = root.highlightIndices.slice();
        while (keys.length <= level) {
            keys.push("");
            indices.push(-1);
        }
        keys[level] = index >= 0 && index < list.length
                      ? root.keyAt(list, index) : "";
        indices[level] = index;
        root.highlightKeys = keys;
        root.highlightIndices = indices;
        root.stateChanged();
    }

    function move(level, delta) {
        root.cancelPending();
        var list = root.levelEntries[level] || [];
        if (list.length === 0)
            return;
        var i = level < root.highlightIndices.length
                ? root.highlightIndices[level] : -1;
        if (i < -1)
            i = -1;
        for (var step = 0; step < list.length; ++step) {
            i = (i + delta + list.length) % list.length;
            if (root.isSelectable(list[i])) {
                root.setHighlight(level, i);
                return;
            }
        }
    }

    function openAt(level, index) {
        var list = root.levelEntries[level] || [];
        var e = list[index];
        if (!e || e.type !== "submenu" || !e.enabled)
            return false;
        root.cancelPending();
        var path = root.openPath.slice(0, level);
        path.push(e.key);
        var keys = root.highlightKeys.slice(0, level + 1);
        var indices = root.highlightIndices.slice(0, level + 1);
        keys[level] = e.key;
        indices[level] = index;
        var child = root.normalizeList(e.submenu || []);
        var first = root.nearestSelectable(child, 0);
        keys.push(root.keyAt(child, first));
        indices.push(first);
        root.openPath = path;
        root.highlightKeys = keys;
        root.highlightIndices = indices;
        root.submenuChanged(level, true);
        root.stateChanged();
        return true;
    }

    // Close the submenu opened from `level` (and everything deeper).
    function closeFrom(level) {
        if (level < 0 || level >= root.openPath.length)
            return;
        root.cancelPending();
        root.openPath = root.openPath.slice(0, level);
        root.highlightKeys = root.highlightKeys.slice(0, level + 1);
        root.highlightIndices = root.highlightIndices.slice(0, level + 1);
        root.submenuChanged(level, false);
        root.stateChanged();
    }

    function closeDeepest() {
        root.closeFrom(root.openPath.length - 1);
    }

    function activateAt(level, index) {
        var list = root.levelEntries[level] || [];
        var key = root.keyAt(list, index);
        var at = key !== "" ? root.indexOfKey(list, key) : index;
        if (at < 0)
            return;
        var e = list[at];
        if (!e || e.type === "separator" || !e.enabled)
            return;
        if (e.type === "submenu") {
            root.openAt(level, at);
            return;
        }
        var raw = (root.levelModels[level] || [])[e.index];
        root.activated(raw, e, e.path || root.pathAt(level, at));
    }

    // --- Type-ahead -------------------------------------------------------

    function typeAheadMatch(prefix, list, current) {
        var count = list.length;
        for (var step = 0; step < count; ++step) {
            var i = (current + 1 + step) % count;
            var e = list[i];
            if (!e || e.type === "separator" || !e.enabled)
                continue;
            if (String(e.label).toLocaleLowerCase().indexOf(prefix) === 0)
                return i;
        }
        return -1;
    }

    function typeAhead(level, text) {
        root.cancelPending();
        if (!text || text.length === 0)
            return;
        var lower = text.toLocaleLowerCase();
        root.typeAheadBuffer += lower;
        typeAheadTimer.restart();
        var list = root.levelEntries[level] || [];
        var current = level < root.highlightIndices.length
                      ? root.highlightIndices[level] : -1;
        var match = root.typeAheadMatch(root.typeAheadBuffer, list, current);
        // A buffer that no longer matches starts over at the newest key, so a
        // second word ("sh" then "se") never dead-ends on the old prefix.
        if (match < 0 && root.typeAheadBuffer.length > lower.length) {
            root.typeAheadBuffer = lower;
            match = root.typeAheadMatch(lower, list, current);
        }
        if (match >= 0)
            root.setHighlight(level, match);
    }

    function clearTypeAhead() {
        root.typeAheadBuffer = "";
        typeAheadTimer.stop();
    }

    // --- Hover corridor ---------------------------------------------------

    function cancelPending() {
        dwellTimer.stop();
        root.pendingLevel = -1;
        root.pendingIndex = -1;
        root.pendingKey = "";
    }

    function hover(level, index) {
        if (!root.menuOpen)
            return;
        // A new row claims the pointer before the leave timer retires the old
        // highlight, so moving between rows never flickers.
        hoverLeaveTimer.stop();
        hoverLeaveLevel = -1;
        if (root.pendingLevel === level && root.pendingIndex === index
                && dwellTimer.running)
            return;
        root.cancelPending();
        var list = root.levelEntries[level] || [];
        var e = list[index];
        root.pendingLevel = level;
        root.pendingIndex = index;
        root.pendingKey = root.keyAt(list, index);
        var openIndex = root.openIndexAt(level);
        if (openIndex >= 0 && openIndex !== index) {
            // Crossing a sibling while a child is open: keep the child until
            // the dwell completes, so a diagonal corridor works.
            dwellTimer.start();
            return;
        }
        if (e && e.enabled && e.type !== "separator")
            root.setHighlight(level, index);
        else if (openIndex < 0 || openIndex === index)
            root.setHighlight(level, -1);
        if (e && e.type === "submenu" && e.enabled && openIndex !== index)
            dwellTimer.start();
    }

    function leave(level) {
        // A row leaving only retires a dwell that row's own level armed. A
        // submenu row leaving must not cancel the parent level's pending
        // sibling change; entering the child already does that explicitly.
        if (root.pendingLevel !== level)
            return;
        // Entering the open child cancels the pending sibling change; a
        // departed row with no child open cannot open one later.
        if (root.openIndexAt(level) >= 0)
            return;
        root.cancelPending();
        // Defer the highlight clear one event-loop turn: a pointer crossing
        // into a sibling delivers that sibling's enter immediately, and the
        // timer is cancelled there. Only a pointer that truly left the menu
        // reaches the clear.
        root.hoverLeaveLevel = level;
        hoverLeaveTimer.restart();
    }

    function enterChild(level) {
        hoverLeaveTimer.stop();
        root.hoverLeaveLevel = -1;
        root.cancelPending();
        var open = root.openIndexAt(level);
        if (open >= 0)
            root.setHighlight(level, open);
    }

    function commitHover() {
        if (!root.menuOpen)
            return;
        var level = root.pendingLevel;
        var key = root.pendingKey;
        root.cancelPending();
        if (level < 0 || key === "")
            return;
        var list = root.levelEntries[level] || [];
        var at = root.indexOfKey(list, key);
        if (at < 0) {
            root.closeFrom(level);
            return;
        }
        root.closeFrom(level);
        var e = list[at];
        if (e && e.enabled && e.type !== "separator")
            root.setHighlight(level, at);
        if (e && e.enabled && e.type === "submenu")
            root.openAt(level, at);
    }

    // --- Lifecycle --------------------------------------------------------

    function reset() {
        root.cancelPending();
        hoverLeaveTimer.stop();
        root.hoverLeaveLevel = -1;
        root.clearTypeAhead();
        root.openPath = [];
        root.highlightKeys = [];
        root.highlightIndices = [];
        root.stateChanged();
    }

    // Follow stable identities across a live model update. A missing open
    // ancestor truncates the chain; a missing highlight moves to the nearest
    // surviving enabled row so the selection never points at a different
    // command.
    property bool reconciling: false
    function reconcile(deferInvalidPath) {
        if (root.reconciling)
            return;
        root.reconciling = true;
        var entries = root.levelEntries;
        var path = root.openPath.slice();
        var keys = root.highlightKeys.slice();
        var indices = root.highlightIndices.slice();
        var maxLevel = entries.length;
        var pathChanged = false;
        var keysChanged = false;
        var indicesChanged = false;
        for (var level = 0; level < maxLevel; ++level) {
            var list = entries[level] || [];
            var opener = level < path.length ? list[root.indexOfKey(list, path[level])]
                                            : null;
            if (level < path.length
                    && (!opener || !opener.enabled || opener.type !== "submenu")) {
                // The entries-change handler may still be evaluating the
                // path binding. Let the model timer remove an invalid branch.
                if (deferInvalidPath === true) {
                    root.reconciling = false;
                    return;
                }
                if (root.pendingLevel >= level)
                    root.cancelPending();
                path = path.slice(0, level);
                keys = keys.slice(0, level + 1);
                indices = indices.slice(0, level + 1);
                maxLevel = level + 1;
                pathChanged = true;
                keysChanged = true;
                indicesChanged = true;
            }
            if (level >= keys.length) {
                keys.push("");
                indices.push(-1);
                keysChanged = true;
                indicesChanged = true;
            }
            var key = keys[level];
            var at = root.indexOfKey(list, key);
            if (!root.isSelectable(list[at])) {
                if (key === "" && indices[level] < 0)
                    continue;
                at = root.nearestSelectable(list, indices[level] >= 0
                                                   ? indices[level] : 0);
                keys[level] = root.keyAt(list, at);
                indices[level] = at;
                keysChanged = true;
                indicesChanged = true;
            } else if (indices[level] !== at) {
                indices[level] = at;
                indicesChanged = true;
            }
        }
        if (keys.length > maxLevel) {
            keys = keys.slice(0, maxLevel);
            indices = indices.slice(0, maxLevel);
            keysChanged = true;
            indicesChanged = true;
        }
        // Only touch the arrays the update actually changed.
        if (pathChanged)
            root.openPath = path;
        if (keysChanged)
            root.highlightKeys = keys;
        if (indicesChanged)
            root.highlightIndices = indices;
        root.reconciling = false;
        if (pathChanged || keysChanged || indicesChanged)
            root.stateChanged();
    }

    // The row's top offset within its level, matching the Column layout.
    function rowY(level, index, rowHeight, padding) {
        var list = root.levelEntries[level] || [];
        var y = padding;
        for (var i = 0; i < index && i < list.length; ++i) {
            y += list[i].type === "separator" ? padding : rowHeight;
        }
        return y;
    }

    // The owner can be destroyed when a top-level menu disappears.
    // Keep the deferred reconciliation tied to the menu's lifetime.
    onModelChanged: reconcileTimer.restart()
    property Timer reconcileTimer: Timer {
        interval: 0
        onTriggered: root.reconcile()
    }

    property Timer dwellTimer: Timer {
        interval: Theme.controls.contextMenu.submenuDelay
        repeat: false
        onTriggered: root.commitHover()
    }

    // Retires a hover highlight after the pointer leaves a row. Interval 0
    // runs on the next event-loop turn, after any sibling enter has had the
    // chance to cancel it.
    property Timer hoverLeaveTimer: Timer {
        interval: 0
        repeat: false
        onTriggered: {
            var level = root.hoverLeaveLevel;
            root.hoverLeaveLevel = -1;
            if (level < 0 || root.pendingLevel === level)
                return;
            if (root.openIndexAt(level) >= 0)
                return;
            root.setHighlight(level, -1);
        }
    }

    property Timer typeAheadTimer: Timer {
        interval: 700
        repeat: false
        onTriggered: root.typeAheadBuffer = ""
    }
}
