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
    // T230b: the named chrome style. A `TitleBar` parented into
    // `titleBarData` inherits it, so a window declares its style once.
    property string windowStyle: "titlebar"
    readonly property real chromeHeight:
        WindowStyle.chromeHeight(root.windowStyle)
    property bool showShadow: true
    // The elevation level the window casts (component.elevation.*). The
    // compositor's SSD shadow uses the same level, so the two agree (FR-2).
    property string shadowLevel: "high"
    property real cornerRadius: Theme.controls.window.radius
    // T206: first-party apps that draw their own app-local glass chrome make
    // the frame/content backing transparent so their sidebar/toolbar alpha
    // composites against the compositor's lower scene. The titlebar keeps an
    // opaque chrome backing, and the app shells paint their own opaque detail
    // sheet. Generic AppWindow users keep the original opaque surface.
    property bool chromeTransparent: false

    implicitWidth: 640
    implicitHeight: 420

    Shadow {
        // The compositor draws the real window shadow. The QML approximation
        // is only a fallback for surfaces without one; with transparent chrome
        // it would leak through the sidebar/toolbar alpha and darken the
        // measured material.
        visible: root.showShadow && !root.chromeTransparent
        level: root.shadowLevel
        width: root.width
        height: root.height
        radius: root.cornerRadius
    }

    Rectangle {
        id: frame
        anchors.fill: parent
        radius: root.cornerRadius
        color: root.chromeTransparent ? "transparent" : Theme.color.surface
        border.width: Theme.controls.window.borderWidth
        border.color: root.active ? Theme.color.border : Theme.color.separator
        antialiasing: true
    }

    // Keep the titlebar on its existing opaque chrome even when the app-local
    // glass starts below it: the T206 effect is clipped to the sidebar and
    // toolbar, never the window controls or the drag strip.
    ChromeSheet {
        id: titleBarBacking
        visible: root.chromeTransparent
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.topMargin: Theme.controls.window.borderWidth
        anchors.leftMargin: Theme.controls.window.borderWidth
        anchors.rightMargin: Theme.controls.window.borderWidth
        height: Math.max(0, titleBarArea.height
                         - Theme.controls.window.borderWidth)
        color: Theme.color.chrome
        topLeftRadius: Math.max(0, root.cornerRadius
                                - Theme.controls.window.borderWidth)
        topRightRadius: Math.max(0, root.cornerRadius
                                 - Theme.controls.window.borderWidth)
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
        // titlebar's translucency. App-local glass chrome disables it and paints
        // its own opaque detail sheet below the toolbar.
        Rectangle {
            anchors.fill: parent
            visible: !root.chromeTransparent
            color: Theme.color.surface
            radius: root.cornerRadius
        }
    }

    Accessible.role: Accessible.Window
    Accessible.name: root.title
}
