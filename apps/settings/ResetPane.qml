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
// migration tool ships), Reset is the destructive action with a typed
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

        SettingsIntro {
            width: parent.width
            text: qsTr("Move your information to another computer, or reset Dragonfruit's own settings.")
        }

        SettingsGroup {
            id: transferGroup
            width: parent.width
            title: qsTr("Data Transfer")

            SettingsNote {
                width: parent.width
                text: qsTr("Use the transfer assistant to move data, settings, and applications to this computer from another computer, a backup, or a disk. You can also transfer information from this computer to another.")
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Transfer data, settings, and applications")
                description: qsTr("No migration tool is installed. Copy your files from your home folder to move them.")
                showSeparator: false
                controlData: Button {
                    objectName: "resetTransferButton"
                    text: qsTr("Open Transfer Assistant…")
                    enabled: false
                    accessibleName: qsTr("Open Transfer Assistant")
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

            // Erase All Content is the one mockup row with no Linux owner: it
            // is a reinstall, not a settings action, so the page says so.
            SettingsNote {
                id: eraseNote
                objectName: "resetEraseNote"
                width: parent.width
                text: qsTr("Erase All Content and Settings is not offered on Linux. Erasing every file and account is a reinstall of the operating system, not a settings action, so this page never pretends to do it.")
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
