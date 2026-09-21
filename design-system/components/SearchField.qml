// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A pill search field with a leading glyph and a clear affordance. The field
// is AT-SPI EditableText marked as a search edit; Escape clears, Return
// commits. All color/spacing comes from tokens.
FocusScope {
    id: root

    property alias text: input.text
    property string placeholderText: qsTr("Search")
    property alias input: input
    property alias hovered: fieldHover.hovered

    signal accepted(string text)
    signal cleared()

    implicitWidth: Theme.controls.searchField.minWidth
    implicitHeight: Theme.controls.searchField.height
    activeFocusOnTab: true

    function clear() {
        if (input.text.length === 0)
            return;
        input.text = "";
        root.cleared();
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.searchField.radius
        color: Theme.color.controlFill
        border.width: Theme.controls.window.borderWidth
        border.color: root.activeFocus ? Theme.color.focusRing : Theme.color.border
        antialiasing: true

        Behavior on border.color {
            ColorAnimation {
                duration: Theme.motion.focus.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.focus.curve
            }
        }
    }

    Icon {
        id: searchGlyph
        name: "search"
        size: Theme.controls.searchField.iconSize
        color: Theme.color.textTertiary
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.searchField.paddingH
        anchors.verticalCenter: parent.verticalCenter
    }

    TextInput {
        id: input
        focus: true
        anchors.left: searchGlyph.right
        anchors.leftMargin: Theme.controls.searchField.paddingH
        anchors.right: clearArea.visible ? clearArea.left : parent.right
        anchors.rightMargin: Theme.controls.searchField.paddingH
        anchors.verticalCenter: parent.verticalCenter
        color: Theme.color.textPrimary
        selectionColor: Theme.color.selection
        selectedTextColor: Theme.color.textPrimary
        font.pixelSize: Theme.controls.button.fontSize
        clip: true
        selectByMouse: true

        onAccepted: root.accepted(input.text)

        Text {
            anchors.fill: parent
            visible: input.text.length === 0
            text: root.placeholderText
            color: Theme.color.textTertiary
            font.pixelSize: input.font.pixelSize
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
    }

    Item {
        id: clearArea
        visible: input.text.length > 0
        width: Theme.controls.searchField.iconSize
        height: width
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.searchField.paddingH
        anchors.verticalCenter: parent.verticalCenter

        Icon {
            anchors.centerIn: parent
            name: "close"
            size: Theme.controls.searchField.iconSize
            color: clearHover.hovered ? Theme.color.textPrimary : Theme.color.textTertiary
        }
        HoverHandler { id: clearHover }
        TapHandler {
            onTapped: root.clear()
        }
    }

    HoverHandler { id: fieldHover }

    TapHandler {
        onTapped: input.forceActiveFocus()
    }

    Keys.onEscapePressed: (event) => {
        root.clear();
        event.accepted = true;
    }

    Accessible.role: Accessible.EditableText
    Accessible.name: root.placeholderText
    Accessible.editable: true
    Accessible.searchEdit: true
    Accessible.focusable: true
}
