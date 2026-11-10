// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Get Info inspector content (T230d): a single scrolling column in a
// 328×1034 pt auxiliary window with a 32 pt title bar. Sections are data:
// the shell builds them from the live selection, so a single item and a
// multi-selection share one layout and one test surface.
Item {
    id: root

    // The selected item's display name (or "N items" for a multi-selection).
    property string itemName: ""
    property string itemKind: ""
    // An `image://files-thumbnail/…` source, or empty for a glyph.
    property string previewSource: ""
    property string previewIcon: "file"
    property bool multiSelection: false
    property int selectionCount: 0
    // [{ id, title, rows: [{ label, value, editable, reason }] }]
    property var sections: []

    signal renameRequested(string name)

    readonly property var sectionTitles: {
        var out = [];
        for (var i = 0; i < root.sections.length; ++i)
            out.push(root.sections[i].title || "");
        return out;
    }
    readonly property real contentPadding: Theme.primitive.spacing.lg
    readonly property real labelWidth: 112

    function section(id) {
        for (var i = 0; i < root.sections.length; ++i) {
            if (root.sections[i].id === id)
                return root.sections[i];
        }
        return null;
    }

    function row(sectionId, label) {
        var s = root.section(sectionId);
        if (!s)
            return null;
        for (var i = 0; i < s.rows.length; ++i) {
            if (s.rows[i].label === label)
                return s.rows[i];
        }
        return null;
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.color.surface
    }

    Flickable {
        id: scroller
        anchors.fill: parent
        contentWidth: width
        contentHeight: contentColumn.height + 2 * root.contentPadding
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        Column {
            id: contentColumn
            x: root.contentPadding
            y: root.contentPadding
            width: scroller.width - 2 * root.contentPadding
            spacing: Theme.primitive.spacing.lg

            // Header: preview (thumbnail or kind glyph) plus name/kind.
            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.md

                Item {
                    id: preview
                    width: Theme.primitive.spacing.xxxl
                    height: width

                    Image {
                        anchors.fill: parent
                        visible: root.previewSource.length > 0
                        source: root.previewSource
                        fillMode: Image.PreserveAspectFit
                        asynchronous: true
                    }
                    Icon {
                        anchors.centerIn: parent
                        visible: root.previewSource.length === 0
                        name: root.previewIcon
                        size: Theme.primitive.spacing.xxl
                        color: Theme.color.textSecondary
                    }
                }

                Column {
                    width: parent.width - preview.width - parent.spacing
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        width: parent.width
                        text: root.itemName
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeLg
                        font.weight: Theme.primitive.font.weightMedium
                        elide: Text.ElideRight
                    }
                    Text {
                        width: parent.width
                        text: root.multiSelection
                              ? qsTr("%1 items selected").arg(root.selectionCount)
                              : root.itemKind
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        elide: Text.ElideRight
                    }
                }
            }

            Repeater {
                model: root.sections

                delegate: Column {
                    id: sectionColumn
                    required property var modelData
                    width: contentColumn.width
                    spacing: Theme.primitive.spacing.xs

                    Text {
                        width: parent.width
                        text: sectionColumn.modelData.title
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.weight: Theme.primitive.font.weightMedium
                    }

                    Rectangle {
                        width: parent.width
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                    }

                    Repeater {
                        model: sectionColumn.modelData.rows

                        delegate: Item {
                            id: rowItem
                            required property var modelData
                            required property int index
                            width: sectionColumn.width
                            height: rowColumn.height

                            Column {
                                id: rowColumn
                                width: parent.width
                                spacing: Theme.primitive.spacing.xxs

                                Row {
                                    width: parent.width
                                    spacing: Theme.primitive.spacing.sm

                                    Text {
                                        id: rowLabel
                                        width: root.labelWidth
                                        text: rowItem.modelData.label
                                        color: Theme.color.textSecondary
                                        font.pixelSize: Theme.primitive.font.sizeSm
                                        elide: Text.ElideRight
                                        anchors.verticalCenter: parent.verticalCenter
                                    }

                                    Text {
                                        id: rowValue
                                        visible: !rowItem.modelData.editable
                                        width: parent.width - rowLabel.width
                                               - parent.spacing
                                        text: rowItem.modelData.value || ""
                                        color: Theme.color.textPrimary
                                        font.pixelSize: Theme.primitive.font.sizeSm
                                        wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                                        anchors.verticalCenter: parent.verticalCenter
                                    }

                                    Row {
                                        id: editRow
                                        visible: rowItem.modelData.editable === true
                                        width: parent.width - rowLabel.width
                                               - parent.spacing
                                        height: Theme.controls.button.height
                                        spacing: Theme.primitive.spacing.sm
                                        anchors.verticalCenter: parent.verticalCenter

                                        Rectangle {
                                            id: nameField
                                            width: Math.max(0, parent.width
                                                               - renameButton.width
                                                               - parent.spacing)
                                            height: Theme.controls.button.height
                                            radius: Theme.controls.button.radius
                                            color: Theme.color.controlFill
                                            border.width: Theme.controls.window.borderWidth
                                            border.color: nameInput.activeFocus
                                                          ? Theme.color.focusRing
                                                          : Theme.color.border

                                            TextInput {
                                                id: nameInput
                                                anchors.fill: parent
                                                anchors.leftMargin: Theme.controls.searchField.paddingH
                                                anchors.rightMargin: Theme.controls.searchField.paddingH
                                                verticalAlignment: TextInput.AlignVCenter
                                                text: root.itemName
                                                color: Theme.color.textPrimary
                                                selectionColor: Theme.color.selection
                                                selectedTextColor: Theme.color.textPrimary
                                                font.pixelSize: Theme.primitive.font.sizeSm
                                                clip: true
                                                selectByMouse: true
                                                onAccepted: root.renameRequested(text)
                                            }
                                        }

                                        Button {
                                            id: renameButton
                                            text: qsTr("Rename")
                                            enabled: nameInput.text.length > 0
                                                     && nameInput.text !== root.itemName
                                            onClicked: root.renameRequested(nameInput.text)
                                        }
                                    }
                                }

                                Text {
                                    width: parent.width
                                    visible: (rowItem.modelData.reason || "").length > 0
                                    text: rowItem.modelData.reason || ""
                                    color: Theme.color.textTertiary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                                }
                            }

                            Connections {
                                target: root
                                function onItemNameChanged() {
                                    nameInput.text = root.itemName;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Get Info")
}
