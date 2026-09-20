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
// Context menus, the window chooser, drag rearrangement, launch, and the
// Trash state/menu are landed (see PROGRESS.md); external drops and the
// scene-graph render path are follow-ups.
Rectangle {
    id: dock

    // The surface includes the transparent magnified-band above/beside the bar
    // (section 2), so the root must not paint; only `dockBar` and the entries
    // draw. A default-color Rectangle here would render the band as an opaque
    // white strip.
    color: "transparent"

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
    // The Trash menu's Empty Trash confirmation step (section 13): selecting
    // Empty Trash replaces the menu model with the confirm/cancel choice
    // before the shell performs the destructive operation.
    property bool trashConfirming: false

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

    // The app entry whose context menu / window chooser is open, plus the
    // entry item each is anchored to. Only one popover is open at a time.
    property var menuEntry: null
    property bool menuOpen: false
    property Item menuAnchor: null
    property var chooserEntry: null
    property bool chooserOpen: false
    property Item chooserAnchor: null

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

    // --- Geometry constants ---------------------------------------------
    readonly property real padding: Theme.controls.dock.padding
    readonly property real gap: Theme.controls.dock.gap
    readonly property real dividerWidth: 1
    readonly property real barThickness: iconSize + 2 * padding
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
    readonly property real hideOffset:
        autoHide && !revealed ? barThickness + Theme.controls.dock.edgeMargin : 0
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
        if (!revealed)
            revealed = true;
    }
    function hide() {
        hideTimer.stop();
        if (autoHide && revealed)
            revealed = false;
    }

    // Re-hide only when nothing holds the Dock open: no popover, no drag, and
    // the pointer has left the surface (T-10 section 15).
    function hideIfIdle() {
        if (autoHide && !popoverOpen && !dragging && !dockHover.hovered)
            hide();
    }

    // Schedule a re-hide when a popover closes and the pointer is elsewhere.
    function scheduleHide() {
        if (autoHide && revealed && !dockHover.hovered)
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
        running: false, trashFull: dock.trashFull, trashCount: dock.trashCount
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
    readonly property var items: {
        var out = appEntries.slice();
        out.push(dividerEntry);
        for (var i = 0; i < minimizedEntries.length; ++i)
            out.push(minimizedEntries[i]);
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
            if (items[i].kind === "divider")
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
        && !popoverOpen && !dragging && revealed

    // Magnification is suppressed while a context menu or chooser is open
    // (T-10 section 14).
    readonly property bool popoverOpen: menuOpen || chooserOpen

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
                && entry.kind !== "minimized";
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
    // Windows, Keep/Remove from Dock, and Quit/Open. The divider menu holds
    // the Dock options; the Trash menu holds Open and Empty Trash. Options ▸
    // (Assign To, Open at Login, Show in Files) and "Show in Files" are later
    // slices (they need the design-system submenu and T-18 Files).
    readonly property var menuModel: {
        var e = menuEntry;
        if (!e)
            return [];
        var out = [];
        if (e.kind === "divider")
            return dividerMenuModel();
        if (e.kind === "trash")
            return trashMenuModel();
        var running = e.running === true;
        var list = e.windowList !== undefined ? e.windowList : [];
        if (running && list.length > 0) {
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

    // The divider menu (T-10 section 13): the magnification and hiding
    // toggles and the Settings entry point. Position-on-screen (T-10 section
    // 5) is not wired into this menu yet; Dock Settings is T-16.
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

    // The open popover's rectangle in Dock-scene coordinates, or an empty
    // rect. The shell renders it into the Dock's `overlay` surface.
    readonly property var popoverRect: {
        // `open` keeps the rect valid while the popover fades in/out (the
        // shell captures the animation); `visible` covers the close tail.
        var popup = (entryMenu.open || entryMenu.visible) ? entryMenu
                 : ((windowChooser.open || windowChooser.visible) ? windowChooser : null);
        if (!popup || popup.width <= 0 || popup.height <= 0)
            return { x: 0, y: 0, w: 0, h: 0 };
        var topLeft = popup.mapToItem(dock, 0, 0);
        return { x: topLeft.x, y: topLeft.y, w: popup.width, h: popup.height };
    }

    function indicatorSpace(entry) {
        return showIndicators && entry.running
                ? gap + Theme.controls.dock.indicatorSize : 0;
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
                if (list[i].kind === "divider")
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
            var extra = isDivider ? 0 : indicatorSpace(list[j]);
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
        if (dragging)
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

            onActivated: (entry) => {
                // The click tree (T-10 section 8): a running app with more
                // than one window opens the chooser; everything else is a
                // shell activation (single window, minimized restore, launch).
                if (entry.running === true && entry.kind !== "minimized"
                        && entry.windowList !== undefined
                        && entry.windowList.length > 1) {
                    dock.openChooser(entry);
                    return;
                }
                dock.entryActivated(entry);
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
            return dock.position === "left"
                    ? dock.menuAnchor.x + dock.menuAnchor.width + 4
                    : dock.menuAnchor.x - width - 4;
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
                    ? dock.chooserAnchor.x + dock.chooserAnchor.width + 4
                    : dock.chooserAnchor.x - width - 4;
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
