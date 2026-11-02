// SPDX-License-Identifier: MIT
import QtQuick

// Edge and corner input for a frameless, client-decorated window.
//
// A first-party Tier-1 app draws its own titlebar on a frameless surface, so
// the compositor never gets a chance to provide resize borders. This item is
// the one shared hit-region set: a press on any edge or corner emits
// `resizeRequested` with the matching `Qt.Edges` flags, and the host window
// forwards that to `startSystemResize`.
//
// The grips sit above the app content (`z: 1000`) but are only a few pixels
// wide, so they never eat a click meant for a row, tile, or the titlebar.
Item {
    id: root

    property int thickness: 6
    property int corner: 12
    signal resizeRequested(int edges)

    z: 1000

    MouseArea {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.thickness
        cursorShape: Qt.SizeVerCursor
        onPressed: root.resizeRequested(Qt.TopEdge)
    }
    MouseArea {
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.thickness
        cursorShape: Qt.SizeVerCursor
        onPressed: root.resizeRequested(Qt.BottomEdge)
    }
    MouseArea {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: root.thickness
        cursorShape: Qt.SizeHorCursor
        onPressed: root.resizeRequested(Qt.LeftEdge)
    }
    MouseArea {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: root.thickness
        cursorShape: Qt.SizeHorCursor
        onPressed: root.resizeRequested(Qt.RightEdge)
    }

    // Corners are declared after the edges so they win the overlap and resize
    // both axes at once.
    MouseArea {
        anchors.top: parent.top
        anchors.left: parent.left
        width: root.corner
        height: root.corner
        cursorShape: Qt.SizeFDiagCursor
        onPressed: root.resizeRequested(Qt.TopEdge | Qt.LeftEdge)
    }
    MouseArea {
        anchors.top: parent.top
        anchors.right: parent.right
        width: root.corner
        height: root.corner
        cursorShape: Qt.SizeBDiagCursor
        onPressed: root.resizeRequested(Qt.TopEdge | Qt.RightEdge)
    }
    MouseArea {
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        width: root.corner
        height: root.corner
        cursorShape: Qt.SizeBDiagCursor
        onPressed: root.resizeRequested(Qt.BottomEdge | Qt.LeftEdge)
    }
    MouseArea {
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        width: root.corner
        height: root.corner
        cursorShape: Qt.SizeFDiagCursor
        onPressed: root.resizeRequested(Qt.BottomEdge | Qt.RightEdge)
    }
}
