// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// The System Settings help control (T286): a 24 pt circle pinned 20 pt from
// the content's right and bottom edges. It opens the live business-logic map
// for the current pane, so it is never a dead placeholder. With no document
// installed the control disables and explains why.
Item {
    id: root

    property bool available: true
    property string unavailableReason: qsTr("The business-logic map is not installed.")
    property string documentUrl: ""
    property string accessibleDescription: qsTr("Open this pane in the business-logic map.")
    property alias circle: circle
    property alias glyph: glyph

    signal helpRequested()

    implicitWidth: Theme.controls.settingsWindow.helpButtonSize
    implicitHeight: Theme.controls.settingsWindow.helpButtonSize
    activeFocusOnTab: root.available

    readonly property bool hovered: helpHover.hovered

    Rectangle {
        id: circle
        anchors.fill: parent
        radius: width / 2
        color: root.hovered && root.available ? Theme.color.controlHover
                                              : Theme.color.controlFill
        border.width: Theme.controls.window.borderWidth
        border.color: Glass.increaseContrast ? Theme.color.textPrimary
                                             : Theme.color.border
        antialiasing: true
    }

    Text {
        id: glyph
        anchors.centerIn: parent
        text: "?" // df-allow-untranslated: help glyph
        color: root.available
               ? (Glass.increaseContrast ? Theme.color.textPrimary
                                         : Theme.color.textSecondary)
               : Theme.color.textTertiary
        font.pixelSize: Theme.primitive.font.sizeSm
        font.weight: Theme.primitive.font.weightSemibold
    }

    HoverHandler { id: helpHover }

    TapHandler {
        onTapped: {
            if (root.available)
                root.helpRequested();
        }
    }

    FocusRing {
        target: root
        cornerRadius: width / 2
        shown: root.activeFocus
    }

    Keys.onPressed: (event) => {
        if (!root.available)
            return;
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                || event.key === Qt.Key_Space) {
            root.helpRequested();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.Button
    Accessible.name: qsTr("Help")
    Accessible.description: root.available
                            ? root.accessibleDescription
                            : root.unavailableReason
    Accessible.focusable: root.available
    Accessible.onPressAction: {
        if (root.available)
            root.helpRequested();
    }
}
