// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Search pane (T227).
//
// Search is a Spotlight-equivalent service (T246); this pane owns the durable
// preferences that service reads. The categories/locations/shortcut/web keys
// are settingsd-owned, so they survive a search-service restart. The indexing
// status/rebuild rows are read from the service and are disabled with an
// explicit reason while it is absent — never a fake progress bar.
Item {
    id: root

    readonly property var categoryCatalog: [
        { id: "applications", label: qsTr("Applications") },
        { id: "files", label: qsTr("Files") },
        { id: "folders", label: qsTr("Folders") },
        { id: "actions", label: qsTr("Actions") },
        { id: "calculations", label: qsTr("Calculations") },
        { id: "definitions", label: qsTr("Definitions") },
        { id: "web", label: qsTr("Web") }
    ]

    readonly property var categories: {
        var value = Settings.values["search.categories"];
        return value === undefined || value === null ? [] : Array.from(value);
    }
    readonly property var locations: {
        var value = Settings.values["search.indexLocations"];
        return value === undefined || value === null ? [] : Array.from(value);
    }
    readonly property string shortcut: Settings.values["search.shortcut"] || ""

    property string locationDraft: ""
    property string shortcutDraft: root.shortcut

    function categoryEnabled(id) {
        return root.categories.indexOf(id) >= 0;
    }

    function setCategory(id, enabled) {
        var list = root.categories.slice();
        var at = list.indexOf(id);
        if (enabled && at < 0)
            list.push(id);
        else if (!enabled && at >= 0)
            list.splice(at, 1);
        else
            return;
        Settings.setList("search.categories", list);
    }

    function locationLabel(token) {
        return token === "home" ? qsTr("Home") : token;
    }

    function addLocation() {
        var token = root.locationDraft.trim();
        if (token.length === 0)
            return;
        var list = root.locations.slice();
        if (list.indexOf(token) < 0)
            list.push(token);
        Settings.setList("search.indexLocations", list);
        root.locationDraft = "";
    }

    function removeLocation(token) {
        var list = root.locations.slice();
        var at = list.indexOf(token);
        if (at < 0)
            return;
        list.splice(at, 1);
        Settings.setList("search.indexLocations", list);
    }

    function applyShortcut() {
        var text = root.shortcutDraft.trim();
        if (text.length === 0)
            return;
        Settings.set("search.shortcut", text);
    }

    // Test surface (used by tst_settings_search_pane.qml).
    property alias categoryRepeater: categoryRepeater
    property alias locationRepeater: locationRepeater
    property alias locationInput: locationInput
    property alias addLocationButton: addLocationButton
    property alias shortcutInput: shortcutInput
    property alias shortcutButton: shortcutButton
    property alias webToggle: webToggle
    property alias menuBarToggle: menuBarToggle
    property alias statusRow: statusRow
    property alias rebuildButton: rebuildButton

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            width: parent.width
            title: qsTr("Search Results")

            Text {
                width: parent.width
                text: qsTr("Choose the categories the desktop search includes.")
                color: Theme.color.textSecondary
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: categoryRepeater
                model: root.categoryCatalog

                delegate: SettingsRow {
                    required property var modelData
                    width: content.width
                    label: modelData.label
                    settingsId: "category-" + modelData.id
                    owner: "settingsd -> search service"
                    settingsKey: "search.categories"

                    controlData: Toggle {
                        text: ""
                        accessibleName: qsTr("Include %1 in Search")
                                        .arg(modelData.label)
                        checked: root.categoryEnabled(modelData.id)
                        onToggled: (checked) => root.setCategory(modelData.id, checked)
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Search Privacy")

            Text {
                width: parent.width
                text: qsTr("Folders the file index includes. The home folder is "
                          + "indexed by default; add any other absolute path.")
                color: Theme.color.textSecondary
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: locationRepeater
                model: root.locations

                delegate: SettingsRow {
                    required property string modelData
                    width: content.width
                    label: root.locationLabel(modelData)
                    settingsId: "location-" + modelData
                    owner: "settingsd -> search service"
                    settingsKey: "search.indexLocations"

                    controlData: Button {
                        text: qsTr("Remove")
                        accessibleName: qsTr("Stop indexing %1")
                                        .arg(root.locationLabel(modelData))
                        onClicked: root.removeLocation(modelData)
                    }
                }
            }

            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                Rectangle {
                    width: parent.width - addLocationButton.width - parent.spacing
                    height: Theme.controls.button.height
                    radius: Theme.controls.button.radius
                    color: Theme.color.controlFill
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border

                    TextInput {
                        id: locationInput
                        anchors.fill: parent
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        selectByMouse: true
                        text: root.locationDraft
                        onTextEdited: root.locationDraft = text
                        onAccepted: root.addLocation()
                        Accessible.role: Accessible.EditableText
                        Accessible.name: qsTr("Folder to index")
                    }
                }

                Button {
                    id: addLocationButton
                    text: qsTr("Add")
                    accessibleName: qsTr("Add the folder to the index")
                    enabled: root.locationDraft.trim().length > 0
                    onClicked: root.addLocation()
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Shortcut")

            SettingsRow {
                width: parent.width
                label: qsTr("Open Search")
                description: qsTr("The global shortcut the search service registers "
                                  + "when it is present.")
                settingsId: "shortcut"
                owner: "settingsd -> search service"
                settingsKey: "search.shortcut"
                showSeparator: false

                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Rectangle {
                        width: 140
                        height: Theme.controls.button.height
                        radius: Theme.controls.button.radius
                        color: Theme.color.controlFill
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        TextInput {
                            id: shortcutInput
                            anchors.fill: parent
                            anchors.leftMargin: Theme.primitive.spacing.sm
                            anchors.rightMargin: Theme.primitive.spacing.sm
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            selectByMouse: true
                            text: root.shortcutDraft
                            onTextEdited: root.shortcutDraft = text
                            onAccepted: root.applyShortcut()
                            Accessible.role: Accessible.EditableText
                            Accessible.name: qsTr("Search keyboard shortcut")
                        }
                    }

                    Button {
                        id: shortcutButton
                        text: qsTr("Apply")
                        enabled: root.shortcutDraft.trim().length > 0
                                 && root.shortcutDraft.trim() !== root.shortcut
                        onClicked: root.applyShortcut()
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Menu Bar")

            SettingsRow {
                width: parent.width
                label: qsTr("Show Search in menu bar")
                settingsId: "menuBar"
                owner: "settingsd -> shell/MenuBar"
                settingsKey: "menu.control.search"
                showSeparator: false

                controlData: Toggle {
                    id: menuBarToggle
                    text: ""
                    onToggled: (checked) => Settings.set("menu.control.search", checked)
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Web Search")

            SettingsRow {
                width: parent.width
                label: qsTr("Web search")
                description: qsTr("Offer a web result for the query in the default "
                                  + "browser.")
                settingsId: "web"
                owner: "settingsd -> search service"
                settingsKey: "search.web"
                showSeparator: false

                controlData: Toggle {
                    id: webToggle
                    text: ""
                    onToggled: (checked) => Settings.set("search.web", checked)
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Indexing")

            SettingsRow {
                id: statusRow
                width: parent.width
                label: qsTr("Indexing status")
                description: qsTr("The desktop search service is not running; "
                                  + "indexing lands with T246.")
                settingsId: "status"
                owner: "search service"
                settingsKey: "search.status"
                enabled: false
                showSeparator: false

                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Rebuild index")
                description: qsTr("Available when the search service is present.")
                settingsId: "rebuild"
                owner: "search service"
                settingsKey: "search.rebuild"
                enabled: false
                showSeparator: false

                controlData: Button {
                    id: rebuildButton
                    text: qsTr("Rebuild")
                    enabled: false
                }
            }
        }
    }

    Binding {
        target: menuBarToggle
        property: "checked"
        value: Settings.values["menu.control.search"] !== false
    }
    Binding {
        target: webToggle
        property: "checked"
        value: Settings.values["search.web"] !== false
    }
}
