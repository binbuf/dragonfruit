// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Dock (T-10): a persistent chrome surface that projects compositor and
// app state into clickable entries. It owns no authoritative state — the
// shell controller feeds it `entries` (pinned and temporary running apps,
// minimized windows) and it renders/interacts.
//
// This is the presentation + interaction core of the first T-10 slice:
//   * the entry regions (apps | divider | minimized | Trash),
//   * one running indicator per app entry,
//   * progress-based, pointer-anchored magnification,
//   * bottom/left/right placement,
//   * the auto-hide translation.
//
// Context menus, the window chooser, drag rearrangement, launch, the Trash
// state/menu, external drops, and the Downloads stack/recents are landed (see
// PROGRESS.md); the scene-graph render path is a follow-up.
Rectangle {
    id: dock

    // The surface includes the transparent magnified-band above/beside the bar
    // (section 2), so the root must not paint; only `dockBar` and the entries
    // draw. A default-color Rectangle here would render the band as an opaque
    // white strip.
    color: "transparent"
    // The Dock takes active focus only for keyboard navigation so the QML
    // `Keys` handler (section 20) receives key events.
    focus: keyboardFocused

    // --- Injected data --------------------------------------------------
    // [{ id, appId, name, kind, running, minimized, launch, attention,
    //    badge, pinned, trashFull }]
    property var entries: []
    property string position: "bottom"          // bottom | left | right
    property real iconSize: Theme.controls.dock.iconSize
    // The `dock.size` mapping range, read by the shell (section 19).
    readonly property real iconSizeMin: Theme.controls.dock.iconSizeMin
    readonly property real iconSizeMax: Theme.controls.dock.iconSizeMax
    property real magnification: 0.0            // 0..1, 0 = off
    property bool showIndicators: true
    property bool minimizeIntoTileIcon: false
    property bool autoHide: false
    property bool animateOpening: true
    property bool showRecentApps: false
    property bool revealed: true
    property bool trashFull: false
    property int trashCount: 0
    // Whether the trash backend is reachable (T-10 section 16 lifecycle:
    // "Trash mount unavailable"). When false the entry is dimmed and its menu
    // is disabled; the session is otherwise unaffected.
    property bool trashAvailable: true
    // The Downloads stack (T-10 section 17): the folder listing (newest
    // first, `{ name, path, isDir }`), its item count, and the new-items
    // badge cleared when the stack is opened.
    property var downloadsItems: []
    property int downloadsCount: 0
    property int downloadsBadge: 0
    // The Trash menu's Empty Trash confirmation step (section 13): selecting
    // Empty Trash replaces the menu model with the confirm/cancel choice
    // before the shell performs the destructive operation.
    property bool trashConfirming: false

    // Keyboard navigation (T-10 section 20). The shell sets
    // `keyboardFocused` when the compositor hands the Dock the keyboard
    // (FocusDock / click); the Dock then owns entry-to-entry navigation.
    property bool keyboardFocused: false
    // The id (`items[i].id`) of the entry carrying the focus ring; "" = none.
    property string focusedItemId: ""

    // --- Interaction state ----------------------------------------------
    // Pointer position along the dock axis in local coordinates, -1 when the
    // pointer is not over the Dock.
    property real pointerAlong: -1
    property bool dragging: false
    // The dragged app entry and its tentative reorder state (T-10 section
    // 12). `dragTargetIndex` is an insertion index in the app region;
    // `dragBaseCenters` are the pre-drag pinned slot centers (excluding the
    // dragged entry) so the target does not feed back into the live layout.
    property string dragEntryId: ""
    property int dragFromAppIndex: -1
    property int dragTargetIndex: -1
    property real dragPointerAlong: -1
    property bool dragOutside: false
    property bool dragOutOfDock: false
    property bool dragPromote: false
    property var dragBaseCenters: []
    property var dragOriginalPinnedIds: []

    // Divider resize (T-10 section 5): dragging the separator changes
    // `dock.size`, growing the icons as the handle moves away from the Dock
    // centre and shrinking them as it moves back. `dockSizePreview` fires on
    // every move (the shell re-lays-out without saving); `dockSizeChanged`
    // fires on release and is persisted.
    property bool resizing: false
    property real resizeStartIconSize: 0
    property real resizeStartDistance: 0

    // --- External drops (T-10 section 12) -------------------------------
    // The shell drives these from its Wayland data-device drag target: the
    // payload kind is known from the drag source's mime types, the target is
    // resolved from the pointer position. The shell owns the payload and
    // performs the resolved action when `externalDropRequested` fires.
    property bool externalDragActive: false
    property bool externalPayloadIsApp: false
    property int externalPayloadCount: 0
    // The id of the entry under the drag pointer ("" = none).
    property string externalTargetId: ""
    // Insertion index in the app region for an application-alias drop; the
    // live gap reflows the layout around a placeholder entry.
    property int externalInsertIndex: -1
    // A stack entry the pointer has dwelled over long enough to spring-load
    // (T-10 section 17); the Downloads stack popover opens when it fires.
    property string springLoadTargetId: ""
    // Spring-loading hover delay (ms), shared with Files (T-18). Mirrors
    // `kSpringLoadMs` in the shell's pure drop core.
    readonly property int springLoadDelay: 500

    // The app entry whose context menu / window chooser is open, plus the
    // entry item each is anchored to. Only one popover is open at a time.
    property var menuEntry: null
    property bool menuOpen: false
    property Item menuAnchor: null
    property var chooserEntry: null
    property bool chooserOpen: false
    property Item chooserAnchor: null
    // The Downloads stack popover (T-10 section 17).
    property bool stackOpen: false
    property Item stackAnchor: null

    signal entryActivated(var entry)
    signal entryContextMenuRequested(var entry, real globalX, real globalY)
    signal dividerContextMenuRequested(real globalX, real globalY)
    signal settingsRequested()
    // A context-menu/chooser action resolved to a shell operation.
    signal menuActionRequested(string action, var payload)
    // A specific window chosen from the window chooser (T-10 FR-5).
    signal windowActivated(string windowId)
    // The popover opened, closed, or resized; the shell re-renders the
    // overlay surface.
    signal popoverChanged()
    // A drag finished with a new pinned set (reorder, promote, or remove);
    // the shell writes it to `dock.pinned`. The list is the complete ordered
    // set of pinned desktop ids (T-10 section 12, FR-9).
    signal pinnedOrderChanged(var desktopIds)
    // An external drag entered/moved/left/dropped. `targetId`/`targetKind`
    // identify the entry under the pointer; the shell resolves and performs
    // the action on the payload it holds (T-10 section 12, FR-9).
    signal externalDropRequested(string targetId, string targetKind, string desktopId, bool payloadIsApp)
    signal externalDragChanged()
    // The divider resize handle moved: `dockSizePreview` is the live
    // un-persisted preview and `dockSizeChanged` is the committed value
    // (T-10 section 5). Both carry a `dock.size` fraction in 0..1.
    signal dockSizePreview(real fraction)
    signal dockSizeChanged(real fraction)
    // A stack entry has been hovered long enough to spring-load (T-10
    // section 12); the Dock opens the Downloads stack popover on the fire.
    signal springLoadRequested(string targetId)
    // The Downloads stack popover (T-10 section 17): a file row was chosen,
    // the folder itself was opened, or the stack was viewed (clear the badge).
    signal downloadActivated(string path)
    signal downloadsFolderRequested()
    signal downloadsViewed()
    // Escape asked to leave Dock keyboard navigation; the shell releases the
    // compositor keyboard focus back to the active window (T-10 section 20).
    signal keyboardFocusReleaseRequested()

    // --- Geometry constants ---------------------------------------------
    readonly property real padding: Theme.controls.dock.padding
    readonly property real gap: Theme.controls.dock.gap
    readonly property real dividerWidth: 1
    // Room reserved below every entry's artwork for a running indicator. It is
    // reserved for all entries, not only running ones, so running and idle
    // icons share one baseline (a per-running reservation lifts the running
    // icons, which reads as a magnification bump).
    readonly property real indicatorSpace:
        showIndicators ? Theme.controls.dock.indicatorGap + Theme.controls.dock.indicatorSize : 0
    readonly property real barThickness: iconSize + indicatorSpace + 2 * padding
    // `dock.magnification` (0..1, 0 = off) maps onto the peak icon factor;
    // 0.5 (the default) lands on the `magnifyPeak` token (T-10 section 19).
    readonly property real magnifyPeakFactor:
        magnification <= 0 ? 1.0
        : 1 + magnification * (Theme.controls.dock.magnifyPeakMax - 1)
    readonly property real magnifyFalloff: Theme.controls.dock.magnifyFalloff
    // How far a lifted (dragged) entry rises above the bar.
    readonly property real dragLift: 8
    // Transparent room above/beside the bar that magnified artwork grows
    // into. It is sized for the maximum magnification so a live
    // `dock.magnification` change never needs to grow the scene. The
    // reserved zone is `barThickness` only (section 2).
    readonly property real magnifyBand:
        Math.ceil((Theme.controls.dock.magnifyPeakMax - 1) * iconSize) + padding
    readonly property bool axisIsX: position === "bottom"
    readonly property string indicatorEdge:
        position === "bottom" ? "bottom" : (position === "left" ? "left" : "right")
    readonly property real axisLength: axisIsX ? width : height
    // The thin sliver of the surface that stays interactive while the Dock is
    // hidden, so a pointer reaching the output edge can summon it back
    // (T-10 section 15).
    readonly property real edgeTrigger: Theme.controls.dock.edgeTrigger

    // --- Auto-hide translation ------------------------------------------
    // The bar is translated off its anchored edge by its own thickness plus
    // the edge margin. `hideOffset` is the magnitude; `hideX`/`hideY` are the
    // per-axis translation for the configured position (T-10 sections 5/15).
    //
    // The slide is animated (FR-14): the scene-graph commit path delivers
    // every frame, so the reveal/hide is a real motion instead of the former
    // snap. Reduced motion collapses the duration to 0 in the token, which
    // makes the transition instant.
    property real hideOffset:
        autoHide && !revealed ? barThickness + Theme.controls.dock.edgeMargin : 0
    Behavior on hideOffset {
        NumberAnimation {
            duration: Theme.motion.dockReveal.duration
            easing.bezierCurve: Theme.motion.dockReveal.curve
        }
    }
    readonly property real hideX:
        position === "left" ? -hideOffset : position === "right" ? hideOffset : 0
    readonly property real hideY: position === "bottom" ? hideOffset : 0

    // The sliver of the surface at the anchored edge that remains part of the
    // input region while hidden (T-10 section 15). It is the only hit area
    // that can reveal the Dock, so it must never move with `hideOffset`.
    readonly property var edgeRect: {
        if (position === "bottom")
            return { x: 0, y: height - edgeTrigger, w: width, h: edgeTrigger };
        if (position === "left")
            return { x: 0, y: 0, w: edgeTrigger, h: height };
        return { x: width - edgeTrigger, y: 0, w: edgeTrigger, h: height };
    }

    function pointInEdgeBand(px, py) {
        var r = edgeRect;
        return px >= r.x && px <= r.x + r.w && py >= r.y && py <= r.y + r.h;
    }

    // `revealStateChanged` lets the shell re-commit the Dock and its input
    // region when the reveal/hide transition changes the translation. The
    // transition snaps rather than animating until the scene-graph render
    // path (FR-14) can commit every frame.
    signal revealStateChanged()
    onRevealedChanged: revealStateChanged()

    function reveal() {
        revealTimer.stop();
        // A reveal supersedes a pending re-hide; without this, a hide timer
        // started before the reveal can fire and slide the Dock straight back
        // out.
        hideTimer.stop();
        if (!revealed)
            revealed = true;
    }
    function hide() {
        hideTimer.stop();
        if (autoHide && revealed)
            revealed = false;
    }

    // Re-hide only when nothing holds the Dock open: no popover, no drag, no
    // Dock keyboard focus, and the pointer has left the surface (T-10 section
    // 15).
    function hideIfIdle() {
        if (autoHide && !popoverOpen && !dragging && !resizing
                && !keyboardFocused && !dockHover.hovered)
            hide();
    }

    // Schedule a re-hide when a popover closes and the pointer is elsewhere.
    function scheduleHide() {
        if (autoHide && revealed && !keyboardFocused && !dockHover.hovered)
            hideTimer.restart();
    }

    // The reveal delay lets a pointer passing over the edge continue without
    // summoning the Dock; a pointer that dwells reveals it (section 15).
    Timer {
        id: revealTimer
        interval: Theme.controls.dock.revealDelay
        onTriggered: dock.reveal()
    }

    Timer {
        id: hideTimer
        interval: Theme.controls.dock.hideDelay
        onTriggered: dock.hideIfIdle()
    }

    // --- Entry regions ---------------------------------------------------
    readonly property var appEntries: {
        var out = [];
        for (var i = 0; i < entries.length; ++i) {
            var k = entries[i].kind;
            if (k === "pinned" || k === "temporary" || k === "recent")
                out.push(entries[i]);
        }
        return out;
    }
    readonly property var minimizedEntries: {
        var out = [];
        if (minimizeIntoTileIcon)
            return out;
        for (var i = 0; i < entries.length; ++i) {
            if (entries[i].kind === "minimized")
                out.push(entries[i]);
        }
        return out;
    }
    // The Trash is permanent and always the last right-region item.
    readonly property var trashEntry: ({
        id: "__trash__", appId: "", name: qsTr("Trash"), kind: "trash",
        running: false, trashFull: dock.trashFull, trashCount: dock.trashCount,
        available: dock.trashAvailable
    })
    // The Downloads stack sits in the right region before the Trash (T-10
    // section 17). It is present even when the folder is empty so it is a
    // stable drop target.
    readonly property var stackEntry: ({
        id: "__downloads__", appId: "", name: qsTr("Downloads"), kind: "stack",
        running: false, stackCount: dock.downloadsCount, badge: dock.downloadsBadge
    })
    readonly property var dividerEntry: ({ id: "__divider__", kind: "divider" })

    // Pinned entries are the prefix of the app region (the shell emits pinned
    // first, then temporary running apps); only this prefix is user-orderable.
    readonly property int pinnedCount: {
        var count = 0;
        for (var i = 0; i < appEntries.length; ++i) {
            if (appEntries[i].kind === "pinned")
                count++;
        }
        return count;
    }

    // During a drag the app region is shown with the dragged entry moved to
    // its tentative slot, so the neighbours animate into the opened gap.
    readonly property var visualAppEntries: {
        if (!dragging || dragEntryId === "" || dragFromAppIndex < 0)
            return appEntries;
        var list = appEntries.slice();
        if (dragFromAppIndex >= list.length)
            return list;
        var moved = list.splice(dragFromAppIndex, 1)[0];
        var to = Math.max(0, Math.min(list.length, dragTargetIndex));
        list.splice(to, 0, moved);
        return list;
    }

    // Ordered items: apps, divider, minimized windows, Trash. The order is
    // stable during a drag (the Repeater must not be reset while a DragHandler
    // holds the pointer); the drag gap is applied in `layout` instead.
    //
    // An application-alias external drop has no delegate holding the pointer,
    // so it reflows safely: a placeholder entry opens a real gap at the
    // insertion index (section 12).
    readonly property bool externalGap:
        externalDragActive && externalPayloadIsApp && externalInsertIndex >= 0
    readonly property var externalPlaceholderEntry: ({
        id: "__external_drop__", kind: "external", name: "", appId: "",
        running: false
    })
    readonly property var items: {
        var out = appEntries.slice();
        if (externalGap) {
            var idx = Math.max(0, Math.min(out.length, externalInsertIndex));
            out.splice(idx, 0, externalPlaceholderEntry);
        }
        out.push(dividerEntry);
        for (var i = 0; i < minimizedEntries.length; ++i)
            out.push(minimizedEntries[i]);
        out.push(stackEntry);
        out.push(trashEntry);
        return out;
    }

    // The slot an app entry occupies while dragging: the app region permuted
    // by the tentative move, without touching the Repeater model.
    function appSlot(appIndex) {
        if (!dragging || dragFromAppIndex < 0)
            return appIndex;
        var order = [];
        for (var i = 0; i < appEntries.length; ++i)
            order.push(i);
        if (dragFromAppIndex >= order.length)
            return appIndex;
        var moved = order.splice(dragFromAppIndex, 1)[0];
        var to = Math.max(0, Math.min(order.length, dragTargetIndex));
        order.splice(to, 0, moved);
        for (var s = 0; s < order.length; ++s) {
            if (order[s] === appIndex)
                return s;
        }
        return appIndex;
    }

    // --- Baseline layout (no magnification) -----------------------------
    readonly property var _baseline: {
        var list = items;
        var n = list.length;
        var sizes = [];
        var i;
        for (i = 0; i < n; ++i)
            sizes.push(list[i].kind === "divider" ? dividerWidth : iconSize);
        var total = 0;
        for (i = 0; i < n; ++i) {
            total += sizes[i];
            if (i < n - 1)
                total += gap;
        }
        var start = (axisLength - total) / 2;
        var positions = [];
        var centers = [];
        var cursor = start;
        for (i = 0; i < n; ++i) {
            positions.push(cursor);
            centers.push(cursor + sizes[i] / 2);
            cursor += sizes[i] + gap;
        }
        return { sizes: sizes, positions: positions, centers: centers, total: total };
    }

    // Index of the icon item nearest the pointer (never the divider).
    readonly property int anchorIndex: {
        if (pointerAlong < 0 || items.length === 0)
            return -1;
        var best = -1;
        var bestDistance = Number.MAX_VALUE;
        for (var i = 0; i < items.length; ++i) {
            if (items[i].kind === "divider" || items[i].kind === "external")
                continue;
            var d = Math.abs(_baseline.centers[i] - pointerAlong);
            if (d < bestDistance) {
                bestDistance = d;
                best = i;
            }
        }
        return best;
    }

    readonly property bool magnifying:
        magnification > 0 && pointerAlong >= 0 && anchorIndex >= 0
        && !popoverOpen && !dragging && !resizing && revealed

    // Magnification is suppressed while a context menu, chooser, or stack
    // popover is open (T-10 section 14).
    readonly property bool popoverOpen: menuOpen || chooserOpen || stackOpen

    // --- Keyboard navigation (T-10 section 20) ---------------------------
    // The entries the arrow keys traverse: every entry except the divider.
    readonly property var navigableItems: {
        var out = [];
        for (var i = 0; i < items.length; ++i) {
            if (items[i].kind !== "divider" && items[i].kind !== "external")
                out.push(items[i]);
        }
        return out;
    }

    function beginKeyboardNavigation() {
        keyboardFocused = true;
        // A pointer-opened popover owns the arrow keys; do not put a focus
        // ring behind it.
        if (popoverOpen)
            return;
        if (focusedItemId !== "" && indexOfItemId(focusedItemId) >= 0)
            return;
        var list = navigableItems;
        focusedItemId = list.length > 0 ? list[0].id : "";
    }

    function endKeyboardNavigation() {
        keyboardFocused = false;
        focusedItemId = "";
        // Keyboard focus suppresses re-hide (section 15); once it leaves, the
        // normal re-hide delay applies if the pointer is elsewhere.
        scheduleHide();
    }

    function moveKeyboardFocus(delta) {
        var list = navigableItems;
        if (list.length === 0) {
            focusedItemId = "";
            return;
        }
        var pos = -1;
        for (var i = 0; i < list.length; ++i) {
            if (list[i].id === focusedItemId) {
                pos = i;
                break;
            }
        }
        pos = (pos < 0 ? 0 : pos + delta + list.length) % list.length;
        focusedItemId = list[pos].id;
    }

    function focusedEntry() {
        var idx = indexOfItemId(focusedItemId);
        return idx >= 0 ? items[idx] : null;
    }

    // The click tree (section 8) shared by pointer activation and Return.
    function activateEntry(entry) {
        if (!entry)
            return;
        // An unavailable Trash is inert: the entry is dimmed and its menu is
        // disabled (T-10 section 16 lifecycle).
        if (entry.kind === "trash" && entry.available === false)
            return;
        // The Downloads stack opens its folder popover, never a launch
        // (T-10 section 17).
        if (entry.kind === "stack") {
            openStack();
            return;
        }
        if (entry.running === true && entry.kind !== "minimized"
                && entry.windowList !== undefined && entry.windowList.length > 1) {
            openChooser(entry);
            return;
        }
        entryActivated(entry);
    }

    // Typing jumps to the first entry whose name starts with the buffer
    // (T-10 section 20). The buffer clears shortly after the last keystroke.
    function jumpToName(text) {
        if (text === "")
            return;
        var lower = text.toLowerCase();
        var list = navigableItems;
        for (var i = 0; i < list.length; ++i) {
            var name = list[i].name !== undefined ? String(list[i].name) : "";
            if (name.toLowerCase().indexOf(lower) === 0) {
                focusedItemId = list[i].id;
                return;
            }
        }
    }

    Timer {
        id: typeBufferTimer
        interval: 800
        onTriggered: dock.typeBuffer = ""
    }
    property string typeBuffer: ""

    Keys.onPressed: (event) => {
        if (!keyboardFocused || popoverOpen)
            return;
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                || event.key === Qt.Key_Space) {
            activateEntry(focusedEntry());
            event.accepted = true;
        } else if ((axisIsX && event.key === Qt.Key_Left)
                   || (!axisIsX && event.key === Qt.Key_Up)) {
            moveKeyboardFocus(-1);
            event.accepted = true;
        } else if ((axisIsX && event.key === Qt.Key_Right)
                   || (!axisIsX && event.key === Qt.Key_Down)) {
            moveKeyboardFocus(1);
            event.accepted = true;
        } else if ((axisIsX && event.key === Qt.Key_Up) || event.key === Qt.Key_Menu) {
            var entry = focusedEntry();
            if (entry)
                openEntryMenu(entry);
            event.accepted = true;
        } else if (event.key === Qt.Key_Escape) {
            // Leave keyboard navigation; the shell releases compositor focus
            // back to the active window (T-10 section 20).
            keyboardFocusReleaseRequested();
            endKeyboardNavigation();
            event.accepted = true;
        } else if (event.text.length === 1 && event.text >= " ") {
            typeBuffer += event.text;
            typeBufferTimer.restart();
            jumpToName(typeBuffer);
            event.accepted = true;
        }
    }

    // --- Context menu ----------------------------------------------------
    function indexOfItemId(id) {
        for (var i = 0; i < items.length; ++i) {
            if (items[i].id === id)
                return i;
        }
        return -1;
    }

    function closePopovers() {
        entryMenu.hide();
        windowChooser.hide();
        stackPopover.hide();
        trashConfirming = false;
    }

    function openEntryMenu(entry) {
        closePopovers();
        hideTimer.stop();
        var idx = indexOfItemId(entry.id);
        menuAnchor = idx >= 0 ? entryRepeater.itemAt(idx) : null;
        menuEntry = entry;
        entryMenu.open = true;
    }

    function openChooser(entry) {
        closePopovers();
        hideTimer.stop();
        var idx = indexOfItemId(entry.id);
        chooserAnchor = idx >= 0 ? entryRepeater.itemAt(idx) : null;
        chooserEntry = entry;
        windowChooser.open = true;
    }

    // Open the Downloads stack popover and clear the new-items badge (T-10
    // section 17). The shell owns the badge state; `downloadsViewed` asks it
    // to mark the folder seen.
    function openStack() {
        closePopovers();
        hideTimer.stop();
        var idx = indexOfItemId(stackEntry.id);
        stackAnchor = idx >= 0 ? entryRepeater.itemAt(idx) : null;
        stackPopover.open = true;
        downloadsViewed();
    }

    // --- Drag rearrangement (T-10 section 12) ----------------------------
    function appIndexOfId(id) {
        for (var i = 0; i < appEntries.length; ++i) {
            if (appEntries[i].id === id)
                return i;
        }
        return -1;
    }

    function isDraggable(entry) {
        return entry && entry.kind !== "divider" && entry.kind !== "trash"
                && entry.kind !== "minimized" && entry.kind !== "external"
                && entry.kind !== "stack";
    }

    function currentPinnedIds() {
        var out = [];
        for (var i = 0; i < appEntries.length; ++i) {
            if (appEntries[i].kind === "pinned")
                out.push(appEntries[i].desktopId);
        }
        return out;
    }

    function resetDrag() {
        dragging = false;
        dragEntryId = "";
        dragFromAppIndex = -1;
        dragTargetIndex = -1;
        dragPointerAlong = -1;
        dragOutside = false;
        dragOutOfDock = false;
        dragPromote = false;
        dragBaseCenters = [];
        dragOriginalPinnedIds = [];
    }

    // Press-and-hold then move beyond the threshold lifts the entry (the
    // DockEntry DragHandler calls this).
    function beginDrag(entry) {
        if (!isDraggable(entry))
            return;
        closePopovers();
        hideTimer.stop();
        revealTimer.stop();
        dragging = true;
        dragEntryId = entry.id;
        dragFromAppIndex = appIndexOfId(entry.id);
        dragOriginalPinnedIds = currentPinnedIds();
        dragTargetIndex = entry.kind === "pinned" ? dragFromAppIndex : pinnedCount;
        dragPointerAlong = -1;
        dragOutside = false;
        dragOutOfDock = false;
        dragPromote = false;
        // Pre-drag pinned slot centers, excluding the dragged entry, so the
        // insertion index is stable while the live layout permutes the slots.
        dragBaseCenters = [];
        for (var i = 0; i < pinnedCount; ++i) {
            if (appEntries[i].id === entry.id)
                continue;
            dragBaseCenters.push(_baseline.centers[i]);
        }
    }

    // Insertion index (0..pinnedCount) for the pointer position, computed
    // against the fixed pre-drag pinned centers.
    function computeDropIndex(along) {
        var idx = 0;
        for (var i = 0; i < dragBaseCenters.length; ++i) {
            if (along > dragBaseCenters[i])
                idx = i + 1;
        }
        return idx;
    }

    function updateDrag(entry, sceneX, sceneY) {
        if (!dragging || entry.id !== dragEntryId)
            return;
        var local = dock.mapFromItem(null, sceneX, sceneY);
        dragTo(entry, axisIsX ? local.x : local.y);
    }

    // Update the tentative target from an already-local axis position; split
    // out from the scene mapping so tests can drive the model directly.
    function dragTo(entry, along) {
        if (!dragging || entry.id !== dragEntryId)
            return;
        dragPointerAlong = along;
        var start = axisIsX ? barRect.x : barRect.y;
        var end = axisIsX ? barRect.x + barRect.w : barRect.y + barRect.h;
        var margin = iconSize;
        dragOutside = along < start - margin || along > end + margin;
        dragOutOfDock = dragOutside && entry.kind === "pinned";
        dragTargetIndex = computeDropIndex(along);
        dragPromote = !dragOutside && entry.kind !== "pinned"
                      && dragTargetIndex <= pinnedCount
                      && entry.desktopId !== undefined
                      && entry.desktopId !== "";
    }

    function pinnedIdsAfterDrag() {
        var out = [];
        if (dragOutOfDock) {
            // Remove: every pinned entry except the dragged one, in order.
            for (var i = 0; i < appEntries.length; ++i) {
                var pe = appEntries[i];
                if (pe.kind === "pinned" && pe.id !== dragEntryId)
                    out.push(pe.desktopId);
            }
            return out;
        }
        var vis = visualAppEntries;
        for (var j = 0; j < vis.length; ++j) {
            var e = vis[j];
            if (e.kind === "pinned")
                out.push(e.desktopId);
            else if (e.id === dragEntryId && dragPromote && e.desktopId !== undefined)
                out.push(e.desktopId);
        }
        return out;
    }

    function finalizeDrag() {
        var next = pinnedIdsAfterDrag();
        var changed = next.join("\n") !== dragOriginalPinnedIds.join("\n");
        resetDrag();
        if (changed)
            pinnedOrderChanged(next);
    }

    // Drop at an already-local axis position (test/introspection hook).
    function dropAt(entry, along) {
        if (!dragging || entry.id !== dragEntryId) {
            resetDrag();
            return;
        }
        dragTo(entry, along);
        finalizeDrag();
    }

    function endDrag(entry, sceneX, sceneY) {
        if (!dragging || entry.id !== dragEntryId) {
            resetDrag();
            return;
        }
        updateDrag(entry, sceneX, sceneY);
        finalizeDrag();
    }

    // The pointer left the Dock surface mid-drag: a pinned entry is removed
    // (dragged out), anything else snaps back. Cross-output drag is not
    // supported (section 12).
    function dragPointerLeft() {
        if (!dragging)
            return;
        var entry = appEntries[appIndexOfId(dragEntryId)];
        dragOutside = true;
        dragOutOfDock = entry !== undefined && entry.kind === "pinned";
        dragPromote = false;
        finalizeDrag();
    }

    // --- Divider resize (T-10 section 5) ---------------------------------
    // The separator is the Dock's resize handle: dragging it away from the
    // Dock centre grows the icons, toward it shrinks them. The icon size maps
    // linearly onto `dock.size` in 0..1 (section 19), so the shell only
    // persists the fraction. The surface is reconfigured live via the preview
    // signal; the committed value is saved on release.
    readonly property real axisCenter: axisLength / 2

    function iconSizeFraction(value) {
        var span = iconSizeMax - iconSizeMin;
        if (span <= 0)
            return 0;
        return Math.max(0, Math.min(1, (value - iconSizeMin) / span));
    }

    function beginDividerResize(sceneX, sceneY) {
        closePopovers();
        hideTimer.stop();
        revealTimer.stop();
        var local = dock.mapFromItem(null, sceneX, sceneY);
        var along = axisIsX ? local.x : local.y;
        resizing = true;
        resizeStartIconSize = iconSize;
        resizeStartDistance = Math.abs(along - axisCenter);
    }

    function dividerResizeIcon(sceneX, sceneY) {
        if (!resizing)
            return iconSize;
        var local = dock.mapFromItem(null, sceneX, sceneY);
        var along = axisIsX ? local.x : local.y;
        var distance = Math.abs(along - axisCenter);
        return Math.max(iconSizeMin,
                        Math.min(iconSizeMax,
                                 resizeStartIconSize + (distance - resizeStartDistance)));
    }

    function updateDividerResize(sceneX, sceneY) {
        updateDividerResizeAt(dividerResizeIcon(sceneX, sceneY));
    }

    // Apply an already-computed icon size (clamped to the size range); split
    // out so tests can drive the model without a live pointer.
    function updateDividerResizeAt(value) {
        if (!resizing)
            return;
        var clamped = Math.max(iconSizeMin, Math.min(iconSizeMax, value));
        iconSize = clamped;
        dockSizePreview(iconSizeFraction(clamped));
    }

    function endDividerResize() {
        if (!resizing)
            return;
        resizing = false;
        dockSizeChanged(iconSizeFraction(iconSize));
    }

    // --- External drops (T-10 section 12) --------------------------------
    // The index of the entry whose layout slot is nearest `localX/localY`, or
    // -1 when the point is outside the bar and its magnified band (a drop on
    // empty Dock is a no-op, section 12).
    function itemIndexAtLocal(localX, localY) {
        var along = axisIsX ? localX : localY;
        var cross = axisIsX ? localY : localX;
        var alongStart = axisIsX ? barRect.x : barRect.y;
        var alongEnd = axisIsX ? barRect.x + barRect.w : barRect.y + barRect.h;
        var crossStart = axisIsX ? barRect.y : barRect.x;
        var crossEnd = axisIsX ? barRect.y + barRect.h : barRect.x + barRect.w;
        if (along < alongStart - iconSize || along > alongEnd + iconSize)
            return -1;
        if (cross < crossStart - magnifyBand || cross > crossEnd + magnifyBand)
            return -1;
        var l = layout;
        var best = -1;
        var bestDistance = Number.MAX_VALUE;
        for (var i = 0; i < items.length; ++i) {
            var center = axisIsX ? (l[i].x + l[i].w / 2) : (l[i].y + l[i].h / 2);
            var d = Math.abs(center - along);
            if (d < bestDistance) {
                bestDistance = d;
                best = i;
            }
        }
        return best;
    }

    // Insertion index (0..appEntries.length) for an application-alias drop.
    // Computed against the current app-entry centers, counting the entries
    // before the pointer, so it is stable while the placeholder reflows.
    function externalInsertionIndex(localAlong) {
        var l = layout;
        var appIdx = 0;
        for (var i = 0; i < items.length; ++i) {
            var k = items[i].kind;
            if (k === "external")
                continue;
            if (k === "divider" || k === "trash" || k === "minimized" || k === "stack")
                break;
            var center = axisIsX ? (l[i].x + l[i].w / 2) : (l[i].y + l[i].h / 2);
            if (localAlong < center)
                return appIdx;
            appIdx++;
        }
        return appEntries.length;
    }

    function beginExternalDrag(payloadIsApp, payloadCount) {
        closePopovers();
        hideTimer.stop();
        revealTimer.stop();
        externalDragActive = true;
        externalPayloadIsApp = payloadIsApp;
        externalPayloadCount = payloadCount;
        externalTargetId = "";
        externalInsertIndex = -1;
        springLoadTargetId = "";
        springLoadTimer.stop();
        externalDragChanged();
    }

    function externalDragTo(sceneX, sceneY) {
        if (!externalDragActive)
            return;
        var local = dock.mapFromItem(null, sceneX, sceneY);
        var idx = itemIndexAtLocal(local.x, local.y);
        var id = idx >= 0 ? items[idx].id : "";
        if (id !== externalTargetId) {
            externalTargetId = id;
            // A new target restarts the spring-load dwell (section 12).
            springLoadTimer.stop();
            springLoadTargetId = "";
            if (idx >= 0 && items[idx].kind === "stack")
                springLoadTimer.start();
        }
        if (externalPayloadIsApp) {
            var insertion = externalInsertionIndex(axisIsX ? local.x : local.y);
            if (insertion !== externalInsertIndex)
                externalInsertIndex = insertion;
        }
        externalDragChanged();
    }

    function externalDragLeft() {
        if (externalDragActive)
            resetExternalDrag();
    }

    // Drop at an already-local axis position (test/introspection hook).
    function externalDropAt(localX, localY) {
        if (!externalDragActive) {
            resetExternalDrag();
            return;
        }
        var scene = dock.mapToItem(null, localX, localY);
        externalDrop(scene.x, scene.y);
    }

    function externalDrop(sceneX, sceneY) {
        if (!externalDragActive) {
            resetExternalDrag();
            return;
        }
        externalDragTo(sceneX, sceneY);
        var idx = indexOfItemId(externalTargetId);
        var kind = idx >= 0 ? items[idx].kind : "";
        var desktopId = idx >= 0 && items[idx].desktopId !== undefined
                        ? String(items[idx].desktopId) : "";
        externalDropRequested(externalTargetId, kind, desktopId, externalPayloadIsApp);
        resetExternalDrag();
    }

    function resetExternalDrag() {
        externalDragActive = false;
        externalPayloadIsApp = false;
        externalPayloadCount = 0;
        externalTargetId = "";
        externalInsertIndex = -1;
        springLoadTargetId = "";
        springLoadTimer.stop();
        externalDragChanged();
    }

    Timer {
        id: springLoadTimer
        interval: dock.springLoadDelay
        onTriggered: {
            dock.springLoadTargetId = dock.externalTargetId;
            dock.springLoadRequested(dock.externalTargetId);
            // The Downloads stack is the first spring-load consumer (T-10
            // section 17): dwelling over it during a drag opens its popover so
            // the drop can target a row.
            var idx = dock.indexOfItemId(dock.externalTargetId);
            if (idx >= 0 && dock.items[idx].kind === "stack")
                dock.openStack();
        }
    }

    // The lifted entry follows the pointer on the dock axis and stays on the
    // bar perpendicular to it.
    function draggedX(itemWidth) {
        if (axisIsX)
            return dragPointerAlong - itemWidth / 2;
        if (position === "right")
            return barRect.x + barThickness - padding - itemWidth;
        return barRect.x + padding;
    }
    function draggedY(itemHeight) {
        if (axisIsX)
            return barRect.y + barThickness - padding - itemHeight - dragLift;
        return dragPointerAlong - itemHeight / 2;
    }

    // The app-entry menu (T-10 section 13): the live window list, Show All
    // Windows, Keep/Remove from Dock, Options ▸, Show in Files, and
    // Quit/Open. The divider menu holds the Dock options; the Trash menu
    // holds Open and Empty Trash.
    readonly property var menuModel: {
        var e = menuEntry;
        if (!e)
            return [];
        var out = [];
        if (e.kind === "divider")
            return dividerMenuModel();
        if (e.kind === "trash")
            return trashMenuModel();
        if (e.kind === "stack")
            return stackMenuModel();
        var running = e.running === true;
        var minimized = e.kind === "minimized";
        var list = e.windowList !== undefined ? e.windowList : [];
        // The window list (T-10 sections 9/13): shown for any entry that
        // carries an app's windows, including a minimized-window entry (its
        // owning app's full list). The frontmost window is checked and each
        // minimized row is marked.
        if (list.length > 0) {
            for (var i = 0; i < list.length; ++i) {
                var w = list[i];
                var label = w.title !== undefined ? w.title : qsTr("Window");
                if (w.minimized === true)
                    label += qsTr(" (minimized)");
                out.push({
                    type: "item", label: label, checked: w.focused === true,
                    shortcut: w.workspaceName !== undefined ? w.workspaceName : "",
                    action: "activate_window",
                    payload: { windowId: w.windowId }
                });
            }
            out.push({ type: "separator" });
            out.push({
                type: "item", label: qsTr("Show All Windows"),
                action: "show_all_windows", payload: { appId: e.appId }
            });
            out.push({ type: "separator" });
        }
        if (minimized) {
            // A minimized-window entry (section 13) is the owning app's
            // window list plus Quit; it is not an app entry, so there is no
            // Keep/Remove or Options.
            out.push({
                type: "item", label: qsTr("Quit"),
                action: "quit", payload: { appId: e.appId }
            });
            return out;
        }
        if (running) {
            if (e.pinned === true) {
                out.push({
                    type: "item", label: qsTr("Remove from Dock"),
                    action: "remove_from_dock", payload: { desktopId: e.desktopId }
                });
            } else {
                out.push({
                    type: "item", label: qsTr("Keep in Dock"),
                    action: "keep_in_dock", payload: { desktopId: e.desktopId }
                });
            }
            out.push({
                type: "submenu", label: qsTr("Options"), submenu: optionsMenuModel(e)
            });
            out.push({ type: "separator" });
            out.push({
                type: "item", label: qsTr("Quit"),
                action: "quit", payload: { appId: e.appId }
            });
        } else if (e.missing !== true) {
            out.push({
                type: "item", label: qsTr("Open"),
                action: "open", payload: { desktopId: e.desktopId }
            });
            out.push({
                type: "submenu", label: qsTr("Options"), submenu: optionsMenuModel(e)
            });
            out.push({
                type: "item", label: qsTr("Show in Files"),
                action: "show_in_files",
                payload: { desktopId: e.desktopId, appId: e.appId }
            });
            if (e.pinned === true) {
                out.push({ type: "separator" });
                out.push({
                    type: "item", label: qsTr("Remove from Dock"),
                    action: "remove_from_dock", payload: { desktopId: e.desktopId }
                });
            }
        }
        return out;
    }

    // The app "Options" submenu (T-10 section 13): Assign To, Open at Login
    // (T-24), and Show in Files (T-18). The design-system submenu is one
    // level, so the three Assign To choices are direct rows; the nested
    // macOS `Assign To ▸` form is deferred with nested submenus in the design
    // system. The shell resolves each action against the live app state.
    function optionsMenuModel(e) {
        var id = e.desktopId !== undefined ? e.desktopId : "";
        var appId = e.appId !== undefined ? e.appId : "";
        var out = [];
        out.push({
            type: "item", label: qsTr("Assign to This Desktop"),
            action: "assign_to",
            payload: { desktopId: id, appId: appId, target: "this" }
        });
        out.push({
            type: "item", label: qsTr("Assign to All Desktops"),
            action: "assign_to",
            payload: { desktopId: id, appId: appId, target: "all" }
        });
        out.push({
            type: "item", label: qsTr("Assign to None"),
            action: "assign_to",
            payload: { desktopId: id, appId: appId, target: "none" }
        });
        out.push({ type: "separator" });
        out.push({
            type: "item", label: qsTr("Open at Login"),
            action: "open_at_login", payload: { desktopId: id }
        });
        out.push({
            type: "item", label: qsTr("Show in Files"),
            action: "show_in_files", payload: { desktopId: id, appId: appId }
        });
        return out;
    }

    // The divider menu (T-10 section 13): the magnification and hiding
    // toggles, the position submenu, and the Settings entry point. Position
    // is written through the shell's live `dock.position` model; Dock
    // Settings is T-16.
    function dividerMenuModel() {
        var out = [];
        out.push({
            type: "item",
            label: magnification > 0 ? qsTr("Turn Magnification Off")
                                     : qsTr("Turn Magnification On"),
            action: "toggle_magnification", checkable: true,
            checked: magnification > 0
        });
        out.push({
            type: "item",
            label: autoHide ? qsTr("Turn Hiding Off") : qsTr("Turn Hiding On"),
            action: "toggle_autohide", checkable: true, checked: autoHide
        });
        out.push({ type: "separator" });
        out.push({
            type: "submenu", label: qsTr("Position on Screen"),
            submenu: [
                {
                    type: "item", label: qsTr("Bottom"),
                    action: "set_position", checkable: true,
                    checked: position === "bottom",
                    payload: { position: "bottom" }
                },
                {
                    type: "item", label: qsTr("Left"),
                    action: "set_position", checkable: true,
                    checked: position === "left",
                    payload: { position: "left" }
                },
                {
                    type: "item", label: qsTr("Right"),
                    action: "set_position", checkable: true,
                    checked: position === "right",
                    payload: { position: "right" }
                }
            ]
        });
        out.push({ type: "separator" });
        out.push({
            type: "item", label: qsTr("Dock Settings…"),
            action: "open_dock_settings"
        });
        return out;
    }

    // The Trash menu (T-10 sections 13/16): Open always; Empty Trash only
    // when non-empty. Selecting Empty Trash swaps in the confirmation step so
    // the destructive operation is never one click away.
    function trashMenuModel() {
        var out = [];
        if (trashConfirming) {
            out.push({
                type: "item", label: qsTr("Empty the Trash?"), enabled: false
            });
            out.push({
                type: "item", label: qsTr("Empty Trash"),
                action: "empty_trash"
            });
            out.push({
                type: "item", label: qsTr("Cancel"),
                action: "cancel_empty_trash"
            });
            return out;
        }
        // An unreachable trash backend degrades to a single disabled row so
        // the menu never offers an operation that cannot succeed (T-10
        // section 16 lifecycle).
        if (!trashAvailable) {
            out.push({
                type: "item", label: qsTr("Trash unavailable"), enabled: false
            });
            return out;
        }
        out.push({
            type: "item", label: qsTr("Open"),
            action: "open_trash"
        });
        out.push({ type: "separator" });
        out.push({
            type: "item", label: qsTr("Empty Trash"),
            action: "empty_trash_confirmation", enabled: trashFull,
            keepOpen: true
        });
        return out;
    }

    // The Downloads stack menu (T-10 section 17): open the folder; the
    // folder listing itself is the click popover.
    function stackMenuModel() {
        var out = [];
        out.push({
            type: "item", label: qsTr("Open Downloads Folder"),
            action: "open_downloads_folder"
        });
        return out;
    }

    // The open popover's rectangle in Dock-scene coordinates, or an empty
    // rect. The shell renders it into the Dock's `overlay` surface. A menu
    // with an open submenu reports the union of both panels so the nested
    // panel is not clipped (`ContextMenu.contentRect`).
    readonly property var popoverRect: {
        // `open` keeps the rect valid while the popover fades in/out (the
        // shell captures the animation); `visible` covers the close tail.
        var popup = (entryMenu.open || entryMenu.visible) ? entryMenu
                 : ((windowChooser.open || windowChooser.visible) ? windowChooser
                 : ((stackPopover.open || stackPopover.visible) ? stackPopover : null));
        if (!popup || popup.width <= 0 || popup.height <= 0)
            return { x: 0, y: 0, w: 0, h: 0 };
        if (popup.contentRect !== undefined) {
            var content = popup.contentRect;
            var origin = popup.mapToItem(dock, content.x, content.y);
            return { x: origin.x, y: origin.y, w: content.w, h: content.h };
        }
        var topLeft = popup.mapToItem(dock, 0, 0);
        return { x: topLeft.x, y: topLeft.y, w: popup.width, h: popup.height };
    }

    function scaledGap(a, b, aDivider, bDivider) {
        if (aDivider || bDivider)
            return gap;
        return gap * (a + b) / (2 * iconSize);
    }

    // The launch/attention bounce translation for an entry (T-10 section
    // 8.1): a sinusoidal hop whose phase the shell drives from the
    // compositor-clock launch/attention clocks. Attention is taller than a
    // launch; reduced motion removes the translation and leaves the state
    // legible through the entry's own indicator/label.
    function entryBounce(entry) {
        if (Theme.reducedMotion)
            return 0;
        var phase = entry.bounce;
        if (phase === undefined || phase < 0)
            return 0;
        // `dock.animateOpening` off suppresses the launch hop; an attention
        // bounce is a notification and still plays (T-10 sections 8.1/19).
        if (!animateOpening && entry.attention !== true)
            return 0;
        var amplitude = entry.attention === true ? barThickness / 2
                                                 : barThickness / 4;
        return amplitude * Math.sin(Math.PI * phase);
    }

    // --- Per-frame layout ------------------------------------------------
    readonly property var layout: {
        var list = items;
        var n = list.length;
        var base = _baseline;
        var sizes = base.sizes.slice();
        var positions = base.positions.slice();
        var magnified = magnifying;

        // A drag permutes the app slots (the gap) without reordering the
        // model, so the active DragHandler's delegate survives.
        if (dragging) {
            for (var a = 0; a < n; ++a) {
                var k = list[a].kind;
                if (k === "divider" || k === "trash" || k === "minimized")
                    continue;
                var slot = appSlot(a);
                positions[a] = base.positions[slot];
                sizes[a] = base.sizes[slot];
            }
        }

        if (magnified) {
            var peak = iconSize * magnifyPeakFactor;
            var falloff = magnifyFalloff * iconSize;
            for (var i = 0; i < n; ++i) {
                if (list[i].kind === "divider" || list[i].kind === "external")
                    continue;
                var d = Math.abs(base.centers[i] - pointerAlong);
                var t = Math.max(0, Math.min(1, 1 - d / falloff));
                sizes[i] = iconSize + (peak - iconSize)
                           * (1 - Math.cos(Math.PI * t)) / 2;
            }
            var anchor = anchorIndex;
            positions[anchor] = base.centers[anchor] - sizes[anchor] / 2;
            for (var r = anchor + 1; r < n; ++r) {
                positions[r] = positions[r - 1] + sizes[r - 1]
                        + scaledGap(sizes[r - 1], sizes[r],
                                    list[r - 1].kind === "divider",
                                    list[r].kind === "divider");
            }
            for (var l = anchor - 1; l >= 0; --l) {
                positions[l] = positions[l + 1] - sizes[l]
                        - scaledGap(sizes[l], sizes[l + 1],
                                    list[l].kind === "divider",
                                    list[l + 1].kind === "divider");
            }
        }

        var band = magnifyBand;
        var out = [];
        for (var j = 0; j < n; ++j) {
            var isDivider = list[j].kind === "divider";
            var extra = isDivider ? 0 : indicatorSpace;
            var bounce = isDivider ? 0 : entryBounce(list[j]);
            if (axisIsX) {
                var h = sizes[j] + extra;
                out.push({
                    x: positions[j],
                    y: band + barThickness - padding - h - bounce + hideY,
                    w: isDivider ? dividerWidth : sizes[j],
                    h: isDivider ? barThickness - 2 * padding : h,
                    iconSize: sizes[j]
                });
            } else {
                var w = sizes[j] + extra;
                // A vertical bar sits on the anchored edge: a left Dock packs
                // entries from the left, a right Dock from the right so the
                // running indicator hugs the screen edge. Bounce moves away
                // from the edge, into the magnify band (T-10 section 14).
                var vx = position === "right" ? width - padding - w - bounce
                                              : padding + bounce;
                out.push({
                    x: vx + hideX,
                    y: positions[j],
                    w: isDivider ? dividerWidth : w,
                    h: isDivider ? barThickness - 2 * padding : sizes[j],
                    iconSize: sizes[j]
                });
            }
        }
        return out;
    }

    // The visible bar slab, sized to the *baseline* content so magnification
    // never grows the reserved strip.
    readonly property var barRect: {
        var base = _baseline;
        if (axisIsX)
            return {
                x: base.positions[0] - padding,
                y: magnifyBand + hideY,
                w: base.total + 2 * padding,
                h: barThickness
            };
        // A vertical Dock's bar hugs its anchored edge, with the transparent
        // magnify band on the interior side (T-10 section 2).
        return {
            x: (position === "right" ? width - barThickness : 0) + hideX,
            y: base.positions[0] - padding,
            w: barThickness,
            h: base.total + 2 * padding
        };
    }

    // The surface input region: the visible bar plus the currently magnified
    // or bouncing icon rectangles. The transparent magnified band passes
    // clicks through to the windows beneath. A hidden auto-hide Dock keeps
    // only the thin edge band so the pointer can summon it back (T-10 FR-13,
    // section 15). The shell commits this every frame.
    readonly property var inputRects: {
        if (autoHide && !revealed)
            return [edgeRect];
        // While dragging, the whole surface keeps pointer input so the drag
        // can move through the magnified band without leaking to a window.
        if (dragging || externalDragActive || resizing)
            return [{ x: 0, y: 0, w: width, h: height }];
        var out = [barRect];
        var l = layout;
        for (var i = 0; i < l.length; ++i) {
            if (items[i].kind === "divider")
                continue;
            var magnified = l[i].iconSize > iconSize + 0.5;
            var bouncing = entryBounce(items[i]) > 0.5;
            if (magnified || bouncing)
                out.push({ x: l[i].x, y: l[i].y, w: l[i].w, h: l[i].h });
        }
        return out;
    }

    // --- Background ------------------------------------------------------
    Rectangle {
        objectName: "dockBar"
        x: dock.barRect.x
        y: dock.barRect.y
        width: dock.barRect.w
        height: dock.barRect.h
        radius: Theme.controls.dock.radius
        color: Theme.color.chrome
        opacity: Theme.material.chromeOpacity
        border.width: 1
        border.color: Theme.color.border
    }

    // --- Entries ---------------------------------------------------------
    Repeater {
        id: entryRepeater
        objectName: "entryRepeater"
        model: dock.items

        delegate: DockEntry {
            required property var modelData
            required property int index

            readonly property bool isDragged:
                dock.dragging && modelData.id === dock.dragEntryId

            entry: modelData
            iconSize: dock.layout.length > index ? dock.layout[index].iconSize : dock.iconSize
            indicatorEdge: dock.indicatorEdge
            showIndicator: dock.showIndicators
            keyboardFocused: dock.keyboardFocused && modelData.id === dock.focusedItemId
            externalDropTarget: dock.externalDragActive
                                && modelData.id === dock.externalTargetId
            dragging: dock.dragging
            lifted: isDragged
            z: isDragged ? 10 : 0
            width: dock.layout.length > index ? dock.layout[index].w : dock.iconSize
            height: dock.layout.length > index ? dock.layout[index].h : dock.iconSize
            x: isDragged ? dock.draggedX(width)
               : (dock.layout.length > index ? dock.layout[index].x : 0)
            y: isDragged ? dock.draggedY(height)
               : (dock.layout.length > index ? dock.layout[index].y : 0)

            // The gap left by a reorder springs open only while dragging;
            // every other layout change (initial configure, magnification,
            // resize) snaps so the on-demand renderer never freezes the Dock
            // mid-animation. Reduced motion (duration 0) snaps.
            Behavior on x {
                enabled: dock.dragging && !isDragged
                NumberAnimation {
                    duration: Theme.motion.dockMagnify.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.dockMagnify.curve
                }
            }
            Behavior on y {
                enabled: dock.dragging && !isDragged
                NumberAnimation {
                    duration: Theme.motion.dockMagnify.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.dockMagnify.curve
                }
            }

            onDragBegan: (entry, sx, sy) => dock.beginDrag(entry)
            onDragMoved: (entry, sx, sy) => dock.updateDrag(entry, sx, sy)
            onDragEnded: (entry, sx, sy) => dock.endDrag(entry, sx, sy)

            onDividerResizeBegan: (sx, sy) => dock.beginDividerResize(sx, sy)
            onDividerResizeMoved: (sx, sy) => dock.updateDividerResize(sx, sy)
            onDividerResizeEnded: () => dock.endDividerResize()

            onActivated: (entry) => {
                // The click tree (T-10 section 8): a running app with more
                // than one window opens the chooser; everything else is a
                // shell activation (single window, minimized restore, launch).
                dock.activateEntry(entry);
            }
            onContextMenuRequested: (entry, gx, gy) => {
                if (entry.kind === "divider") {
                    dock.dividerContextMenuRequested(gx, gy);
                } else {
                    dock.entryContextMenuRequested(entry, gx, gy);
                }
                dock.openEntryMenu(entry);
            }
        }
    }

    // --- Context menu and window chooser (T-10 sections 9/13) -------------
    ContextMenu {
        id: entryMenu
        objectName: "entryMenu"
        model: dock.menuModel
        accessibleName: dock.menuEntry && dock.menuEntry.name !== undefined
                        ? dock.menuEntry.name : qsTr("Dock options")
        // Above the entry on a bottom Dock, beside it on a vertical Dock,
        // clamped to the surface.
        x: {
            if (!dock.menuAnchor)
                return 0;
            if (dock.axisIsX)
                return Math.max(0, Math.min(dock.width - width,
                    dock.menuAnchor.x + (dock.menuAnchor.width - width) / 2));
            // Beside the bar (not the entry), so the popover always clears the
            // bar regardless of the entry's inset.
            return dock.position === "left"
                    ? dock.barRect.x + dock.barRect.w + 4
                    : dock.barRect.x - width - 4;
        }
        y: {
            if (!dock.menuAnchor)
                return 0;
            // A bottom Dock's menu opens upward; negative y is covered by the
            // offscreen scene's headroom (the shell grows the window).
            if (dock.axisIsX)
                return dock.menuAnchor.y - height - 4;
            return Math.max(0, Math.min(dock.height - height,
                dock.menuAnchor.y + (dock.menuAnchor.height - height) / 2));
        }
        onOpened: {
            dock.menuOpen = true;
            dock.popoverChanged();
        }
        onClosed: {
            dock.menuOpen = false;
            dock.popoverChanged();
            dock.scheduleHide();
        }
        onTriggered: (index, item) => {
            if (item.action === "empty_trash_confirmation") {
                // Swap in the confirmation step; `keepOpen` on the menu item
                // keeps the menu up while the model changes.
                dock.trashConfirming = true;
                return;
            }
            if (item.action === "cancel_empty_trash") {
                dock.trashConfirming = false;
                return;
            }
            if (item.action === "empty_trash")
                dock.trashConfirming = false;
            dock.menuActionRequested(item.action, item.payload);
        }
    }

    DockWindowChooser {
        id: windowChooser
        objectName: "windowChooser"
        entry: dock.chooserEntry
        anchorItem: dock.chooserAnchor
        x: {
            if (!dock.chooserAnchor)
                return 0;
            if (dock.axisIsX)
                return Math.max(0, Math.min(dock.width - width,
                    dock.chooserAnchor.x + (dock.chooserAnchor.width - width) / 2));
            return dock.position === "left"
                    ? dock.barRect.x + dock.barRect.w + 4
                    : dock.barRect.x - width - 4;
        }
        y: {
            if (!dock.chooserAnchor)
                return 0;
            // A bottom Dock's chooser opens upward; negative y is covered by
            // the offscreen scene's headroom.
            if (dock.axisIsX)
                return dock.chooserAnchor.y - height - 4;
            return Math.max(0, Math.min(dock.height - height,
                dock.chooserAnchor.y + (dock.chooserAnchor.height - height) / 2));
        }
        onOpened: {
            dock.chooserOpen = true;
            dock.popoverChanged();
        }
        onClosed: {
            dock.chooserOpen = false;
            dock.popoverChanged();
            dock.scheduleHide();
        }
        onWindowActivated: (windowId) => dock.windowActivated(windowId)
        onShowAllWindows: () => dock.menuActionRequested(
            "show_all_windows", { appId: dock.chooserEntry ? dock.chooserEntry.appId : "" })
    }

    // The Downloads stack popover (T-10 section 17). It is anchored and placed
    // exactly like the window chooser; the shell renders it into the Dock's
    // overlay surface and performs the resolved open action.
    DockStackPopover {
        id: stackPopover
        objectName: "stackPopover"
        items: dock.downloadsItems
        anchorItem: dock.stackAnchor
        x: {
            if (!dock.stackAnchor)
                return 0;
            if (dock.axisIsX)
                return Math.max(0, Math.min(dock.width - width,
                    dock.stackAnchor.x + (dock.stackAnchor.width - width) / 2));
            return dock.position === "left"
                    ? dock.barRect.x + dock.barRect.w + 4
                    : dock.barRect.x - width - 4;
        }
        y: {
            if (!dock.stackAnchor)
                return 0;
            if (dock.axisIsX)
                return dock.stackAnchor.y - height - 4;
            return Math.max(0, Math.min(dock.height - height,
                dock.stackAnchor.y + (dock.stackAnchor.height - height) / 2));
        }
        onOpened: {
            dock.stackOpen = true;
            dock.popoverChanged();
        }
        onClosed: {
            dock.stackOpen = false;
            dock.popoverChanged();
            dock.scheduleHide();
        }
        onItemActivated: (path) => dock.downloadActivated(path)
        onOpenFolder: () => dock.downloadsFolderRequested()
    }

    // Clicking empty Dock space dismisses an open popover (T-10 section 13).
    TapHandler {
        onTapped: (eventPoint) => {
            var p = eventPoint.position;
            for (var i = 0; i < dock.layout.length; ++i) {
                var r = dock.layout[i];
                if (p.x >= r.x && p.x <= r.x + r.w
                        && p.y >= r.y && p.y <= r.y + r.h)
                    return;
            }
            dock.closePopovers();
        }
    }

    // A pinned entry dragged off the Dock shows a "Remove" affordance; there
    // is no poof animation (T-10 section 12).
    Text {
        objectName: "dragRemoveLabel"
        visible: dock.dragging && dock.dragOutOfDock
        text: qsTr("Remove")
        color: Theme.color.textPrimary
        font.pixelSize: Theme.controls.button.fontSize
        z: 3000
        x: dock.axisIsX
           ? Math.max(0, Math.min(dock.width - width, dock.dragPointerAlong - width / 2))
           : (dock.position === "right" ? dock.barRect.x - width - 8
                                        : dock.barRect.x + dock.barThickness + 8)
        y: dock.axisIsX
           ? Math.max(0, dock.barRect.y - height - 4)
           : Math.max(0, Math.min(dock.height - height, dock.dragPointerAlong - height / 2))
    }

    // Pointer tracking for magnification. HoverHandlers do not consume
    // events, so the per-entry handlers still work.
    HoverHandler {
        id: dockHover
        onPointChanged: {
            var p = point.position;
            dock.pointerAlong = dock.axisIsX ? p.x : p.y;
            // While hidden the only hit area is the edge band, so a dwell
            // there reveals the Dock after the reveal delay (section 15).
            if (dock.autoHide && !dock.revealed) {
                if (dock.pointInEdgeBand(p.x, p.y))
                    revealTimer.restart();
                else
                    revealTimer.stop();
            } else {
                revealTimer.stop();
            }
            if (dock.autoHide && dock.revealed)
                hideTimer.stop();
        }
        onHoveredChanged: {
            if (hovered) {
                hideTimer.stop();
            } else {
                dock.pointerAlong = -1;
                revealTimer.stop();
                // A drag that leaves the surface is a remove (pinned) or a
                // snap-back (T-10 section 12).
                if (dock.dragging)
                    dock.dragPointerLeft();
                if (dock.autoHide && dock.revealed)
                    hideTimer.restart();
            }
        }
    }

    // Test/introspection hooks.
    function itemAt(index) { return entryRepeater.itemAt(index); }
    function itemCount() { return entryRepeater.count; }
}
