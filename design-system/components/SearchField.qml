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
    // A flat field draws no background/border so it blends into its host
    // surface; the caret still shows focus. Default keeps the standard pill
    // field. Hosts with a larger measured field (the Applications drawer)
    // override the geometry/colour properties below instead of re-rolling the
    // search anatomy.
    property bool flat: false
    property real fieldHeight: Theme.controls.searchField.height
    property real fieldRadius: Theme.controls.searchField.radius
    property color fieldFill: Theme.color.controlFill
    property color fieldBorder: root.activeFocus ? Theme.color.focusRing
                                                 : Theme.color.border
    property real iconSize: Theme.controls.searchField.iconSize
    property real paddingH: Theme.controls.searchField.paddingH
    property real fontSize: Theme.controls.button.fontSize
    // A soft top sheen for a field that sits on a glass card (the Applications
    // drawer). 0 keeps the flat field used by every other host.
    property real fieldHighlight: 0.0

    signal accepted(string text)
    signal cleared()
    // Emitted before Escape clears the field, so a host can also dismiss its
    // overlay when the field was already empty.
    signal escapePressed()

    implicitWidth: Theme.controls.searchField.minWidth
    implicitHeight: root.fieldHeight
    activeFocusOnTab: true

    function clear() {
        if (input.text.length === 0)
            return;
        input.text = "";
        root.cleared();
    }

    Rectangle {
        anchors.fill: parent
        visible: !root.flat
        radius: root.fieldRadius
        color: root.fieldFill
        border.width: Theme.controls.window.borderWidth
        border.color: root.fieldBorder
        antialiasing: true

        // The inner top sheen reads as the lit face of the glass field. It
        // sits inside the fill, below the icon and text, so it never touches
        // the glyphs.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: Math.max(0, parent.radius - 1)
            visible: root.fieldHighlight > 0
            gradient: Gradient {
                GradientStop {
                    position: 0.0
                    color: Qt.rgba(1, 1, 1, root.fieldHighlight)
                }
                GradientStop {
                    position: 0.42
                    color: Qt.rgba(1, 1, 1, 0)
                }
                GradientStop {
                    position: 1.0
                    color: Qt.rgba(1, 1, 1, 0)
                }
            }
        }

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
        size: root.iconSize
        color: Theme.color.textTertiary
        anchors.left: parent.left
        anchors.leftMargin: root.paddingH
        anchors.verticalCenter: parent.verticalCenter
    }

    TextInput {
        id: input
        focus: true
        anchors.left: searchGlyph.right
        anchors.leftMargin: root.paddingH
        anchors.right: clearArea.visible ? clearArea.left : parent.right
        anchors.rightMargin: root.paddingH
        anchors.verticalCenter: parent.verticalCenter
        color: Theme.color.textPrimary
        selectionColor: Theme.color.selection
        selectedTextColor: Theme.color.textPrimary
        font.pixelSize: root.fontSize
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
        width: root.iconSize
        height: width
        anchors.right: parent.right
        anchors.rightMargin: root.paddingH
        anchors.verticalCenter: parent.verticalCenter

        Icon {
            anchors.centerIn: parent
            name: "close"
            size: root.iconSize
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
        root.escapePressed();
        root.clear();
        event.accepted = true;
    }

    Accessible.role: Accessible.EditableText
    Accessible.name: root.placeholderText
    Accessible.editable: true
    Accessible.searchEdit: true
    Accessible.focusable: true
}
