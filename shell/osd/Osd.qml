// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The on-screen display (T-11.4a): a brief, non-interactive volume/brightness
// overlay shown on the active output. The shell controller renders this into a
// centered `overlay` surface and drives every property; the QML is a pure view.
//
// `fade` is the model's presentation curve. `ShellController` sets it each
// frame from `OsdModel::fade`, so the reduced-motion path arrives here as an
// immediate 1.0 (no QML animation to disable) and the auto-dismiss is the
// controller's tick. The item reserves no input.
Item {
    id: root

    // "volume" | "brightness"; empty draws nothing meaningful but is harmless.
    property string kind: ""
    // Level in [0, 1] for the track fill.
    property real value: 0.0
    property bool muted: false
    // Presentation opacity [0, 1] from the OSD model.
    property real fade: 0.0

    // T-11.4b: an AT-SPI/assistive client (or a keyboard Escape) can dismiss
    // the transient alert early. The shell forwards this to `OsdModel::hide`.
    signal dismissed()

    readonly property bool isBrightness: root.kind === "brightness"
    readonly property string iconName: root.isBrightness ? "brightness" : "volume"
    readonly property string accessibleName: {
        var label = root.isBrightness ? qsTr("Brightness") : qsTr("Volume");
        if (root.muted)
            return qsTr("%1 muted").arg(label);
        return qsTr("%1 %2%").arg(label).arg(Math.round(root.value * 100));
    }
    // The alert's second AT-SPI line: the state plus how to clear it. The
    // surface never takes keyboard focus, so the shell both announces this and
    // routes Escape to dismissal (T-11.4b).
    readonly property string accessibleDescription: {
        if (root.muted)
            return qsTr("Volume is muted. Press Escape to dismiss.");
        return root.isBrightness
                ? qsTr("Brightness %1 percent. Press Escape to dismiss.").arg(Math.round(root.value * 100))
                : qsTr("Volume %1 percent. Press Escape to dismiss.").arg(Math.round(root.value * 100));
    }

    opacity: root.fade

    Accessible.role: Accessible.Alert
    Accessible.name: root.accessibleName
    Accessible.description: root.accessibleDescription
    // Exposed to AT-SPI as an invokable alert; the view never steals keyboard
    // focus (its window is unfocused), so `activeFocus` stays false in a real
    // session. `Accessible.onPressAction` lets a screen reader clear it.
    Accessible.focusable: true
    Accessible.onPressAction: root.dismissed()

    Keys.onEscapePressed: (event) => {
        root.dismissed();
        event.accepted = true;
    }

    Shadow {
        anchors.fill: card
        radius: card.radius
        level: "overlay"
    }

    Rectangle {
        id: card
        objectName: "osdCard"
        anchors.centerIn: parent
        width: Theme.controls.osd.width
        height: Theme.controls.osd.height
        radius: Theme.controls.osd.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        Column {
            id: content
            anchors.centerIn: parent
            width: parent.width - 2 * Theme.controls.osd.padding
            spacing: Theme.controls.osd.gap

            Icon {
                id: glyph
                objectName: "osdIcon"
                anchors.horizontalCenter: parent.horizontalCenter
                name: root.iconName
                size: Theme.controls.osd.iconSize
                color: root.muted ? Theme.color.textTertiary : Theme.color.textPrimary
            }

            Rectangle {
                id: track
                objectName: "osdTrack"
                width: parent.width
                height: Theme.controls.osd.trackHeight
                radius: Theme.controls.osd.trackRadius
                color: Theme.color.controlFill
                antialiasing: true

                Rectangle {
                    id: fill
                    objectName: "osdFill"
                    width: parent.width * (root.muted ? 0.0 : Math.max(0.0, Math.min(1.0,
                                                                                   root.value)))
                    height: parent.height
                    radius: parent.radius
                    color: Theme.color.accent
                    antialiasing: true
                }
            }

            Text {
                id: label
                objectName: "osdLabel"
                anchors.horizontalCenter: parent.horizontalCenter
                text: root.muted ? qsTr("Muted")
                                 : qsTr("%1%").arg(Math.round(root.value * 100))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.controls.osd.fontSize
            }
        }
    }
}