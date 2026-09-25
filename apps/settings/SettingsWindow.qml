// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings top-level window (T-09.1a). It is a first-party Tier-1 app, so
// it draws its own titlebar (SettingsShell's design-system TitleBar) on a
// frameless window surface; the compositor does not add SSD on top. The shell
// reports window-control intent and this file maps it to `Window` state.
Window {
    id: win

    width: 900
    height: 620
    minimumWidth: 720
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
}