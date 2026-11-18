// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The session power confirmation dialog. It is a pure view: the shell
// controller chooses the action and owns the logind call, while this scene
// draws the icon, the confirmation copy, the live countdown, and the two
// actions. The action button is the default (Return confirms); Escape and
// Cancel dismiss. The countdown starts at 60 and emits `expired` at zero so
// an unattended dialog still completes the action.
Item {
    id: root

    // "shut-down" or "restart".
    property string action: "shut-down"
    // Seconds left before the action runs automatically. The shell resets this
    // to 60 when it opens the dialog.
    property int secondsRemaining: 60
    // True while the dialog is the live scene. Clearing it stops the countdown
    // so a hidden scene cannot keep waking the shell.
    property bool active: false

    readonly property bool isRestart: root.action === "restart"
    readonly property string heading: root.isRestart ? qsTr("Restart") : qsTr("Shut Down")
    readonly property string question: root.isRestart
        ? qsTr("Are you sure you want to restart your computer now?")
        : qsTr("Are you sure you want to shut down your computer now?")
    readonly property string countdownText: root.isRestart
        ? (root.secondsRemaining === 1
            ? qsTr("If you do nothing, the computer will restart automatically in 1 second.")
            : qsTr("If you do nothing, the computer will restart automatically in %1 seconds.").arg(root.secondsRemaining))
        : (root.secondsRemaining === 1
            ? qsTr("If you do nothing, the computer will shut down automatically in 1 second.")
            : qsTr("If you do nothing, the computer will shut down automatically in %1 seconds.").arg(root.secondsRemaining))

    signal confirmed()
    signal cancelled()
    signal expired()

    function confirm() {
        root.confirmed();
    }

    function cancel() {
        root.cancelled();
    }

    // The default action owns the keyboard, so Return confirms and the focus
    // ring starts on the action the user is about to take.
    function focusDefault() {
        confirmButton.forceActiveFocus();
    }

    // One countdown step. The timer calls this once per second; the shell's
    // QML test calls it directly so a 60-second wait stays a unit test.
    function advanceCountdown() {
        if (!root.active)
            return;
        if (root.secondsRemaining <= 0) {
            root.expired();
            return;
        }
        root.secondsRemaining = root.secondsRemaining - 1;
        if (root.secondsRemaining === 0)
            root.expired();
    }

    Timer {
        id: countdownTimer
        objectName: "powerCountdownTimer"
        interval: 1000
        repeat: true
        running: root.active && root.secondsRemaining > 0
        onTriggered: root.advanceCountdown()
    }

    Accessible.role: Accessible.Dialog
    Accessible.name: root.heading
    Accessible.description: root.question

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
        objectName: "powerCard"
        anchors.fill: parent
        radius: Theme.controls.window.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        Column {
            id: body
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Theme.primitive.spacing.lg
            spacing: Theme.primitive.spacing.md

            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.md

                Rectangle {
                    id: glyphDisc
                    width: 48
                    height: 48
                    radius: width / 2
                    color: Theme.color.controlFill

                    PhosphorIcon {
                        objectName: "powerGlyph"
                        anchors.centerIn: parent
                        name: root.isRestart ? "arrows-clockwise" : "power"
                        size: 26
                        color: Theme.color.textPrimary
                    }
                }

                Text {
                    id: headingText
                    objectName: "powerHeading"
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - glyphDisc.width - parent.spacing
                    text: root.heading
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.primitive.font.sizeLg
                    font.weight: Theme.primitive.font.weightSemibold
                    elide: Text.ElideRight
                }
            }

            Text {
                id: questionText
                objectName: "powerQuestion"
                width: parent.width
                text: root.question
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                id: countdownText
                objectName: "powerCountdown"
                width: parent.width
                text: root.countdownText
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        Row {
            id: footer
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: Theme.primitive.spacing.lg
            spacing: Theme.controls.dialog.buttonGap

            Button {
                id: cancelButton
                objectName: "powerCancel"
                text: qsTr("Cancel")
                variant: "secondary"
                onClicked: root.cancel()
            }

            Button {
                id: confirmButton
                objectName: "powerConfirm"
                text: root.heading
                variant: "primary"
                onClicked: root.confirm()
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.cancel();
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.confirm();
        event.accepted = true;
    }
    Keys.onEnterPressed: (event) => {
        root.confirm();
        event.accepted = true;
    }
    // Keep Tab inside the two actions. The scene is a modal surface, so there
    // is nowhere else for traversal to go.
    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Tab || event.key === Qt.Key_Backtab) {
            if (confirmButton.activeFocus)
                cancelButton.forceActiveFocus();
            else
                confirmButton.forceActiveFocus();
            event.accepted = true;
        }
    }
}
