// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings debug view (T222): pane and row owners from the executable
// business-logic map. It is the in-app half of the map contract: the current
// pane's owner/absence/deep link, every row's owner/key/absence/state, and a
// link to the human document. The Help menu and `DF_SETTINGS_DEBUG=1` open it.
Dialog {
    id: root

    objectName: "settingsDebugView"
    title: qsTr("Pane & row owners")
    message: qsTr("Every visible row has an owner and an absence path. "
                  + "Source: %1").arg(SettingsMap.docPath)
    dismissible: true

    // The pane whose rows are shown; empty falls back to the first shipped
    // pane. The shell keeps this in step with the selected pane while open.
    property string paneId: ""

    // T264: the view has to fit the 720x480 minimum window. The content width
    // follows the window, and the row list shortens instead of pushing the
    // dialog card past the frame; the list itself scrolls.
    readonly property real contentWidth: Math.max(
        260, Math.min(680, root.width - 2 * root.padding - 8))
    readonly property real listHeight: Math.max(
        120, Math.min(320, root.height - 340))

    readonly property var pane: SettingsMap.pane(root.paneId)
    readonly property var paneChoices: SettingsMap.panes.map(function(entry) {
        return { label: entry.title, value: entry.id };
    })

    // The selector follows the shell when it changes pane while the view is
    // open (Help -> Pane & Row Owners… stays truthful).
    onPaneIdChanged: paneSelect.currentIndex =
        Math.max(0, SettingsMap.paneIndex(root.paneId))

    property alias paneSelect: paneSelect
    property alias mapButton: mapButton
    property alias closeButton: closeButton
    property alias paneMeta: paneMeta
    property alias removedText: removedText
    property alias rowList: rowList
    property alias rowRepeater: rowList

    function stateLabel(state) {
        if (state === "shipped")
            return qsTr("visible");
        if (state === "omitted")
            return qsTr("omitted");
        return qsTr("planned");
    }

    function show(id) {
        var next = id;
        if (!next || SettingsMap.pane(next) === null) {
            for (var i = 0; i < SettingsMap.panes.length; ++i) {
                if (SettingsMap.panes[i].shipped) {
                    next = SettingsMap.panes[i].id;
                    break;
                }
            }
        }
        root.paneId = next || "";
        paneSelect.currentIndex = Math.max(0, SettingsMap.paneIndex(root.paneId));
        root.open = true;
    }

    function hide() {
        root.open = false;
    }

    contentData: Column {
        width: root.contentWidth
        spacing: Theme.primitive.spacing.md

        Row {
            width: parent.width
            spacing: Theme.primitive.spacing.md

            Select {
                id: paneSelect
                width: 320
                model: root.paneChoices
                accessibleName: qsTr("Pane")
                Accessible.description: qsTr("Choose the pane whose rows are shown")
                onSelected: (value) => root.paneId = value
            }

            Button {
                id: mapButton
                text: qsTr("Open Business-Logic Map")
                enabled: Settings.mapDocUrl.toString().length > 0
                onClicked: Qt.openUrlExternally(Settings.mapDocUrl)
            }
        }

        Column {
            id: paneMeta
            width: parent.width
            spacing: Theme.primitive.spacing.xxs

            Text {
                width: parent.width
                text: root.pane
                      ? qsTr("%1  |  owner: %2  |  absence: %3")
                            .arg(root.pane.title).arg(root.pane.owner)
                            .arg(root.pane.absence)
                      : qsTr("No pane selected.")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                wrapMode: Text.WordWrap
                Accessible.role: Accessible.StaticText
                Accessible.name: text
            }
            Text {
                width: parent.width
                text: root.pane
                      ? qsTr("shipped: %1  |  deep link: %2")
                            .arg(root.pane.shipped ? qsTr("yes") : qsTr("no"))
                            .arg(root.pane.deepLink)
                      : ""
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                Accessible.role: Accessible.StaticText
                Accessible.name: text
            }
            Text {
                id: removedText
                width: parent.width
                visible: SettingsMap.removed.length > 0
                text: {
                    var lines = [];
                    for (var i = 0; i < SettingsMap.removed.length; ++i) {
                        var entry = SettingsMap.removed[i];
                        lines.push(entry.label + ": " + entry.reason);
                    }
                    return lines.join("\n");
                }
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                Accessible.role: Accessible.StaticText
                Accessible.name: text
            }
        }

        Rectangle {
            id: rowListFrame
            width: parent.width
            height: root.listHeight
            radius: Theme.controls.settingsGroup.radius
            color: Theme.color.surfaceMuted
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border

            FocusRing {
                target: rowListFrame
                cornerRadius: Theme.controls.settingsGroup.radius
                shown: rowList.activeFocus
            }

            ListView {
                id: rowList
                anchors.fill: parent
                anchors.margins: Theme.primitive.spacing.sm
                clip: true
                model: SettingsMap.rowsFor(root.paneId)
                spacing: Theme.primitive.spacing.xs
                boundsBehavior: Flickable.StopAtBounds
                activeFocusOnTab: true

                delegate: Item {
                    required property var modelData
                    width: rowList.width
                    height: rowColumn.implicitHeight + Theme.primitive.spacing.sm

                    Column {
                        id: rowColumn
                        width: parent.width
                        spacing: 1

                        Text {
                            width: parent.width
                            text: modelData.label + "  [" + root.stateLabel(modelData.state) + "]"
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: qsTr("owner: %1  |  key/API: %2")
                                  .arg(modelData.owner).arg(modelData.key)
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: qsTr("absence: %1  |  deep link: %2")
                                  .arg(modelData.absence)
                                  .arg(SettingsMap.rowDeepLink(root.paneId, modelData.id))
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Accessible.role: Accessible.ListItem
                    Accessible.name: modelData.label
                    Accessible.description: qsTr("Owner %1, key %2, absence %3, %4")
                        .arg(modelData.owner).arg(modelData.key)
                        .arg(modelData.absence)
                        .arg(root.stateLabel(modelData.state))
                    Accessible.focusable: true
                }

                Accessible.role: Accessible.List
                Accessible.name: qsTr("Pane rows")
                Accessible.description: qsTr("Every row in the selected pane and its owner")
            }
        }
    }

    buttonsData: Button {
        id: closeButton
        text: qsTr("Close")
        onClicked: root.reject()
    }

    onClosed: root.paneId = ""
}
