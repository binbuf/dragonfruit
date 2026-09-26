// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The design-system polkit authentication dialog (T-13.6): the visual half of
// the shell's authentication agent. `ShellController` drives every property
// from `PolkitAgent` (which owns the D-Bus agent and relays the host helper's
// PAM conversation); this view is pure and never talks to D-Bus.
//
// The password is never typed here: the shell feeds captured key events into
// the agent's response buffer (the lock-screen path), and this view renders the
// buffer masked (or in the clear when PAM asked for an echo-on answer).
Item {
    id: root

    // The action's human message (from polkit), e.g. "Authentication is
    // required to manage system services.".
    property string message: ""
    // The polkit action id, shown in the details.
    property string actionId: ""
    // The account being authenticated (a username).
    property string identityLabel: ""
    // The action's key/value details, as a list of {key, value} maps.
    property var details: []
    // The current PAM question, e.g. "Password:".
    property string prompt: qsTr("Password:")
    // True when the answer may be shown (PAM_PROMPT_ECHO_ON).
    property bool promptEcho: false
    // Non-fatal PAM information text.
    property string info: ""
    // A PAM error or a failed attempt; empty when clean.
    property string errorText: ""
    // True while a PAM attempt is in flight.
    property bool busy: false
    // The current response buffer; only shown when `promptEcho` is true.
    property string responseText: ""
    // The length of the response buffer (for the password mask).
    property int responseLength: 0

    readonly property bool canSubmit: responseLength > 0 && !busy

    function mask() {
        var value = "";
        for (var i = 0; i < root.responseLength; ++i)
            value += "•";
        return value;
    }

    function submit() {
        if (root.canSubmit)
            root.submitted();
    }

    signal submitted()
    signal cancelled()

    Accessible.role: Accessible.Dialog
    Accessible.name: qsTr("Authentication Required")

    // The subtle full-bleed scrim; the surface is exactly the card's size.
    Rectangle {
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: 0.35
    }

    Shadow {
        anchors.fill: card
        radius: card.radius
        level: "overlay"
    }

    Rectangle {
        id: card
        objectName: "polkitCard"
        anchors.fill: parent
        radius: Theme.controls.window.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        Column {
            id: body
            anchors.fill: parent
            anchors.margins: Theme.primitive.spacing.lg
            spacing: Theme.primitive.spacing.md

            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.md

                // The identity disc with the account's initial.
                Rectangle {
                    id: avatar
                    objectName: "polkitAvatar"
                    anchors.verticalCenter: parent.verticalCenter
                    width: 44
                    height: 44
                    radius: width / 2
                    color: Theme.color.accent

                    Text {
                        anchors.centerIn: parent
                        text: root.identityLabel.length > 0
                              ? root.identityLabel.charAt(0).toUpperCase() : "?"
                        color: Theme.color.accentContent
                        font.pixelSize: Theme.primitive.font.sizeXl
                        font.weight: Theme.primitive.font.weightSemibold
                    }
                }

                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - avatar.width - parent.spacing
                    spacing: 2

                    Text {
                        id: heading
                        objectName: "polkitTitle"
                        width: parent.width
                        text: qsTr("Authentication Required")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeLg
                        font.weight: Theme.primitive.font.weightSemibold
                        elide: Text.ElideRight
                    }

                    Text {
                        id: subtitle
                        objectName: "polkitIdentity"
                        width: parent.width
                        visible: root.identityLabel.length > 0
                        text: root.identityLabel
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        elide: Text.ElideRight
                    }
                }
            }

            Text {
                id: messageText
                objectName: "polkitMessage"
                width: parent.width
                text: root.message
                visible: root.message.length > 0
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            // The prompt label + the response field.
            Text {
                id: promptLabel
                objectName: "polkitPrompt"
                width: parent.width
                text: root.prompt
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Rectangle {
                id: field
                objectName: "polkitField"
                width: parent.width
                height: 34
                radius: Theme.primitive.radius.md
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: root.errorText.length > 0 ? Theme.color.danger
                                                        : Theme.color.border
                antialiasing: true

                Text {
                    objectName: "polkitFieldPlaceholder"
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.primitive.spacing.md
                    visible: root.responseLength === 0
                    text: root.busy ? qsTr("Authenticating…") : qsTr("Enter your password")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeMd
                }

                Text {
                    objectName: "polkitFieldValue"
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.primitive.spacing.md
                    visible: root.responseLength > 0
                    text: root.promptEcho ? root.responseText : root.mask()
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.primitive.font.sizeMd
                    elide: Text.ElideRight
                    width: parent.width - 2 * Theme.primitive.spacing.md
                }
            }

            Text {
                id: statusText
                objectName: "polkitStatus"
                width: parent.width
                height: (root.errorText.length > 0 || root.info.length > 0) ? implicitHeight : 0
                visible: root.errorText.length > 0 || root.info.length > 0
                text: root.errorText.length > 0 ? root.errorText : root.info
                color: root.errorText.length > 0 ? Theme.color.danger : Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            // The action details, collapsed by default. The action id is always
            // included; clicking the header expands the rest.
            Item {
                id: detailsBox
                width: parent.width
                height: detailsHeader.height + (detailsList.visible ? detailsList.height : 0)

                Text {
                    id: detailsHeader
                    objectName: "polkitDetailsToggle"
                    width: parent.width
                    text: detailsList.visible ? qsTr("Hide details") : qsTr("Show details")
                    color: Theme.color.accent
                    font.pixelSize: Theme.primitive.font.sizeSm
                    MouseArea {
                        anchors.fill: parent
                        onClicked: detailsList.visible = !detailsList.visible
                    }
                }

                Column {
                    id: detailsList
                    objectName: "polkitDetails"
                    anchors.top: detailsHeader.bottom
                    width: parent.width
                    visible: false

                    Text {
                        width: parent.width
                        text: qsTr("Action: %1").arg(root.actionId)
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        elide: Text.ElideRight
                        visible: root.actionId.length > 0
                    }

                    Repeater {
                        model: root.details
                        delegate: Text {
                            required property var modelData
                            width: parent.width
                            text: "%1: %2".arg(modelData.key).arg(modelData.value)
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Row {
                id: footer
                width: parent.width
                height: Theme.controls.button.height
                spacing: Theme.controls.dialog.buttonGap

                Item {
                    width: parent.width - cancelButton.width - acceptButton.width
                            - 2 * footer.spacing
                    height: 1
                }

                Button {
                    id: cancelButton
                    objectName: "polkitCancel"
                    text: qsTr("Cancel")
                    variant: "secondary"
                    onClicked: root.cancelled()
                }

                Button {
                    id: acceptButton
                    objectName: "polkitAccept"
                    text: qsTr("Authenticate")
                    variant: "primary"
                    enabled: root.canSubmit
                    onClicked: root.submit()
                }
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.cancelled();
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.submit();
        event.accepted = true;
    }
    Keys.onEnterPressed: (event) => {
        root.submit();
        event.accepted = true;
    }
}