// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// A wallpaper thumbnail (T286): a preview plate, a selected ring, and the
// collection item's name. The pane supplies the preview source or colour.
Item {
    id: root

    property string label: ""
    property url source: ""
    property color previewColor: Theme.color.surfaceSunken
    property bool selected: false
    property alias preview: preview
    property alias ring: ring

    signal clicked()

    implicitWidth: Theme.controls.settingsWallTile.width
    implicitHeight: Theme.controls.settingsWallTile.height
                    + labelText.implicitHeight + Theme.primitive.spacing.sm
    activeFocusOnTab: true

    Rectangle {
        id: preview
        width: parent.width
        height: Theme.controls.settingsWallTile.height
        radius: Theme.controls.settingsWallTile.radius
        color: root.previewColor
        clip: true
        antialiasing: true

        Image {
            anchors.fill: parent
            visible: root.source.toString().length > 0
            source: root.source
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            cache: true
        }
    }

    Rectangle {
        id: ring
        anchors.fill: preview
        radius: Theme.controls.settingsWallTile.radius
        color: "transparent"
        border.width: root.selected ? Theme.controls.focusRing.width
                                    : Theme.controls.window.borderWidth
        border.color: root.selected ? Theme.color.accent
                                    : Theme.color.border
        antialiasing: true
    }

    Text {
        id: labelText
        anchors.top: preview.bottom
        anchors.topMargin: Theme.primitive.spacing.xs
        width: parent.width
        text: root.label
        color: Theme.color.textSecondary
        font.pixelSize: Theme.primitive.font.sizeSm
        elide: Text.ElideRight
        horizontalAlignment: Text.AlignHCenter
    }

    TapHandler {
        onTapped: root.clicked()
    }

    FocusRing {
        target: root
        cornerRadius: Theme.controls.settingsWallTile.radius
        shown: root.activeFocus
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                || event.key === Qt.Key_Space) {
            root.clicked();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.ListItem
    Accessible.name: root.label
    Accessible.selected: root.selected
    Accessible.selectable: true
    Accessible.focusable: true
    Accessible.onPressAction: root.clicked()
}
