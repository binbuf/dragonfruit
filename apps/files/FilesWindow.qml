// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files top-level window (T-10.4a). It is a first-party Tier-1 app, so it
// draws its own titlebar (FilesShell's design-system TitleBar) on a frameless
// window surface; the compositor does not add SSD on top. The shell reports
// window-control intent and this file maps it to `Window` state.
Window {
    id: win

    width: 980
    height: 640
    minimumWidth: 720
    minimumHeight: 480
    visible: true
    title: qsTr("Dragonfruit Files")
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint

    FilesShell {
        id: shell
        anchors.fill: parent
        windowActive: win.active

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
    }
}
