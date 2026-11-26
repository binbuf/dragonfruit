// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// The user/account avatar (T286): a round plate with the account's initials,
// or an image when the provider supplies one. The host owns the identity; this
// component never invents a name.
Item {
    id: root

    property string initials: ""
    property url source: ""
    property color fillColor: Theme.color.controlFill
    property alias label: initialsText
    property alias image: avatarImage

    implicitWidth: Theme.controls.settingsAvatar.size
    implicitHeight: Theme.controls.settingsAvatar.size

    Rectangle {
        anchors.fill: parent
        radius: width / 2
        color: root.fillColor
        clip: true
        antialiasing: true

        Image {
            id: avatarImage
            anchors.fill: parent
            visible: root.source.toString().length > 0
            source: root.source
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            cache: true
        }

        Text {
            id: initialsText
            anchors.centerIn: parent
            visible: root.source.toString().length === 0
            text: root.initials
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeMd
            font.weight: Theme.primitive.font.weightSemibold
        }
    }

    Accessible.role: Accessible.Graphic
    Accessible.name: root.initials
}
