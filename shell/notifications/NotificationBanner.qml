// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One notification banner card (T-11.1a).
//
// The shell controller renders this into the top-right `notification` overlay
// surface; the properties are the newest active notification from the
// service's `Banners()` view. Banners are display-only in T-11.1a; T-11.1b
// adds activation and inline actions.
Item {
    id: root

    property string appName: ""
    property string summary: ""
    property string body: ""
    property string urgency: "normal"
    property bool hasActions: false

    readonly property int padding: Theme.primitive.spacing.md
    readonly property color accent: root.urgency === "critical" ? Theme.color.danger
                                    : root.urgency === "low" ? Theme.color.textTertiary
                                    : Theme.color.accent

    // The scene is exactly the card; the transparent area below is never
    // visible because the surface is sized to the card.
    width: 380
    height: 96

    Shadow {
        anchors.fill: card
        radius: card.radius
        level: "overlay"
    }

    Rectangle {
        id: card
        anchors.fill: parent
        radius: Theme.controls.popup.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        // The urgency stripe, inset with the card's rounded corner left alone.
        Rectangle {
            id: stripe
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: Theme.primitive.spacing.sm
            width: 4
            radius: width / 2
            color: root.accent
        }

        Column {
            id: content
            anchors.left: stripe.right
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.leftMargin: Theme.primitive.spacing.sm
            anchors.rightMargin: root.padding
            anchors.topMargin: root.padding
            anchors.bottomMargin: root.padding
            spacing: Theme.primitive.spacing.xxs

            Text {
                width: parent.width
                visible: root.appName.length > 0
                objectName: "appName"
                text: root.appName
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                visible: root.summary.length > 0
                objectName: "summary"
                text: root.summary
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                visible: root.body.length > 0
                objectName: "body"
                text: root.body
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeMd
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }
        }
    }

    Accessible.role: Accessible.Alert
    Accessible.name: root.summary
    Accessible.description: root.body
}