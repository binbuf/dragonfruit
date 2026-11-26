// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// Stacked or inline radio buttons (T286): a 16 pt circle with an 8 pt accent
// dot. `options` is a list of strings or `{ label, description }` objects.
// Up/Down (stacked) or Left/Right (inline) move the selection; Space/Return
// activates the focused option. The group is one AT-SPI radio group.
FocusScope {
    id: root

    property var options: []
    property int currentIndex: -1
    property bool inline: false
    // T289: the group's setting name for AT-SPI. The default keeps the
    // generic component usable; a pane names its own group.
    property string accessibleName: qsTr("Options")
    property alias repeater: repeater

    signal activated(int index)

    readonly property int count: root.options.length

    implicitWidth: root.inline ? content.implicitWidth : 240
    implicitHeight: content.implicitHeight
    activeFocusOnTab: false

    function optionAt(index) {
        var option = root.options[index];
        if (option === undefined || option === null)
            return { label: "", description: "" };
        if (typeof option === "string")
            return { label: option, description: "" };
        return { label: String(option.label || ""),
                 description: String(option.description || "") };
    }

    function move(delta) {
        if (root.count === 0)
            return;
        var next = root.currentIndex < 0 ? 0 : root.currentIndex + delta;
        next = Math.max(0, Math.min(root.count - 1, next));
        if (next === root.currentIndex)
            return;
        // Move the selection and the roving focus together, so Space/Return
        // confirms the newly selected option instead of the one the user
        // arrowed away from.
        var item = root.repeater.itemAt(next);
        if (item)
            item.forceActiveFocus();
        root.currentIndex = next;
        root.activated(next);
    }

    function select(index) {
        if (index < 0 || index >= root.count)
            return;
        root.currentIndex = index;
        root.activated(index);
    }

    Flow {
        id: content
        width: root.width
        spacing: root.inline ? Theme.primitive.spacing.lg
                             : Theme.primitive.spacing.xxs

        Repeater {
            id: repeater
            model: root.options

            delegate: Item {
                id: optionRow
                required property var modelData
                required property int index

                readonly property var option: root.optionAt(index)
                readonly property bool selected: root.currentIndex === index

                width: root.inline
                       ? radio.implicitWidth + labelText.implicitWidth
                         + Theme.primitive.spacing.sm
                       : root.width
                height: Math.max(Theme.controls.settingsRow.height,
                                 labelColumn.implicitHeight
                                 + 2 * Theme.primitive.spacing.xs)
                activeFocusOnTab: true

                function choose() {
                    // A pointer pick is also the keyboard focus, so the focus
                    // ring and the selected dot never disagree.
                    optionRow.forceActiveFocus();
                    root.select(optionRow.index);
                }

                Rectangle {
                    id: radio
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.controls.settingsRadio.size
                    height: width
                    radius: width / 2
                    color: "transparent"
                    border.width: Theme.controls.window.borderWidth
                    border.color: optionRow.selected ? Theme.color.accent
                                                     : Theme.color.textTertiary
                    antialiasing: true

                    Rectangle {
                        anchors.centerIn: parent
                        width: Theme.controls.settingsRadio.dot
                        height: width
                        radius: width / 2
                        visible: optionRow.selected
                        color: Theme.color.accent
                        antialiasing: true
                    }
                }

                Column {
                    id: labelColumn
                    anchors.left: root.inline ? radio.right : radio.right
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        id: labelText
                        width: parent.width
                        text: optionRow.option.label
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                        horizontalAlignment: root.LayoutMirroring.enabled
                                             ? Text.AlignRight : Text.AlignLeft
                    }

                    Text {
                        width: parent.width
                        visible: optionRow.option.description.length > 0
                        text: optionRow.option.description
                        color: Glass.increaseContrast ? Theme.color.textPrimary
                                                      : Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                        horizontalAlignment: root.LayoutMirroring.enabled
                                             ? Text.AlignRight : Text.AlignLeft
                    }
                }

                HoverHandler { id: optionHover }

                TapHandler {
                    onTapped: optionRow.choose()
                }

                FocusRing {
                    target: optionRow
                    cornerRadius: Theme.primitive.radius.sm
                    shown: optionRow.activeFocus
                }

                Keys.onPressed: (event) => {
                    switch (event.key) {
                    case Qt.Key_Down:
                    case Qt.Key_Right:
                        root.move(1);
                        event.accepted = true;
                        break;
                    case Qt.Key_Up:
                    case Qt.Key_Left:
                        root.move(-1);
                        event.accepted = true;
                        break;
                    case Qt.Key_Space:
                    case Qt.Key_Return:
                    case Qt.Key_Enter:
                        optionRow.choose();
                        event.accepted = true;
                        break;
                    }
                }

                Accessible.role: Accessible.RadioButton
                Accessible.name: optionRow.option.label
                Accessible.description: optionRow.option.description
                Accessible.checkable: true
                Accessible.checked: optionRow.selected
                Accessible.focusable: true
                Accessible.onPressAction: optionRow.choose()
            }
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.accessibleName
}
