// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// A labelled on/off switch. Keyboard operable, AT-SPI Switch role, and a
// reduced-motion variant (the knob jumps, the state change stays legible).
Item {
    id: root

    property bool checked: false
    property string text: ""
    property string description: ""
    property alias hovered: toggleHover.hovered

    signal toggled(bool checked)

    implicitWidth: track.width + (label.text.length > 0
                   ? Theme.controls.toggle.labelGap + label.implicitWidth : 0)
    implicitHeight: Math.max(track.height, label.implicitHeight)
    activeFocusOnTab: true
    opacity: enabled ? 1.0 : 0.45

    function toggle() {
        if (!root.enabled)
            return;
        root.checked = !root.checked;
        root.toggled(root.checked);
    }

    Rectangle {
        id: track
        width: Theme.controls.toggle.width
        height: Theme.controls.toggle.height
        radius: height / 2
        anchors.verticalCenter: parent.verticalCenter
        color: root.checked ? Theme.color.accent : Theme.color.controlFill
        border.width: root.checked ? 0 : Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        Behavior on color {
            ColorAnimation {
                duration: Theme.motion.toggle.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.toggle.curve
            }
        }

        Rectangle {
            id: knob
            width: Theme.controls.toggle.knob
            height: width
            radius: width / 2
            color: Theme.color.controlKnob
            y: (track.height - height) / 2
            x: root.checked
               ? track.width - width - Theme.controls.toggle.inset
               : Theme.controls.toggle.inset
            antialiasing: true

            Behavior on x {
                NumberAnimation {
                    duration: Theme.motion.toggle.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.toggle.curve
                }
            }
        }
    }

    Column {
        id: labels
        anchors.left: track.right
        anchors.leftMargin: Theme.controls.toggle.labelGap
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xxs

        Text {
            id: label
            text: root.text
            visible: text.length > 0
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.titlebar.fontSize
        }
        Text {
            text: root.description
            visible: text.length > 0
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
    }

    HoverHandler { id: toggleHover }

    TapHandler {
        onTapped: root.toggle()
    }

    FocusRing {
        target: root
        cornerRadius: root.height / 2
        shown: root.activeFocus
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                || event.key === Qt.Key_Enter) {
            root.toggle();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.Switch
    Accessible.name: root.text
    Accessible.description: root.description
    Accessible.checkable: true
    Accessible.checked: root.checked
    Accessible.focusable: true
    Accessible.onPressAction: root.toggle()
}
