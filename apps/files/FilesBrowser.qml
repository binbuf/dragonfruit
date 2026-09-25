// SPDX-License-Identifier: MIT
import QtQuick

// The Files browsing model (T-10.4a): the current location, per-window
// back/forward history, and per-location view state. It owns no pixels; the
// shell binds the toolbar and sidebar to it, and T-10.4b attaches the
// files-core directory listing to `currentUri`.
//
// The location is an opaque URI (`file:///…`, `trash://`): resolving it to a
// real directory is the platform seam's job (FilesBridge / files-core), not
// the UI's.
QtObject {
    id: root

    // The location a fresh window opens on (the Home directory).
    property string homeUri: ""
    // The location the window is showing.
    property string currentUri: ""
    // The visited locations, oldest first, and the cursor into it. Selecting
    // a new location truncates the redo tail, Finder-style.
    property var history: []
    property int historyIndex: -1
    // Per-location view state: uri -> "icon" | "list". The schema names all
    // four view kinds from day one (see 09-files.md#views); only the MVP two
    // are selectable until the later views ship.
    property var viewState: ({})
    property string defaultView: "icon"

    // One location really changed (not a programmatic no-op).
    signal navigated(string uri)

    readonly property bool canGoBack: historyIndex > 0
    readonly property bool canGoForward: historyIndex >= 0
                                      && historyIndex < history.length - 1
    readonly property string currentView: viewFor(currentUri)

    // Open `uri` and record it in history. Re-navigating the current location
    // is a no-op so selection echo does not grow the stack.
    function navigate(uri) {
        if (!uri || uri === currentUri)
            return;
        var next = history.slice(0, historyIndex + 1);
        next.push(uri);
        history = next;
        historyIndex = next.length - 1;
        currentUri = uri;
        navigated(uri);
    }

    // Seed the first location without recording a history step.
    function reset(uri) {
        if (!uri)
            return;
        history = [uri];
        historyIndex = 0;
        currentUri = uri;
    }

    function back() {
        if (!canGoBack)
            return;
        historyIndex -= 1;
        currentUri = history[historyIndex];
        navigated(currentUri);
    }

    function forward() {
        if (!canGoForward)
            return;
        historyIndex += 1;
        currentUri = history[historyIndex];
        navigated(currentUri);
    }

    function viewFor(uri) {
        var kind = viewState[uri];
        return kind !== undefined ? kind : defaultView;
    }

    // Switch the current location's view. The choice is remembered for that
    // location, and other locations keep theirs.
    function setView(kind) {
        if (kind !== "icon" && kind !== "list")
            return;
        if (viewFor(currentUri) === kind)
            return;
        var next = {};
        for (var key in viewState)
            next[key] = viewState[key];
        next[currentUri] = kind;
        viewState = next;
    }
}