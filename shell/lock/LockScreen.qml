// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The first-party lock screen (T-12.3a): a pure view the shell controller
// renders into the compositor's `ext-session-lock-v1` surfaces, one per
// output, so the locked session covers every screen. Authentication (PAM) and
// input capture land in T-12.3b/T-12.3c; this slice presents the fail-secure
// lock and its identity.
Item {
    id: root

    // The signed-in account shown on the card. The session owner is fixed for
    // T-12.3a; PAM authentication (T-12.3b) binds the real user.
    property string userName: qsTr("Dragonfruit")
    // Live clock, driven by the shell controller each render.
    property string timeText: ""
    property string dateText: ""
    // Authentication state (T-12.3b). Until then the field is present but
    // inert, so the layout is final.
    property bool authEnabled: false
    property bool authBusy: false
    // A short status line under the field (wrong password, PAM unavailable).
    property string message: ""

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Lock screen")
    Accessible.description: root.message.length > 0 ? root.message
                                                     : qsTr("Session locked.")

    // The dimmed backdrop. The compositor composites this above the desktop,
    // so it is opaque and hides every window underneath.
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop {
                position: 0.0
                color: Qt.darker(Theme.color.surfaceSunken, 1.15)
            }
            GradientStop {
                position: 1.0
                color: Theme.color.surfaceSunken
            }
        }
    }

    // The clock: large, high on the screen, the primary lock-screen anchor.
    Column {
        id: clock
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: Math.max(Theme.primitive.spacing.xxxl,
                                    Math.round(parent.height * 0.12))
        spacing: Theme.primitive.spacing.xxs

        Text {
            objectName: "lockClock"
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.timeText
            color: Theme.color.textPrimary
            font.pixelSize: Math.round(Theme.primitive.font.sizeDisplay * 2.4)
            font.weight: Theme.primitive.font.weightSemibold
        }
        Text {
            objectName: "lockDate"
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.dateText
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeXl
            font.weight: Theme.primitive.font.weightMedium
        }
    }

    // The identity card: avatar, name, password field, and status line.
    Shadow {
        anchors.fill: card
        radius: card.radius
        level: "overlay"
    }

    Rectangle {
        id: card
        objectName: "lockCard"
        anchors.centerIn: parent
        width: Math.min(360, parent.width - 2 * Theme.primitive.spacing.xl)
        height: content.implicitHeight + 2 * Theme.primitive.spacing.xl
        radius: Theme.primitive.radius.xl
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        Column {
            id: content
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: Theme.primitive.spacing.xl
            spacing: Theme.primitive.spacing.md

            // The avatar disc with the account's initial.
            Rectangle {
                id: avatar
                objectName: "lockAvatar"
                anchors.horizontalCenter: parent.horizontalCenter
                width: 72
                height: 72
                radius: width / 2
                color: Theme.color.accent

                Text {
                    anchors.centerIn: parent
                    text: root.userName.length > 0 ? root.userName.charAt(0).toUpperCase() : "?"
                    color: Theme.color.accentContent
                    font.pixelSize: Theme.primitive.font.sizeDisplay
                    font.weight: Theme.primitive.font.weightSemibold
                }
            }

            Text {
                objectName: "lockUserName"
                anchors.horizontalCenter: parent.horizontalCenter
                text: root.userName
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeXl
                font.weight: Theme.primitive.font.weightSemibold
            }

            // The password field. T-12.3b wires it to PAM; here it is the
            // visual contract the compositor's input capture will feed.
            Rectangle {
                id: field
                objectName: "lockPasswordField"
                width: parent.width
                height: 34
                radius: Theme.primitive.radius.md
                color: Theme.color.controlFill
                border.width: root.authEnabled ? Theme.controls.window.borderWidth : 0
                border.color: Theme.color.border
                antialiasing: true

                Text {
                    objectName: "lockPasswordPlaceholder"
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.primitive.spacing.md
                    text: root.authBusy ? qsTr("Authenticating…")
                                        : (root.authEnabled ? qsTr("Enter Password")
                                                            : qsTr("Press Enter to unlock"))
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeMd
                }
            }

            Text {
                objectName: "lockMessage"
                anchors.horizontalCenter: parent.horizontalCenter
                visible: root.message.length > 0
                text: root.message
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }
    }

    // The lock glyph above the card, so the state reads at a glance.
    Item {
        id: glyph
        objectName: "lockGlyph"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: card.top
        anchors.bottomMargin: Theme.primitive.spacing.xl
        width: 44
        height: 52

        Rectangle {
            // The shackle.
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top: parent.top
            width: 34
            height: 34
            radius: width / 2
            color: "transparent"
            border.width: 5
            border.color: Theme.color.textSecondary
            antialiasing: true
        }
        Rectangle {
            // The body, drawn over the shackle's lower half.
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            width: 44
            height: 34
            radius: Theme.primitive.radius.md
            color: Theme.color.textPrimary
            antialiasing: true

            Rectangle {
                anchors.centerIn: parent
                width: 6
                height: 6
                radius: width / 2
                color: Theme.color.surfaceElevated
            }
        }
    }
}