// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings top-level window (T-09.1a). It is a first-party Tier-1 app, so
// it draws its own titlebar (SettingsShell's design-system TitleBar) on a
// frameless window surface; the compositor does not add SSD on top. The shell
// reports window-control intent and this file maps it to `Window` state.
Window {
    id: win

    // The macOS reference capture (Desktop & Dock, Accessibility, ...) is
    // ~620x627 in design-system token units, with the sidebar ~31% of the
    // width. macOS only resizes System Settings vertically, so the width is
    // effectively fixed; see docs/reference/settings-layout-measurements.md.
    width: 640
    height: 640
    minimumWidth: 560
    minimumHeight: 480
    visible: true
    title: qsTr("Settings")
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint

    SettingsShell {
        id: shell
        anchors.fill: parent

        onCloseRequested: win.close()
        onMinimizeRequested: win.showMinimized()
        onZoomRequested: win.visibility === Window.Maximized
                         ? win.showNormal() : win.showMaximized()
        onMoveRequested: (x, y) => win.startSystemMove()
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
            default:
                if (action.indexOf("pane.") === 0)
                    shell.selectPane(action.substring(5));
                break;
            }
        }
    }
}