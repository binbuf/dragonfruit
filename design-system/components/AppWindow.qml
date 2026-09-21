// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// First-party application window chrome: a rounded surface with a titlebar
// slot and a content slot. Exported as `AppWindow` rather than `Window` so it
// never shadows `QtQuick.Window` for consumers that import both modules.
Item {
    id: root

    default property alias contentData: contentArea.data
    property alias titleBarData: titleBarArea.data
    property alias titleBar: titleBarArea
    property alias content: contentArea

    property string title: ""
    property bool active: true
    property bool showShadow: true
    property real cornerRadius: Theme.controls.window.radius

    implicitWidth: 640
    implicitHeight: 420

    Shadow {
        visible: root.showShadow
        width: root.width
        height: root.height
        radius: root.cornerRadius
    }

    Rectangle {
        id: frame
        anchors.fill: parent
        radius: root.cornerRadius
        color: Theme.color.surface
        border.width: Theme.controls.window.borderWidth
        border.color: root.active ? Theme.color.border : Theme.color.separator
        antialiasing: true
    }

    Item {
        id: titleBarArea
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: childrenRect.height
    }

    Item {
        id: contentArea
        anchors.top: titleBarArea.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom

        // Opaque content backing so the surface reads as one sheet under the
        // titlebar's translucency.
        Rectangle {
            anchors.fill: parent
            color: Theme.color.surface
            radius: root.cornerRadius
        }
    }

    Accessible.role: Accessible.Window
    Accessible.name: root.title
}
