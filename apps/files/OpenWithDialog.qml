// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The Files Open With picker (T334): the installed handlers for one file, the
// MIME default preselected, and an explicit "Set as Default" path. The list is
// the bridge's `Files.handlers(uri)` result; this component owns presentation
// and selection only. A file with no registered handler shows a visible
// message instead of closing silently.
Dialog {
    id: root

    property string fileName: ""
    property string mimeName: ""
    // [{ id, name, path, isDefault }], default first.
    property var handlers: []
    property string selectedId: ""
    property string notice: ""

    signal openRequested(string desktopId)
    signal defaultRequested(string desktopId)

    readonly property var selectedHandler: {
        for (var i = 0; i < root.handlers.length; ++i) {
            if (root.handlers[i].id === root.selectedId)
                return root.handlers[i];
        }
        return null;
    }
    readonly property bool hasHandlers: root.handlers.length > 0
    readonly property bool selectionIsDefault:
        root.selectedHandler !== null
        && root.selectedHandler.isDefault === true

    title: qsTr("Open With")
    message: {
        if (!root.hasHandlers) {
            var reason = qsTr("No application is registered to open “%1”.")
                    .arg(root.fileName);
            if (root.mimeName.length > 0)
                reason += qsTr(" Its type is %1.").arg(root.mimeName);
            return reason;
        }
        var text = qsTr("Choose an application to open “%1”.")
                .arg(root.fileName);
        if (root.mimeName.length > 0)
            text += qsTr(" This file is %1.").arg(root.mimeName);
        return text;
    }
    showCloseAction: false
    // Return in the dialog opens the selected handler; a dialog with no
    // handler has nothing to accept.
    onAccepted: {
        if (root.selectedHandler !== null)
            root.openRequested(root.selectedId);
    }

    contentData: Column {
        width: parent.width
        spacing: Theme.primitive.spacing.xs

        Repeater {
            model: root.handlers

            delegate: Button {
                required property var modelData
                width: parent.width
                text: modelData.isDefault
                      ? qsTr("%1  (Default)").arg(modelData.name)
                      : modelData.name
                variant: root.selectedId === modelData.id ? "primary"
                                                          : "secondary"
                accessibleName: modelData.isDefault
                                ? qsTr("%1, default application")
                                      .arg(modelData.name)
                                : modelData.name
                onClicked: root.selectedId = modelData.id
            }
        }

        Text {
            width: parent.width
            visible: !root.hasHandlers
            text: qsTr("Install an application that handles this file type, "
                       + "then try again.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        Text {
            width: parent.width
            visible: root.notice.length > 0
            text: root.notice
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
    }

    buttonsData: Row {
        spacing: Theme.controls.dialog.buttonGap

        Button {
            id: cancelButton
            text: qsTr("Cancel")
            variant: "ghost"
            onClicked: root.reject()
        }
        Button {
            id: defaultButton
            text: qsTr("Set as Default")
            variant: "secondary"
            enabled: root.selectedHandler !== null
            accessibleName: qsTr("Set the selected application as the default")
            onClicked: root.defaultRequested(root.selectedId)
        }
        Button {
            id: openButton
            text: qsTr("Open")
            variant: "primary"
            enabled: root.selectedHandler !== null
            onClicked: root.openRequested(root.selectedId)
        }
    }
}
