// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A single-choice value popup: the current label with a trailing chevron that
// opens a ContextMenu of the other choices (T-09.4). This is the settings-pane
// "popup" row control (`Bottom`, `Genie Effect`, `Zoom`).
//
// The owner controls the selections: the component only reports intent through
// `activated(index)` / `selected(value)` and re-reads `currentIndex` when the
// owner changes it, so the T-09.1b "write on interaction, bind to the settings
// map" pattern holds. Keyboard: Space/Enter/Down opens, the menu owns the
// arrows, Escape closes. AT-SPI exposes a ComboBox role.
Item {
    id: root

    property var model: []
    property int currentIndex: 0
    property string accessibleName: ""
    property alias hovered: selectHover.hovered
    property alias menu: menu

    signal activated(int index)
    signal selected(var value)

    readonly property var entries: {
        var out = [];
        for (var i = 0; i < root.model.length; ++i) {
            var m = root.model[i];
            if (typeof m === "string")
                out.push({ label: m, value: m, enabled: true });
            else
                out.push({ label: m.label || "", value: (m.value !== undefined ? m.value : i),
                           enabled: m.enabled !== false });
        }
        return out;
    }
    readonly property var currentEntry: (root.currentIndex >= 0
                                         && root.currentIndex < root.entries.length)
                                         ? root.entries[root.currentIndex] : null
    readonly property string currentLabel: root.currentEntry ? root.currentEntry.label : ""
    readonly property var currentValue: root.currentEntry ? root.currentEntry.value : undefined

    implicitWidth: Math.max(Theme.controls.select.minWidth,
                            valueText.implicitWidth + Theme.controls.select.chevronSize
                            + Theme.controls.select.chevronGap
                            + 2 * Theme.controls.select.paddingH)
    implicitHeight: Theme.controls.select.height
    activeFocusOnTab: true
    opacity: enabled ? 1.0 : 0.45

    function openMenu() {
        if (!root.enabled)
            return;
        menu.showAt(0, root.height + Theme.controls.popup.offset);
    }

    function closeMenu() {
        menu.hide();
    }

    function activateIndex(index) {
        var e = root.entries[index];
        if (!e || !e.enabled)
            return;
        root.currentIndex = index;
        root.activated(index);
        root.selected(e.value);
        menu.hide();
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
        text: root.currentLabel
        color: Theme.color.textPrimary
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

    ContextMenu {
        id: menu
        x: 0
        y: root.height + Theme.controls.popup.offset
        preferredWidth: root.width
        accessibleName: root.accessibleName
        model: root.entries.map(function(entry) {
            return { label: entry.label, enabled: entry.enabled,
                     checked: entry.value === root.currentValue, checkable: true };
        })
        onTriggered: (index, item) => root.activateIndex(index)
        onClosed: if (root.enabled) root.forceActiveFocus()
    }

    HoverHandler { id: selectHover }

    TapHandler {
        onTapped: {
            if (menu.open)
                root.closeMenu();
            else
                root.openMenu();
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
            root.openMenu();
            event.accepted = true;
            break;
        }
    }

    Accessible.role: Accessible.ComboBox
    Accessible.name: root.accessibleName
    Accessible.focusable: true
    Accessible.onPressAction: root.openMenu()
}