// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Reset sub-pane (T226).
//
// Resets every Dragonfruit settings key to its schema default. It is not a
// factory reset: personal files, installed apps, and host configuration are
// untouched. The confirmation is explicit and destructive-styled.
Item {
    id: root

    property alias resetButton: resetButton
    property alias confirmDialog: confirmDialog
    property alias confirmButton: confirmButton
    property alias doneText: doneText

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Text {
            width: parent.width
            text: qsTr("Reset Dragonfruit's appearance, Dock, menu bar, and other desktop preferences to their defaults.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Reset all Dragonfruit settings")
                description: qsTr("Personal files and installed apps are not touched.")
                showSeparator: false
                controlData: Button {
                    id: resetButton
                    objectName: "resetAllButton"
                    text: qsTr("Reset…")
                    onClicked: confirmDialog.show()
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
    }

    Dialog {
        id: confirmDialog
        objectName: "resetConfirmDialog"
        title: qsTr("Reset All Settings?")
        message: qsTr("This resets every Dragonfruit setting to its default. "
                      + "Personal files, installed apps, and host configuration "
                      + "are not touched.")
        dismissible: true

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
                variant: "primary"
                onClicked: {
                    Settings.resetSettings();
                    confirmDialog.accept();
                    doneText.visible = true;
                }
            }
        }
    }
}
