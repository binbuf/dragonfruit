// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Transfer or Reset sub-pane (T226; T270 completion).
//
// Resets every Dragonfruit settings key to its schema default. It is not a
// factory reset: personal files, installed apps, and host configuration are
// untouched. The confirmation is explicit and destructive-styled.
//
// T270: the macOS Transfer or Reset structure. Transfer is a delegated row (no
// migration assistant ships), Reset is the destructive action with a typed
// confirmation, and Erase All Content is omitted with a Linux reason instead of
// pretending to be a sealed-system erase.
Item {
    id: root

    property alias resetButton: resetButton
    property alias confirmDialog: confirmDialog
    property alias confirmField: confirmField
    property alias confirmButton: confirmButton
    property alias doneText: doneText
    property alias transferGroup: transferGroup
    property alias resetGroup: resetGroup
    property alias eraseNote: eraseNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Text {
            width: parent.width
            text: qsTr("Move your information to another computer, or reset Dragonfruit's own settings.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            id: transferGroup
            width: parent.width
            title: qsTr("Transfer")

            SettingsRow {
                width: parent.width
                label: qsTr("Transfer to another computer")
                description: qsTr("No migration assistant is installed. Copy your files from your home folder to move them.")
                controlData: Button {
                    objectName: "resetTransferButton"
                    text: qsTr("Open…")
                    enabled: false
                    accessibleName: qsTr("Transfer to another computer")
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Transfer from another computer")
                description: qsTr("Mount the other computer's drive and copy the files you need into your home folder.")
                showSeparator: false
                controlData: Button {
                    objectName: "resetTransferFromButton"
                    text: qsTr("Open…")
                    enabled: false
                    accessibleName: qsTr("Transfer from another computer")
                }
            }
        }

        SettingsGroup {
            id: resetGroup
            width: parent.width
            title: qsTr("Reset")

            SettingsRow {
                width: parent.width
                label: qsTr("Reset all Dragonfruit settings")
                description: qsTr("Appearance, Dock, menu bar, and other desktop preferences return to their defaults. Personal files, installed apps, and host configuration are not touched.")
                showSeparator: false
                controlData: Button {
                    id: resetButton
                    objectName: "resetAllButton"
                    text: qsTr("Reset…")
                    variant: "danger"
                    onClicked: {
                        confirmField.text = "";
                        confirmDialog.show();
                    }
                }
            }
        }

        Text {
            id: doneText
            objectName: "resetDoneText"
            width: parent.width
            visible: false
            text: qsTr("Dragonfruit settings were reset to their defaults.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Erase All Content and Settings")

            Text {
                id: eraseNote
                objectName: "resetEraseNote"
                width: parent.width
                text: qsTr("Not offered on Linux. Erasing every file and account is a reinstall of the operating system, not a settings action, so this page never pretends to do it.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }

    Dialog {
        id: confirmDialog
        objectName: "resetConfirmDialog"
        title: qsTr("Reset All Settings?")
        message: qsTr("This resets every Dragonfruit setting to its default. "
                      + "Personal files, installed apps, and host configuration "
                      + "are not touched.")
        dismissible: true

        contentData: Column {
            width: parent ? parent.width : 0
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("Type RESET to confirm.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Rectangle {
                width: parent.width
                height: Theme.controls.select.height
                radius: Theme.controls.select.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border

                TextInput {
                    id: confirmField
                    objectName: "resetConfirmField"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.controls.select.paddingH
                    anchors.rightMargin: Theme.controls.select.paddingH
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    Accessible.name: qsTr("Type RESET to confirm")
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: confirmDialog.reject()
            }

            Button {
                id: confirmButton
                objectName: "resetConfirmButton"
                text: qsTr("Reset")
                variant: "danger"
                enabled: confirmField.text === "RESET"
                onClicked: {
                    Settings.resetSettings();
                    confirmDialog.accept();
                    doneText.visible = true;
                }
            }
        }
    }
}
