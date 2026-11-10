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

    // The macOS reference capture (Desktop & Dock, Accessibility, ...) is
    // ~620x627 in design-system token units, with the sidebar ~31% of the
    // width. We keep the reference-matched default size, but unlike macOS
    // System Settings the window is freely resizable in both axes so it
    // behaves like the rest of the desktop; the minimum keeps the sidebar and
    // detail pane usable. See
    // docs/reference/settings-layout-measurements.md.
    width: 800
    height: 640
    minimumWidth: 720
    minimumHeight: 480
    visible: !startAboutPanel
    title: qsTr("Settings")
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint

    SettingsShell {
        id: shell
        anchors.fill: parent
        windowActive: win.active
        onAboutPageOpened: {
            win.width = 740;
            win.height = 820;
        }

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
