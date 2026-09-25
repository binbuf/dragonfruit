// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The application navigation sidebar. Sections carry a title; items carry an
// optional glyph and badge. Selection is single and keyboard-driven
// (Up/Down/Home/End), rows are AT-SPI list items, and the whole thing is the
// one place a first-party app's source list is drawn.
FocusScope {
    id: root

    // [{ title: string, items: [{ label, icon, badge }] }]
    property var sections: []
    property int currentIndex: -1
    property alias hoveredIndex: hoverTracker.index

    signal activated(int index, var item)

    readonly property var entries: {
        var out = [];
        var index = 0;
        for (var s = 0; s < root.sections.length; ++s) {
            var section = root.sections[s] || {};
            // Untitled sections are a flat list with no header row (the
            // Settings pane list); titled sections keep their header.
            if ((section.title || "").length > 0) {
                out.push({
                    index: index++,
                    type: "header",
                    label: section.title || "",
                    icon: "",
                    badge: ""
                });
            }
            var items = section.items || [];
            for (var i = 0; i < items.length; ++i) {
                var item = items[i] || {};
                out.push({
                    index: index++,
                    type: "item",
                    label: item.label || "",
                    icon: item.icon || "",
                    badge: item.badge || "",
                    section: s,
                    itemIndex: i
                });
            }
        }
        return out;
    }
    readonly property var currentItem: {
        var e = root.entries[root.currentIndex];
        return e && e.type === "item" ? e : null;
    }

    implicitWidth: Theme.controls.sidebar.width
    implicitHeight: 320
    activeFocusOnTab: true

    function move(delta) {
        var count = root.entries.length;
        if (count === 0)
            return;
        var i = root.currentIndex;
        for (var step = 0; step < count; ++step) {
            i = (i + delta + count) % count;
            if (root.entries[i].type === "item") {
                root.currentIndex = i;
                return;
            }
        }
    }

    function activateIndex(index) {
        var e = root.entries[index];
        if (!e || e.type !== "item")
            return;
        root.currentIndex = index;
        root.activated(index, e);
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.color.surfaceMuted
    }
    Rectangle {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.controls.window.borderWidth
        color: Theme.color.border
    }

    Flickable {
        id: list
        anchors.fill: parent
        anchors.margins: Theme.controls.sidebar.padding
        clip: true
        contentWidth: width
        contentHeight: rows.implicitHeight
        boundsBehavior: Flickable.StopAtBounds

        Column {
            id: rows
            width: list.width

            Repeater {
                model: root.entries
                delegate: SidebarRow {
                    required property var modelData
                    entry: modelData
                }
            }
        }
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
            root.currentIndex = -1;
            root.move(1);
            event.accepted = true;
            break;
        case Qt.Key_End:
            root.currentIndex = root.entries.length;
            root.move(-1);
            event.accepted = true;
            break;
        case Qt.Key_Return:
        case Qt.Key_Enter:
        case Qt.Key_Space:
            root.activateIndex(root.currentIndex);
            event.accepted = true;
            break;
        }
    }

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Sidebar")

    component SidebarRow: Item {
        id: row

        required property var entry
        readonly property bool isHeader: entry.type === "header"
        readonly property bool selected: root.currentIndex === entry.index
        readonly property bool hovered: rowHover.hovered

        width: parent ? parent.width : 0
        height: isHeader ? headerText.implicitHeight + Theme.controls.sidebar.sectionGap
                         : Theme.controls.sidebar.rowHeight

        Text {
            id: headerText
            visible: row.isHeader
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.leftMargin: Theme.controls.sidebar.padding
            text: row.entry.label
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeXs
            font.weight: Theme.primitive.font.weightSemibold
            font.capitalization: Font.AllUppercase
            elide: Text.ElideRight
        }

        // Hover fill (animated). The selection fill is a second, instant
        // layer: a freshly opened window must not paint a half-faded
        // selection highlight (the hover animation can stall while the client
        // is idle and no frames are delivered).
        Rectangle {
            visible: !row.isHeader
            anchors.fill: parent
            radius: Theme.controls.sidebar.rowRadius
            color: row.hovered && !row.selected ? Theme.color.controlHover
                                                : "transparent"
            antialiasing: true

            Behavior on color {
                ColorAnimation {
                    duration: Theme.motion.hover.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.hover.curve
                }
            }
        }

        Rectangle {
            visible: !row.isHeader
            anchors.fill: parent
            radius: Theme.controls.sidebar.rowRadius
            color: row.selected ? Theme.color.accent : "transparent"
            antialiasing: true
        }

        Icon {
            id: rowIcon
            visible: !row.isHeader && row.entry.icon.length > 0
            name: row.entry.icon
            size: Theme.controls.sidebar.iconSize
            color: row.selected ? Theme.color.accentContent : Theme.color.textSecondary
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.sidebar.padding
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: rowLabel
            visible: !row.isHeader
            anchors.left: row.entry.icon.length > 0 ? rowIcon.right : parent.left
            anchors.leftMargin: Theme.controls.sidebar.padding
            anchors.right: badge.visible ? badge.left : parent.right
            anchors.rightMargin: Theme.controls.sidebar.padding
            anchors.verticalCenter: parent.verticalCenter
            text: row.entry.label
            color: row.selected ? Theme.color.accentContent : Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            elide: Text.ElideRight
        }

        Rectangle {
            id: badge
            visible: !row.isHeader && row.entry.badge.length > 0
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.sidebar.padding
            anchors.verticalCenter: parent.verticalCenter
            width: badgeLabel.implicitWidth + 2 * Theme.primitive.spacing.sm
            height: badgeLabel.implicitHeight + Theme.primitive.spacing.xxs
            radius: height / 2
            color: row.selected ? Theme.color.accentContent : Theme.color.controlActive

            Text {
                id: badgeLabel
                anchors.centerIn: parent
                text: row.entry.badge
                color: row.selected ? Theme.color.accent : Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeXs
            }
        }

        HoverHandler {
            id: rowHover
            onHoveredChanged: {
                if (hovered && !row.isHeader)
                    root.hoveredIndex = row.entry.index;
            }
        }

        TapHandler {
            onTapped: {
                if (!row.isHeader)
                    root.activateIndex(row.entry.index);
            }
        }

        FocusRing {
            visible: !row.isHeader
            target: row
            cornerRadius: Theme.controls.sidebar.rowRadius
            shown: row.selected && root.activeFocus
        }

        Accessible.role: row.isHeader ? Accessible.StaticText : Accessible.ListItem
        Accessible.name: row.entry.label
        Accessible.selected: !row.isHeader && row.selected
        Accessible.selectable: !row.isHeader
        Accessible.focusable: !row.isHeader
        Accessible.onPressAction: {
            if (!row.isHeader)
                root.activateIndex(row.entry.index);
        }
    }

    QtObject {
        id: hoverTracker
        property int index: -1
    }
}
