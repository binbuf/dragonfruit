// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The Files top-level window (T-10.4a). It is a first-party Tier-1 app, so it
// draws its own titlebar (FilesShell's design-system TitleBar) on a frameless
// window surface; the compositor does not add SSD on top. The shell reports
// window-control intent and this file maps it to `Window` state.
//
// T333: a window shows one or more tabs (the shell's `FilesTabs`); the title
// follows the active tab. New Window launches a second process
// (`Files.newWindow`), so two windows never share a crash domain — the same
// rule as the desktop's folder launch. A component asking for a location
// (`--activation-handoff`) reaches the running instance as a new tab; that
// instance keeps `org.dragonfruit.Files1`, and every other window process
// runs without it.
Window {
    id: win

    // The `org.dragonfruit.Files1` hand-off target (T333): the entry point
    // invokes this on the root object. An empty location raises the window;
    // any other opens as a tab (or activates the tab already showing it).
    function activateLocation(uri) {
        return shell.activateLocation(uri);
    }

    width: 980
    height: 640
    minimumWidth: 720
    minimumHeight: 480
    visible: true
    title: qsTr("Files")
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint

    FilesShell {
        id: shell
        anchors.fill: parent
        windowActive: win.active

        // The window title and the Dock's grouping follow the active tab
        // (T333). A browser window is titled with its location, as before.
        Binding {
            target: win
            property: "title"
            value: shell.tabs.count > 1
                   ? qsTr("%1 — Files").arg(shell.locationTitle)
                   : shell.locationTitle
        }

        onCloseRequested: win.close()
        onMinimizeRequested: win.showMinimized()
        onZoomRequested: win.visibility === Window.Maximized
                         ? win.showNormal() : win.showMaximized()
        onMoveRequested: (x, y) => win.startSystemMove()
        onResizeRequested: (edges) => win.startSystemResize(edges)
        onOpenRequested: (uri) => {
            if (!Files.open(uri))
                shell.notice = qsTr("No application can open %1").arg(uri)
        }
        // New Window: a second process on the active (or the requested)
        // location. The capture/test double records the request instead.
        onNewWindowRequested: (uri) => {
            var target = uri !== undefined && uri.length > 0 ? uri
                                                             : shell.currentUri;
            if (!Files.newWindow(target))
                shell.notice = qsTr("Could not open a new window");
        }
        // Closing the last tab closes the window (the shell decided it).
        onWindowClosed: win.close()
        onRaiseRequested: {
            win.show();
            win.raise();
            win.requestActivate();
        }
    }

    // T334: the broker's activation endpoint delivers a published menu action
    // back to the app that owns the current generation. The same shell
    // dispatcher serves a local click/accelerator and a test, so the two paths
    // cannot diverge. `completeActivation` returns the handled result to the
    // broker, which surfaces a rejection instead of claiming success.
    Connections {
        target: typeof MenuPublisher !== "undefined" ? MenuPublisher : null
        function onActivationRequested(action, itemJson, generation, windowId) {
            if (typeof MenuPublisher !== "undefined")
                MenuPublisher.completeActivation(shell.dispatchMenuAction(action));
        }
    }
}
