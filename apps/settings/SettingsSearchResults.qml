// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The ranked Settings search results list (T223, grouped for T265). It replaces
// the sidebar pane list while the search field has text, shows pane-grouped
// results from `SettingsSearch`, and activates a result's deep link.
//
// One pane group renders a small pane header above its first row. A capped
// group ends with a "Show all" row; activating it expands that group instead
// of navigating. The top line carries the result count.
//
// Keyboard: Up/Down move, Home/End jump, Return opens, Escape dismisses. The
// list is a real AT-SPI list: every item names its pane and section, and the
// no-match state is announced. The search field's Down key hands focus here,
// so the whole path is keyboard-only.
FocusScope {
    id: root

    property var results: []
    property int currentIndex: -1
    property string query: ""

    signal activated(var result)
    signal dismissed()

    readonly property bool hasResults: root.results.length > 0
    readonly property int resultCount: SettingsSearch.resultCount(root.results)
    readonly property int groupCount: SettingsSearch.groupCount(root.results)
    readonly property string resultSummary: {
        if (root.groupCount > 1) {
            return qsTr("%1 results in %2 sections")
                .arg(root.resultCount).arg(root.groupCount);
        }
        if (root.resultCount === 1)
            return qsTr("1 result");
        return qsTr("%1 results").arg(root.resultCount);
    }
    property string announcedQuery: ""
    property alias listView: list
    property alias emptyText: emptyText
    property alias countText: countText

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
        // A no-match query is announced once, not on every debounce tick.
        if (root.query.trim().length > 0 && root.results.length === 0
                && root.announcedQuery !== root.query) {
            root.announcedQuery = root.query;
            Accessible.announce(qsTr("No results for %1").arg(root.query));
        }
    }

    onQueryChanged: root.announcedQuery = ""

    onCurrentIndexChanged: {
        if (root.currentIndex >= 0 && root.currentIndex < root.results.length)
            list.positionViewAtIndex(root.currentIndex, ListView.Contain);
    }

    Component.onCompleted: {
        root.currentIndex = root.results.length > 0 ? 0 : -1;
    }

    Column {
        anchors.fill: parent
        spacing: Theme.primitive.spacing.xs

        Text {
            id: countText
            objectName: "settingsSearchResultCount"
            width: parent.width
            visible: root.hasResults
            leftPadding: Theme.primitive.spacing.sm
            rightPadding: Theme.primitive.spacing.sm
            text: root.resultSummary
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeXs
            elide: Text.ElideRight
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }

        ListView {
            id: list
            width: parent.width
            height: Math.max(0, parent.height - (countText.visible
                                                 ? countText.height + parent.spacing
                                                 : 0))
            visible: root.hasResults
            clip: true
            model: root.results
            boundsBehavior: Flickable.StopAtBounds
            spacing: Theme.primitive.spacing.xxs

            delegate: Item {
                id: resultRow
                required property var modelData
                required property int index

                readonly property bool current: root.currentIndex === index
                readonly property bool hovered: rowHover.hovered
                readonly property bool isShowAll: modelData.kind === "showAll"
                readonly property bool hasGroupHeader:
                    modelData.groupStart === true
                readonly property string kindLabel: modelData.kind === "pane"
                        ? qsTr("Pane")
                        : modelData.kind === "section" ? qsTr("Section")
                        : modelData.kind === "showAll" ? qsTr("Show all")
                                                       : qsTr("Setting")
                readonly property string accessibleName:
                    modelData.kind === "pane" ? modelData.paneTitle
                    : modelData.kind === "section" ? modelData.label
                                                   : modelData.label
                readonly property string accessibleDetail: {
                    if (modelData.kind === "pane")
                        return qsTr("Pane");
                    if (modelData.kind === "section")
                        return qsTr("Section in %1").arg(modelData.paneTitle);
                    if (modelData.kind === "showAll")
                        return qsTr("Show every result in %1")
                            .arg(modelData.paneTitle);
                    return qsTr("%1 in %2")
                        .arg(modelData.section).arg(modelData.paneTitle);
                }

                width: ListView.view ? ListView.view.width : 0
                height: groupHeader.height + rowBody.height

                Item {
                    id: groupHeader
                    width: parent.width
                    height: resultRow.hasGroupHeader
                            ? headerLabel.implicitHeight + Theme.primitive.spacing.xs
                            : 0
                    visible: resultRow.hasGroupHeader

                    Text {
                        id: headerLabel
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        text: resultRow.modelData.groupTitle || ""
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeXs
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        Accessible.role: Accessible.StaticText
                        Accessible.name: text
                    }
                }

                Rectangle {
                    id: rowBody
                    anchors.top: groupHeader.bottom
                    width: parent.width
                    height: Math.max(Theme.controls.sidebar.rowHeight
                                     + Theme.primitive.spacing.sm,
                                     labelText.implicitHeight
                                     + subtitleText.implicitHeight
                                     + 2 * Theme.primitive.spacing.xs)
                    radius: Theme.controls.sidebar.rowRadius
                    color: resultRow.current ? Theme.color.accent
                                             : (resultRow.hovered
                                                ? Theme.color.controlHover
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
                            name: resultRow.isShowAll
                                  ? "chevron-down"
                                  : resultRow.modelData.kind === "pane"
                                    ? resultRow.modelData.icon
                                    : resultRow.modelData.kind === "section"
                                      ? "folder" : "file"
                            size: Theme.controls.sidebar.iconSize
                            color: resultRow.current
                                   ? Theme.color.accentContent
                                   : Theme.color.textSecondary
                        }

                        Column {
                            width: Math.max(0, parent.width
                                            - Theme.controls.sidebar.iconSize
                                            - Theme.primitive.spacing.sm)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                id: labelText
                                width: parent.width
                                text: resultRow.modelData.label
                                color: resultRow.current
                                       ? Theme.color.accentContent
                                       : Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                elide: Text.ElideRight
                            }

                            Text {
                                id: subtitleText
                                width: parent.width
                                visible: text.length > 0
                                text: resultRow.modelData.subtitle
                                color: resultRow.current
                                       ? Theme.color.accentContent
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
                }

                Accessible.role: Accessible.ListItem
                Accessible.name: resultRow.accessibleName
                Accessible.description: resultRow.accessibleDetail
                Accessible.selected: resultRow.current
                Accessible.selectable: true
                Accessible.focusable: true
                Accessible.onPressAction: {
                    root.currentIndex = resultRow.index;
                    root.activateCurrent();
                }
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
        Accessible.role: Accessible.StaticText
        Accessible.name: text
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
    Accessible.name: root.hasResults
                    ? qsTr("Search results, %1").arg(root.resultSummary)
                    : qsTr("Search results, no matches")
}
