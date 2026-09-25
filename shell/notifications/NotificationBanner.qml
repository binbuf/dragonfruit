// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One notification banner card (T-11.1a; actions and activation T-11.1b).
//
// The shell controller renders this into the top-right `notification` overlay
// surface; the properties are the newest active notification from the
// service's `Banners()` view. The card grows to fit an inline action row when
// the app supplied actions, and the whole body clicks (the app's `default`
// action if it registered one, otherwise a dismiss). `actionInvoked` and
// `activated` are the shell controller's hooks into the notification service.
Item {
    id: root

    property string appName: ""
    property string summary: ""
    property string body: ""
    property string urgency: "normal"
    // The service's `actions` view: a list of { key, label } maps.
    property var actions: []

    signal actionInvoked(string key)
    signal activated()

    readonly property bool hasActions: actions.length > 0
    readonly property int cardHeight: hasActions ? 132 : 96

    readonly property int padding: Theme.primitive.spacing.md
    readonly property color accent: root.urgency === "critical" ? Theme.color.danger
                                    : root.urgency === "low" ? Theme.color.textTertiary
                                    : Theme.color.accent

    // The scene is exactly the surface; the transparent area below the card
    // passes clicks through (the shell only makes the card clickable).
    width: 380
    height: 132

    Shadow {
        anchors.fill: card
        radius: card.radius
        level: "overlay"
    }

    Rectangle {
        id: card
        width: parent.width
        height: root.cardHeight
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
            // Leave room for the action row when the card carries one.
            anchors.bottom: root.hasActions ? actionsRow.top : parent.bottom
            anchors.leftMargin: Theme.primitive.spacing.sm
            anchors.rightMargin: root.padding
            anchors.topMargin: root.padding
            anchors.bottomMargin: root.hasActions ? Theme.primitive.spacing.xs : root.padding
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
                maximumLineCount: root.hasActions ? 1 : 2
                elide: Text.ElideRight
            }
        }

        // Clicking the body fires the app's default action (or dismisses).
        MouseArea {
            id: bodyArea
            anchors.left: content.left
            anchors.right: content.right
            anchors.top: content.top
            anchors.bottom: root.hasActions ? actionsRow.top : content.bottom
            onClicked: root.activated()
        }

        Row {
            id: actionsRow
            objectName: "actionsRow"
            visible: root.hasActions
            height: 28
            anchors.left: content.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.rightMargin: root.padding
            anchors.bottomMargin: Theme.primitive.spacing.sm
            spacing: Theme.primitive.spacing.sm

            Repeater {
                model: root.actions
                delegate: Rectangle {
                    required property var modelData
                    objectName: "action_" + modelData.key
                    height: actionsRow.height
                    width: actionLabel.implicitWidth + 2 * Theme.primitive.spacing.md
                    radius: Theme.controls.sidebar.rowRadius
                    color: actionArea.containsMouse ? Theme.color.surfaceMuted
                                                    : "transparent"
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border

                    Text {
                        id: actionLabel
                        anchors.centerIn: parent
                        text: modelData.label
                        color: Theme.color.accent
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.weight: Theme.primitive.font.weightMedium
                    }

                    MouseArea {
                        id: actionArea
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: root.actionInvoked(modelData.key)
                    }
                }
            }
        }
    }

    Accessible.role: Accessible.Alert
    Accessible.name: root.summary
    Accessible.description: root.body
}