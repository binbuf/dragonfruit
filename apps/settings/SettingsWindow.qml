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

    Component.onCompleted: applyStoredGeometry()

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
        visible: win.startAboutPanel
        title: qsTr("About This System")
        color: "transparent"
        flags: Qt.Tool | Qt.FramelessWindowHint
        AboutPanel {
            id: aboutPanel
            anchors.fill: parent
            onCloseRequested: aboutWindow.close()
            onMoreInfoRequested: {
                aboutWindow.close();
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
    // the window-level verbs and the pane jumps. The `edit.*` verbs are the
    // standard text-editing actions applied to the focused text input.
    Connections {
        target: SettingsMenu
        function onActivated(action, item) {
            switch (action) {
            case "close":
                win.close();
                break;
            case "minimize":
                win.showMinimized();
                break;
            case "zoom":
                win.visibility === Window.Maximized
                        ? win.showNormal() : win.showMaximized();
                break;
            case "fullscreen":
                win.visibility === Window.FullScreen
                        ? win.showNormal() : win.showFullScreen();
                break;
            case "help":
            case "help.debug":
                shell.showDebugView();
                break;
            case "help.map":
                Qt.openUrlExternally(Settings.mapDocUrl);
                break;
            default:
                if (action.indexOf("pane.") === 0)
                    shell.selectPane(action.substring(5));
                break;
            }
        }
    }
}
