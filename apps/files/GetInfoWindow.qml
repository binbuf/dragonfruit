// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files Get Info auxiliary window (T230d): the measured 328×1034 pt
// single-column inspector with a 32 pt title bar. It is a separate window,
// not a sheet inside the browser, so it can stay open while the selection
// changes in the main window.
Window {
    id: win

    width: Theme.controls.files.getInfoWidth
    height: Theme.controls.files.getInfoHeight
    minimumWidth: Theme.controls.files.getInfoWidth
    minimumHeight: 320
    title: qsTr("Get Info")
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint

    property alias inspector: inspector
    property alias titleBar: titleBar
    property alias itemName: inspector.itemName
    property alias itemKind: inspector.itemKind
    property alias previewSource: inspector.previewSource
    property alias previewIcon: inspector.previewIcon
    property alias multiSelection: inspector.multiSelection
    property alias selectionCount: inspector.selectionCount
    property alias sections: inspector.sections

    signal closeRequested()
    signal renameRequested(string name)

    AppWindow {
        anchors.fill: parent
        title: win.title
        windowStyle: "titlebar"
        chromeTransparent: true

        titleBarData: TitleBar {
            id: titleBar
            title: win.title
            fillColor: Theme.color.chrome
            onCloseRequested: win.closeRequested()
            onMinimizeRequested: win.showMinimized()
        }

        GetInfoInspector {
            id: inspector
            anchors.fill: parent
            onRenameRequested: (name) => win.renameRequested(name)
        }
    }

    onClosing: (close) => {
        close.accepted = false;
        win.closeRequested();
    }
}
