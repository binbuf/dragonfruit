// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings top-level window (T-09.1a). It is a first-party Tier-1 app, so
// it draws its own titlebar (SettingsShell's design-system TitleBar) on a
// frameless window surface; the compositor does not add SSD on top. The shell
// reports window-control intent and this file maps it to `Window` state.
Window {
    id: win
    property bool startAboutPanel: false
    // T232: the running instance's deep-link endpoint (a context property from
    // the executable). QML tests that create the window without it get null.
    property var deepLinks: typeof SettingsDeepLinks !== "undefined"
                            ? SettingsDeepLinks : null
    property alias aboutWindow: aboutWindow
    property alias aboutPanel: aboutPanel
    property alias settingsShell: shell
    // T264: the About utility panel is a second window of the same app. It
    // opens over the main window from the app menu; the cold-launch
    // `--about-system` path sets `startAboutPanel` instead.
    property bool aboutPanelOpen: false
    // The geometry the user had before an About surface (the panel or the
    // in-window About page) opened. Restored when About closes, so a
    // user-resized window is never left at the 740x820 default.
    property var preAboutGeometry: null
    property bool aboutGeometryHeld: false

    // T263: the measured macOS Tahoe System Settings window. 740x820 pt with
    // a 52 pt unified toolbar, a 232 pt grouped sidebar, and 508 pt of detail
    // content. The window is freely resizable; the 720x480 minimum keeps the
    // sidebar scrollable and the detail card inside the frame.
    width: Theme.controls.settingsWindow.width
    height: Theme.controls.settingsWindow.height
    minimumWidth: Theme.controls.settingsWindow.minWidth
    minimumHeight: Theme.controls.settingsWindow.minHeight
    visible: !startAboutPanel
    // The compositor/taskbar name follows the current pane/sub-pane, never a
    // constant (T263).
    title: shell.windowTitle
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint

    // T263: geometry persistence through settingsd. Restoring is deferred
    // until the first value snapshot; a user resize after that wins over a
    // later daemon snapshot. Writes are debounced so a drag is one Set.
    property bool geometryRestored: false
    property bool geometryTouched: false
    property bool restoringGeometry: false

    function applyStoredGeometry() {
        if (!Settings.windowStatePersistence)
            return;
        restoringGeometry = true;
        var storedWidth = Number(Settings.values["settings.window.width"]);
        var storedHeight = Number(Settings.values["settings.window.height"]);
        if (isFinite(storedWidth) && storedWidth >= minimumWidth)
            win.width = Math.round(storedWidth);
        if (isFinite(storedHeight) && storedHeight >= minimumHeight)
            win.height = Math.round(storedHeight);
        if (Settings.values["settings.window.positioned"] === true) {
            var storedX = Number(Settings.values["settings.window.x"]);
            var storedY = Number(Settings.values["settings.window.y"]);
            if (isFinite(storedX))
                win.x = Math.round(storedX);
            if (isFinite(storedY))
                win.y = Math.round(storedY);
        }
        restoringGeometry = false;
        geometryRestored = true;
    }

    function persistGeometry() {
        if (!Settings.windowStatePersistence || !win.geometryRestored)
            return;
        // Keep the normal (windowed) geometry; a maximized/fullscreen close
        // must not overwrite the remembered size with the screen size.
        if (win.visibility !== Window.Windowed)
            return;
        Settings.set("settings.window.width", Math.round(win.width));
        Settings.set("settings.window.height", Math.round(win.height));
        Settings.set("settings.window.x", Math.round(win.x));
        Settings.set("settings.window.y", Math.round(win.y));
        Settings.set("settings.window.positioned", true);
    }

    // T264: About geometry hand-off. The About panel and the in-window About
    // page both open over the user's window; closing them restores the size
    // the user had, never the 740x820 design default.
    function holdAboutGeometry() {
        if (win.aboutGeometryHeld)
            return;
        win.preAboutGeometry = { width: win.width, height: win.height,
                                 x: win.x, y: win.y };
        win.aboutGeometryHeld = true;
    }

    function releaseAboutGeometry() {
        if (!win.aboutGeometryHeld)
            return;
        win.aboutGeometryHeld = false;
        var saved = win.preAboutGeometry;
        win.preAboutGeometry = null;
        if (saved === null || saved === undefined
                || win.visibility !== Window.Windowed)
            return;
        win.restoringGeometry = true;
        win.width = saved.width;
        win.height = saved.height;
        win.x = saved.x;
        win.y = saved.y;
        win.restoringGeometry = false;
    }

    function maybeReleaseAboutGeometry() {
        if (!win.aboutPanelOpen && !win.settingsShell.aboutOpen)
            win.releaseAboutGeometry();
    }

    function showAboutPanel() {
        win.holdAboutGeometry();
        win.aboutPanelOpen = true;
        win.raise();
        win.requestActivate();
    }

    function closeAboutPanel() {
        win.aboutPanelOpen = false;
        win.maybeReleaseAboutGeometry();
    }

    // The focused editable control, unwrapped from a FocusScope host such as
    // SearchField. Edit menu verbs apply to it.
    function editTarget() {
        var item = win.activeFocusItem;
        if (item !== null && item !== undefined) {
            // A FocusScope host (SearchField) exposes its editable input.
            var inner = item["input"];
            if (inner !== undefined && inner !== null && inner !== item)
                item = inner;
        }
        return item;
    }

    function applyEdit(action) {
        var item = win.editTarget();
        if (item === null || item === undefined)
            return false;
        switch (action) {
        case "edit.undo":
            if (item.undo !== undefined) { item.undo(); return true; }
            break;
        case "edit.redo":
            if (item.redo !== undefined) { item.redo(); return true; }
            break;
        case "edit.cut":
            if (item.cut !== undefined) { item.cut(); return true; }
            break;
        case "edit.copy":
            if (item.copy !== undefined) { item.copy(); return true; }
            break;
        case "edit.paste":
            if (item.paste !== undefined) { item.paste(); return true; }
            break;
        case "edit.select-all":
            if (item.selectAll !== undefined) { item.selectAll(); return true; }
            break;
        }
        return false;
    }

    // Dispatch every action the published model can carry. Returns whether
    // the app performed it; an unknown action never reaches this point
    // (`SettingsMenu.activate` rejects it).
    function dispatchMenuAction(action) {
        switch (action) {
        case "about":
            win.showAboutPanel();
            return true;
        case "settings":
        case "show-all":
            win.show();
            win.raise();
            win.requestActivate();
            return true;
        case "hide":
            win.hide();
            return true;
        case "hide-others":
            // The About panel is this app's only other window.
            win.closeAboutPanel();
            return true;
        case "quit":
            win.close();
            win.aboutWindow.close();
            Qt.quit();
            return true;
        case "close":
            win.close();
            return true;
        case "minimize":
            win.showMinimized();
            return true;
        case "zoom":
            win.visibility === Window.Maximized
                    ? win.showNormal() : win.showMaximized();
            return true;
        case "fullscreen":
            win.visibility === Window.FullScreen
                    ? win.showNormal() : win.showFullScreen();
            return true;
        case "back":
            win.settingsShell.back();
            return true;
        case "forward":
            win.settingsShell.forward();
            return true;
        case "bring-all-to-front":
            win.show();
            win.raise();
            win.requestActivate();
            return true;
        case "help":
            return win.settingsShell.openHelp(win.settingsShell.currentPaneId);
        case "help.debug":
            win.settingsShell.showDebugView();
            return true;
        case "help.map":
            if (Settings.mapDocUrl.toString().length === 0)
                return false;
            Qt.openUrlExternally(Settings.mapDocUrl);
            return true;
        default:
            if (action.indexOf("pane.") === 0) {
                win.settingsShell.activateSidebarItem(action.substring(5));
                return true;
            }
            if (action.indexOf("edit.") === 0)
                return win.applyEdit(action);
            return false;
        }
    }

    Component.onCompleted: applyStoredGeometry()

    onAboutPanelOpenChanged: {
        if (!win.aboutPanelOpen)
            win.maybeReleaseAboutGeometry();
    }

    Connections {
        target: shell
        function onAboutOpenChanged() {
            if (shell.aboutOpen)
                win.holdAboutGeometry();
            else
                win.maybeReleaseAboutGeometry();
        }
    }

    Connections {
        target: Settings
        function onValuesChanged() {
            if (!win.geometryTouched)
                win.applyStoredGeometry();
        }
    }

    Timer {
        id: geometryTimer
        interval: 250
        repeat: false
        onTriggered: win.persistGeometry()
    }

    onWidthChanged: {
        if (!win.restoringGeometry) {
            win.geometryTouched = true;
            if (win.geometryRestored)
                geometryTimer.restart();
        }
    }
    onHeightChanged: {
        if (!win.restoringGeometry) {
            win.geometryTouched = true;
            if (win.geometryRestored)
                geometryTimer.restart();
        }
    }
    onXChanged: {
        if (!win.restoringGeometry) {
            win.geometryTouched = true;
            if (win.geometryRestored)
                geometryTimer.restart();
        }
    }
    onYChanged: {
        if (!win.restoringGeometry) {
            win.geometryTouched = true;
            if (win.geometryRestored)
                geometryTimer.restart();
        }
    }
    onClosing: (close) => win.persistGeometry()

    SettingsShell {
        id: shell
        anchors.fill: parent
        windowActive: win.active

        onCloseRequested: win.close()
        onMinimizeRequested: win.showMinimized()
        onZoomRequested: win.visibility === Window.Maximized
                         ? win.showNormal() : win.showMaximized()
        onMoveRequested: (x, y) => win.startSystemMove()
        // The shell owns the edge hit regions; the window asks the compositor
        // for the interactive resize (`xdg_toplevel.resize`), which honors the
        // min-size hints above.
        onResizeRequested: (edges) => win.startSystemResize(edges)
    }

    Window {
        id: aboutWindow
        transientParent: null
        width: 280
        height: 503
        minimumWidth: 280
        maximumWidth: 280
        minimumHeight: 503
        maximumHeight: 503
        // The app menu shows this panel over the main window; the cold-launch
        // `--about-system` path keeps the main window hidden.
        visible: win.startAboutPanel || win.aboutPanelOpen
        title: qsTr("About This System")
        color: "transparent"
        flags: Qt.Tool | Qt.FramelessWindowHint
        onClosing: win.closeAboutPanel()
        AboutPanel {
            id: aboutPanel
            anchors.fill: parent
            onCloseRequested: aboutWindow.close()
            onMoreInfoRequested: {
                aboutWindow.close();
                win.closeAboutPanel();
                // The main window was hidden for the panel; restore the
                // remembered geometry before it maps.
                if (!win.geometryRestored)
                    win.applyStoredGeometry();
                win.show();
                shell.openAboutPage();
                win.raise();
                win.requestActivate();
            }
        }
    }

    // T232: a `settings://pane/<id>` link delivered to the running instance.
    // The shell's extras raise it; the app navigates in place and raises the
    // window instead of starting a second process.
    Connections {
        target: win.deepLinks
        function onRequested(url) {
            shell.deepLink(url);
            win.show();
            win.raise();
            win.requestActivate();
        }
    }

    // The published menu model's actions (T-09.6a). The menu bar is in the
    // shell process, so this is the app-side end of the dispatch seam: when
    // the broker routes an activated action back (T-14.2b), the app performs
    // the window-level verbs, the pane jumps, and the edit verbs. T264 makes
    // every published action real; `SettingsMenu.activate` rejects an unknown
    // action before it reaches this handler.
    Connections {
        target: SettingsMenu
        function onActivated(action, item) {
            win.dispatchMenuAction(action);
        }
    }

    // T-14.2b: the broker's activation endpoint delivers a published action
    // back to the app that owns the current menu generation. The local
    // dispatcher is the same one a local menu row uses, so a global click and
    // an accelerator cannot diverge. `completeActivation` returns its result
    // to the broker, which surfaces a rejection instead of claiming success.
    Connections {
        target: typeof MenuPublisher !== "undefined" ? MenuPublisher : null
        function onActivationRequested(action, itemJson, generation, windowId) {
            if (typeof MenuPublisher !== "undefined")
                MenuPublisher.completeActivation(win.dispatchMenuAction(action));
        }
    }
}
