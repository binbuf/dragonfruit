// SPDX-License-Identifier: GPL-3.0-only
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
    // True while `activateIndex()` moves the selection to the row it is about
    // to activate. An owner that mirrors the selection in
    // `onCurrentIndexChanged` can use this to avoid running the same
    // navigation twice (once for the index move, once for `activated`).
    property bool activating: false
    // T206: the app-local material fill. The default is the existing opaque
    // source-list surface; first-party apps pass the resolved glass tint/alpha
    // and round the bottom-left window corner.
    property color fillColor: Theme.color.surfaceMuted
    property real bottomLeftRadius: 0
    // T286: the Settings app opts into the mockup's density and neutral
    // selection without changing the shared defaults. Other apps keep the
    // 28 pt accent-selected source list.
    property real rowHeight: Theme.controls.sidebar.rowHeight
    property real rowRadius: Theme.controls.sidebar.rowRadius
    property real padding: Theme.controls.sidebar.padding
    property real sectionSpacer: Theme.controls.sidebar.sectionGap
    // "accent" (default) or "neutral" (System Settings).
    property string selectionStyle: "accent"
    property color neutralSelectionFill: Theme.color.controlActive
    // T286: a selected row is kept inside the viewport when the list scrolls.
    property bool scrollSelectedIntoView: false
    // T227: `appearance.sidebarIconSize` (small/medium/large) is mapped by the
    // app owner onto these two slots; the row height stays the design token.
    property real iconSize: Theme.controls.sidebar.iconSize
    property real categoryIconSize: Theme.controls.sidebar.categoryIconSize
    // T267: RTL text alignment. The row geometry mirrors through the shell's
    // LayoutMirroring; this only fixes the physical text alignment, which Qt
    // does not mirror automatically.
    property bool rtl: false

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
                    // T286: the update row is a real status entry: selectable,
                    // with a badge and a status line.
                    type: item.spacer === true ? "spacer"
                          : item.status === true ? "status" : "item",
                    id: item.id || "",
                    label: item.label || "",
                    statusLabel: item.statusLabel || "",
                    icon: item.icon || "",
                    badge: item.badge || "",
                    category: item.category || null,
                    section: s,
                    itemIndex: i
                });
            }
        }
        return out;
    }
    readonly property var currentItem: {
        var e = root.entries[root.currentIndex];
        return e && (e.type === "item" || e.type === "status") ? e : null;
    }
    // Category tiles are wider than a plain line glyph. If any item carries a
    // category, every item keeps that tile-width slot so labels stay in one
    // column; items without a mapping center their plain glyph in it.
    readonly property bool hasCategoryItems: {
        for (var i = 0; i < root.entries.length; ++i) {
            if (root.entries[i].type === "item" && root.entries[i].category)
                return true;
        }
        return false;
    }

    implicitWidth: Theme.controls.sidebar.width
    implicitHeight: 320
    activeFocusOnTab: true
    // T263: the grouped source list can exceed a short window. The list is a
    // Flickable; expose its content/viewport so the shell's size cases can
    // assert the rows scroll instead of overflowing the card.
    readonly property real contentHeight: list.contentHeight
    readonly property real viewportHeight: list.height
    readonly property bool scrollable: list.contentHeight > list.height
    property alias flickable: list

    function move(delta) {
        var count = root.entries.length;
        if (count === 0)
            return;
        var i = root.currentIndex;
        for (var step = 0; step < count; ++step) {
            i = (i + delta + count) % count;
            if (root.entries[i].type === "item"
                    || root.entries[i].type === "status") {
                root.currentIndex = i;
                return;
            }
        }
    }

    // T286: keep the selected row inside the viewport. The list is a Column in
    // a Flickable, so the row's y is already the content offset.
    function ensureVisible(index) {
        if (!root.scrollSelectedIntoView)
            return;
        var row = root.rowAtIndex(index);
        if (row === null || row === undefined)
            return;
        var top = row.y;
        var bottom = row.y + row.height;
        if (top < list.contentY)
            list.contentY = top;
        else if (bottom > list.contentY + list.height)
            list.contentY = bottom - list.height;
    }

    onCurrentIndexChanged: root.ensureVisible(root.currentIndex)

    // T263: the rendered row delegate for an entry index. The stationary-click
    // regression uses it to click a row's own coordinates without a prior
    // pointer motion.
    function rowAtIndex(index) {
        return repeater.itemAt(index);
    }

    function activateIndex(index) {
        var e = root.entries[index];
        if (!e || (e.type !== "item" && e.type !== "status"))
            return;
        root.activating = true;
        root.currentIndex = index;
        root.activating = false;
        root.activated(index, e);
    }

    ChromeSheet {
        anchors.fill: parent
        color: root.fillColor
        bottomLeftRadius: root.bottomLeftRadius
    }
    Rectangle {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.controls.window.borderWidth
        color: Glass.increaseContrast ? Theme.color.textPrimary
                                      : Theme.color.border
    }

    Flickable {
        id: list
        anchors.fill: parent
        anchors.margins: root.padding
        clip: true
        contentWidth: width
        contentHeight: rows.implicitHeight
        boundsBehavior: Flickable.StopAtBounds

        Column {
            id: rows
            width: list.width

            Repeater {
                id: repeater
                model: root.entries
                delegate: SidebarRow {
                    required property var modelData
                    entry: modelData
                }
            }
        }
    }

    Keys.onPressed: (event) => {
        var action = Keymap.actionFor("sidebar", event.key,
                                      event.modifiers);
        switch (action) {
        case "next":
            root.move(1);
            event.accepted = true;
            break;
        case "previous":
            root.move(-1);
            event.accepted = true;
            break;
        case "first":
            root.currentIndex = -1;
            root.move(1);
            event.accepted = true;
            break;
        case "last":
            root.currentIndex = root.entries.length;
            root.move(-1);
            event.accepted = true;
            break;
        case "activate":
            root.activateIndex(root.currentIndex);
            event.accepted = true;
            break;
        }
    }

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Sidebar")
    Accessible.description: qsTr("System Settings panes")

    component SidebarRow: Item {
        id: row

        required property var entry
        readonly property bool isHeader: entry.type === "header"
        // T285: an inter-group spacer is not selectable and not announced; the
        // keyboard path skips it because `move()` only lands on items.
        readonly property bool isSpacer: entry.type === "spacer"
        // T286: the update row is a status entry, not a pane. It is selectable
        // and keyboard-reachable like an item but renders the status anatomy.
        readonly property bool isStatus: entry.type === "status"
        readonly property bool isRow: !row.isHeader && !row.isSpacer
        readonly property bool selected: root.currentIndex === entry.index
        readonly property bool hovered: rowHover.hovered
        // T286: the Settings app opts into the neutral selection fill; the
        // shared default keeps the accent selection for other apps.
        readonly property color selectionFill:
            root.selectionStyle === "neutral" ? root.neutralSelectionFill
                                              : Theme.color.accent
        readonly property color selectedContent:
            root.selectionStyle === "neutral" ? Theme.color.textPrimary
                                              : Theme.color.accentContent
        // A resolved `SettingsCategoryIcon` style, when the item carries one.
        // Items without it keep the plain `Icon` glyph (T-19.1b).
        readonly property var category: entry.category && entry.category.source
                ? entry.category : null
        // The resolved Phosphor name, kept out of the `SettingsCategoryIcon`
        // block so the static glyph-reference scan only sees a bound value.
        readonly property string categoryGlyph: row.category ? row.category.source : ""
        readonly property bool hasGlyph: row.isRow && !row.isStatus
                && (row.category !== null || entry.icon.length > 0)
        readonly property real glyphSize: row.category !== null
                ? root.categoryIconSize
                : root.iconSize
        // The space reserved in front of the label. A mixed list (a category
        // tile beside a plain fallback glyph) keeps one label column.
        readonly property real glyphSlot: row.category !== null || root.hasCategoryItems
                ? root.categoryIconSize
                : root.iconSize
        readonly property string sectionTitle: {
            var section = root.sections[row.entry.section];
            return section && section.title ? section.title : "";
        }

        width: parent ? parent.width : 0
        height: isHeader ? headerText.implicitHeight + root.sectionSpacer
                         : isSpacer ? root.sectionSpacer
                                    : root.rowHeight

        Text {
            id: headerText
            objectName: "sidebarHeaderText"
            visible: row.isHeader
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.leftMargin: root.padding
            text: row.entry.label
            color: Glass.increaseContrast ? Theme.color.textPrimary
                                          : Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeXs
            font.weight: Theme.primitive.font.weightSemibold
            font.capitalization: Font.AllUppercase
            elide: Text.ElideRight
            horizontalAlignment: root.rtl ? Text.AlignRight : Text.AlignLeft
        }

        // Hover fill (animated). The selection fill is a second, instant
        // layer: a freshly opened window must not paint a half-faded
        // selection highlight (the hover animation can stall while the client
        // is idle and no frames are delivered).
        Rectangle {
            visible: row.isRow
            anchors.fill: parent
            radius: root.rowRadius
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
            visible: row.isRow
            anchors.fill: parent
            radius: root.rowRadius
            color: row.selected ? row.selectionFill : "transparent"
            antialiasing: true
        }

        // The update status row keeps its own anatomy (tile, label, status
        // line, badge); it never goes through the plain pane-row path.
        SettingsSidebarStatusRow {
            id: statusRow
            visible: row.isStatus
            anchors.fill: parent
            label: row.entry.label
            statusLabel: row.entry.statusLabel
            icon: row.entry.icon
            badge: row.entry.badge
            category: row.category
            selected: row.selected
            rtl: root.rtl
        }

        Item {
            id: rowGlyph
            visible: row.hasGlyph
            width: row.glyphSlot
            height: row.glyphSlot
            anchors.left: parent.left
            anchors.leftMargin: root.padding
            anchors.verticalCenter: parent.verticalCenter

            SettingsCategoryIcon {
                anchors.fill: parent
                visible: row.category !== null
                source: row.categoryGlyph
                gradientStart: row.category ? row.category.gradientStart : Theme.color.accent
                gradientEnd: row.category ? row.category.gradientEnd : Theme.color.accent
            }

            Icon {
                id: rowIcon
                anchors.centerIn: parent
                visible: row.category === null
                name: row.category === null ? row.entry.icon : ""
                size: row.glyphSize
                color: row.selected ? row.selectedContent : Theme.color.textSecondary
            }
        }

        Text {
            id: rowLabel
            objectName: "sidebarRowLabel"
            visible: row.isRow && !row.isStatus
            anchors.left: rowGlyph.visible ? rowGlyph.right : parent.left
            anchors.leftMargin: root.padding
            anchors.right: badge.visible ? badge.left : parent.right
            anchors.rightMargin: root.padding
            anchors.verticalCenter: parent.verticalCenter
            text: row.entry.label
            color: row.selected ? row.selectedContent : Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.weight: row.selected && root.selectionStyle === "neutral"
                         ? Theme.primitive.font.weightSemibold
                         : Theme.primitive.font.weightRegular
            elide: Text.ElideRight
            horizontalAlignment: root.rtl ? Text.AlignRight : Text.AlignLeft
        }

        Rectangle {
            id: badge
            visible: row.isRow && !row.isStatus && row.entry.badge.length > 0
            anchors.right: parent.right
            anchors.rightMargin: root.padding
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
                if (hovered && row.isRow)
                    root.hoveredIndex = row.entry.index;
                else if (root.hoveredIndex === row.entry.index)
                    root.hoveredIndex = -1;
            }
        }

        TapHandler {
            onTapped: {
                if (row.isRow)
                    root.activateIndex(row.entry.index);
            }
        }

        FocusRing {
            visible: row.isRow
            target: row
            cornerRadius: root.rowRadius
            shown: row.selected && root.activeFocus
        }

        Accessible.ignored: row.isSpacer
        Accessible.role: row.isHeader ? Accessible.StaticText : Accessible.ListItem
        Accessible.name: row.entry.label
        Accessible.description: row.isHeader || row.isSpacer ? ""
                                : (row.sectionTitle.length > 0
                                   ? row.sectionTitle
                                   : root.sections[row.entry.section].accessibleTitle || qsTr("Pane"))
        Accessible.selected: row.isRow && row.selected
        Accessible.selectable: row.isRow
        Accessible.focusable: row.isRow
        Accessible.onPressAction: {
            if (row.isRow)
                root.activateIndex(row.entry.index);
        }
    }

    QtObject {
        id: hoverTracker
        property int index: -1
    }
}
