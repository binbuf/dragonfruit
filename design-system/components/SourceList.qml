// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A flat source list with optional tree indentation and disclosure. Items are
// `{ label, icon, depth, hasChildren, expanded, badge }`; `expanded` seeds the
// initial state and `expandedChanged` reports toggles. Rows are AT-SPI tree
// items when the model is hierarchical, list items otherwise.
FocusScope {
    id: root

    property var model: []
    property int currentIndex: -1
    property alias hoveredIndex: hoverTracker.index

    signal activated(int index, var item)
    signal expandedChanged(int index, bool expanded)

    readonly property int rowHeight: Theme.controls.sourceList.rowHeight
    readonly property bool isTree: {
        for (var i = 0; i < root.model.length; ++i) {
            if ((root.model[i] || {}).hasChildren === true)
                return true;
        }
        return false;
    }

    // Explicit expansion overrides keyed by model index. Empty means "use the
    // model's `expanded` seed". Stored as a plain object so assigning a copy
    // re-fires the `visibleEntries` binding.
    property var expansion: ({})
    readonly property var visibleEntries: {
        var out = [];
        var hiddenBelowDepth = -1;
        for (var i = 0; i < root.model.length; ++i) {
            var m = root.model[i] || {};
            var depth = m.depth || 0;
            if (hiddenBelowDepth >= 0 && depth > hiddenBelowDepth)
                continue;
            hiddenBelowDepth = -1;
            out.push({
                index: i,
                label: m.label || "",
                icon: m.icon || "",
                badge: m.badge || "",
                depth: depth,
                hasChildren: m.hasChildren === true,
                expanded: root.isExpanded(i)
            });
            if (m.hasChildren === true && !root.isExpanded(i))
                hiddenBelowDepth = depth;
        }
        return out;
    }
    readonly property int visiblePosition: {
        for (var i = 0; i < root.visibleEntries.length; ++i) {
            if (root.visibleEntries[i].index === root.currentIndex)
                return i;
        }
        return -1;
    }

    implicitWidth: Theme.controls.sidebar.width
    implicitHeight: 240
    activeFocusOnTab: true

    function isExpanded(index) {
        if (root.expansion[index] !== undefined)
            return root.expansion[index];
        var m = root.model[index] || {};
        return m.expanded !== false;
    }

    function toggleExpanded(index) {
        var m = root.model[index] || {};
        if (m.hasChildren !== true)
            return;
        var next = {};
        for (var key in root.expansion)
            next[key] = root.expansion[key];
        next[index] = !root.isExpanded(index);
        root.expansion = next;
        root.expandedChanged(index, next[index]);
    }

    function activateIndex(index) {
        var m = root.model[index];
        if (!m)
            return;
        root.currentIndex = index;
        root.activated(index, m);
    }

    function moveVisible(delta) {
        var count = root.visibleEntries.length;
        if (count === 0)
            return;
        var position = root.visiblePosition;
        position = (position + delta + count) % count;
        root.currentIndex = root.visibleEntries[position].index;
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.color.surface
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
                model: root.visibleEntries
                delegate: SourceRow {
                    required property var modelData
                    entry: modelData
                }
            }
        }
    }

    Keys.onPressed: (event) => {
        var entry = root.model[root.currentIndex] || {};
        switch (event.key) {
        case Qt.Key_Down:
            root.moveVisible(1);
            event.accepted = true;
            break;
        case Qt.Key_Up:
            root.moveVisible(-1);
            event.accepted = true;
            break;
        case Qt.Key_Right:
            if (entry.hasChildren === true && !root.isExpanded(root.currentIndex))
                root.toggleExpanded(root.currentIndex);
            else
                root.moveVisible(1);
            event.accepted = true;
            break;
        case Qt.Key_Left:
            if (entry.hasChildren === true && root.isExpanded(root.currentIndex)) {
                root.toggleExpanded(root.currentIndex);
            } else {
                var position = root.visiblePosition;
                for (var i = position - 1; i >= 0; --i) {
                    if (root.visibleEntries[i].depth < (entry.depth || 0)) {
                        root.currentIndex = root.visibleEntries[i].index;
                        break;
                    }
                }
            }
            event.accepted = true;
            break;
        case Qt.Key_Home:
            if (root.visibleEntries.length > 0)
                root.currentIndex = root.visibleEntries[0].index;
            event.accepted = true;
            break;
        case Qt.Key_End:
            if (root.visibleEntries.length > 0)
                root.currentIndex = root.visibleEntries[root.visibleEntries.length - 1].index;
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

    Accessible.role: root.isTree ? Accessible.Tree : Accessible.List
    Accessible.name: qsTr("Source list")

    component SourceRow: Item {
        id: row

        required property var entry
        readonly property bool selected: root.currentIndex === entry.index
        readonly property bool hovered: rowHover.hovered
        readonly property real indent: Theme.controls.sidebar.padding
                                       + entry.depth * Theme.controls.sourceList.indent

        width: parent ? parent.width : 0
        height: root.rowHeight

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.sourceList.rowRadius
            color: row.selected ? Theme.color.accent
                                : (row.hovered ? Theme.color.controlHover : "transparent")
            antialiasing: true

            Behavior on color {
                ColorAnimation {
                    duration: Theme.motion.hover.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.hover.curve
                }
            }
        }

        Icon {
            id: disclosure
            visible: row.entry.hasChildren
            name: row.entry.expanded ? "chevron-down" : "chevron-right"
            size: Theme.controls.sidebar.iconSize
            color: row.selected ? Theme.color.accentContent : Theme.color.textTertiary
            anchors.left: parent.left
            anchors.leftMargin: row.indent
            anchors.verticalCenter: parent.verticalCenter
        }

        MouseArea {
            anchors.fill: disclosure
            enabled: row.entry.hasChildren
            onClicked: root.toggleExpanded(row.entry.index)
        }

        Icon {
            id: rowIcon
            visible: !row.entry.hasChildren && row.entry.icon.length > 0
            name: row.entry.icon
            size: Theme.controls.sidebar.iconSize
            color: row.selected ? Theme.color.accentContent : Theme.color.textSecondary
            anchors.left: parent.left
            anchors.leftMargin: row.indent + (row.entry.hasChildren
                                             ? Theme.controls.sidebar.iconSize + Theme.primitive.spacing.sm : 0)
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: rowLabel
            anchors.left: parent.left
            anchors.leftMargin: row.indent + (row.entry.hasChildren
                                              ? Theme.controls.sidebar.iconSize + Theme.primitive.spacing.sm
                                              : (row.entry.icon.length > 0
                                                 ? Theme.controls.sidebar.iconSize + Theme.primitive.spacing.sm : 0))
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
            visible: row.entry.badge.length > 0
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
                if (hovered)
                    root.hoveredIndex = row.entry.index;
            }
        }

        TapHandler {
            onTapped: root.activateIndex(row.entry.index)
        }

        FocusRing {
            target: row
            cornerRadius: Theme.controls.sourceList.rowRadius
            shown: row.selected && root.activeFocus
        }

        Accessible.role: root.isTree ? Accessible.TreeItem : Accessible.ListItem
        Accessible.name: row.entry.label
        Accessible.selected: row.selected
        Accessible.selectable: true
        Accessible.focusable: true
        Accessible.onPressAction: root.activateIndex(row.entry.index)
    }

    QtObject {
        id: hoverTracker
        property int index: -1
    }
}
