// SPDX-License-Identifier: GPL-3.0-only
import QtQuick

// The Files window's tab model (T333). A tab is a browsing context — its
// location, its own back/forward history and per-location view state, and its
// selection — not a second application. The window owns one `FilesTabs`; the
// shell renders the tab bar, binds the location/contents/breadcrumb to the
// active tab, and every other window state (the app-local clipboard, the
// Get Info window, the notice) stays window-scoped.
//
// Plain JavaScript objects cannot be mutated in place (a QML binding would
// never see the change), so every edit replaces the whole array. Each entry is
// `{ id, title, history, historyIndex }` where `title` is the tab's last known
// location name — the tab bar label, so a background tab still reads sensibly.
//
// Closing the last tab leaves zero tabs: the shell decides whether that shows
// an empty state or closes the window (it closes the window; see
// `FilesShell.closeTab` and 09-files.md#windows-tabs-and-navigation).
QtObject {
    id: root

    // Open tabs, oldest (leftmost) first: `{ id, title, history, historyIndex }`.
    property var tabs: []
    // The visible tab's index into `tabs`.
    property int currentIndex: -1
    // The next tab id; ids are stable for the window's lifetime so the tab bar
    // can key its delegates and the shell can name a tab across reorders.
    property int nextId: 1

    readonly property int count: tabs.length
    readonly property var current: root.validIndex(currentIndex)
                                        ? tabs[currentIndex] : null
    readonly property bool isValid: current !== null
    // The active tab's history stack and its cursor. The window's location,
    // back/forward, and every tab-derived binding read these.
    readonly property var currentHistory: current ? (current.history || [])
                                                   : []
    readonly property int currentHistoryIndex: current ? current.historyIndex : -1
    readonly property bool canGoBack: currentHistoryIndex > 0
    readonly property bool canGoForward: currentHistoryIndex >= 0
                                      && currentHistoryIndex < currentHistory.length - 1

    function validIndex(index) {
        return index >= 0 && index < tabs.length;
    }

    function copyTabs() {
        var out = [];
        for (var i = 0; i < tabs.length; ++i)
            out.push(tabs[i]);
        return out;
    }

    // The tab `index` with `changes` applied, as a new object: the id is kept,
    // everything else a caller passes replaces the tab's.
    function mergedTab(index, changes) {
        var tab = tabs[index];
        var next = { id: tab.id, title: tab.title, history: tab.history,
                     historyIndex: tab.historyIndex };
        for (var key in changes) {
            if (key === "id")
                continue;
            next[key] = changes[key];
        }
        return next;
    }

    // A copy of `tabs` with tab `index` replaced by `mergedTab(index, changes)`.
    function withTab(index, changes) {
        if (!root.validIndex(index))
            return root.copyTabs();
        var list = root.copyTabs();
        list[index] = root.mergedTab(index, changes);
        return list;
    }

    // Open `uri` as a new tab and make it current; returns the new index (or
    // -1). `at` inserts the tab at that slot; New Tab and the tab bar's
    // disclosure button append at the end.
    function openTab(uri, title, at) {
        if (!uri)
            return -1;
        var list = root.copyTabs();
        var opened = { id: root.nextId, title: title || "", history: [uri],
                       historyIndex: 0 };
        root.nextId += 1;
        var index = (at === undefined || at === null) ? list.length
                                                      : Math.max(0, Math.min(at, list.length));
        list.splice(index, 0, opened);
        root.tabs = list;
        root.currentIndex = index;
        return index;
    }

    function selectTab(index) {
        if (!root.validIndex(index) || index === root.currentIndex)
            return false;
        root.currentIndex = index;
        return true;
    }

    // Close the tab at `index` (default: the current one). The neighbour right
    // of the closed tab becomes current; closing the rightmost tab falls back
    // to the new rightmost. Returns the index that is current afterwards, or
    // -1 when no tab is left.
    function closeTab(index) {
        var at = (index === undefined || index === null) ? root.currentIndex : index;
        if (!root.validIndex(at))
            return root.currentIndex;
        var list = root.copyTabs();
        list.splice(at, 1);
        root.tabs = list;
        if (list.length === 0) {
            root.currentIndex = -1;
            return -1;
        }
        if (at < root.currentIndex)
            root.currentIndex = root.currentIndex - 1;
        else if (at === root.currentIndex)
            root.currentIndex = Math.min(at, list.length - 1);
        return root.currentIndex;
    }

    // Navigate the tab at `index` (default: the current). Re-navigating the
    // same location is a no-op so selection echo does not grow the stack. Only
    // the addressed tab's history changes: two tabs keep independent stacks.
    function navigate(uri, title, index) {
        var at = (index === undefined || index === null) ? root.currentIndex : index;
        if (!root.validIndex(at) || !uri)
            return false;
        var tab = tabs[at];
        if (tab.history[tab.historyIndex] === uri)
            return false;
        var history = tab.history.slice(0, tab.historyIndex + 1);
        history.push(uri);
        root.updateTab(at, { history: history, historyIndex: history.length - 1,
                             title: title !== undefined && title !== null
                                  ? title : tab.title });
        return true;
    }

    function back(index) {
        var at = (index === undefined || index === null) ? root.currentIndex : index;
        if (!root.validIndex(at))
            return false;
        var tab = tabs[at];
        if (tab.historyIndex <= 0)
            return false;
        root.updateTab(at, { historyIndex: tab.historyIndex - 1 });
        return true;
    }

    function forward(index) {
        var at = (index === undefined || index === null) ? root.currentIndex : index;
        if (!root.validIndex(at))
            return false;
        var tab = tabs[at];
        if (tab.historyIndex >= tab.history.length - 1)
            return false;
        root.updateTab(at, { historyIndex: tab.historyIndex + 1 });
        return true;
    }

    // Replace one tab (a new object, so every binding re-evaluates). Fields in
    // `changes` replace the tab's; the id is never changed.
    function updateTab(index, changes) {
        if (!root.validIndex(index))
            return false;
        root.tabs = root.withTab(index, changes);
        return true;
    }

    // The location the tab at `index` is showing.
    function uriAt(index) {
        if (!root.validIndex(index))
            return "";
        var tab = tabs[index];
        return tab.history[tab.historyIndex] || "";
    }

    // The tab bar label: the location name, or the URI when the shell has no
    // name for it (an unlisted location still gets a row).
    function tabTitle(index) {
        if (!root.validIndex(index))
            return "";
        return tabs[index].title.length > 0 ? tabs[index].title
                                            : root.uriAt(index);
    }

    function indexOfId(id) {
        for (var i = 0; i < tabs.length; ++i) {
            if (tabs[i].id === id)
                return i;
        }
        return -1;
    }
}
