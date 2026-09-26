// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The design-system FileChooser picker (T-13.2b): the visual half of the
// portal presenter seam. `ShellController` drives every property from the
// `ChooserBridge` (which owns the request and returns the selection to the
// portal); this view is pure and never calls D-Bus. The one browsing
// implementation is files-core, in the bridge, exactly as Files and the
// portal's `ListDirectory` use it.
//
// A folder row navigates on activation; a file row selects. The accept button
// is labelled by the request's `accept_label` (falling back to Open/Save/
// Choose), so a SaveFile panel reads as the caller intended.
Item {
    id: root

    // The title the caller supplied (may be empty).
    property string title: ""
    // "open" | "save" | "save-files".
    property string kind: "open"
    // The accept button's label.
    property string acceptLabel: "Open"
    // The currently browsed folder's canonical `file://` URI.
    property string currentUri: ""
    // Human-readable folder path for the breadcrumb.
    property string currentLabel: ""
    // One row per entry: {name, uri, directory, size, detail}.
    property var entries: []
    // The selected row, or -1.
    property int selectedIndex: -1
    // The typed filename for SaveFile.
    property string saveName: ""
    // Whether the request allows (or requires) a folder.
    property bool directoryMode: false
    property bool multiple: false
    // A short message under the list (no selection, a failed listing).
    property string errorText: ""

    readonly property bool saveMode: root.kind === "save" || root.kind === "save-files"
    readonly property bool canGoUp: root.currentUri.length > 0
                                          && root.currentUri !== "file:///"
    readonly property bool canAccept: {
        if (root.saveMode)
            return root.saveName.trim().length > 0;
        if (root.directoryMode)
            return true;
        if (root.selectedIndex < 0 || root.selectedIndex >= root.entries.length)
            return false;
        return !root.entries[root.selectedIndex].directory;
    }

    signal selectionChanged(int index)
    signal entryActivated(int index)
    signal browseRequested(string uri)
    signal upRequested()
    signal nameEdited(string name)
    signal accepted()
    signal cancelled()

    function activate() {
        if (root.canAccept)
            root.accepted();
    }

    Accessible.role: Accessible.Dialog
    Accessible.name: root.title.length > 0 ? root.title : qsTr("File chooser")

    // The dialog scrim and centred card. The surface is exactly the card's
    // size in the shell overlay, so the scrim is a subtle full-bleed tint.
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
        objectName: "chooserCard"
        anchors.centerIn: parent
        width: parent.width
        height: parent.height
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

            Text {
                id: heading
                objectName: "chooserTitle"
                width: parent.width
                text: root.title.length > 0 ? root.title
                                            : (root.saveMode ? qsTr("Save") : qsTr("Open"))
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                elide: Text.ElideRight
            }

            // The location bar: an Up control plus the current folder path.
            Row {
                id: locationRow
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                Button {
                    id: upButton
                    objectName: "chooserUp"
                    text: qsTr("Up")
                    accessibleName: qsTr("Enclosing folder")
                    variant: "ghost"
                    enabled: root.canGoUp
                    onClicked: root.upRequested()
                }

                Rectangle {
                    id: pathField
                    width: parent.width - upButton.width - locationRow.spacing
                    height: Theme.controls.button.height
                    radius: Theme.controls.button.radius
                    color: Theme.color.controlFill

                    Text {
                        objectName: "chooserPath"
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.primitive.spacing.md
                        text: root.currentLabel
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideMiddle
                    }
                }
            }

            // The SaveFile filename field.
            Rectangle {
                id: nameField
                objectName: "chooserSaveField"
                visible: root.saveMode
                width: parent.width
                height: Theme.controls.button.height
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: saveInput.activeFocus ? Theme.controls.focusRing.width : 0
                border.color: Theme.color.focusRing

                TextInput {
                    id: saveInput
                    objectName: "chooserSaveInput"
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.primitive.spacing.md
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.primitive.spacing.md
                    text: root.saveName
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    clip: true
                    onTextEdited: root.nameEdited(text)
                    onAccepted: root.activate()

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: saveInput.text.length === 0
                        text: qsTr("Name")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
            }

            // The entry list.
            Rectangle {
                id: listFrame
                width: parent.width
                height: parent.height - y - footer.height - Theme.primitive.spacing.md
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.separator
                clip: true

                ListView {
                    id: entryList
                    objectName: "chooserList"
                    anchors.fill: parent
                    anchors.margins: Theme.primitive.spacing.xs
                    clip: true
                    model: root.entries
                    boundsBehavior: Flickable.StopAtBounds
                    activeFocusOnTab: true
                    keyNavigationEnabled: true
                    currentIndex: root.selectedIndex

                    delegate: Rectangle {
                        id: row
                        objectName: "chooserRow"
                        required property int index
                        required property var modelData
                        width: entryList.width
                        height: 32
                        radius: Theme.primitive.radius.sm
                        color: row.index === root.selectedIndex ? Theme.color.selection
                             : (rowHover.hovered ? Theme.color.controlHover : "transparent")
                        antialiasing: true

                        Row {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.leftMargin: Theme.primitive.spacing.sm
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.primitive.spacing.sm
                            spacing: Theme.primitive.spacing.sm

                            Icon {
                                anchors.verticalCenter: parent.verticalCenter
                                name: row.modelData.directory ? "folder" : "file"
                                size: 16
                                color: row.index === root.selectedIndex
                                       ? Theme.color.accentContent : Theme.color.textSecondary
                            }
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                width: parent.width - 16 - parent.spacing - detailText.width - parent.spacing
                                text: row.modelData.name
                                color: row.index === root.selectedIndex
                                       ? Theme.color.accentContent : Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                elide: Text.ElideRight
                            }
                            Text {
                                id: detailText
                                anchors.verticalCenter: parent.verticalCenter
                                text: row.modelData.detail || ""
                                color: row.index === root.selectedIndex
                                       ? Theme.color.accentContent : Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                        }

                        HoverHandler {
                            id: rowHover
                        }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.selectionChanged(row.index)
                            onDoubleClicked: root.entryActivated(row.index)
                        }
                    }

                    Keys.onReturnPressed: (event) => {
                        if (root.selectedIndex >= 0)
                            root.entryActivated(root.selectedIndex);
                        event.accepted = true;
                    }
                    Keys.onEnterPressed: (event) => {
                        if (root.selectedIndex >= 0)
                            root.entryActivated(root.selectedIndex);
                        event.accepted = true;
                    }
                }
            }

            // The status line (a missing selection, a listing error).
            Text {
                id: statusText
                objectName: "chooserError"
                width: parent.width
                height: root.errorText.length > 0 ? implicitHeight : 0
                visible: root.errorText.length > 0
                text: root.errorText
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                elide: Text.ElideRight
            }

            // The footer: the caller's accept label plus Cancel.
            Row {
                id: footer
                width: parent.width
                height: Theme.controls.button.height
                spacing: Theme.controls.dialog.buttonGap

                Item {
                    width: parent.width - acceptButton.width - cancelButton.width
                            - 2 * footer.spacing
                    height: 1
                }

                Button {
                    id: cancelButton
                    objectName: "chooserCancel"
                    text: qsTr("Cancel")
                    variant: "secondary"
                    onClicked: root.cancelled()
                }

                Button {
                    id: acceptButton
                    objectName: "chooserAccept"
                    text: root.acceptLabel
                    variant: "primary"
                    enabled: root.canAccept
                    onClicked: root.accepted()
                }
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.cancelled();
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.activate();
        event.accepted = true;
    }
}