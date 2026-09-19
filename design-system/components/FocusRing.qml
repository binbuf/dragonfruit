// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// The one focus indicator in the desktop. Components show it on
// `activeFocus`; it is part of every component's definition of done
// (10-design-system.md) rather than something apps re-invent.
Rectangle {
    id: root

    property Item target: parent
    property real cornerRadius: Theme.controls.focusRing.radius
    property bool shown: false

    anchors.fill: target
    anchors.margins: -Theme.controls.focusRing.offset
    color: "transparent"
    radius: cornerRadius + Theme.controls.focusRing.offset
    border.width: Theme.controls.focusRing.width
    border.color: Theme.color.focusRing
    antialiasing: true
    visible: shown
    z: 100

    Behavior on opacity {
        NumberAnimation {
            duration: Theme.motion.focus.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: Theme.motion.focus.curve
        }
    }
    opacity: shown ? 1 : 0
}
