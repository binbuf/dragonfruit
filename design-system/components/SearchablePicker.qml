// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// A single-choice value control whose option list is searchable (T270).
//
// The anatomy matches `Select` (current label, trailing chevron, row label as
// the accessible name) and adds one thing a long list needs: a search field at
// the top of the popup. The owner stays authoritative: the component reports
// `activated(index)`/`selected(value)` and re-reads `currentIndex`, so the
// "write on interaction, bind to the owner" pattern holds.
//
// The list is bounded and scrolls; the keyboard path is Down/Up to move the
// highlight, Return to choose, Escape to close. Typing filters by a
// case-insensitive substring match over the label, so `berlin` finds
// `Europe/Berlin` without scrolling.
Item {
    id: root

    property var model: []
    property int currentIndex: 0
    property string accessibleName: ""
    // T266: the owning `SettingsRow` supplies the name when the pane does not.
    property string rowLabel: ""
    property string placeholderText: qsTr("Search")
    // The control's own text when no entry is selected (currentIndex < 0).
    property string placeholderLabel: ""
    property string emptyText: qsTr("No matches")
    property alias hovered: pickerHover.hovered
    property alias popup: popup
    property alias field: field
    property alias listView: listView
    // The live query. Exposed for the tests and for a host that wants to show
    // what the user typed.
    readonly property string query: field.text
    property int highlighted: -1

    signal activated(int index)
    signal selected(var value)

    readonly property var entries: {
        var out = [];
        var model = root.model !== undefined && root.model !== null ? root.model : [];
        for (var i = 0; i < model.length; ++i) {
            var m = model[i];
            if (typeof m === "string")
                out.push({ index: i, label: m, value: m, enabled: true });
            else
                out.push({ index: i,
                           label: m.label !== undefined ? String(m.label) : "",
                           value: (m.value !== undefined ? m.value : i),
                           enabled: m.enabled !== false });
        }
        return out;
    }
    // Substring match over the label, case-insensitive. A query that is only
    // whitespace matches everything, so opening the picker never shows an
    // empty list before the user types.
    readonly property var filteredEntries: {
        var query = String(root.query || "").trim().toLocaleLowerCase();
        if (query.length === 0)
            return root.entries;
        var out = [];
        for (var i = 0; i < root.entries.length; ++i) {
            var entry = root.entries[i];
            if (entry.label.toLocaleLowerCase().indexOf(query) >= 0)
                out.push(entry);
        }
        return out;
    }
    readonly property int visibleCount: root.filteredEntries
        ? root.filteredEntries.length : 0
    readonly property var currentEntry: (root.entries !== undefined
                                         && root.currentIndex >= 0
                                         && root.currentIndex < root.entries.length)
                                         ? root.entries[root.currentIndex] : null
    readonly property string currentLabel: root.currentEntry ? root.currentEntry.label : ""
    readonly property var currentValue: root.currentEntry ? root.currentEntry.value : undefined

    implicitWidth: Math.max(Theme.controls.select.minWidth,
                            Math.max(labelMetrics.width, valueText.implicitWidth)
                            + Theme.controls.select.chevronSize
                            + Theme.controls.select.chevronGap
                            + 2 * Theme.controls.select.paddingH)
    implicitHeight: Theme.controls.select.height
    activeFocusOnTab: true
    opacity: enabled ? 1.0 : 0.45

    TextMetrics {
        id: labelMetrics
        font.pixelSize: Theme.controls.button.fontSize
        text: root.currentLabel
    }

    function openPicker() {
        if (!root.enabled)
            return;
        root.highlighted = -1;
        popup.show();
        Qt.callLater(function() {
            if (!popup.open)
                return;
            field.text = "";
            field.forceActiveFocus();
            root.moveHighlight(1);
        });
    }

    function closePicker() {
        popup.hide();
    }

    // Choose a model index (the same call the tests and the Select use).
    function activateIndex(index) {
        var entry = root.entries[index];
        if (!entry || !entry.enabled)
            return;
        root.currentIndex = index;
        root.activated(index);
        root.selected(entry.value);
        popup.hide();
    }

    function activateHighlighted() {
        if (root.highlighted < 0
                || root.highlighted >= root.filteredEntries.length)
            return;
        root.activateIndex(root.filteredEntries[root.highlighted].index);
    }

    function moveHighlight(delta) {
        var count = root.filteredEntries.length;
        if (count === 0) {
            root.highlighted = -1;
            return;
        }
        var i = root.highlighted;
        for (var step = 0; step < count; ++step) {
            i = (i + delta + count) % count;
            if (root.filteredEntries[i].enabled) {
                root.highlighted = i;
                root.keepRowVisible(i);
                return;
            }
        }
    }

    function keepRowVisible(index) {
        var flickable = listView.flickable;
        if (!flickable || flickable.height <= 0)
            return;
        var rowHeight = Theme.controls.contextMenu.rowHeight;
        var top = index * rowHeight;
        if (top < flickable.contentY)
            flickable.contentY = Math.max(0, top);
        else if (top + rowHeight > flickable.contentY + flickable.height)
            flickable.contentY = Math.min(top + rowHeight - flickable.height,
                                          Math.max(0, flickable.contentHeight
                                                      - flickable.height));
    }

    onQueryChanged: {
        root.highlighted = -1;
        root.moveHighlight(1);
    }
    onFilteredEntriesChanged: {
        if (root.highlighted >= root.filteredEntries.length)
            root.highlighted = root.filteredEntries.length > 0 ? 0 : -1;
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.select.radius
        color: root.hovered ? Theme.color.controlHover : Theme.color.controlFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    Text {
        id: valueText
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.select.paddingH
        anchors.right: chevron.left
        anchors.rightMargin: Theme.controls.select.chevronGap
        anchors.verticalCenter: parent.verticalCenter
        text: root.currentLabel.length > 0 ? root.currentLabel : root.placeholderLabel
        color: root.currentLabel.length > 0 ? Theme.color.textPrimary
                                            : Theme.color.textTertiary
        elide: Text.ElideRight
        font.pixelSize: Theme.controls.button.fontSize
    }

    Icon {
        id: chevron
        name: "chevron-down"
        size: Theme.controls.select.chevronSize
        color: Theme.color.textTertiary
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.select.paddingH
        anchors.verticalCenter: parent.verticalCenter
    }

    Popup {
        id: popup
        objectName: "searchablePickerPopup"
        anchorItem: root
        preferredWidth: Math.max(root.width, Theme.controls.contextMenu.minWidth)
        horizontalPadding: Theme.controls.contextMenu.padding
        verticalPadding: Theme.controls.contextMenu.padding
        motionPreset: "menu"
        accessibleRole: Accessible.PopupMenu
        accessibleName: root.accessibleName.length > 0 ? root.accessibleName
                                                       : root.rowLabel
        accessibleDescription: qsTr("%1 options").arg(root.entries.length)
        onClosed: root.highlighted = -1

        Column {
            spacing: Theme.primitive.spacing.xs

            SearchField {
                id: field
                objectName: "searchablePickerField"
                width: parent.width
                placeholderText: root.placeholderText
                onAccepted: root.activateHighlighted()
                onEscapePressed: root.closePicker()
                Keys.onPressed: (event) => {
                    switch (event.key) {
                    case Qt.Key_Down:
                        root.moveHighlight(1);
                        event.accepted = true;
                        break;
                    case Qt.Key_Up:
                        root.moveHighlight(-1);
                        event.accepted = true;
                        break;
                    case Qt.Key_Home:
                        root.highlighted = -1;
                        root.moveHighlight(1);
                        event.accepted = true;
                        break;
                    case Qt.Key_End:
                        root.highlighted = root.filteredEntries.length;
                        root.moveHighlight(-1);
                        event.accepted = true;
                        break;
                    default:
                        break;
                    }
                }
            }

            Text {
                width: parent.width
                visible: root.filteredEntries.length === 0
                text: root.emptyText
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                horizontalAlignment: root.LayoutMirroring.enabled
                                     ? Text.AlignRight : Text.AlignLeft
            }

            ScrollView {
                id: listView
                objectName: "searchablePickerList"
                width: parent.width
                height: Math.min(8 * Theme.controls.contextMenu.rowHeight,
                                 Math.max(Theme.controls.contextMenu.rowHeight,
                                          root.filteredEntries.length
                                          * Theme.controls.contextMenu.rowHeight))
                visible: root.filteredEntries.length > 0
                interactive: root.filteredEntries.length
                             * Theme.controls.contextMenu.rowHeight > height
                scrollbarVisible: interactive

                Column {
                    width: Math.max(0, listView.width
                                    - (listView.scrollbarVisible
                                       ? Theme.controls.scrollView.scrollbarWidth
                                         + Theme.controls.scrollView.scrollbarMargin
                                       : 0))

                    Repeater {
                        model: root.filteredEntries
                        delegate: MenuRow {
                            required property var modelData
                            required property int index
                            entry: {
                                return { index: index,
                                         key: String(modelData.index),
                                         label: modelData.label,
                                         enabled: modelData.enabled,
                                         checked: modelData.index === root.currentIndex };
                            }
                            constrainedWidth: parent.width
                            highlighted: root.highlighted === index
                            onRowHovered: (row) => root.highlighted = row
                            onRowActivated: (row) => root.activateIndex(
                                root.filteredEntries[row].index)
                        }
                    }
                }
            }
        }
    }

    HoverHandler { id: pickerHover }

    TapHandler {
        onTapped: {
            if (popup.open)
                root.closePicker();
            else
                root.openPicker();
        }
    }

    FocusRing {
        target: root
        cornerRadius: Theme.controls.select.radius
        shown: root.activeFocus
    }

    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_Space:
        case Qt.Key_Return:
        case Qt.Key_Enter:
        case Qt.Key_Down:
            root.openPicker();
            event.accepted = true;
            break;
        }
    }

    Accessible.role: Accessible.ComboBox
    Accessible.name: root.accessibleName.length > 0 ? root.accessibleName
                                                     : root.rowLabel
    Accessible.description: qsTr("Current value %1").arg(root.currentLabel)
    Accessible.focusable: true
    Accessible.onPressAction: root.openPicker()
}
