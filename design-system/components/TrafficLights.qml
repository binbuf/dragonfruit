// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Left-side close / minimize / zoom controls. Glyphs reveal on hover of the
// cluster, stay colorless otherwise (05-window-decorations.md). This is the
// single implementation shared by first-party `TitleBar`s; the compositor's
// SSD titlebar draws the same tokens (FR-3).
Item {
    id: root

    property bool active: true
    property bool forceReveal: false
    property alias hovered: clusterHover.hovered
    property alias closeButton: closeLight
    property alias minimizeButton: minimizeLight
    property alias zoomButton: zoomLight

    signal closeClicked()
    signal minimizeClicked()
    signal zoomClicked()

    implicitWidth: row.implicitWidth
    implicitHeight: Theme.controls.trafficLights.diameter

    readonly property bool reveal: root.forceReveal || root.hovered

    Row {
        id: row
        spacing: Theme.controls.trafficLights.gap
        anchors.verticalCenter: parent.verticalCenter

        TrafficLight {
            id: closeLight
            kind: "close"
            baseColor: Theme.color.close
            label: qsTr("Close")
            onActivated: root.closeClicked()
        }
        TrafficLight {
            id: minimizeLight
            kind: "minimize"
            baseColor: Theme.color.minimize
            label: qsTr("Minimize")
            onActivated: root.minimizeClicked()
        }
        TrafficLight {
            id: zoomLight
            kind: "zoom"
            baseColor: Theme.color.zoom
            label: qsTr("Zoom")
            onActivated: root.zoomClicked()
        }
    }

    HoverHandler { id: clusterHover }

    component TrafficLight: Item {
        id: light

        property string kind: "close"
        property color baseColor: Theme.color.close
        property string label: ""
        signal activated()

        width: Theme.controls.trafficLights.diameter
        height: width
        activeFocusOnTab: true

        Accessible.role: Accessible.Button
        Accessible.name: light.label
        Accessible.focusable: true
        Accessible.onPressAction: light.activated()

        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: root.active ? light.baseColor : Theme.color.controlActive
            border.width: root.active ? 0 : 1
            border.color: Theme.color.border
            antialiasing: true

            Behavior on color {
                ColorAnimation {
                    duration: Theme.motion.hover.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.hover.curve
                }
            }
        }

        Icon {
            anchors.centerIn: parent
            name: light.kind === "zoom" ? "zoom" : light.kind
            size: Theme.controls.trafficLights.glyphSize
            color: Theme.color.trafficGlyph
            opacity: root.reveal ? 1 : 0

            Behavior on opacity {
                NumberAnimation {
                    duration: Theme.motion.hover.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.hover.curve
                }
            }
        }

        HoverHandler { id: lightHover }

        TapHandler {
            onTapped: light.activated()
        }

        FocusRing {
            target: light
            cornerRadius: light.width / 2
            shown: light.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                light.activated();
                event.accepted = true;
            }
        }
    }
}
