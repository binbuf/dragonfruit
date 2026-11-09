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
    // The 16 pt button frame is the hit target; the visible circle is 14 pt.
    implicitHeight: Theme.controls.trafficLights.buttonSize

    readonly property bool reveal: root.forceReveal || root.hovered

    Row {
        id: row
        // The measured 23 pt spacing is centre-to-centre; Qt's Row wants the
        // edge gap between two 16 pt button frames.
        spacing: WindowStyle.trafficLightGap()
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

        width: Theme.controls.trafficLights.buttonSize
        height: width
        activeFocusOnTab: true

        Accessible.role: Accessible.Button
        Accessible.name: light.label
        Accessible.focusable: true
        Accessible.onPressAction: light.activated()

        Rectangle {
            // The AX frame is 16 pt, but AppKit draws a 14 pt circle centred
            // inside it. Drawing the frame itself made the lights too big.
            anchors.centerIn: parent
            width: Theme.controls.trafficLights.diameter
            height: width
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
            // A 7 pt glyph centred in a 16 pt frame lands on a half pixel;
            // rounding keeps it crisp at 1x and at fractional scales.
            x: Math.round((parent.width - width) / 2)
            y: Math.round((parent.height - height) / 2)
            name: light.kind === "zoom" ? "zoom" : light.kind
            size: Theme.controls.trafficLights.glyphSize
            // The 7 pt token is Tabler's 24-unit box: the 12-unit path plus
            // the 4-unit stroke is a ~4.7 pt mark with a ~1.2 pt stroke,
            // matching the compositor's rasterized SSD glyphs.
            strokeWidth: 4
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
            cornerRadius: Theme.controls.trafficLights.diameter / 2
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
