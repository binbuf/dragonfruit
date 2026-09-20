// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A modal dialog: a scrim over the host plus a centred card. `show()`/
// `accept()`/`reject()` drive it; Escape rejects (when dismissible) and Return
// accepts. The card is AT-SPI Dialog and traps focus while open. Use Sheet for
// the window-attached variant.
FocusScope {
    id: root

    property string title: ""
    property string message: ""
    property alias contentData: contentColumn.data
    property alias content: contentColumn
    property alias buttonsData: buttonRow.data
    property alias buttons: buttonRow
    property bool open: false
    property bool dismissible: true
    property alias card: card

    signal accepted()
    signal rejected()
    signal opened()
    signal closed()

    readonly property int padding: Theme.controls.dialog.padding
    readonly property real cardWidth: Math.max(Theme.controls.dialog.minWidth,
        Math.max(titleText.implicitWidth,
                 Math.max(messageText.implicitWidth,
                          Math.max(contentColumn.implicitWidth,
                                   buttonRow.implicitWidth))) + 2 * root.padding)

    width: parent ? parent.width : 480
    height: parent ? parent.height : 320
    visible: opacity > 0
    z: 3000
    focus: root.open
    activeFocusOnTab: root.open

    function show() { root.open = true; }
    function hide() { root.open = false; }
    function accept() {
        root.hide();
        root.accepted();
    }
    function reject() {
        if (!root.dismissible)
            return;
        root.hide();
        root.rejected();
    }

    onOpenChanged: {
        if (root.open) {
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: root.open ? 0.4 : 0.0

        Behavior on opacity {
            NumberAnimation {
                duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve : Theme.motion.popupClose.curve
            }
        }
    }

    Item {
        id: card
        anchors.centerIn: parent
        width: root.cardWidth
        height: body.implicitHeight + 2 * root.padding
        scale: root.open ? 1.0 : 0.97
        opacity: root.open ? 1.0 : 0.0
        transformOrigin: Item.Center

        Behavior on opacity {
            NumberAnimation {
                duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve : Theme.motion.popupClose.curve
            }
        }
        Behavior on scale {
            NumberAnimation {
                duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve : Theme.motion.popupClose.curve
            }
        }

        Shadow {
            width: card.width
            height: card.height
            radius: Theme.controls.dialog.radius
        }

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.dialog.radius
            color: Theme.color.surfaceElevated
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        Column {
            id: body
            x: root.padding
            y: root.padding
            width: card.width - 2 * root.padding
            spacing: Theme.primitive.spacing.md

            Text {
                id: titleText
                width: parent.width
                visible: root.title.length > 0
                text: root.title
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                wrapMode: Text.WordWrap
            }

            Text {
                id: messageText
                width: parent.width
                visible: root.message.length > 0
                text: root.message
                color: Theme.color.textSecondary
                font.pixelSize: Theme.controls.button.fontSize
                wrapMode: Text.WordWrap
            }

            Column {
                id: contentColumn
                width: parent.width
            }

            Item {
                width: parent.width
                height: buttonRow.height

                Row {
                    id: buttonRow
                    anchors.right: parent.right
                    spacing: Theme.controls.dialog.buttonGap
                }
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.reject();
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.accept();
        event.accepted = true;
    }

    Accessible.role: Accessible.Dialog
    Accessible.name: root.title
}
