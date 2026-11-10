// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The widget gallery (T234): a separate 1027×552 pt window with a 240 pt
// category sidebar and the 19 measured categories.
//
// The gallery is deliberately not a sheet inside Notification Center
// (structural review §5.4 rule 4). It renders the catalogue the shell owns,
// previews widgets at the gallery thumbnail size (102×102 pt, not the live
// 164 pt panel size), and raises add/remove/size requests. The shell owns the
// widget model and the live data; this component never invents a widget.
Item {
    id: root

    // The measured gallery window (structural review §5.1).
    readonly property int measuredWidth: 1027
    readonly property int measuredHeight: 552
    readonly property int sidebarWidth: 240

    width: measuredWidth
    height: measuredHeight

    // `{ kind, title, category, description, sizes }` entries.
    property var catalog: []
    // The 19 category names; `All` is prepended by the view.
    property var categories: []
    // The widgets already in the panel, `{ id, kind, size, title }`.
    property var widgets: []
    property bool available: true

    property string searchText: ""
    property string selectedCategory: "All"

    signal addRequested(string kind, string size)
    signal removeRequested(string id)
    signal resizeRequested(string id, string size)
    signal closeRequested()

    readonly property var allCategories: {
        var list = [qsTr("All")];
        for (var i = 0; i < categories.length; ++i) {
            if (list.indexOf(String(categories[i])) < 0)
                list.push(String(categories[i]));
        }
        return list;
    }

    function widgetFor(kind) {
        for (var i = 0; i < widgets.length; ++i) {
            if (widgets[i] && String(widgets[i].kind) === String(kind))
                return widgets[i];
        }
        return null;
    }

    function matches(entry) {
        var category = root.selectedCategory;
        if (category !== qsTr("All") && String(entry.category) !== category)
            return false;
        var query = root.searchText.trim().toLowerCase();
        if (query.length === 0)
            return true;
        var haystack = (String(entry.title) + " " + String(entry.description)
                        + " " + String(entry.category)).toLowerCase();
        return haystack.indexOf(query) >= 0;
    }

    readonly property var filteredCatalog: {
        var out = [];
        for (var i = 0; i < catalog.length; ++i) {
            if (root.matches(catalog[i]))
                out.push(catalog[i]);
        }
        return out;
    }

    Accessible.role: Accessible.Window
    Accessible.name: qsTr("Widget Gallery")

    Keys.onEscapePressed: (event) => {
        root.closeRequested();
        event.accepted = true;
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.window.radius
        color: Theme.color.surface
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        // ── Header ───────────────────────────────────────────────────────
        Item {
            id: header
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 52

            Text {
                objectName: "galleryTitle"
                anchors.left: parent.left
                anchors.leftMargin: Theme.primitive.spacing.lg
                anchors.verticalCenter: parent.verticalCenter
                text: qsTr("Widget Gallery")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeXl
                font.weight: Theme.primitive.font.weightSemibold
            }

            Rectangle {
                id: doneButton
                objectName: "galleryDone"
                anchors.right: parent.right
                anchors.rightMargin: Theme.primitive.spacing.lg
                anchors.verticalCenter: parent.verticalCenter
                width: doneLabel.implicitWidth + 2 * Theme.primitive.spacing.lg
                height: 28
                radius: Theme.controls.button.radius
                color: doneArea.containsMouse ? Theme.color.accentMuted
                                              : Theme.color.controlFill

                Accessible.role: Accessible.Button
                Accessible.name: qsTr("Done")

                Text {
                    id: doneLabel
                    anchors.centerIn: parent
                    text: qsTr("Done")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }

                MouseArea {
                    id: doneArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: root.closeRequested()
                }
            }
        }

        // ── Category sidebar (240 pt) ────────────────────────────────────
        Rectangle {
            id: sidebar
            anchors.left: parent.left
            anchors.top: header.bottom
            anchors.bottom: parent.bottom
            width: root.sidebarWidth
            color: Theme.color.surfaceMuted
            border.width: 0

            Rectangle {
                id: searchField
                objectName: "gallerySearch"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: Theme.primitive.spacing.md
                height: 28
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border

                TextInput {
                    id: searchInput
                    objectName: "gallerySearchInput"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    selectByMouse: true
                    text: root.searchText
                    onTextEdited: root.searchText = text
                    Accessible.role: Accessible.EditableText
                    Accessible.name: qsTr("Search widgets")
                }

                Text {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    verticalAlignment: Text.AlignVCenter
                    visible: searchInput.text.length === 0
                    text: qsTr("Search")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            ListView {
                id: categoryList
                objectName: "galleryCategoryList"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: searchField.bottom
                anchors.bottom: parent.bottom
                anchors.topMargin: Theme.primitive.spacing.sm
                anchors.bottomMargin: Theme.primitive.spacing.md
                clip: true
                model: root.allCategories
                spacing: 2

                delegate: Rectangle {
                    id: categoryRow
                    required property var modelData
                    required property int index
                    readonly property bool selected:
                        String(modelData) === root.selectedCategory
                    width: categoryList.width
                    height: 30
                    color: selected ? Theme.color.accentMuted
                                    : categoryArea.containsMouse
                                      ? Theme.color.surfaceElevated : "transparent"

                    Accessible.role: Accessible.ListItem
                    Accessible.name: String(modelData)
                    Accessible.selected: selected

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        text: String(categoryRow.modelData)
                        color: categoryRow.selected ? Theme.color.accent
                                                    : Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        elide: Text.ElideRight
                    }

                    MouseArea {
                        id: categoryArea
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: root.selectedCategory = String(categoryRow.modelData)
                    }
                }
            }
        }

        // ── Catalogue grid ───────────────────────────────────────────────
        Item {
            id: content
            anchors.left: sidebar.right
            anchors.right: parent.right
            anchors.top: header.bottom
            anchors.bottom: parent.bottom
            anchors.margins: Theme.primitive.spacing.lg

            Text {
                id: categoryTitle
                objectName: "galleryCategoryTitle"
                width: parent.width
                text: root.selectedCategory
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                elide: Text.ElideRight
            }

            Text {
                id: emptyNote
                objectName: "galleryEmpty"
                anchors.centerIn: parent
                width: parent.width - 2 * Theme.primitive.spacing.xl
                visible: root.filteredCatalog.length === 0
                text: root.available ? qsTr("No widgets match this search.")
                                     : qsTr("Widgets are unavailable.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeMd
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                Accessible.role: Accessible.StaticText
                Accessible.name: text
            }

            GridView {
                id: grid
                objectName: "galleryGrid"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: categoryTitle.bottom
                anchors.bottom: parent.bottom
                anchors.topMargin: Theme.primitive.spacing.md
                clip: true
                cellWidth: 176
                cellHeight: 156
                model: root.filteredCatalog

                delegate: Rectangle {
                    id: preview
                    required property var modelData
                    readonly property var existing: root.widgetFor(modelData.kind)
                    width: 164
                    height: 144
                    radius: Theme.controls.popup.radius
                    color: Theme.color.surfaceMuted
                    border.width: Theme.controls.window.borderWidth
                    border.color: existing ? Theme.color.accent : Theme.color.border

                    Accessible.role: Accessible.Grouping
                    Accessible.name: String(modelData.title)

                    // The gallery thumbnail is deliberately 102×102 pt; the
                    // live panel card is 164 pt.
                    Rectangle {
                        id: thumbnail
                        objectName: "galleryThumbnail"
                        anchors.top: parent.top
                        anchors.topMargin: Theme.primitive.spacing.sm
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 102
                        height: 102
                        radius: Theme.controls.sidebar.rowRadius
                        color: Theme.color.controlFill
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        Text {
                            anchors.centerIn: parent
                            text: String(preview.modelData.title)
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeXs
                            width: parent.width - 2 * Theme.primitive.spacing.xs
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.WordWrap
                        }
                    }

                    Text {
                        objectName: "galleryWidgetTitle"
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: thumbnail.bottom
                        anchors.topMargin: Theme.primitive.spacing.xs
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        text: String(preview.modelData.title)
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeXs
                        font.weight: Theme.primitive.font.weightMedium
                        elide: Text.ElideRight
                    }

                    // Size is a live-panel property; the gallery preview
                    // stays 102×102. The control is only offered for a widget
                    // already in the panel.
                    Rectangle {
                        id: sizeButton
                        objectName: "gallerySize_" + String(preview.modelData.kind)
                        visible: preview.existing !== null
                        anchors.left: parent.left
                        anchors.bottom: parent.bottom
                        anchors.margins: Theme.primitive.spacing.xs
                        width: sizeLabel.implicitWidth
                               + 2 * Theme.primitive.spacing.sm
                        height: 22
                        radius: Theme.controls.sidebar.rowRadius
                        color: sizeArea.containsMouse
                               ? Theme.color.surfaceElevated : "transparent"
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        readonly property var sizes:
                            ["small", "medium", "large", "extra-large"]

                        function nextSize(current) {
                            var at = sizes.indexOf(String(current));
                            return sizes[(at + 1) % sizes.length];
                        }

                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Change %1 size")
                            .arg(String(preview.modelData.title))

                        Text {
                            id: sizeLabel
                            anchors.centerIn: parent
                            text: preview.existing
                                ? String(preview.existing.size) : qsTr("medium")
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeXs
                        }

                        MouseArea {
                            id: sizeArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: {
                                if (preview.existing)
                                    root.resizeRequested(
                                        String(preview.existing.id),
                                        sizeButton.nextSize(
                                            preview.existing.size));
                            }
                        }
                    }

                    Rectangle {
                        id: toggleButton
                        objectName: "galleryToggle_" + String(preview.modelData.kind)
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.margins: Theme.primitive.spacing.xs
                        width: toggleLabel.implicitWidth + 2 * Theme.primitive.spacing.sm
                        height: 22
                        radius: Theme.controls.sidebar.rowRadius
                        color: toggleArea.containsMouse
                               ? Theme.color.surfaceElevated : "transparent"
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        Accessible.role: Accessible.Button
                        Accessible.name: preview.existing
                            ? qsTr("Remove %1").arg(String(preview.modelData.title))
                            : qsTr("Add %1").arg(String(preview.modelData.title))

                        Text {
                            id: toggleLabel
                            anchors.centerIn: parent
                            text: preview.existing ? qsTr("Remove") : qsTr("Add")
                            color: preview.existing ? Theme.color.danger
                                                    : Theme.color.accent
                            font.pixelSize: Theme.primitive.font.sizeXs
                            font.weight: Theme.primitive.font.weightMedium
                        }

                        MouseArea {
                            id: toggleArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: {
                                if (preview.existing)
                                    root.removeRequested(String(preview.existing.id));
                                else
                                    root.addRequested(String(preview.modelData.kind),
                                                      "medium");
                            }
                        }
                    }
                }
            }
        }
    }
}
