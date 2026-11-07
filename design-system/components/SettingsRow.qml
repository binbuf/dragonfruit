// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One label + control row inside a SettingsGroup (T-09.1a; row anatomy T223).
//
// The row owns one anatomy for every settings surface: the label column, the
// optional description, the control slot, an optional disclosure chevron, an
// optional trailing action, and an optional help affordance. It never draws the
// control itself. `settingsId`, `owner`, and `settingsKey` link the rendered row
// back to the executable map (`SettingsMap.qml`); T223's search and deep links
// use `settingsId` first and fall back to the label/synonyms.
//
// `highlighted` is the deep-link landing state: the row paints an accent wash
// that the shell clears after a short hold. AT-SPI sees a labelled grouping so
// a screen reader reads the control with its setting name.
Item {
    id: root

    property string label: ""
    property string description: ""
    property alias controlData: controlSlot.data
    property alias control: controlSlot
    property bool showSeparator: true

    // T223 map wiring. A row without an id is still rendered; it is simply not
    // an individually searchable/deep-linkable target until a later pane task
    // gives it the matching map id.
    property string settingsId: ""
    property string owner: ""
    property string settingsKey: ""
    property var synonyms: []
    // Help is indexed by SettingsSearch; a non-empty help string shows the row's
    // trailing `?` affordance. `helpUrl` is optional (the pane usually points
    // it at the map document anchor).
    property string help: ""
    property url helpUrl: ""

    // Disclosure rows (a right chevron) and trailing action rows (a ghost
    // button such as `Details...`) share the same trailing cell.
    property bool disclosure: false
    property string trailingAction: ""
    signal disclosureClicked()
    signal trailingActionClicked()
    signal helpRequested(string topic)

    // The deep-link landing wash.
    property bool highlighted: false

    readonly property bool hasTrailing: root.disclosure
                                          || root.trailingAction.length > 0
                                          || root.help.length > 0

    // Test/consumer aliases for the one anatomy.
    property alias labelText: labelText
    property alias descriptionText: descriptionText
    property alias labelColumn: labelColumn
    property alias trailingRow: trailingRow
    property alias disclosureArea: disclosureArea
    property alias trailingButton: trailingButton
    property alias helpArea: helpArea
    property alias highlightRect: highlightRect

    implicitWidth: 480
    implicitHeight: Math.max(Theme.controls.settingsRow.height,
                             labelColumn.implicitHeight + 2 * Theme.primitive.spacing.sm)
    activeFocusOnTab: true

    // T223: a deep-linked row gets an accent wash that fades in and out. It
    // sits behind the row content so the control and label stay legible.
    Rectangle {
        id: highlightRect
        anchors.fill: parent
        radius: Theme.controls.settingsGroup.radius
        color: Theme.color.accentMuted
        opacity: root.highlighted ? 1.0 : 0.0
        visible: opacity > 0
        antialiasing: true

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.motion.hover.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.hover.curve
            }
        }
    }

    // The trailing cell: trailing action, disclosure, then help, right to left
    // in that order so the chevron stays the closest mark to the control.
    Row {
        id: trailingRow
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.settingsRow.paddingH
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xs
        visible: root.hasTrailing

        Button {
            id: trailingButton
            visible: root.trailingAction.length > 0
            text: root.trailingAction
            variant: "ghost"
            accessibleName: root.trailingAction
            onClicked: root.trailingActionClicked()
        }

        Item {
            id: disclosureArea
            visible: root.disclosure
            width: Theme.controls.button.iconSize + 2 * Theme.primitive.spacing.xxs
            height: width

            Icon {
                anchors.centerIn: parent
                name: "chevron-right"
                size: Theme.controls.button.iconSize
                color: Theme.color.textTertiary
            }

            TapHandler {
                onTapped: root.disclosureClicked()
            }

            Accessible.role: Accessible.Button
            Accessible.name: root.label
            Accessible.description: qsTr("Show details")
            Accessible.focusable: true
            Accessible.onPressAction: root.disclosureClicked()
        }

        Item {
            id: helpArea
            visible: root.help.length > 0
            width: Theme.controls.button.iconSize + 2 * Theme.primitive.spacing.xxs
            height: width

            Rectangle {
                anchors.fill: parent
                radius: width / 2
                color: helpHover.hovered ? Theme.color.controlHover : "transparent"
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border
                antialiasing: true
            }

            Text {
                anchors.centerIn: parent
                text: "?" // df-allow-untranslated: help glyph
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightSemibold
            }

            HoverHandler { id: helpHover }

            TapHandler {
                onTapped: {
                    root.helpRequested(root.help);
                    if (root.helpUrl.toString().length > 0)
                        Qt.openUrlExternally(root.helpUrl);
                }
            }

            Accessible.role: Accessible.Button
            Accessible.name: qsTr("Help for %1").arg(root.label)
            Accessible.focusable: true
            Accessible.onPressAction: {
                root.helpRequested(root.help);
                if (root.helpUrl.toString().length > 0)
                    Qt.openUrlExternally(root.helpUrl);
            }
        }
    }

    Item {
        id: controlSlot
        anchors.right: root.hasTrailing ? trailingRow.left : parent.right
        anchors.rightMargin: root.hasTrailing
                ? Theme.controls.settingsRow.controlGap
                : Theme.controls.settingsRow.paddingH
        anchors.verticalCenter: parent.verticalCenter
        width: childrenRect.width
        height: childrenRect.height
    }

    Column {
        id: labelColumn
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.settingsRow.paddingH
        anchors.right: controlSlot.left
        anchors.rightMargin: Theme.controls.settingsRow.controlGap
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xxs

        Text {
            id: labelText
            width: parent.width
            text: root.label
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            elide: Text.ElideRight
        }
        Text {
            id: descriptionText
            width: parent.width
            visible: root.description.length > 0
            text: root.description
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
    }

    Rectangle {
        visible: root.showSeparator
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.settingsRow.paddingH
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.settingsRow.paddingH
        anchors.bottom: parent.bottom
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
    }

    // Move keyboard focus into the row's control when a deep link lands on it.
    function focusFirstControl() {
        var found = root.findFocusable(controlSlot);
        if (found !== null) {
            found.forceActiveFocus();
            return;
        }
        if (root.hasTrailing)
            root.forceActiveFocus();
        else
            root.forceActiveFocus();
    }

    function findFocusable(item) {
        if (item === null || item === undefined)
            return null;
        if (item.activeFocusOnTab === true && item.visible)
            return item;
        var children = item.children;
        for (var i = 0; i < children.length; ++i) {
            var found = root.findFocusable(children[i]);
            if (found !== null)
                return found;
        }
        return null;
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                || event.key === Qt.Key_Space) {
            if (root.disclosure) {
                root.disclosureClicked();
                event.accepted = true;
            } else if (root.trailingAction.length > 0) {
                root.trailingActionClicked();
                event.accepted = true;
            }
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.label
    Accessible.description: root.help.length > 0
                           ? root.description + (root.description.length > 0 ? " " : "")
                             + root.help
                           : root.description
    Accessible.focusable: true
    Accessible.onPressAction: {
        if (root.disclosure)
            root.disclosureClicked();
        else if (root.trailingAction.length > 0)
            root.trailingActionClicked();
    }
}
