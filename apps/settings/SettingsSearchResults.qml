// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The ranked Settings search results list (T223). It replaces the sidebar pane
// list while the search field has text, shows pane/section/row results from
// `SettingsSearch`, and activates a result's deep link.
//
// Keyboard: Up/Down move, Home/End jump, Return opens, Escape dismisses. The
// list is a real AT-SPI list with one list item per result; the search field's
// Down key hands focus here, so the whole path is keyboard-only.
FocusScope {
    id: root

    property var results: []
    property int currentIndex: -1
    property string query: ""

    signal activated(var result)
    signal dismissed()

    readonly property bool hasResults: root.results.length > 0
    property alias listView: list
    property alias emptyText: emptyText

    implicitWidth: Theme.controls.sidebar.width
    implicitHeight: 240
    activeFocusOnTab: true

    function move(delta) {
        if (root.results.length === 0) {
            root.currentIndex = -1;
            return;
        }
        var next = root.currentIndex + delta;
        if (next < 0)
            next = root.results.length - 1;
        else if (next >= root.results.length)
            next = 0;
        root.currentIndex = next;
    }

    function activateCurrent() {
        if (root.currentIndex < 0 || root.currentIndex >= root.results.length)
            return;
        root.activated(root.results[root.currentIndex]);
    }

    function focusList() {
        root.forceActiveFocus();
        if (root.currentIndex < 0 && root.results.length > 0)
            root.currentIndex = 0;
    }

    onResultsChanged: {
        root.currentIndex = root.results.length > 0 ? 0 : -1;
    }

    onCurrentIndexChanged: {
        if (root.currentIndex >= 0 && root.currentIndex < root.results.length)
            list.positionViewAtIndex(root.currentIndex, ListView.Contain);
    }

    Component.onCompleted: {
        root.currentIndex = root.results.length > 0 ? 0 : -1;
    }

    ListView {
        id: list
        anchors.fill: parent
        visible: root.hasResults
        clip: true
        model: root.results
        boundsBehavior: Flickable.StopAtBounds
        spacing: Theme.primitive.spacing.xxs

        delegate: Rectangle {
            id: resultRow
            required property var modelData
            required property int index

            readonly property bool current: root.currentIndex === index
            readonly property bool hovered: rowHover.hovered
            readonly property string kindLabel: modelData.kind === "pane"
                    ? qsTr("Pane")
                    : modelData.kind === "section" ? qsTr("Section")
                                                   : qsTr("Setting")

            width: ListView.view ? ListView.view.width : 0
            height: Math.max(Theme.controls.sidebar.rowHeight + Theme.primitive.spacing.sm,
                             labelText.implicitHeight + subtitleText.implicitHeight
                             + 2 * Theme.primitive.spacing.xs)
            radius: Theme.controls.sidebar.rowRadius
            color: resultRow.current ? Theme.color.accent
                                     : (resultRow.hovered ? Theme.color.controlHover
                                                          : "transparent")
            antialiasing: true

            Row {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: Theme.primitive.spacing.sm
                anchors.rightMargin: Theme.primitive.spacing.sm
                anchors.verticalCenter: parent.verticalCenter
                spacing: Theme.primitive.spacing.sm

                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: resultRow.modelData.kind === "pane"
                          ? resultRow.modelData.icon
                          : resultRow.modelData.kind === "section" ? "folder" : "file"
                    size: Theme.controls.sidebar.iconSize
                    color: resultRow.current ? Theme.color.accentContent
                                             : Theme.color.textSecondary
                }

                Column {
                    width: Math.max(0, parent.width - Theme.controls.sidebar.iconSize
                                    - Theme.primitive.spacing.sm)
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        id: labelText
                        width: parent.width
                        text: resultRow.modelData.label
                        color: resultRow.current ? Theme.color.accentContent
                                                 : Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Text {
                        id: subtitleText
                        width: parent.width
                        visible: text.length > 0
                        text: resultRow.modelData.subtitle
                        color: resultRow.current ? Theme.color.accentContent
                                                 : Theme.color.textTertiary
                        opacity: resultRow.current ? 0.85 : 1.0
                        font.pixelSize: Theme.primitive.font.sizeXs
                        elide: Text.ElideRight
                    }
                }
            }

            HoverHandler { id: rowHover }

            TapHandler {
                onTapped: {
                    root.currentIndex = resultRow.index;
                    root.activateCurrent();
                }
            }

            Accessible.role: Accessible.ListItem
            Accessible.name: resultRow.modelData.label
            Accessible.description: resultRow.modelData.subtitle
                                    + " " + resultRow.kindLabel
            Accessible.selected: resultRow.current
            Accessible.selectable: true
            Accessible.focusable: true
            Accessible.onPressAction: {
                root.currentIndex = resultRow.index;
                root.activateCurrent();
            }
        }
    }

    Text {
        id: emptyText
        objectName: "settingsSearchNoResults"
        visible: !root.hasResults
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.topMargin: Theme.primitive.spacing.md
        anchors.leftMargin: Theme.primitive.spacing.sm
        anchors.rightMargin: Theme.primitive.spacing.sm
        text: qsTr("No results for \u201c%1\u201d.").arg(root.query)
        color: Theme.color.textTertiary
        font.pixelSize: Theme.primitive.font.sizeSm
        wrapMode: Text.WordWrap
    }

    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_Down:
            root.move(1);
            event.accepted = true;
            break;
        case Qt.Key_Up:
            root.move(-1);
            event.accepted = true;
            break;
        case Qt.Key_Home:
            root.currentIndex = root.results.length > 0 ? 0 : -1;
            event.accepted = true;
            break;
        case Qt.Key_End:
            root.currentIndex = root.results.length - 1;
            event.accepted = true;
            break;
        case Qt.Key_Return:
        case Qt.Key_Enter:
            root.activateCurrent();
            event.accepted = true;
            break;
        case Qt.Key_Escape:
            root.dismissed();
            event.accepted = true;
            break;
        }
    }

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Search results")
}
