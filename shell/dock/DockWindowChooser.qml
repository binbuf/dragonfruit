// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Dock's window chooser (T-10 section 9): an overlay popover anchored to
// an app entry, listing that app's windows across all Spaces, most-recent
// first. The first row is "Show All Windows"; each window row carries a
// checkmark when frontmost, a minimized marker, and its Space name.
//
// T-14.7m adds per-window actions: a stateful **Minimize / Restore** button
// and a destructive **Close** button, revealed on row hover. The buttons ask;
// they never mutate `windowList`. The shell performs the compositor
// round-trip and the next projection refreshes the rows, so a close drops its
// row and a minimize flips the button label while the chooser stays open.
//
// The shell renders this into the Dock's `overlay` chrome surface. The
// component owns presentation only: the Dock feeds it `entry` (the app entry
// with its `windowList`) and relays `windowActivated`/`windowCloseRequested`/
// `windowMinimizeRequested`/`showAllWindows` to the shell, which performs the
// compositor round-trips.
Item {
    id: root

    property var entry: null
    property Item anchorItem: null
    property bool open: false
    // Capture/demo seam (T-14.7m): force the hover treatment onto the row at
    // this index so the live visual check can show the revealed actions
    // without a synthetic pointer. Never set in a normal session.
    property int fixtureHoverIndex: -1

    signal windowActivated(string windowId)
    signal windowCloseRequested(string windowId)
    signal windowMinimizeRequested(string windowId, bool minimized)
    signal showAllWindows()
    signal opened()
    signal closed()

    // The per-row action buttons (T-14.7m). A square target derived from the
    // row height keeps them inside the row without growing the popover; the
    // reserved slot keeps the layout stable while they are hidden.
    readonly property real actionButtonSize: Math.round(rowHeight * 0.72)
    readonly property real actionGap: Theme.primitive.spacing.xxs

    readonly property var windows:
        entry && entry.windowList !== undefined ? entry.windowList : []
    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real arrowSize: Theme.controls.popover.arrowSize
    readonly property real radius: Theme.controls.contextMenu.radius

    function hide() { root.open = false; }
    function activateWindow(index) {
        var row = root.windows[index];
        if (row && row.windowId !== undefined) {
            root.windowActivated(row.windowId);
            root.hide();
        }
    }
    function activateShowAll() {
        root.showAllWindows();
        root.hide();
    }
    // The row's stateful minimize/restore action: it asks for the opposite of
    // the window's current state and leaves the chooser open (T-14.7m).
    function minimizeWindow(index) {
        var row = root.windows[index];
        if (!row || row.windowId === undefined)
            return;
        root.windowMinimizeRequested(row.windowId, row.minimized !== true);
    }
    // Close one window; the chooser stays open and its row disappears with the
    // next projection (T-14.7m).
    function closeWindow(index) {
        var row = root.windows[index];
        if (!row || row.windowId === undefined)
            return;
        root.windowCloseRequested(row.windowId);
    }
    // The row's accessible name keeps its active/minimized state (T-14.7m).
    function windowAccessibleName(title, focused, minimized) {
        var name = title !== undefined ? title : "";
        if (focused === true)
            name += " " + qsTr("(active)");
        if (minimized === true)
            name += " " + qsTr("(minimized)");
        return name;
    }

    onOpenChanged: {
        if (root.open)
            root.opened();
        else
            root.closed();
    }

    width: Math.max(Theme.controls.contextMenu.minWidth,
                    rows.implicitWidth + 2 * root.padding)
    height: root.padding + headerRow.height + separator.height + rows.height
            + root.padding + root.arrowSize / 2
    visible: opacity > 0
    scale: root.open ? 1.0 : 0.97
    transformOrigin: Item.Bottom
    opacity: root.open ? 1.0 : 0.0
    z: 2000

    Behavior on opacity {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve
                                          : Theme.motion.popupClose.curve
        }
    }
    Behavior on scale {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve
                                          : Theme.motion.popupClose.curve
        }
    }

    // An icon-only, keyboard-inert per-row action (T-14.7m). It carries an
    // accessible name that includes the window title, never takes focus (so
    // the chooser keeps it), and tints destructively for Close. The reference
    // is behavior only; the geometry is our own (ADR 0103 clean-room note).
    component ChooserActionButton: Item {
        id: action

        property string glyph: ""
        property bool destructive: false
        property bool revealed: false
        property string accessibleLabel: ""

        signal activated()

        width: 20
        height: 20
        visible: revealed
        // Never a tab stop: the chooser owns the keyboard (T-14.7m).
        activeFocusOnTab: false

        readonly property color contentColor:
            destructive ? Theme.color.danger : Theme.color.textSecondary

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.button.radius
            color: action.destructive
                   ? Qt.alpha(Theme.color.danger,
                              (actionHover.hovered || actionTap.pressed) ? 0.22 : 0.0)
                   : ((actionHover.hovered || actionTap.pressed)
                      ? Theme.color.controlHover : "transparent")
            border.width: action.activeFocus ? Theme.controls.focusRing.width : 0
            border.color: Theme.color.focusRing
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
            anchors.centerIn: parent
            name: action.glyph
            size: Math.round(action.width * 0.66)
            color: action.contentColor
        }
        HoverHandler { id: actionHover }
        TapHandler { id: actionTap; onTapped: action.activated() }
        Accessible.role: Accessible.Button
        Accessible.name: action.accessibleLabel
        Accessible.onPressAction: action.activated()
    }

    Shadow {
        width: root.width
        height: root.height
        radius: root.radius
        blur: Theme.controls.popup.shadowBlur
    }

    Rectangle {
        objectName: "chooserSurface"
        anchors.fill: parent
        anchors.bottomMargin: root.arrowSize / 2
        radius: root.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    // The arrow ties the popover to its entry (points down at the Dock).
    Rectangle {
        objectName: "chooserArrow"
        width: root.arrowSize
        height: root.arrowSize
        rotation: 45
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        x: {
            if (!root.anchorItem || !root.parent)
                return root.width / 2 - width / 2;
            var p = root.anchorItem.mapToItem(root.parent, root.anchorItem.width / 2, 0);
            return Math.max(root.radius,
                            Math.min(root.width - root.radius, p.x - root.x)) - width / 2;
        }
        y: root.height - height
        z: -1
    }

    Item {
        id: headerRow
        objectName: "chooserHeader"
        x: root.padding
        y: root.padding
        width: root.width - 2 * root.padding
        height: root.rowHeight

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.focusRing.radius
            color: headerHover.hovered ? Theme.color.controlFill : "transparent"
        }
        Text {
            id: headerText
            text: qsTr("Show All Windows")
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
        }
        HoverHandler { id: headerHover }
        TapHandler { onTapped: root.activateShowAll() }
        Accessible.role: Accessible.MenuItem
        Accessible.name: qsTr("Show All Windows")
        Accessible.onPressAction: root.activateShowAll()
    }

    Rectangle {
        id: separator
        x: root.padding
        y: headerRow.y + headerRow.height
        width: root.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
        visible: root.windows.length > 0
    }

    Column {
        id: rows
        objectName: "chooserRows"
        x: root.padding
        y: separator.y + separator.height
        width: root.width - 2 * root.padding

        Repeater {
            model: root.windows

            delegate: Item {
                id: row
                required property var modelData
                required property int index

                width: rows.width
                height: root.rowHeight
                readonly property bool focused: modelData.focused === true
                readonly property bool minimized: modelData.minimized === true
                // `rowHover.hovered` is the live pointer; the fixture index
                // forces it for the capture seam.
                readonly property bool hovered:
                    rowHover.hovered || row.index === root.fixtureHoverIndex
                readonly property string windowTitle:
                    modelData.title !== undefined ? modelData.title : ""
                readonly property string spaceName:
                    modelData.workspaceName !== undefined ? modelData.workspaceName : ""
                // The two actions reveal together on hover (or row focus); the
                // slot below always reserves their footprint (T-14.7m).
                readonly property bool showActions: row.hovered || row.activeFocus

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.controls.focusRing.radius
                    color: row.hovered ? Theme.color.accent : "transparent"
                }
                Icon {
                    id: check
                    visible: row.focused
                    name: "check"
                    size: Theme.controls.button.fontSize
                    color: row.hovered ? Theme.color.accentContent : Theme.color.accent
                    x: Theme.controls.contextMenu.padding
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    id: title
                    text: row.windowTitle
                    color: row.hovered ? Theme.color.accentContent : Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    elide: Text.ElideRight
                    anchors.left: check.visible ? check.right : parent.left
                    anchors.leftMargin: check.visible ? Theme.primitive.spacing.sm
                                                      : Theme.controls.contextMenu.padding
                    anchors.right: space.left
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    id: space
                    text: (row.minimized ? qsTr("minimized") + (row.spaceName.length > 0 ? " · " : "")
                                         : "")
                          + row.spaceName
                    visible: text.length > 0
                    color: row.hovered ? Theme.color.accentContent : Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    anchors.right: actionSlot.left
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                }
                // The reserved action slot. It always occupies the same width,
                // so revealing the buttons never resizes the popover; a click
                // on the empty slot falls through to the row's activate tap.
                Item {
                    id: actionSlot
                    objectName: "chooserRowActions"
                    width: 2 * root.actionButtonSize + root.actionGap
                    height: parent.height
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.controls.contextMenu.padding

                    ChooserActionButton {
                        id: minimizeAction
                        objectName: "chooserMinimizeAction"
                        anchors.right: closeAction.left
                        anchors.rightMargin: root.actionGap
                        anchors.verticalCenter: parent.verticalCenter
                        width: root.actionButtonSize
                        height: root.actionButtonSize
                        glyph: row.minimized ? "restore" : "minimize"
                        revealed: row.showActions
                        accessibleLabel: (row.minimized ? qsTr("Restore ")
                                                        : qsTr("Minimize "))
                                         + row.windowTitle
                        onActivated: root.minimizeWindow(row.index)
                    }
                    ChooserActionButton {
                        id: closeAction
                        objectName: "chooserCloseAction"
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: root.actionButtonSize
                        height: root.actionButtonSize
                        glyph: "close"
                        destructive: true
                        revealed: row.showActions
                        accessibleLabel: qsTr("Close ") + row.windowTitle
                        onActivated: root.closeWindow(row.index)
                    }
                }
                HoverHandler { id: rowHover }
                TapHandler { onTapped: root.activateWindow(row.index) }
                Accessible.role: Accessible.MenuItem
                Accessible.name: root.windowAccessibleName(row.windowTitle, row.focused,
                                                           row.minimized)
                Accessible.checkable: true
                Accessible.checked: row.focused
                Accessible.onPressAction: root.activateWindow(row.index)
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.hide();
        event.accepted = true;
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: root.entry && root.entry.name !== undefined ? root.entry.name : ""
}