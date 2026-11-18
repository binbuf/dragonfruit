// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The About pane hero (T286): the product mark, the host name, and a subtitle.
// The host identity comes from the provider; the pane never invents it.
Item {
    id: root

    property string title: ""
    property string subtitle: ""
    property url artwork: ""
    property alias titleText: titleLabel
    property alias subtitleText: subtitleLabel
    property alias mark: mark

    implicitWidth: 320
    implicitHeight: layout.implicitHeight

    Row {
        id: layout
        width: parent.width
        spacing: Theme.primitive.spacing.lg

        Item {
            id: mark
            width: Theme.primitive.spacing.xxxl
            height: width
            anchors.verticalCenter: parent.verticalCenter

            Image {
                anchors.fill: parent
                visible: root.artwork.toString().length > 0
                source: root.artwork
                fillMode: Image.PreserveAspectFit
                asynchronous: true
            }

            Rectangle {
                anchors.fill: parent
                visible: root.artwork.toString().length === 0
                radius: width / 2
                color: Theme.color.controlFill
                antialiasing: true
            }
        }

        Column {
            width: Math.max(0, layout.width - mark.width - layout.spacing)
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.primitive.spacing.xxs

            Text {
                id: titleLabel
                width: parent.width
                text: root.title
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeXxl
                font.weight: Theme.primitive.font.weightBold
                elide: Text.ElideRight
                horizontalAlignment: root.LayoutMirroring.enabled
                                     ? Text.AlignRight : Text.AlignLeft
            }

            Text {
                id: subtitleLabel
                width: parent.width
                visible: root.subtitle.length > 0
                text: root.subtitle
                color: Theme.color.textSecondary
                font.pixelSize: Theme.controls.button.fontSize
                wrapMode: Text.WordWrap
                horizontalAlignment: root.LayoutMirroring.enabled
                                     ? Text.AlignRight : Text.AlignLeft
            }
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.title
    Accessible.description: root.subtitle
}
