// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// The Appearance mode thumbnail (T286): a small preview plate, a label, and a
// selected ring. Light/Dark/Auto are three of these, not a segmented control.
Item {
    id: root

    property string label: ""
    property color previewFill: Theme.color.surface
    property color previewAccent: Theme.color.accent
    property bool selected: false
    property alias preview: preview
    property alias ring: ring

    signal clicked()

    implicitWidth: Theme.controls.settingsModeTile.width
    implicitHeight: Theme.controls.settingsModeTile.height
                    + labelText.implicitHeight + Theme.primitive.spacing.sm
    activeFocusOnTab: true

    Rectangle {
        id: preview
        width: parent.width
        height: Theme.controls.settingsModeTile.height
        radius: Theme.controls.settingsModeTile.radius
        color: root.previewFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: parent.width * 0.32
            radius: parent.radius
            color: Qt.rgba(root.previewAccent.r, root.previewAccent.g,
                           root.previewAccent.b, 0.18)
            antialiasing: true
        }

        Rectangle {
            anchors.centerIn: parent
            width: parent.width * 0.34
            height: width
            radius: width / 2
            color: root.previewAccent
            opacity: 0.85
            antialiasing: true
        }
    }

    Rectangle {
        id: ring
        anchors.fill: preview
        anchors.margins: -Theme.primitive.spacing.xxs
        radius: Theme.controls.settingsModeTile.radius
                 + Theme.primitive.spacing.xxs
        color: "transparent"
        border.width: root.selected ? Theme.controls.focusRing.width
                                    : Theme.controls.window.borderWidth
        border.color: root.selected ? Theme.color.accent
                                    : "transparent"
        antialiasing: true
    }

    Text {
        id: labelText
        anchors.top: preview.bottom
        anchors.topMargin: Theme.primitive.spacing.xs
        width: parent.width
        text: root.label
        color: Theme.color.textPrimary
        font.pixelSize: Theme.primitive.font.sizeSm
        elide: Text.ElideRight
        horizontalAlignment: Text.AlignHCenter
    }

    TapHandler {
        onTapped: root.clicked()
    }

    FocusRing {
        target: root
        cornerRadius: Theme.controls.settingsModeTile.radius
        shown: root.activeFocus
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                || event.key === Qt.Key_Space) {
            root.clicked();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.RadioButton
    Accessible.name: root.label
    Accessible.checkable: true
    Accessible.checked: root.selected
    Accessible.focusable: true
    Accessible.onPressAction: root.clicked()
}
