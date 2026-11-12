// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Dock's window chooser (T-10 section 9): an overlay popover anchored to
// an app entry, listing that app's windows across all Spaces, most-recent
// first. The first row is "Show Overview"; each window row carries a
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
// `windowMinimizeRequested`/`showOverview` to the shell, which performs the
// compositor round-trips.
//
// T-14.7n bounds the row list: at most `maxRows` rows are visible
// (`component.dock.chooser.maxRows`), a longer list scrolls through the
// design-system `ScrollView` (the stack popover's proven viewport idiom), and
// the "Show Overview" header and separator stay pinned above it. The
// popover therefore never grows past its pre-sized headroom (ADR 0100), and
// every row (and its T-14.7m actions) stays reachable by scroll or keyboard.
FocusScope {
    id: root

    property var entry: null
    property Item anchorItem: null
    // The rectangle, in parent coordinates, the chooser may occupy. The Dock
    // passes the intersection of the output area and its render budget so the
    // bounded list never exceeds the committed overlay.
    property var availableRect: null
    property bool open: false
    // T-21.2c: the Dock applies the compositor's Popup role state here.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)
    // The visible-row cap; a longer window list scrolls through the rest. The
    // default is the token, not a literal.
    property int maxRows: Theme.controls.dock.chooser.maxRows
    // The keyboard-highlighted row; -1 is the pinned "Show Overview"
    // header action. It keeps the focused row scrolled into the viewport.
    property int currentIndex: -1
    // The stable identity behind `currentIndex`. A projection can reorder the
    // window rows (the focused window leads) or drop a window; following the
    // ID keeps Return on the window the user selected instead of whatever row
    // later occupies the same number.
    property string currentWindowId: ""
    // The row order shown while the chooser is open, keyed by window ID. The
    // projection puts the focused window first, so a focus-only update would
    // otherwise shuffle the rows under the pointer. Each row still reads the
    // current projection object, so the focused and minimized markers update
    // live.
    property var windowOrder: []
    // Capture/demo seam (T-14.7m): force the hover treatment onto the row at
    // this index so the live visual check can show the revealed actions
    // without a synthetic pointer. Never set in a normal session.
    property int fixtureHoverIndex: -1

    signal windowActivated(string windowId)
    signal windowCloseRequested(string windowId)
    signal windowMinimizeRequested(string windowId, bool minimized)
    signal showOverview()
    signal opened()
    signal closed()

    // The per-row action buttons (T-14.7m). A square target derived from the
    // row height keeps them inside the row without growing the popover; the
    // reserved slot keeps the layout stable while they are hidden.
    readonly property real actionButtonSize: Math.round(rowHeight * 0.72)
    readonly property real actionGap: Theme.primitive.spacing.xxs

    // The live rows from the projection. The projection orders the focused
    // window first, so this list changes order on an ordinary focus change.
    readonly property var liveWindows:
        entry && entry.windowList !== undefined && entry.windowList !== null
            ? entry.windowList : []
    // The rows in the order the user last saw them while the chooser is open.
    // A window that appears later joins at the end; a window that disappears
    // drops out. The live objects are reused, so focused/minimized markers
    // still track the latest projection.
    readonly property var windows: {
        var live = root.liveWindows;
        if (!root.open || root.windowOrder.length === 0)
            return live;
        var byId = ({});
        for (var i = 0; i < live.length; ++i) {
            var row = live[i];
            if (!row || row.windowId === undefined || row.windowId === null)
                return live;
            byId[String(row.windowId)] = row;
        }
        var out = [];
        var used = ({});
        for (var j = 0; j < root.windowOrder.length; ++j) {
            var id = root.windowOrder[j];
            if (byId[id] !== undefined && used[id] !== true) {
                out.push(byId[id]);
                used[id] = true;
            }
        }
        for (var k = 0; k < live.length; ++k) {
            var liveRow = live[k];
            var liveId = String(liveRow.windowId);
            if (used[liveId] !== true) {
                out.push(liveRow);
                used[liveId] = true;
            }
        }
        return out;
    }
    readonly property int rowCount: windows.length
    readonly property int visibleRows: Math.min(rowCount, maxRows)
    readonly property real availableWidth:
        availableRect && availableRect.w > 0 ? availableRect.w : Number.MAX_VALUE
    readonly property real availableHeight:
        availableRect && availableRect.h > 0 ? availableRect.h : Number.MAX_VALUE
    // The fixed header, separator, padding, and arrow footprint. The list
    // takes whatever is left inside the available rectangle.
    readonly property real chromeHeight:
        padding + rowHeight + separator.height + padding + arrowSize / 2
    readonly property real maxListHeight:
        availableHeight > 0 ? Math.max(0, availableHeight - chromeHeight)
                            : Number.MAX_VALUE
    readonly property real listHeight:
        Math.min(visibleRows * rowHeight, maxListHeight)
    readonly property bool scrolls: rowCount * rowHeight > listHeight + 0.5

    // A projection can reorder, close, or minimize windows while the chooser
    // is open. Follow the selected window ID rather than its row number; a
    // stale index would make Return act on a row the user is not looking at.
    onLiveWindowsChanged: root.syncWindowOrder()
    onWindowsChanged: root.reconcileSelection()
    // The scrollbar overlays the viewport's right edge; a scrolling list
    // reserves this gutter to its right so the T-14.7m actions never sit under
    // the thumb. A short list reserves nothing (no scrollbar, unchanged rows).
    readonly property real scrollbarGutter: scrolls
        ? Theme.controls.dock.chooser.scrollbarWidth
          + Theme.controls.dock.chooser.scrollbarMargin
        : 0

    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real arrowSize: Theme.controls.popover.arrowSize
    readonly property real radius: Theme.controls.contextMenu.radius

    function hide() { root.open = false; }
    function windowIdAt(index) {
        var row = root.windows[index];
        return row && row.windowId !== undefined && row.windowId !== null
               ? String(row.windowId) : "";
    }
    function indexOfWindowId(windowId) {
        if (windowId === "")
            return -1;
        for (var i = 0; i < root.windows.length; ++i) {
            if (root.windowIdAt(i) === windowId)
                return i;
        }
        return -1;
    }
    // The nearest row that survived a projection update. Window rows are all
    // enabled, so the closest in-range position is the answer; -1 keeps the
    // pinned header when no window rows remain.
    function nearestWindowIndex(preferred) {
        if (root.rowCount === 0)
            return -1;
        return Math.max(0, Math.min(root.rowCount - 1, preferred));
    }
    function setCurrentIndex(index) {
        var next = index < 0 ? -1
                             : Math.max(0, Math.min(root.rowCount - 1, index));
        if (root.currentIndex !== next)
            root.currentIndex = next;
        root.currentWindowId = next >= 0 ? root.windowIdAt(next) : "";
        if (next >= 0)
            root.keepSelectionVisible();
    }
    // Follow the selected window ID across a projection update. When it
    // disappeared, move to the nearest surviving row (or the header) and let
    // the highlight and accessible state move with it.
    function reconcileSelection() {
        var previous = root.currentIndex;
        var id = root.currentWindowId;
        if (id === "" && previous >= 0)
            id = root.windowIdAt(previous);
        if (id === "" && previous < 0) {
            root.setCurrentIndex(-1);
            return;
        }
        var index = root.indexOfWindowId(id);
        if (index < 0)
            index = root.nearestWindowIndex(previous);
        root.setCurrentIndex(index);
    }
    // Keep the display order stable across a projection update: surviving
    // windows hold their places and new windows join at the end.
    function syncWindowOrder() {
        var live = root.liveWindows;
        var ids = [];
        for (var i = 0; i < live.length; ++i) {
            var row = live[i];
            if (!row || row.windowId === undefined || row.windowId === null)
                continue;
            ids.push(String(row.windowId));
        }
        if (!root.open) {
            root.windowOrder = ids;
            return;
        }
        var present = ({});
        var out = [];
        for (var j = 0; j < root.windowOrder.length; ++j) {
            var id = root.windowOrder[j];
            if (ids.indexOf(id) >= 0 && present[id] !== true) {
                out.push(id);
                present[id] = true;
            }
        }
        for (var k = 0; k < ids.length; ++k) {
            if (present[ids[k]] !== true) {
                out.push(ids[k]);
                present[ids[k]] = true;
            }
        }
        root.windowOrder = out;
    }
    onCurrentIndexChanged: {
        if (root.currentIndex < 0) {
            if (root.currentWindowId !== "")
                root.currentWindowId = "";
            return;
        }
        var id = root.windowIdAt(root.currentIndex);
        if (root.currentWindowId !== id)
            root.currentWindowId = id;
        root.keepSelectionVisible();
    }
    function activateWindow(index) {
        var row = root.windows[index];
        if (row && row.windowId !== undefined && row.windowId !== null)
            root.activateWindowById(String(row.windowId));
    }
    function activateWindowById(windowId) {
        // The target can disappear while a press is armed; never activate
        // whichever window later took its place.
        if (windowId === "" || root.indexOfWindowId(windowId) < 0)
            return;
        root.windowActivated(windowId);
        root.hide();
    }
    function activateShowOverview() {
        root.showOverview();
        root.hide();
    }
    // The row's stateful minimize/restore action: it asks for the opposite of
    // the window's current state and leaves the chooser open (T-14.7m).
    function minimizeWindow(index) {
        var row = root.windows[index];
        if (row && row.windowId !== undefined && row.windowId !== null)
            root.minimizeWindowById(String(row.windowId));
    }
    function minimizeWindowById(windowId) {
        var index = root.indexOfWindowId(windowId);
        if (index < 0)
            return;
        var row = root.windows[index];
        root.windowMinimizeRequested(windowId, row.minimized !== true);
    }
    // Close one window; the chooser stays open and its row disappears with the
    // next projection (T-14.7m).
    function closeWindow(index) {
        var row = root.windows[index];
        if (row && row.windowId !== undefined && row.windowId !== null)
            root.closeWindowById(String(row.windowId));
    }
    function closeWindowById(windowId) {
        if (windowId === "" || root.indexOfWindowId(windowId) < 0)
            return;
        root.windowCloseRequested(windowId);
    }
    // The row's accessible name keeps its active/minimized state (T-14.7m).
    function windowAccessibleName(title, focused, minimized) {
        var name = title !== undefined && title !== null ? title : "";
        if (focused === true)
            name += " " + qsTr("(active)");
        if (minimized === true)
            name += " " + qsTr("(minimized)");
        return name;
    }

    // Keyboard traversal over the pinned header (-1) and every row (T-14.7n).
    function moveSelection(delta) {
        var last = root.rowCount - 1;
        var next = root.currentIndex + delta;
        root.setCurrentIndex(Math.max(-1, Math.min(last, next)));
    }

    function moveSelectionTo(index) {
        root.setCurrentIndex(index);
    }

    // Scroll the highlighted row fully into the bounded viewport, the same
    // math the App picker and stack popover use.
    function keepSelectionVisible() {
        if (root.currentIndex < 0)
            return;
        var flickable = rowsView.flickable;
        if (!flickable)
            return;
        var top = root.currentIndex * root.rowHeight;
        var bottom = top + root.rowHeight;
        if (top < flickable.contentY)
            flickable.contentY = top;
        else if (bottom > flickable.contentY + flickable.height)
            flickable.contentY = Math.min(bottom - flickable.height,
                                          Math.max(0, flickable.contentHeight - flickable.height));
    }

    // Return activates the highlighted thing: the header action (-1) or the
    // highlighted row. Activating a row closes the chooser, matching a click.
    function activateCurrent() {
        if (root.currentIndex < 0) {
            root.activateShowOverview();
            return;
        }
        // Resolve the selected identity against the rows that exist now, not
        // the index captured when the highlight moved.
        var index = root.indexOfWindowId(root.currentWindowId);
        if (index < 0) {
            root.reconcileSelection();
            index = root.indexOfWindowId(root.currentWindowId);
        }
        if (index >= 0)
            root.activateWindow(index);
    }

    // Capture/demo seam (T-14.7n): scroll the bounded viewport to a row so the
    // live visual check can show a long list mid-scroll. Never set in a normal
    // session.
    function scrollToRow(index) {
        var flickable = rowsView.flickable;
        if (!flickable)
            return;
        flickable.contentY = Math.max(0, Math.min(index * root.rowHeight,
            Math.max(0, flickable.contentHeight - flickable.height)));
    }

    onOpenChanged: {
        if (root.open) {
            root.syncWindowOrder();
            root.setCurrentIndex(-1);
            if (rowsView.flickable)
                rowsView.flickable.contentY = 0;
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }

    width: Math.min(Math.max(Theme.controls.contextMenu.minWidth,
                             rowsContent.implicitWidth + 2 * root.padding),
                    root.availableWidth)
    height: Math.min(root.chromeHeight + root.listHeight, root.availableHeight)
    // Logical close retires input immediately; the opacity/scale fade below
    // only finishes the visual transition (reliability §10). A closing popover
    // must not stay enabled in the same scene that receives input for the one
    // replacing it.
    enabled: root.open
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
        // The window this action belongs to. The press remembers it so a
        // projection that removes or replaces the row cannot retarget the
        // release onto a different window.
        property string targetId: ""
        property string pressedTargetId: ""
        // The verified identity for the current emission. Empty for a direct
        // `activated()` call from a fixture, which falls back to `targetId`.
        property string activationTargetId: ""

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
        TapHandler {
            id: actionTap
            onPressedChanged: if (pressed)
                                  action.pressedTargetId = action.targetId
            onTapped: {
                var target = action.pressedTargetId !== ""
                             ? action.pressedTargetId : action.targetId;
                action.pressedTargetId = "";
                // A target that disappeared during the press cancels the
                // action instead of retargeting it onto the row that took its
                // place.
                if (target !== "" && root.indexOfWindowId(target) < 0)
                    return;
                action.activationTargetId = target;
                action.activated();
                action.activationTargetId = "";
            }
        }
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
        color: Material.menuFill("menuDetached", root.materialBlurActive, root.materialTier, root.materialFallback, true)
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
        color: Material.menuFill("menuDetached", root.materialBlurActive, root.materialTier, root.materialFallback, true)
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
            color: root.currentIndex === -1 ? Theme.color.controlActive
                 : (headerHover.hovered ? Theme.color.controlFill : "transparent")
        }
        Text {
            id: headerText
            text: qsTr("Show Overview")
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
        }
        HoverHandler { id: headerHover }
        TapHandler { onTapped: root.activateShowOverview() }
        Accessible.role: Accessible.MenuItem
        Accessible.name: qsTr("Show Overview")
        Accessible.selected: root.currentIndex === -1
        Accessible.onPressAction: root.activateShowOverview()
    }

    Rectangle {
        id: separator
        x: root.padding
        y: headerRow.y + headerRow.height
        width: root.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
        visible: root.rowCount > 0
    }

    // The bounded row viewport (T-14.7n). The height is
    // `min(rows, maxRows) * rowHeight`; a longer list flick-scrolls through the
    // design-system ScrollView while the header stays pinned above it. A short
    // list is not interactive and shows no scrollbar or gutter.
    ScrollView {
        id: rowsView
        objectName: "chooserRows"
        x: root.padding
        y: separator.y + separator.height
        width: root.width - 2 * root.padding
        height: root.listHeight
        interactive: root.scrolls
        scrollbarVisible: root.scrolls

        Column {
            id: rowsContent
            width: rowsView.width

            Repeater {
                model: root.windows

                delegate: Item {
                    id: row
                    required property var modelData
                    required property int index

                    width: rowsContent.width
                    height: root.rowHeight
                    readonly property bool focused: modelData.focused === true
                    readonly property bool minimized: modelData.minimized === true
                    readonly property string windowId:
                        modelData.windowId !== undefined && modelData.windowId !== null
                        ? String(modelData.windowId) : ""
                    // `rowHover.hovered` is the live pointer; the fixture index
                    // forces it for the capture seam.
                    readonly property bool hovered:
                        rowHover.hovered || row.index === root.fixtureHoverIndex
                    // The keyboard-highlighted row (T-14.7n).
                    readonly property bool active: row.index === root.currentIndex
                    readonly property string windowTitle:
                        modelData.title !== undefined ? modelData.title : ""
                    readonly property string spaceName:
                        modelData.workspaceName !== undefined ? modelData.workspaceName : ""
                    // The two actions reveal together on hover, row focus, or
                    // keyboard highlight; the slot below always reserves their
                    // footprint (T-14.7m, T-14.7n).
                    readonly property bool showActions:
                        row.hovered || row.activeFocus || row.active

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.controls.focusRing.radius
                        color: row.hovered ? Theme.color.accent
                             : (row.active ? Theme.color.controlActive : "transparent")
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
                    // The reserved action slot. It always occupies the same
                    // width, so revealing the buttons never resizes the popover;
                    // a click on the empty slot falls through to the row's
                    // activate tap. A scrolling list shifts the slot left of the
                    // scrollbar gutter (T-14.7n).
                    Item {
                        id: actionSlot
                        objectName: "chooserRowActions"
                        width: 2 * root.actionButtonSize + root.actionGap
                        height: parent.height
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.contextMenu.padding
                                            + root.scrollbarGutter

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
                            targetId: row.windowId
                            onActivated: root.minimizeWindowById(
                                minimizeAction.activationTargetId !== ""
                                ? minimizeAction.activationTargetId
                                : row.windowId)
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
                            targetId: row.windowId
                            onActivated: root.closeWindowById(
                                closeAction.activationTargetId !== ""
                                ? closeAction.activationTargetId
                                : row.windowId)
                        }
                    }
                    HoverHandler { id: rowHover }
                    TapHandler {
                        id: rowTap
                        property string pressedWindowId: ""
                        onPressedChanged: if (pressed)
                                              rowTap.pressedWindowId = row.windowId
                        onTapped: {
                            var target = rowTap.pressedWindowId !== ""
                                         ? rowTap.pressedWindowId : row.windowId;
                            rowTap.pressedWindowId = "";
                            root.activateWindowById(target);
                        }
                    }
                    Accessible.role: Accessible.MenuItem
                    Accessible.name: root.windowAccessibleName(row.windowTitle, row.focused,
                                                               row.minimized)
                    Accessible.checkable: true
                    Accessible.checked: row.focused
                    Accessible.selected: row.active
                    Accessible.onPressAction: root.activateWindowById(row.windowId)
                }
            }
        }
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Escape) {
            root.hide();
            event.accepted = true;
        } else if (event.key === Qt.Key_Down) {
            root.moveSelection(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Up) {
            root.moveSelection(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Home) {
            root.moveSelectionTo(0);
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            root.moveSelectionTo(root.rowCount - 1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.activateCurrent();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: root.entry && root.entry.name !== undefined ? root.entry.name : ""
}