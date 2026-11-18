// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A collection heading with an optional "Show All" link (T286): the Wallpaper
// collection rows use one of these above their tiles.
Item {
    id: root

    property string title: ""
    property string actionText: ""
    property alias titleText: titleLabel
    property alias actionLink: actionLabel

    signal actionClicked()

    implicitWidth: 320
    implicitHeight: Math.max(titleLabel.implicitHeight,
                             actionLabel.visible ? actionLabel.implicitHeight : 0)
                      + Theme.controls.settingsGroup.headingBottom

    Text {
        id: titleLabel
        anchors.left: parent.left
        anchors.right: actionLabel.visible ? actionLabel.left : parent.right
        anchors.rightMargin: Theme.primitive.spacing.sm
        anchors.verticalCenter: parent.verticalCenter
        text: root.title
        color: Theme.color.textPrimary
        font.pixelSize: Theme.controls.settingsGroup.headingFontSize
        font.weight: Theme.controls.settingsGroup.headingFontWeight
        elide: Text.ElideRight
        horizontalAlignment: root.LayoutMirroring.enabled
                             ? Text.AlignRight : Text.AlignLeft
    }

    Text {
        id: actionLabel
        visible: root.actionText.length > 0
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        text: root.actionText
        color: Theme.color.accent
        font.pixelSize: Theme.primitive.font.sizeSm
        activeFocusOnTab: true

        TapHandler {
            onTapped: root.actionClicked()
        }

        FocusRing {
            target: actionLabel
            cornerRadius: Theme.primitive.radius.xs
            shown: actionLabel.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                    || event.key === Qt.Key_Space) {
                root.actionClicked();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.Link
        Accessible.name: root.actionText
        Accessible.focusable: true
        Accessible.onPressAction: root.actionClicked()
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.title
}
