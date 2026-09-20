// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The one push button. Supporting component (not one of the twenty library
// entries) used by Toolbar, Dialog, Sheet, Popover, and SettingsRow so those
// components never hand-roll a clickable control. Variants map to semantic
// roles; every value comes from the token layer.
Item {
    id: root

    property string text: ""
    property string icon: ""
    // primary | secondary | danger | ghost
    property string variant: "secondary"
    property alias hovered: hoverHandler.hovered
    property alias pressed: tapHandler.pressed

    signal clicked()

    readonly property color fill: {
        switch (root.variant) {
        case "primary":
            return root.hovered ? Theme.color.accentHover : Theme.color.accent;
        case "danger":
            return Theme.color.danger;
        case "ghost":
            return root.hovered ? Theme.color.controlHover : "transparent";
        default:
            return root.hovered ? Theme.color.controlHover : Theme.color.controlFill;
        }
    }
    readonly property color contentColor: {
        switch (root.variant) {
        case "primary":
        case "danger":
            return Theme.color.accentContent;
        default:
            return Theme.color.textPrimary;
        }
    }

    implicitWidth: Math.max(Theme.controls.button.height,
                            content.implicitWidth + 2 * Theme.controls.button.paddingH)
    implicitHeight: Theme.controls.button.height
    activeFocusOnTab: true
    opacity: enabled ? 1.0 : 0.45

    function activate() {
        if (root.enabled)
            root.clicked();
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.button.radius
        color: root.fill
        border.width: root.variant === "ghost" ? 0 : Theme.controls.window.borderWidth
        border.color: root.variant === "secondary" ? Theme.color.border : "transparent"
        antialiasing: true

        Behavior on color {
            ColorAnimation {
                duration: Theme.motion.hover.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.hover.curve
            }
        }
    }

    Row {
        id: content
        anchors.centerIn: parent
        spacing: Theme.primitive.spacing.sm

        Icon {
            visible: root.icon.length > 0
            name: root.icon
            size: Theme.controls.button.iconSize
            color: root.contentColor
            anchors.verticalCenter: parent.verticalCenter
        }
        Text {
            visible: root.text.length > 0
            text: root.text
            color: root.contentColor
            font.pixelSize: Theme.controls.button.fontSize
            font.weight: Theme.controls.button.fontWeight
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    HoverHandler { id: hoverHandler }

    TapHandler {
        id: tapHandler
        onTapped: root.activate()
    }

    FocusRing {
        target: root
        cornerRadius: Theme.controls.button.radius
        shown: root.activeFocus
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                || event.key === Qt.Key_Enter) {
            root.activate();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.Button
    Accessible.name: root.text.length > 0 ? root.text : root.icon
    Accessible.focusable: true
    Accessible.onPressAction: root.activate()
}
