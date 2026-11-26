// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// The design-system FileChooser picker (T-13.2b): the visual half of the
// portal presenter seam. `ShellController` drives every property from the
// `ChooserBridge` (which owns the request and returns the selection to the
// portal); this view is pure and never calls D-Bus. The one browsing
// implementation is files-core, in the bridge, exactly as Files and the
// portal's `ListDirectory` use it.
//
// The dialog serves all four portal kinds:
//
//   * Open File    — pick one or more files (`multiple`).
//   * Open Folder  — `directoryMode`; the current folder or a selected folder.
//   * Save File    — pick a folder and type a name (`saveMode`).
//   * Save Folder  — `saveFilesMode`; pick the destination folder. The portal
//                    appends the caller's file names to the result.
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
    // The requesting application's id (informational; may be empty).
    property string appId: ""
    // The accept button's label.
    property string acceptLabel: "Open"
    // The currently browsed folder's canonical `file://` URI.
    property string currentUri: ""
    // Human-readable folder path for the breadcrumb.
    property string currentLabel: ""
    // The breadcrumb trail: [{label, uri}].
    property var breadcrumbs: []
    // The sidebar's favorites: [{label, icon, uri}].
    property var favorites: []
    // One row per entry: {name, uri, directory, size, detail}.
    property var entries: []
    // Bumped by the bridge when the listing changes. Kept as a property so a
    // test or capture can wait on it; the list itself binds to `entries`.
    property int entriesRevision: 0
    // True while files-core is still streaming the folder.
    property bool loading: false
    // The keyboard/click anchor row, or -1.
    property int selectedIndex: -1
    // Every selected row (multi-select returns all of them).
    property var selectedIndices: []
    // The typed filename for SaveFile.
    property string saveName: ""
    // The caller's names for SaveFiles.
    property var saveFiles: []
    // Whether the request allows (or requires) a folder.
    property bool directoryMode: false
    property bool multiple: false
    // The overwrite confirmation for SaveFile.
    property bool confirmReplace: false
    property string replaceName: ""
    // A short message under the list (no selection, a failed listing).
    property string errorText: ""
    // True while this is the live dialog; the shell sets it on map/unmap.
    property bool active: false

    readonly property bool saveMode: root.kind === "save"
    readonly property bool saveFilesMode: root.kind === "save-files"
    readonly property bool folderMode: root.directoryMode || root.saveFilesMode
    readonly property bool canGoUp: root.currentUri.length > 0
                                          && root.currentUri !== "file:///"
    readonly property string defaultTitle: root.saveMode ? qsTr("Save")
                                          : root.folderMode ? qsTr("Choose Folder")
                                          : qsTr("Open")
    readonly property bool hasFileSelection: {
        for (var i = 0; i < root.entries.length; ++i) {
            if (root.selectedIndices.indexOf(i) >= 0 && !root.entries[i].directory)
                return true;
        }
        return false;
    }
    readonly property bool hasFolderSelection: {
        for (var i = 0; i < root.entries.length; ++i) {
            if (root.selectedIndices.indexOf(i) >= 0 && root.entries[i].directory)
                return true;
        }
        return false;
    }
    readonly property bool canAccept: {
        if (root.confirmReplace)
            return false;
        if (root.saveMode)
            return root.saveName.trim().length > 0;
        if (root.saveFilesMode)
            return true;
        if (root.directoryMode)
            return root.hasFolderSelection || root.selectedIndices.length === 0;
        // Open File: a selected file accepts, a selected folder navigates.
        return root.selectedIndices.length > 0;
    }

    readonly property string currentFolderName: root.breadcrumbs.length > 0
                                                 ? root.breadcrumbs[root.breadcrumbs.length - 1].label
                                                 : ""
    readonly property string selectedFolderName: {
        for (var i = 0; i < root.entries.length; ++i) {
            if (root.selectedIndices.indexOf(i) >= 0 && root.entries[i].directory)
                return root.entries[i].name;
        }
        return "";
    }
    readonly property string targetFolderName: root.selectedFolderName.length > 0
                                               ? root.selectedFolderName : root.currentFolderName

    signal selectionChanged(int index, int modifiers)
    signal entryActivated(int index)
    signal browseRequested(string uri)
    signal upRequested()
    signal nameEdited(string name)
    signal newFolderRequested()
    signal selectAllRequested()
    signal accepted()
    signal replaceAccepted()
    signal replaceDismissed()
    signal cancelled()

    focus: true
    activeFocusOnTab: true

    function activate() {
        if (root.confirmReplace) {
            root.replaceAccepted();
            return;
        }
        if (root.canAccept)
            root.accepted();
    }

    // The shell calls this when the request opens so the right control owns
    // the keyboard: the name field for SaveFile, the list otherwise.
    function focusInitial() {
        if (!root.active)
            return;
        if (root.saveMode) {
            saveInput.forceActiveFocus();
            var dot = saveInput.text.lastIndexOf(".");
            if (dot > 0)
                saveInput.select(0, dot);
            else
                saveInput.selectAll();
        } else {
            entryList.forceActiveFocus();
        }
    }

    function moveCurrent(delta, modifiers) {
        if (root.entries.length === 0)
            return;
        var base = root.selectedIndex;
        if (base < 0)
            base = delta > 0 ? -1 : root.entries.length;
        var target = Math.max(0, Math.min(root.entries.length - 1, base + delta));
        root.selectionChanged(target, modifiers);
        entryList.positionViewAtIndex(target, ListView.Contain);
    }

    function moveCurrentTo(index, modifiers) {
        if (root.entries.length === 0)
            return;
        var target = Math.max(0, Math.min(root.entries.length - 1, index));
        root.selectionChanged(target, modifiers);
        entryList.positionViewAtIndex(target, ListView.Contain);
    }

    Accessible.role: Accessible.Dialog
    Accessible.name: root.title.length > 0 ? root.title : root.defaultTitle

    onActiveChanged: {
        if (root.active)
            root.focusInitial();
    }

    onSaveNameChanged: {
        // Keep the field in sync when the bridge changes the name (a file row
        // was activated, or a new request opened) after the user has typed,
        // which breaks the field's initial binding.
        if (saveInput.text !== root.saveName)
            saveInput.text = root.saveName;
    }

    onSelectedIndexChanged: {
        if (root.selectedIndex >= 0)
            entryList.positionViewAtIndex(root.selectedIndex, ListView.Contain);
    }

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
        cutoutBounds: true
    }

    Rectangle {
        id: card
        objectName: "chooserCard"
        anchors.fill: parent
        radius: Theme.controls.window.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
        clip: true

        // --- header -------------------------------------------------------
        Item {
            id: header
            objectName: "chooserHeader"
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 48

            Text {
                id: heading
                objectName: "chooserTitle"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: Theme.primitive.spacing.lg
                anchors.rightMargin: Theme.primitive.spacing.lg
                anchors.verticalCenter: parent.verticalCenter
                text: root.title.length > 0 ? root.title : root.defaultTitle
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                elide: Text.ElideRight
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }
        }

        // --- footer -------------------------------------------------------
        Rectangle {
            id: footer
            objectName: "chooserFooter"
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: footerColumn.implicitHeight + 2 * Theme.primitive.spacing.md
            color: Theme.color.surfaceElevated

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Column {
                id: footerColumn
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Theme.primitive.spacing.lg
                anchors.rightMargin: Theme.primitive.spacing.lg
                spacing: Theme.primitive.spacing.sm

                // The overwrite confirmation replaces the normal footer
                // affordances until the user decides.
                Rectangle {
                    id: replaceBanner
                    objectName: "chooserReplaceBanner"
                    visible: root.confirmReplace
                    width: parent.width
                    height: visible ? replaceRow.implicitHeight + 2 * Theme.primitive.spacing.sm : 0
                    radius: Theme.primitive.radius.sm
                    color: Theme.color.controlFill
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border

                    Row {
                        id: replaceRow
                        anchors.fill: parent
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.rightMargin: Theme.primitive.spacing.md
                        spacing: Theme.primitive.spacing.sm

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - replaceButton.width - keepButton.width
                                   - 2 * parent.spacing
                            text: qsTr("\"%1\" already exists. Do you want to replace it?")
                                      .arg(root.replaceName)
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }

                        Button {
                            id: keepButton
                            objectName: "chooserKeepEditing"
                            anchors.verticalCenter: parent.verticalCenter
                            text: qsTr("Keep Editing")
                            variant: "secondary"
                            onClicked: root.replaceDismissed()
                        }

                        Button {
                            id: replaceButton
                            objectName: "chooserReplace"
                            anchors.verticalCenter: parent.verticalCenter
                            text: qsTr("Replace")
                            variant: "danger"
                            onClicked: root.replaceAccepted()
                        }
                    }
                }

                // SaveFile: the name field. SaveFiles has no single name; its
                // footer explains where the caller's files will land.
                Row {
                    id: saveRow
                    objectName: "chooserSaveRow"
                    visible: root.saveMode
                    width: parent.width
                    height: visible ? Theme.controls.button.height : 0
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        id: saveLabel
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Save As:")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Rectangle {
                        id: nameField
                        objectName: "chooserSaveField"
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - saveLabel.width - parent.spacing
                        height: Theme.controls.button.height
                        radius: Theme.controls.button.radius
                        color: Theme.color.controlFill
                        border.width: saveInput.activeFocus ? Theme.controls.focusRing.width : 1
                        border.color: saveInput.activeFocus ? Theme.color.focusRing
                                                            : Theme.color.border

                        TextInput {
                            id: saveInput
                            objectName: "chooserSaveInput"
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.leftMargin: Theme.primitive.spacing.md
                            anchors.rightMargin: Theme.primitive.spacing.md
                            text: root.saveName
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            selectByMouse: true
                            clip: true
                            activeFocusOnTab: true
                            onTextEdited: root.nameEdited(text)

                            Keys.onTabPressed: (event) => {
                                const next = saveInput.nextItemInFocusChain(
                                                 !(event.modifiers & Qt.ShiftModifier));
                                if (next)
                                    next.forceActiveFocus();
                                event.accepted = true;
                            }

                            Keys.onReturnPressed: (event) => {
                                root.activate();
                                event.accepted = true;
                            }
                            Keys.onEnterPressed: (event) => {
                                root.activate();
                                event.accepted = true;
                            }

                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: saveInput.text.length === 0
                                text: qsTr("Name")
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.controls.button.fontSize
                            }
                        }
                    }
                }

                // SaveFile: a selected folder is the destination.
                Text {
                    id: saveIntoNote
                    objectName: "chooserSaveIntoNote"
                    visible: root.saveMode && root.selectedFolderName.length > 0
                    width: parent.width
                    height: visible ? implicitHeight : 0
                    text: qsTr("Saving into \"%1\".").arg(root.selectedFolderName)
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    elide: Text.ElideRight
                }

                // SaveFiles: name the destination, not a filename.
                Text {
                    id: saveFilesNote
                    objectName: "chooserSaveFilesNote"
                    visible: root.saveFilesMode
                    width: parent.width
                    height: visible ? implicitHeight : 0
                    text: root.saveFiles.length === 0
                          ? qsTr("The files will be saved in \"%1\".").arg(root.targetFolderName)
                          : root.saveFiles.length === 1
                            ? qsTr("1 file will be saved in \"%1\".").arg(root.targetFolderName)
                            : qsTr("%1 files will be saved in \"%2\".")
                                  .arg(root.saveFiles.length).arg(root.targetFolderName)
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    elide: Text.ElideRight
                }

                // Open Folder: say which folder the Choose button will return.
                Text {
                    id: chooseFolderNote
                    objectName: "chooserChooseFolderNote"
                    visible: root.directoryMode && !root.saveFilesMode
                    width: parent.width
                    height: visible ? implicitHeight : 0
                    text: root.selectedFolderName.length > 0
                          ? qsTr("Choosing \"%1\".").arg(root.selectedFolderName)
                          : qsTr("Choose \"%1\" or select a folder below.")
                                .arg(root.currentFolderName)
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    elide: Text.ElideRight
                }

                // Error + buttons.
                Item {
                    id: actionRow
                    width: parent.width
                    height: Theme.controls.button.height

                    Text {
                        id: statusText
                        objectName: "chooserError"
                        anchors.left: parent.left
                        anchors.right: buttonRow.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.errorText
                        visible: root.errorText.length > 0
                        color: Theme.color.danger
                        font.pixelSize: Theme.primitive.font.sizeSm
                        elide: Text.ElideRight
                    }

                    Row {
                        id: buttonRow
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.controls.dialog.buttonGap

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
        }

        // --- body ---------------------------------------------------------
        Item {
            id: body
            anchors.top: header.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: footer.top

            // The favorites sidebar.
            Rectangle {
                id: sidebar
                objectName: "chooserSidebar"
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: 164
                color: Theme.color.surface

                Rectangle {
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: Theme.controls.window.borderWidth
                    color: Theme.color.separator
                }

                Column {
                    id: favoriteColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.primitive.spacing.sm
                    spacing: Theme.primitive.spacing.xxs

                    Repeater {
                        model: root.favorites
                        delegate: Rectangle {
                            id: favoriteRow
                            required property var modelData
                            objectName: "chooserFavorite"
                            width: favoriteColumn.width
                            height: Theme.controls.sourceList.rowHeight
                            radius: Theme.controls.sourceList.rowRadius
                            color: root.currentUri === favoriteRow.modelData.uri
                                   ? Theme.color.selection
                                   : (favoriteHover.hovered ? Theme.color.controlHover
                                                            : "transparent")

                            Row {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.primitive.spacing.sm
                                anchors.rightMargin: Theme.primitive.spacing.sm
                                spacing: Theme.primitive.spacing.sm

                                TablerIcon {
                                    anchors.verticalCenter: parent.verticalCenter
                                    name: favoriteRow.modelData.icon
                                    size: Theme.controls.sidebar.iconSize
                                    color: root.currentUri === favoriteRow.modelData.uri
                                           ? Theme.color.textPrimary
                                           : Theme.color.textSecondary
                                }

                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: parent.width - Theme.controls.sidebar.iconSize
                                           - parent.spacing
                                    text: favoriteRow.modelData.label
                                    color: Theme.color.textPrimary
                                    font.pixelSize: Theme.controls.button.fontSize
                                    elide: Text.ElideRight
                                }
                            }

                            HoverHandler { id: favoriteHover }

                            MouseArea {
                                anchors.fill: parent
                                onClicked: root.browseRequested(favoriteRow.modelData.uri)
                            }
                        }
                    }
                }
            }

            // The content column: path bar, column header, entries.
            Item {
                id: content
                anchors.left: sidebar.right
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.bottom: parent.bottom

                // The path bar: Up plus clickable breadcrumbs plus New Folder.
                Item {
                    id: pathBar
                    objectName: "chooserPathBar"
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: Theme.primitive.spacing.md
                    anchors.rightMargin: Theme.primitive.spacing.md
                    height: 40

                    Rectangle {
                        id: upButton
                        objectName: "chooserUp"
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        width: 28
                        height: 28
                        radius: Theme.primitive.radius.sm
                        color: upHover.hovered && root.canGoUp
                               ? Theme.color.controlHover : "transparent"
                        opacity: root.canGoUp ? 1.0 : 0.4

                        TablerIcon {
                            anchors.centerIn: parent
                            name: "arrow-up"
                            size: 16
                            color: Theme.color.textSecondary
                        }

                        HoverHandler { id: upHover }

                        MouseArea {
                            anchors.fill: parent
                            enabled: root.canGoUp
                            onClicked: root.upRequested()
                        }

                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Enclosing folder")
                    }

                    Rectangle {
                        id: newFolderButton
                        objectName: "chooserNewFolder"
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: newFolderRow.implicitWidth + 2 * Theme.primitive.spacing.sm
                        height: 28
                        radius: Theme.primitive.radius.sm
                        color: newFolderHover.hovered ? Theme.color.controlHover : "transparent"

                        Row {
                            id: newFolderRow
                            anchors.centerIn: parent
                            spacing: Theme.primitive.spacing.xs

                            TablerIcon {
                                anchors.verticalCenter: parent.verticalCenter
                                name: "folder"
                                size: 16
                                color: Theme.color.textSecondary
                            }

                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: qsTr("New Folder")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                            }
                        }

                        HoverHandler { id: newFolderHover }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.newFolderRequested()
                        }

                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("New Folder")
                    }

                    Flickable {
                        id: crumbBar
                        objectName: "chooserBreadcrumbs"
                        anchors.left: upButton.right
                        anchors.right: newFolderButton.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: Theme.primitive.spacing.xs
                        anchors.rightMargin: Theme.primitive.spacing.xs
                        clip: true
                        interactive: false
                        contentWidth: crumbRow.implicitWidth
                        contentHeight: height
                        boundsBehavior: Flickable.StopAtBounds

                        onContentWidthChanged: scrollCrumbsToEnd()
                        onWidthChanged: scrollCrumbsToEnd()

                        function scrollCrumbsToEnd() {
                            contentX = Math.max(0, contentWidth - width);
                        }

                        Row {
                            id: crumbRow
                            height: crumbBar.height
                            spacing: 0

                            Repeater {
                                model: root.breadcrumbs
                                delegate: Row {
                                    id: crumb
                                    required property int index
                                    required property var modelData
                                    height: crumbBar.height
                                    spacing: 0

                                    Rectangle {
                                        id: crumbButton
                                        objectName: "chooserCrumb"
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: crumbLabel.implicitWidth
                                               + 2 * Theme.primitive.spacing.sm
                                        height: 24
                                        radius: Theme.primitive.radius.sm
                                        color: crumbHover.hovered
                                               ? Theme.color.controlHover : "transparent"

                                        Text {
                                            id: crumbLabel
                                            anchors.centerIn: parent
                                            text: crumb.modelData.label
                                            color: Theme.color.textSecondary
                                            font.pixelSize: Theme.controls.button.fontSize
                                        }

                                        HoverHandler { id: crumbHover }

                                        MouseArea {
                                            anchors.fill: parent
                                            onClicked: root.browseRequested(
                                                           crumb.modelData.uri)
                                        }
                                    }

                                    TablerIcon {
                                        anchors.verticalCenter: parent.verticalCenter
                                        visible: crumb.index < root.breadcrumbs.length - 1
                                        name: "chevron-right"
                                        size: 12
                                        color: Theme.color.textTertiary
                                    }
                                }
                            }
                        }
                    }
                }

                // The column header.
                Item {
                    id: listHeader
                    objectName: "chooserListHeader"
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: pathBar.bottom
                    anchors.leftMargin: Theme.primitive.spacing.md
                    anchors.rightMargin: Theme.primitive.spacing.md
                    height: 24

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.primitive.spacing.md
                                         + Theme.primitive.spacing.xs
                                         + Theme.primitive.spacing.sm + 16
                                         + Theme.primitive.spacing.sm
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Name")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Text {
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.primitive.spacing.md
                                             + Theme.primitive.spacing.xs
                                             + Theme.primitive.spacing.sm
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Size")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                    }
                }

                // The entry list.
                Rectangle {
                    id: listFrame
                    objectName: "chooserListFrame"
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: listHeader.bottom
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: Theme.primitive.spacing.md
                    anchors.rightMargin: Theme.primitive.spacing.md
                    anchors.bottomMargin: Theme.primitive.spacing.md
                    color: Theme.color.surfaceSunken
                    border.width: Theme.controls.window.borderWidth
                    border.color: entryList.activeFocus ? Theme.color.focusRing
                                                        : Theme.color.separator
                    radius: Theme.primitive.radius.md
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
                        keyNavigationEnabled: false
                        currentIndex: root.selectedIndex

                        delegate: Rectangle {
                            id: row
                            objectName: "chooserRow"
                            required property int index
                            required property var modelData
                            width: entryList.width
                            height: 28
                            radius: Theme.primitive.radius.sm
                            color: root.selectedIndices.indexOf(row.index) >= 0
                                   ? Theme.color.selection
                                   : (rowHover.hovered ? Theme.color.controlHover : "transparent")
                            antialiasing: true

                            readonly property bool isSelected:
                                root.selectedIndices.indexOf(row.index) >= 0

                            Row {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.primitive.spacing.sm
                                anchors.rightMargin: Theme.primitive.spacing.sm
                                spacing: Theme.primitive.spacing.sm

                                TablerIcon {
                                    anchors.verticalCenter: parent.verticalCenter
                                    name: row.modelData.directory ? "folder" : "file"
                                    size: 16
                                    color: row.isSelected ? Theme.color.textPrimary
                                          : row.modelData.directory ? Theme.color.accent
                                                                    : Theme.color.textSecondary
                                }

                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: parent.width - 16 - parent.spacing
                                           - detailText.width - parent.spacing
                                    text: row.modelData.name
                                    color: Theme.color.textPrimary
                                    font.pixelSize: Theme.controls.button.fontSize
                                    elide: Text.ElideRight
                                }

                                Text {
                                    id: detailText
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: row.modelData.detail || ""
                                    color: row.isSelected ? Theme.color.textPrimary
                                                          : Theme.color.textTertiary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                }
                            }

                            HoverHandler { id: rowHover }

                            MouseArea {
                                anchors.fill: parent
                                onClicked: (mouse) => root.selectionChanged(row.index,
                                                                            mouse.modifiers)
                                onDoubleClicked: (mouse) => root.entryActivated(row.index)
                            }

                            Accessible.role: Accessible.ListItem
                            Accessible.name: row.modelData.name
                            Accessible.selected: row.isSelected
                        }

                        Keys.onPressed: (event) => {
                            if ((event.modifiers & (Qt.ControlModifier | Qt.MetaModifier))
                                    && event.key === Qt.Key_A) {
                                root.selectAllRequested();
                                event.accepted = true;
                                return;
                            }
                            switch (event.key) {
                            case Qt.Key_Down:
                                root.moveCurrent(1, event.modifiers);
                                event.accepted = true;
                                break;
                            case Qt.Key_Up:
                                root.moveCurrent(-1, event.modifiers);
                                event.accepted = true;
                                break;
                            case Qt.Key_PageDown:
                                root.moveCurrent(8, event.modifiers);
                                event.accepted = true;
                                break;
                            case Qt.Key_PageUp:
                                root.moveCurrent(-8, event.modifiers);
                                event.accepted = true;
                                break;
                            case Qt.Key_Home:
                                root.moveCurrentTo(0, event.modifiers);
                                event.accepted = true;
                                break;
                            case Qt.Key_End:
                                root.moveCurrentTo(root.entries.length - 1, event.modifiers);
                                event.accepted = true;
                                break;
                            case Qt.Key_Return:
                            case Qt.Key_Enter:
                                if (root.selectedIndex >= 0)
                                    root.entryActivated(root.selectedIndex);
                                event.accepted = true;
                                break;
                            default:
                                break;
                            }
                        }
                    }

                    // Empty and loading states.
                    Column {
                        anchors.centerIn: parent
                        spacing: Theme.primitive.spacing.sm
                        visible: root.entries.length === 0

                        TablerIcon {
                            anchors.horizontalCenter: parent.horizontalCenter
                            name: "folder"
                            size: 28
                            color: Theme.color.textTertiary
                            opacity: 0.7
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: root.loading ? qsTr("Loading…") : qsTr("Folder is Empty")
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                        }
                    }
                }
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        if (root.confirmReplace)
            root.replaceDismissed();
        else
            root.cancelled();
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.activate();
        event.accepted = true;
    }
}
