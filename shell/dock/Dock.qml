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
// PROGRESS.md); the scene-graph render path (FR-14) commits every frame.
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
    // A new projection arrived (T-14.7m): keep the open window chooser bound
    // to its app's fresh entry/rows (or dismiss it when the app is gone). The
    // dependent `items` binding and the entry Repeater settle after this
    // signal, so refresh on the next event-loop turn.
    onEntriesChanged: Qt.callLater(refreshPopoversAfterProjection)
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
    // Opt-in minimize-to-icon reaction (T-14.7s): a bounded one-hop bounce on
    // the acting app's entry when one of its windows minimizes. Default off;
    // the shell only publishes a `minimize` phase when the key is on, and the
    // reduced-motion gate below removes the translation.
    property bool minimizeReaction: false
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
    // The folder's display name (its basename), so the hover label and the
    // popover header name the folder without the artwork carrying text
    // (T-14.7h); the Downloads special case just uses "Downloads".
    property string downloadsName: qsTr("Downloads")
    // The Downloads stack's absolute path, so the default member can be told
    // apart from a user folder pin (T-14.7k).
    property string downloadsPath: ""
    // The user's pinned folder stacks (T-14.7k), injected by the shell: each
    // is `{ id, path, name, items, count, badge, missing }`. They render in
    // the stacks region immediately before the Trash through the same widget
    // and geometry tokens as the Downloads stack.
    property var folderPins: []
    // The Trash menu's Empty Trash confirmation step (section 13): selecting
    // Empty Trash replaces the menu model with the confirm/cancel choice
    // before the shell performs the destructive operation.
    property bool trashConfirming: false
    // The Empty Trash operation's visible state (T-14.7r). The shell owns the
    // asynchronous operation; the Dock renders one phase at a time in a
    // dedicated popover anchored to the Trash entry. `idle` means no operation
    // is showing. `trashBusyVisible` gates the busy indicator behind a short
    // delay so a fast empty never flickers through it (the shell reports the
    // one-shot result via `handleTrashEmptyResult`).
    property string trashEmptyPhase: "idle"      // idle | emptying | succeeded | failed
    property bool trashBusyVisible: false
    property int trashEmptyRemoved: 0
    property string trashEmptyError: ""
    property bool trashEmptyOpen: false
    property Item trashEmptyAnchor: null

    // The fixed popover buffer budget (T-14.7c). The shell pre-sizes the
    // offscreen render target to this and the Dock clamps every popover into
    // it, so opening a menu/chooser/stack never resizes the window (and thus
    // never reallocates the render target on the open frame).
    property real popoverHeadroom: 0
    property real popoverGutter: 0

    // Per-entry launch/attention phases keyed by entry id (T-14.7c). The
    // shell updates this map every committed frame instead of rebuilding
    // `entries`, so the Repeater model — and every live delegate's hover and
    // press state — survives a bounce. Each value is
    // `{ phase: 0..1, attention: bool }` and, for a minimize reaction,
    // `{ minimize: true, minimizePhase: 0..1 }` (T-14.7s).
    property var bouncePhases: ({})

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
    // The pointer position the magnification geometry actually reads (T-14.7b).
    // It tracks `pointerAlong` but is smoothed with `motion.dockMagnify` while
    // magnifying, so a coarse pointer sampling reads as a continuous slide. It
    // snaps (tracks raw) when magnification is off and under reduced motion.
    property real smoothPointerAlong: pointerAlong
    // The pointer geometry reads `smoothPointerAlong`. It snaps while the
    // pointer enters/leaves the Dock, magnification is off, or reduced motion
    // is on, and springs between two real Dock positions while magnifying —
    // `motion.dockMagnify`'s bezier already carries the slight overshoot. The
    // decision is made imperatively from the *previous* smoothed value, so
    // there is no binding cycle between the flag and the value it gates.
    NumberAnimation {
        id: smoothPointerAnimation
        target: dock
        property: "smoothPointerAlong"
        duration: Theme.motion.dockMagnify.duration
        easing.type: Easing.Bezier
        easing.bezierCurve: Theme.motion.dockMagnify.curve
    }
    onPointerAlongChanged: {
        if (dock.magnifying && !Theme.reducedMotion && dock.smoothPointerAlong >= 0) {
            smoothPointerAnimation.from = dock.smoothPointerAlong;
            smoothPointerAnimation.to = dock.pointerAlong;
            smoothPointerAnimation.restart();
        } else {
            smoothPointerAnimation.stop();
            dock.smoothPointerAlong = dock.pointerAlong;
        }
    }
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
    property bool dragIsFolderPin: false
    property string dragFolderPath: ""
    property var dragBaseCenters: []
    property var dragOriginalPinnedIds: []

    // Discrete layout changes (`dock.size`, overflow, popovers) animate with
    // the design-system spring; the very first configure snaps so the initial
    // placement is exact and the idle shell never freezes mid-animation
    // (T-14.7c). Magnification stays progress-based (`magnifying` disables the
    // per-entry Behaviors).
    property bool laidOutOnce: false
    Behavior on iconSize {
        enabled: dock.laidOutOnce && !dock.resizing
        NumberAnimation {
            duration: Theme.motion.dockMagnify.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: Theme.motion.dockMagnify.curve
        }
    }
    // A drag that ends (drop or cancel) lets the opened gap close with the
    // same spring instead of snapping.
    property bool dragSettling: false
    Component.onCompleted: dock.laidOutOnce = dock.width > 0 && dock.height > 0
    onWidthChanged: if (!dock.laidOutOnce) layoutSettleTimer.restart()
    onHeightChanged: if (!dock.laidOutOnce) layoutSettleTimer.restart()
    onDraggingChanged: {
        if (dock.dragging) {
            dragSettleTimer.stop();
            dock.dragSettling = false;
            dock.hideTooltip();
        } else {
            dock.dragSettling = true;
            dragSettleTimer.restart();
        }
    }
    Timer {
        id: layoutSettleTimer
        interval: 0
        onTriggered: dock.laidOutOnce = dock.width > 0 && dock.height > 0
    }
    Timer {
        id: dragSettleTimer
        interval: Theme.motion.dockMagnify.fullDuration + 40
        onTriggered: dock.dragSettling = false
    }

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
    // A single dropped folder (T-14.7k), pinned as a stack rather than opened.
    property bool externalPayloadIsFolder: false
    property int externalPayloadCount: 0
    // The id of the entry under the drag pointer ("" = none).
    property string externalTargetId: ""
    // Insertion index in the app region for an application-alias drop; the
    // live gap reflows the layout around a placeholder entry.
    property int externalInsertIndex: -1
    // The resolved identity of the dragged payload (T-14.7f): an app alias's
    // real name/icon from app-index, or the file count and first file's name.
    // Filled by the shell once the enter-time read completes.
    property string externalPayloadName: ""
    property string externalPayloadIconPath: ""
    property string externalPayloadFirstName: ""
    // The pinned entry a duplicate app-alias drop pulsed ("" = none). It
    // drives a brief highlight so the no-op is visible (T-14.7f).
    property string duplicateFlashId: ""
    // A stack entry the pointer has dwelled over long enough to spring-load
    // (T-10 section 17); the Downloads stack popover opens when it fires.
    property string springLoadTargetId: ""
    // Spring-loading hover delay (ms), shared with Files (T-18). Mirrors
    // `kSpringLoadMs` in the shell's pure drop core.
    readonly property int springLoadDelay: 500
    // How long a duplicate-pin highlight stays visible (T-14.7f).
    readonly property int duplicateFlashMs: 700
    // The double-click window for a folder stack (T-14.7h). The first tap
    // opens the popover immediately; a second tap inside this window opens the
    // folder in Files.
    readonly property int stackDoubleClickMs: 400
    property real lastStackTapTime: 0

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
    // The stack entry whose popover is open (T-14.7k); drives the popover's
    // items/title and tells the Downloads default from a user pin.
    property var stackEntryRef: null
    readonly property var stackPopoverItems: {
        if (stackEntryRef !== null && stackEntryRef.id !== undefined
                && stackEntryRef.id !== "__downloads__")
            return stackEntryRef.items !== undefined ? stackEntryRef.items : [];
        return downloadsItems;
    }
    readonly property string stackPopoverTitle: {
        if (stackEntryRef !== null && stackEntryRef.id !== undefined
                && stackEntryRef.id !== "__downloads__"
                && stackEntryRef.name !== undefined && stackEntryRef.name.length > 0)
            return stackEntryRef.name;
        return downloadsName;
    }
    // The Add Application picker (T-14.7e, ADR 0090): the app-index corpus the
    // shell builds with the pure `buildAppPickerList` helper, whether the
    // service is reachable, and the open/anchor state.
    property var appPickerItems: []
    property bool appIndexAvailable: true
    property bool appPickerOpen: false
    property Item appPickerAnchor: null

    // The terminal overflow cell's "More Windows" list (T-14.7q). The cell is
    // a model entry (`kind: "overflow"`); the popover lists its hidden running
    // groups and either activates a single window or opens the chooser for a
    // grouped app, anchored to the cell.
    property var overflowEntryRef: null
    property bool overflowOpen: false
    property Item overflowAnchor: null
    readonly property var overflowGroups:
        overflowEntryRef !== null && overflowEntryRef.groups !== undefined
            ? overflowEntryRef.groups : []

    // --- Hover name label (T-14.7i, ADR 0093) ---------------------------
    // The entry the pointer is dwelling over and the delegate it anchors to.
    // The Tooltip follows the delegate, so the label tracks a magnified entry
    // frame by frame; the dwell timer and all suppression live here, not in
    // the design-system component, which stays passive.
    property var tooltipEntry: null
    property Item tooltipAnchor: null
    property bool tooltipOpen: false
    readonly property int tooltipDwell: Theme.controls.tooltip.dwell
    // "above" for a bottom Dock; the interior side for a vertical Dock so the
    // label never crosses the screen edge.
    readonly property string tooltipPlacement:
        position === "bottom" ? "above" : (position === "left" ? "right" : "left")
    readonly property string tooltipText:
        tooltipAnchor !== null && tooltipAnchor.tooltipLabel !== undefined
            ? tooltipAnchor.tooltipLabel : ""

    Timer {
        id: tooltipDwellTimer
        interval: dock.tooltipDwell
        onTriggered: dock.showTooltip()
    }

    // --- Hover-open window chooser (T-14.7p) ----------------------------
    // When `chooserOnHover` is on, dwelling on a grouped app entry opens its
    // window chooser; moving along the Dock retargets the same popover to the
    // newly hovered grouped entry instead of closing and reopening. The chooser
    // is anchored to a snapshot proxy (below), not the live delegate, so a
    // projection rebuild never orphans it. Default off: the shipping macOS
    // click contract is unchanged.
    property bool chooserOnHover: false
    readonly property int chooserHoverDwell: Theme.controls.dock.chooser.hoverDwell
    readonly property int chooserHoverCloseDelay:
        Theme.controls.dock.chooser.hoverCloseDelay
    // The grouped entry the pointer is dwelling toward, and whether the open
    // chooser was hover-opened (a click-opened chooser is never closed by a
    // pointer leave).
    property var chooserHoverEntry: null
    property bool chooserHoverOpened: false

    // The geometry snapshot the chooser anchors to (T-14.7p). It captures the
    // target delegate's rect in the Dock's coordinates and outlives that
    // delegate, so a Repeater rebuild cannot destroy the anchor. It never
    // allocates or resizes the Dock surface: it is a geometry proxy only.
    Item {
        id: chooserAnchorProxy
        objectName: "chooserAnchorProxy"
        visible: false
        width: 0
        height: 0

        function capture(entryItem) {
            if (!entryItem)
                return;
            var topLeft = entryItem.mapToItem(dock, 0, 0);
            chooserAnchorProxy.x = topLeft.x;
            chooserAnchorProxy.y = topLeft.y;
            chooserAnchorProxy.width = entryItem.width;
            chooserAnchorProxy.height = entryItem.height;
        }

        // Re-parent the proxy to force the chooser's anchored bindings and the
        // arrow's `mapToItem` to re-evaluate after a delegate rebuild.
        function reposition() {
            var previous = chooserAnchorProxy.parent;
            chooserAnchorProxy.parent = null;
            chooserAnchorProxy.parent = previous;
        }
    }

    // The geometry snapshot the Trash empty popover anchors to (T-14.7r), the
    // same outlive-a-rebuild pattern as `chooserAnchorProxy`: an empty changes
    // the Trash state, which rebuilds its delegate, so the popover must not
    // hold a live delegate.
    Item {
        id: trashEmptyAnchorProxy
        objectName: "trashEmptyAnchorProxy"
        visible: false
        width: 0
        height: 0

        function capture(entryItem) {
            if (!entryItem)
                return;
            var topLeft = entryItem.mapToItem(dock, 0, 0);
            trashEmptyAnchorProxy.x = topLeft.x;
            trashEmptyAnchorProxy.y = topLeft.y;
            trashEmptyAnchorProxy.width = entryItem.width;
            trashEmptyAnchorProxy.height = entryItem.height;
        }

        function reposition() {
            var previous = trashEmptyAnchorProxy.parent;
            trashEmptyAnchorProxy.parent = null;
            trashEmptyAnchorProxy.parent = previous;
        }
    }

    Timer {
        id: chooserHoverTimer
        interval: dock.chooserHoverDwell
        onTriggered: dock.openChooserOnHover()
    }
    Timer {
        id: chooserCloseTimer
        interval: dock.chooserHoverCloseDelay
        onTriggered: dock.closeHoverChooser()
    }

    // A grouped app entry is a hover-open target: running, not a minimized
    // per-window row, and with more than one window. Single-window entries are
    // explicitly out of scope (activate-on-hover is a different interaction).
    function chooserEligible(entry) {
        return dock.chooserOnHover && entry
            && entry.running === true && entry.kind !== "minimized"
            && entry.windowList !== undefined && entry.windowList.length > 1;
    }

    function stopChooserHover() {
        chooserHoverTimer.stop();
        chooserCloseTimer.stop();
        chooserHoverEntry = null;
        chooserHoverOpened = false;
    }

    // The pointer dwelled onto an entry. A grouped entry starts the chooser
    // dwell; everything else starts the hover name label, unless a hover-opened
    // chooser is open, in which case the pointer giving way starts its close
    // delay. Drag, external drag, resize, keyboard navigation, and another
    // popover all suppress the hover-open path.
    function entryHoverBegan(entry, entryItem) {
        if (!entry || entry.kind === "divider" || entry.kind === "external") {
            if (chooserOpen && chooserHoverOpened)
                chooserCloseTimer.restart();
            return;
        }
        if (dragging || externalDragActive || resizing)
            return;
        // The chooser itself is the one popover a hover may retarget; any other
        // popover owns the stage.
        if (popoverOpen && !chooserOpen)
            return;
        if (keyboardFocused)
            return;

        if (chooserEligible(entry)) {
            // One Tooltip at a time: the chooser suppresses the name label.
            hideTooltip();
            chooserCloseTimer.stop();
            if (chooserOpen && chooserHoverEntry === entry)
                return;
            chooserHoverEntry = entry;
            chooserHoverTimer.restart();
            return;
        }

        // Not a chooser target: a hover-opened chooser closes after the delay,
        // so releasing onto a single-window entry dismisses it cleanly.
        if (chooserHoverOpened) {
            chooserHoverEntry = null;
            chooserCloseTimer.restart();
        }
        if (chooserOpen)
            return;
        if (tooltipAnchor === entryItem && (tooltipOpen || tooltipDwellTimer.running))
            return;
        hideTooltip();
        tooltipEntry = entry;
        tooltipAnchor = entryItem;
        tooltipDwellTimer.restart();
    }

    // The pointer left an entry. A leave from an entry that is no longer the
    // anchor is a move to another entry and must not clear the new label. A
    // hover-opened chooser survives the gap between entries on the close delay.
    function entryHoverEnded(entryItem) {
        if (chooserOpen && chooserHoverOpened)
            chooserCloseTimer.restart();
        if (entryItem !== undefined && entryItem !== tooltipAnchor)
            return;
        hideTooltip();
    }

    // The dwell fired: open the chooser on a grouped entry, or retarget the
    // already-open popover to the newly hovered one without a close/reopen
    // flash. A late drag/resize/keyboard/navigation cancels.
    function openChooserOnHover() {
        var entry = chooserHoverEntry;
        if (!entry)
            return;
        if (dragging || externalDragActive || resizing || keyboardFocused)
            return;
        if (popoverOpen && !chooserOpen)
            return;
        if (chooserOpen) {
            retargetChooser(entry);
            chooserHoverOpened = true;
            return;
        }
        openChooser(entry, true);
    }

    function closeHoverChooser() {
        chooserHoverEntry = null;
        if (chooserOpen && chooserHoverOpened)
            windowChooser.hide();
        chooserHoverOpened = false;
    }

    // Re-anchor the open chooser to `entry` without closing it (T-14.7p).
    function retargetChooser(entry) {
        if (!chooserOpen || !entry)
            return;
        var idx = indexOfItemId(entry.id);
        chooserAnchorProxy.capture(idx >= 0 ? entryRepeater.itemAt(idx) : null);
        chooserAnchorProxy.reposition();
        chooserEntry = entry;
        chooserHoverEntry = entry;
    }

    function showTooltip() {
        if (!tooltipAnchor || tooltipText.length === 0 || tooltipOpen)
            return;
        if (dragging || externalDragActive || resizing || popoverOpen)
            return;
        tooltipOpen = true;
        popoverChanged();
    }

    // Close the label but keep its text and anchor until the fade finishes, so
    // the capsule does not empty mid-animation; the Tooltip clears them when it
    // becomes invisible.
    function hideTooltip() {
        tooltipDwellTimer.stop();
        if (tooltipOpen) {
            tooltipOpen = false;
            popoverChanged();
        }
        // If the capsule never appeared (or has finished fading), release the
        // anchor now; otherwise the Tooltip clears it when it turns invisible.
        if (!dockTooltip.visible) {
            tooltipAnchor = null;
            tooltipEntry = null;
        }
    }

    signal entryActivated(var entry)
    signal entryContextMenuRequested(var entry, real globalX, real globalY)
    signal dividerContextMenuRequested(real globalX, real globalY)
    signal settingsRequested()
    // The Add Application picker (T-14.7e): the divider menu asks the shell to
    // refresh the app-index corpus; a row toggle asks the shell to write
    // `dock.pinned` (the single writer path).
    signal appPickerRequested()
    signal appPinToggled(string desktopId, bool pinned)
    // A context-menu/chooser action resolved to a shell operation.
    signal menuActionRequested(string action, var payload)
    // A specific window chosen from the window chooser (T-10 FR-5).
    signal windowActivated(string windowId)
    // The chooser's per-window actions (T-14.7m): close one window, or
    // minimize/restore it to `minimized`. The chooser stays open; the shell
    // performs the compositor round-trip and the next projection refreshes
    // the rows.
    signal windowCloseRequested(string windowId)
    signal windowMinimizeRequested(string windowId, bool minimized)
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
    // A pinned folder stack (T-14.7k): a row was chosen, the folder itself was
    // opened, the popover was viewed (clear its badge), or the pin was removed.
    signal folderOpenRequested(string path)
    signal folderViewed(string path)
    signal folderPinRemoved(string path)
    // Escape asked to leave Dock keyboard navigation; the shell releases the
    // compositor keyboard focus back to the active window (T-10 section 20).
    signal keyboardFocusReleaseRequested()
    // The acted-on entry's icon tile in output coordinates (T-14.7l), so the
    // shell can hand it to the compositor before launching and the new window
    // appears from (and minimizes/restores into) the real icon. One rect, not
    // a stream; re-emitted only when a settled re-layout moves it.
    signal entryTileRect(string desktopId, real x, real y, real w, real h)

    // --- Geometry constants ---------------------------------------------
    // `padding` is the cross-axis inset (artwork <-> plate edge on the side
    // away from the anchored edge); `paddingAlong` is the along-axis inset at
    // the plate's two ends (T-14.7a). `edgeMargin` floats the plate off its
    // anchored screen edge.
    readonly property real padding: Theme.controls.dock.padding
    readonly property real paddingAlong: Theme.controls.dock.paddingAlong
    readonly property real gap: Theme.controls.dock.gap
    // The over-sized room on each side of a region divider (T-14.7v). It
    // replaces the icon `gap` at a divider boundary so a rule reads as a
    // region break rather than as one more icon.
    readonly property real dividerGap: Theme.controls.dock.divider.gap
    readonly property real edgeMargin: Theme.controls.dock.edgeMargin
    readonly property real dividerWidth: 1
    // Room reserved beyond every entry's artwork for a running indicator on the
    // anchored-edge side. It is reserved for all entries, not only running
    // ones, so running and idle icons share one baseline (a per-running
    // reservation lifts the running icons, which reads as a magnification
    // bump).
    //
    // T-14.7u: the indicator lives *inside* the cross-axis padding rather than
    // in a band added on top of it. `barThickness` is one icon plus two
    // paddings on every side, and `indicatorSpace` (dot + gap) must not exceed
    // `padding`, so the dot sits `indicatorGap` below the artwork while the
    // artwork keeps the same inset from both plate edges.
    readonly property real indicatorSpace:
        showIndicators ? Theme.controls.dock.indicatorGap + Theme.controls.dock.indicatorSize : 0
    readonly property real barThickness: iconSize + 2 * padding
    // `dock.magnification` (0..1, 0 = off) maps onto the peak icon factor;
    // 0.5 (the default) lands on the `magnifyPeak` token (T-10 section 19).
    readonly property real magnifyPeakFactor:
        magnification <= 0 ? 1.0
        : 1 + magnification * (Theme.controls.dock.magnifyPeakMax - 1)
    readonly property real magnifyFalloff: Theme.controls.dock.magnifyFalloff
    // How far a lifted (dragged) entry rises above the bar.
    readonly property real dragLift: 8
    // Transparent room above/beside the plate that magnified artwork grows
    // into. It is sized for the maximum magnification so a live
    // `dock.magnification` change never needs to grow the scene.
    readonly property real magnifyBand:
        Math.ceil((Theme.controls.dock.magnifyPeakMax - 1) * iconSize) + padding
    // The perpendicular extent of the whole layer surface: the plate, the
    // interior magnify band, and the edge gap. The shell sizes the surface
    // from `surfaceThickness` and reserves `reservedThickness` (the resting
    // plate plus its edge margin), so windows and Zoom never underlap the
    // floating plate (ADR 0089). Both are derived here, never in the shell,
    // so the plate math has one source of truth.
    readonly property real surfaceThickness: barThickness + magnifyBand + edgeMargin
    readonly property real reservedThickness: barThickness + edgeMargin
    readonly property bool axisIsX: position === "bottom"
    readonly property string indicatorEdge:
        position === "bottom" ? "bottom" : (position === "left" ? "left" : "right")
    readonly property real axisLength: axisIsX ? width : height
    // The thin sliver of the surface that stays interactive while the Dock is
    // hidden, so a pointer reaching the output edge can summon it back
    // (T-10 section 15).
    readonly property real edgeTrigger: Theme.controls.dock.edgeTrigger
    // The compositor-space origin of this surface's (0, 0) (T-14.7l). The
    // shell derives it from the primary output geometry and the Dock edge; the
    // tile rect reported to the shell adds it so the rect is already in the
    // compositor's coordinates. Zero until the output is known.
    property real outputOriginX: 0
    property real outputOriginY: 0

    // The last entry whose tile was reported, and the rect as reported, so a
    // settled re-layout can re-emit only when the geometry actually moved
    // (T-14.7l).
    property string tileEntryId: ""
    property string tileDesktopId: ""
    property var lastTileRect: null

    function launchIdentity(entry) {
        if (!entry)
            return "";
        if (entry.desktopId !== undefined && entry.desktopId !== "")
            return String(entry.desktopId);
        return String(entry.id);
    }

    function entryTileRectFor(entry) {
        var idx = indexOfItemId(entry.id);
        if (idx < 0 || idx >= layout.length)
            return null;
        var r = layout[idx];
        if (r.w <= 0 || r.h <= 0)
            return null;
        return { x: outputOriginX + r.x, y: outputOriginY + r.y, w: r.w, h: r.h };
    }

    // Report the acted-on entry's tile (T-14.7l). The shell remembers it and
    // hands it to the compositor at launch.
    function publishEntryTile(entry) {
        var rect = entryTileRectFor(entry);
        if (!rect)
            return;
        tileEntryId = entry.id;
        tileDesktopId = launchIdentity(entry);
        lastTileRect = rect;
        entryTileRect(tileDesktopId, rect.x, rect.y, rect.w, rect.h);
    }

    // Re-report the remembered tile when a settled layout moved it. Suppressed
    // while magnifying (the continuous follow would be a stream); the rest
    // state republishes once magnification ends.
    function republishEntryTile() {
        if (tileEntryId === "" || lastTileRect === null || magnifying || dragging)
            return;
        var idx = indexOfItemId(tileEntryId);
        if (idx < 0 || idx >= layout.length)
            return;
        var r = layout[idx];
        var nx = outputOriginX + r.x;
        var ny = outputOriginY + r.y;
        if (r.w <= 0 || r.h <= 0)
            return;
        if (nx === lastTileRect.x && ny === lastTileRect.y
                && r.w === lastTileRect.w && r.h === lastTileRect.h)
            return;
        lastTileRect = { x: nx, y: ny, w: r.w, h: r.h };
        entryTileRect(tileDesktopId, nx, ny, r.w, r.h);
    }

    onLayoutChanged: republishEntryTile()

    // --- Auto-hide translation ------------------------------------------
    // The plate is translated off its anchored edge by the surface thickness
    // that lies between the plate and that edge (its own thickness plus the
    // edge margin). `hideOffset` is the magnitude; `hideX`/`hideY` are the
    // per-axis translation for the configured position (T-10 sections 5/15).
    //
    // The slide is animated (FR-14): the scene-graph commit path delivers
    // every frame, so the reveal/hide is a real motion instead of the former
    // snap. Reduced motion collapses the duration to 0 in the token, which
    // makes the transition instant.
    property real hideOffset:
        autoHide && !revealed ? barThickness + edgeMargin : 0
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
    // slide is animated through the scene-graph commit path (FR-14), which
    // delivers every frame.
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
            if (k === "pinned" || k === "temporary" || k === "recent" || k === "overflow")
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
        id: "__downloads__", appId: "", name: dock.downloadsName, kind: "stack",
        running: false, stackCount: dock.downloadsCount, badge: dock.downloadsBadge,
        path: dock.downloadsPath, items: dock.downloadsItems, canRemove: false,
        missing: false
    })
    // The user's pinned folder stacks (T-14.7k), normalized to the same entry
    // shape as `stackEntry`. They are removable (drag out or context menu),
    // unlike the built-in Downloads member.
    readonly property var folderEntries: {
        var out = [];
        for (var i = 0; i < folderPins.length; ++i) {
            var p = folderPins[i];
            if (!p || p.path === undefined)
                continue;
            out.push({
                id: p.id !== undefined ? p.id : ("folder:" + p.path),
                appId: "", name: p.name !== undefined ? p.name : "",
                kind: "stack", running: false,
                path: p.path,
                stackCount: p.count !== undefined ? p.count : 0,
                badge: p.badge !== undefined ? p.badge : 0,
                missing: p.missing === true,
                items: p.items !== undefined ? p.items : [],
                canRemove: true
            });
        }
        return out;
    }
    readonly property var fixedEntries: {
        // The fixed right region: the built-in Downloads stack, the user
        // folder pins, and the permanent Trash. It is never empty (the Trash
        // is always present), so it is the anchor for the leading rule.
        var out = [stackEntry];
        for (var f = 0; f < folderEntries.length; ++f)
            out.push(folderEntries[f]);
        out.push(trashEntry);
        return out;
    }
    // The resize-handle divider (T-10 section 5): the boundary between the app
    // region and the right (minimized/stacks/Trash) region. It is the only
    // divider that carries the drag handle. The pinned | temporary/recent rule
    // and the minimized | stacks/Trash rule are non-interactive markers
    // (T-14.7v).
    readonly property var dividerEntry: ({
        id: "__divider__", kind: "divider", resizeHandle: true
    })
    readonly property var pinnedDividerEntry: ({
        id: "__divider_pinned__", kind: "divider", resizeHandle: false
    })
    readonly property var minimizedDividerEntry: ({
        id: "__divider_minimized__", kind: "divider", resizeHandle: false
    })

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

    // Ordered items: the app region (split at the pinned prefix), the
    // minimized windows, and the fixed stacks/Trash tail, with a divider
    // between each pair of adjacent non-empty regions (T-14.7v). The order is
    // stable during a drag (the Repeater must not be reset while a DragHandler
    // holds the pointer); the drag gap is applied in `layout` instead.
    //
    // An application-alias external drop has no delegate holding the pointer,
    // so it reflows safely: a placeholder entry opens a real gap at the
    // insertion index (section 12).
    readonly property bool externalGap:
        externalDragActive && (externalPayloadIsApp || externalPayloadIsFolder)
        && externalInsertIndex >= 0
    readonly property var externalPlaceholderEntry: ({
        id: "__external_drop__", kind: "external",
        name: externalPayloadName,
        appId: externalPayloadIsApp ? externalPayloadName : "",
        iconPath: externalPayloadIconPath,
        externalFolder: externalPayloadIsFolder,
        running: false
    })
    readonly property var items: {
        var appSeq = appEntries.slice();
        if (externalGap) {
            var idx = Math.max(0, Math.min(appSeq.length, externalInsertIndex));
            appSeq.splice(idx, 0, externalPlaceholderEntry);
        }
        // Split at the pinned prefix so the pinned | temporary/recent boundary
        // can carry its own rule. A placeholder inserted inside the prefix
        // stays with it.
        var split = pinnedCount;
        if (externalGap && externalInsertIndex <= pinnedCount)
            split = Math.min(appSeq.length, pinnedCount + 1);
        var pinnedPart = appSeq.slice(0, split);
        var tailPart = appSeq.slice(split);

        var groups = [];
        if (pinnedPart.length > 0)
            groups.push({ region: "pinned", list: pinnedPart });
        if (tailPart.length > 0)
            groups.push({ region: "tail", list: tailPart });
        if (minimizedEntries.length > 0)
            groups.push({ region: "minimized", list: minimizedEntries });
        groups.push({ region: "fixed", list: fixedEntries });

        var out = [];
        var resizeAssigned = false;
        for (var g = 0; g < groups.length; ++g) {
            if (g > 0) {
                var prevApp = groups[g - 1].region === "pinned"
                              || groups[g - 1].region === "tail";
                var curApp = groups[g].region === "pinned"
                             || groups[g].region === "tail";
                if (prevApp && !curApp && !resizeAssigned) {
                    // The app region meets the right region: the resize handle.
                    out.push(dividerEntry);
                    resizeAssigned = true;
                } else if (groups[g - 1].region === "pinned"
                           && groups[g].region === "tail") {
                    out.push(pinnedDividerEntry);
                } else {
                    out.push(minimizedDividerEntry);
                }
            }
            for (var k = 0; k < groups[g].list.length; ++k)
                out.push(groups[g].list[k]);
        }
        return out;
    }

    // The index in `items` of each app-region entry, in `appEntries` order.
    // An inserted divider shifts the tail entries, so drag/cursor math that
    // speaks in `appEntries` indices must map through this (T-14.7v).
    readonly property var appItemIndices: {
        var out = [];
        for (var i = 0; i < items.length; ++i) {
            var k = items[i].kind;
            if (k === "pinned" || k === "temporary" || k === "recent" || k === "overflow")
                out.push(i);
        }
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

    // The gap between two adjacent items: a divider boundary gets the
    // over-sized `dividerGap`, every icon pair the icon `gap` (T-14.7v).
    function gapBetween(i, j) {
        return (items[i].kind === "divider" || items[j].kind === "divider")
               ? dividerGap : gap;
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
                total += gapBetween(i, i + 1);
        }
        var start = (axisLength - total) / 2;
        var positions = [];
        var centers = [];
        var cursor = start;
        for (i = 0; i < n; ++i) {
            positions.push(cursor);
            centers.push(cursor + sizes[i] / 2);
            cursor += sizes[i] + (i < n - 1 ? gapBetween(i, i + 1) : 0);
        }
        return { sizes: sizes, positions: positions, centers: centers, total: total };
    }

    // Index of the icon item nearest the pointer (never the divider). The
    // smoothed pointer is what the geometry reads; the raw `pointerAlong`
    // gates whether the pointer is over the Dock at all (T-14.7b).
    readonly property int anchorIndex: {
        if (pointerAlong < 0 || items.length === 0)
            return -1;
        var along = Math.max(0, smoothPointerAlong);
        var best = -1;
        var bestDistance = Number.MAX_VALUE;
        for (var i = 0; i < items.length; ++i) {
            if (items[i].kind === "divider" || items[i].kind === "external")
                continue;
            var d = Math.abs(_baseline.centers[i] - along);
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
    readonly property bool popoverOpen:
        menuOpen || chooserOpen || stackOpen || appPickerOpen || overflowOpen
        || trashEmptyOpen
    // A tooltip never shares the stage with a context menu/chooser/stack.
    onPopoverOpenChanged: if (popoverOpen) hideTooltip()

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
        // Keyboard navigation suppresses the hover-open chooser (T-14.7p): it
        // owns the arrow keys, so a hover-opened popover must give way.
        if (chooserHoverOpened) {
            stopChooserHover();
            if (chooserOpen)
                windowChooser.hide();
        }
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

    // --- Keyboard reordering (T-14.7t) -----------------------------------
    // The chord moves the focused pinned entry one slot; the axis mapping
    // matches the Dock position (Left/Right on a bottom Dock, Up/Down on a
    // vertical one). The move is a no-op at the pinned region's ends.
    readonly property string reorderChordHint:
        axisIsX
            ? qsTr("Pinned. Control+Shift+Left or Right moves this item")
            : qsTr("Pinned. Control+Shift+Up or Down moves this item")

    // The accessible reorder hint for an entry: pinned entries can be moved,
    // everything else (temporary, recent, minimized, stack, Trash, overflow)
    // cannot.
    function reorderHintFor(entry) {
        return entry && entry.kind === "pinned" ? reorderChordHint : "";
    }

    // The pure move helper, mirroring `movePinnedEntry` in dockmodel: the id at
    // `index` moves by `delta` within the pinned list. A move off either end is
    // a no-op (never a wrap), and an out-of-range index or `delta` of 0 leaves
    // the list unchanged.
    function movePinnedEntry(ids, index, delta) {
        if (!ids || index < 0 || index >= ids.length || delta === 0)
            return ids;
        var target = index + delta;
        if (target < 0 || target >= ids.length)
            return ids;
        var out = ids.slice();
        var moved = out.splice(index, 1)[0];
        out.splice(target, 0, moved);
        return out;
    }

    // Reorder the focused pinned entry by `delta`. Returns true when the order
    // actually changed. The new complete pinned list goes out through the same
    // `pinnedOrderChanged` signal the drag path uses, so there is one writer
    // (`dock.pinned`); the moved entry keeps the focus ring (its id is stable)
    // so repeated chord presses reorder continuously. The move is announced
    // with the entry's 1-based position in the pinned region.
    function reorderFocusedPinned(delta) {
        var entry = focusedEntry();
        if (!entry || entry.kind !== "pinned")
            return false;
        var ids = currentPinnedIds();
        var index = ids.indexOf(entry.desktopId);
        if (index < 0)
            return false;
        var next = movePinnedEntry(ids, index, delta);
        if (next.join("\n") === ids.join("\n"))
            return false;
        var position = index + delta + 1;
        focusedItemId = entry.id;
        pinnedOrderChanged(next);
        Accessible.announce(qsTr("%1 moved to position %2").arg(entry.name).arg(position));
        return true;
    }

    // The click tree (section 8) shared by pointer activation and Return.
    function activateEntry(entry) {
        if (!entry)
            return;
        // An unavailable Trash is inert: the entry is dimmed and its menu is
        // disabled (T-10 section 16 lifecycle).
        if (entry.kind === "trash" && entry.available === false)
            return;
        // A folder stack (the Downloads default or a pin) opens its folder
        // popover, never a launch (T-10 section 17, T-14.7k).
        if (entry.kind === "stack") {
            openStackFor(entry);
            return;
        }
        // The overflow cell opens its "More Windows" list (T-14.7q).
        if (entry.kind === "overflow") {
            openOverflow(entry);
            return;
        }
        // Report this entry's tile before the click resolves, so the shell can
        // hand it to the compositor if the action launches or restores the app
        // (T-14.7l).
        if (entry.kind !== "trash")
            publishEntryTile(entry);
        if (entry.running === true && entry.kind !== "minimized"
                && entry.windowList !== undefined && entry.windowList.length > 1) {
            openChooser(entry);
            return;
        }
        entryActivated(entry);
    }

    // A double-click on a folder stack opens the folder itself in Files
    // (T-14.7h, ADR 0092) instead of opening the popover.
    function doubleActivateEntry(entry) {
        if (!entry)
            return;
        if (entry.kind === "stack") {
            if (stackOpen)
                closePopovers();
            if (entry.id === stackEntry.id)
                downloadsFolderRequested();
            else if (entry.path !== undefined)
                folderOpenRequested(entry.path);
        }
    }

    // Resolve an entry tap: a folder stack distinguishes a single click (open
    // its popover) from a double click (open the folder in Files), while every
    // other entry activates immediately. The double-click timer lives on the
    // Dock, not the entry delegate, because clearing the stack's new-items
    // badge can recreate the delegate between the two taps.
    function handleEntryTap(entry) {
        if (!entry)
            return;
        // A click is never a hover: cancel a pending dwell so it cannot open a
        // chooser after the click has committed (T-14.7p), and the name label
        // goes away as the action commits (T-14.7i).
        chooserHoverTimer.stop();
        hideTooltip();
        if (entry.kind !== "stack") {
            activateEntry(entry);
            return;
        }
        var now = Date.now();
        if (lastStackTapTime > 0 && now - lastStackTapTime < stackDoubleClickMs) {
            lastStackTapTime = 0;
            doubleActivateEntry(entry);
        } else {
            lastStackTapTime = now;
            activateEntry(entry);
        }
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
        // Ctrl+Shift+Arrow reorders the focused pinned entry (T-14.7t). The
        // key mapping is axis-aware to match the Dock position: Left/Right on
        // a bottom Dock, Up/Down on a vertical one. Any other modifier+arrow
        // combination falls through to the existing bindings.
        var ctrlShift = (event.modifiers & Qt.ControlModifier) !== 0
                        && (event.modifiers & Qt.ShiftModifier) !== 0;
        var reorderKey = axisIsX
                ? (event.key === Qt.Key_Left || event.key === Qt.Key_Right)
                : (event.key === Qt.Key_Up || event.key === Qt.Key_Down);
        if (ctrlShift && reorderKey) {
            var reorderDelta = (event.key === Qt.Key_Left || event.key === Qt.Key_Up)
                               ? -1 : 1;
            reorderFocusedPinned(reorderDelta);
            event.accepted = true;
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
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
        appPicker.hide();
        overflowPopover.hide();
        // An in-flight Empty Trash owns its popover (T-14.7r): a stray
        // dismiss must not hide the operation. A finished (or idle) one
        // closes normally.
        if (trashEmptyPhase !== "emptying")
            trashEmptyPopover.hide();
        trashConfirming = false;
        overflowEntryRef = null;
        chooserHoverTimer.stop();
        chooserCloseTimer.stop();
        chooserHoverEntry = null;
    }

    // --- Trash Empty progress/result (T-14.7r) ---------------------------
    // Entering the operation. Called from the confirmation step after the
    // destructive action is confirmed: the menu closes and the progress
    // popover opens at the Trash entry. The busy indicator waits out
    // `busyDelay` so a fast empty skips it. The shell starts the actual work
    // from the emitted `menuActionRequested("empty_trash")` that follows.
    function beginTrashEmpty() {
        trashConfirming = false;
        entryMenu.hide();
        trashEmptyPhase = "emptying";
        trashEmptyRemoved = 0;
        trashEmptyError = "";
        trashBusyVisible = false;
        var idx = indexOfItemId("__trash__");
        trashEmptyAnchorProxy.capture(idx >= 0 ? entryRepeater.itemAt(idx) : null);
        trashEmptyAnchor = trashEmptyAnchorProxy;
        trashBusyDelayTimer.restart();
        trashEmptyPopover.open = true;
        announceTrashEmpty(qsTr("Emptying the Trash"));
    }

    // The shell's one-shot result from the TrashBridge worker. Success shows
    // the check and the removed count; failure shows the message and the Try
    // Again button. A result for an already-dismissed popover is dropped.
    function handleTrashEmptyResult(ok, removed, error) {
        if (!trashEmptyOpen) {
            resetTrashEmptyState();
            return;
        }
        trashBusyDelayTimer.stop();
        trashBusyVisible = false;
        if (ok) {
            trashEmptyPhase = "succeeded";
            trashEmptyRemoved = removed;
            trashEmptyError = "";
            // An empty rebuilds the Trash delegate; re-capture the snapshot
            // anchor from the fresh one once the Repeater has settled.
            Qt.callLater(function() {
                if (!trashEmptyOpen)
                    return;
                var idx = indexOfItemId("__trash__");
                trashEmptyAnchorProxy.capture(idx >= 0 ? entryRepeater.itemAt(idx) : null);
                trashEmptyAnchorProxy.reposition();
            });
        } else {
            trashEmptyPhase = "failed";
            trashEmptyRemoved = 0;
            trashEmptyError = error && error.length > 0
                              ? error : qsTr("The Trash could not be emptied");
        }
    }

    // Try Again from the failure state: re-run without re-opening the menu or
    // re-confirming. One operation at a time: a second request while emptying
    // is ignored.
    function retryTrashEmpty() {
        if (trashEmptyPhase === "emptying")
            return;
        trashEmptyPhase = "emptying";
        trashEmptyRemoved = 0;
        trashEmptyError = "";
        trashBusyVisible = false;
        trashBusyDelayTimer.restart();
        announceTrashEmpty(qsTr("Emptying the Trash"));
        menuActionRequested("empty_trash", ({}));
    }

    function resetTrashEmptyState() {
        trashBusyDelayTimer.stop();
        trashEmptyPhase = "idle";
        trashBusyVisible = false;
        trashEmptyRemoved = 0;
        trashEmptyError = "";
    }

    function announceTrashEmpty(text) {
        if (text && text.length > 0)
            Accessible.announce(text);
    }

    // Capture/demo seam (T-14.7r): drive the Trash empty popover into a state
    // without running a real operation. `busy`, `success`, or `failed`. Never
    // set in a normal session.
    function trashEmptyFixture(mode) {
        var idx = indexOfItemId("__trash__");
        trashEmptyAnchorProxy.capture(idx >= 0 ? entryRepeater.itemAt(idx) : null);
        trashEmptyAnchor = trashEmptyAnchorProxy;
        trashBusyDelayTimer.stop();
        if (mode === "success") {
            trashEmptyPhase = "succeeded";
            trashEmptyRemoved = 3;
            trashEmptyError = "";
            trashBusyVisible = false;
        } else if (mode === "failed") {
            trashEmptyPhase = "failed";
            trashEmptyRemoved = 0;
            trashEmptyError = qsTr("The Trash could not be emptied");
            trashBusyVisible = false;
        } else {
            trashEmptyPhase = "emptying";
            trashEmptyRemoved = 0;
            trashEmptyError = "";
            trashBusyVisible = true;
        }
        trashEmptyPopover.open = true;
    }

    function openEntryMenu(entry) {
        // The overflow cell has no app context menu; a right-click is inert
        // (T-14.7q). Its list is the left-click affordance.
        if (!entry || entry.kind === "overflow")
            return;
        closePopovers();
        hideTooltip();
        hideTimer.stop();
        var idx = indexOfItemId(entry.id);
        menuAnchor = idx >= 0 ? entryRepeater.itemAt(idx) : null;
        menuEntry = entry;
        entryMenu.open = true;
    }

    // Open the window chooser on `entry`. A click passes no second argument;
    // the hover path passes `true` so a pointer leave may close it again
    // (T-14.7p). The anchor is the snapshot proxy, captured from the live
    // delegate, so a later Repeater rebuild cannot orphan the popover.
    function openChooser(entry, fromHover) {
        closePopovers();
        hideTooltip();
        hideTimer.stop();
        var idx = indexOfItemId(entry.id);
        chooserAnchorProxy.capture(idx >= 0 ? entryRepeater.itemAt(idx) : null);
        chooserAnchor = chooserAnchorProxy;
        chooserEntry = entry;
        chooserHoverOpened = fromHover === true;
        if (chooserHoverOpened)
            chooserHoverEntry = entry;
        windowChooser.open = true;
    }

    // The window chooser survives a projection rebuild (T-14.7m): a minimize
    // or close never dismisses it. A full entry rebuild recreates the app
    // delegate, so re-resolve the bound entry and re-capture the snapshot
    // anchor from the fresh delegate (T-14.7p); the rows themselves read the
    // fresh `windowList`. When the app has no windows left its entry is gone
    // and the chooser dismisses.
    function refreshChooserAfterProjection() {
        if (!chooserOpen)
            return;
        // A chooser opened from the overflow list is bound to a hidden group,
        // not a live delegate (T-14.7q): re-resolve it from the fresh overflow
        // entry's groups, still anchored to the cell.
        if (chooserEntry && chooserEntry.overflowGroup === true) {
            var oidx = indexOfItemId("__overflow__");
            if (oidx < 0) {
                windowChooser.hide();
                return;
            }
            var groups = items[oidx].groups !== undefined ? items[oidx].groups : [];
            var gid = chooserEntry.id !== undefined ? chooserEntry.id : "";
            var found = null;
            for (var g = 0; g < groups.length; ++g) {
                if (groups[g].id === gid) {
                    found = groups[g];
                    break;
                }
            }
            if (!found) {
                windowChooser.hide();
                return;
            }
            chooserEntry = found;
            chooserAnchorProxy.capture(entryRepeater.itemAt(oidx));
            chooserAnchorProxy.reposition();
            return;
        }
        var id = chooserEntry && chooserEntry.id !== undefined ? chooserEntry.id : "";
        var idx = id.length > 0 ? indexOfItemId(id) : -1;
        if (idx < 0) {
            windowChooser.hide();
            return;
        }
        chooserEntry = items[idx];
        if (chooserHoverEntry)
            chooserHoverEntry = items[idx];
        chooserAnchorProxy.capture(entryRepeater.itemAt(idx));
        chooserAnchorProxy.reposition();
    }

    // Keep the overflow list bound to its fresh entry, or dismiss it when the
    // overflow cell is gone (T-14.7q).
    function refreshOverflowAfterProjection() {
        if (!overflowOpen)
            return;
        var idx = indexOfItemId("__overflow__");
        if (idx < 0) {
            overflowPopover.hide();
            return;
        }
        overflowEntryRef = items[idx];
        overflowAnchor = entryRepeater.itemAt(idx);
    }

    function refreshPopoversAfterProjection() {
        refreshChooserAfterProjection();
        refreshOverflowAfterProjection();
    }

    // Open the "More Windows" list anchored to the overflow cell (T-14.7q).
    function openOverflow(entry) {
        if (!entry || entry.kind !== "overflow")
            return;
        closePopovers();
        hideTooltip();
        hideTimer.stop();
        overflowEntryRef = entry;
        var idx = indexOfItemId(entry.id);
        overflowAnchor = idx >= 0 ? entryRepeater.itemAt(idx) : null;
        overflowPopover.open = true;
    }

    // A group chosen from the overflow list: a single-window group activates
    // that window (or falls back to the app's recent window); a multi-window
    // group opens the chooser anchored to the overflow cell (T-14.7q).
    function activateOverflowGroup(group) {
        if (!group)
            return;
        var list = group.windowList !== undefined && group.windowList !== null
                   ? group.windowList : [];
        if (list.length > 1) {
            openOverflowGroupChooser(group);
            return;
        }
        if (list.length === 1 && list[0].windowId !== undefined) {
            overflowPopover.hide();
            windowActivated(String(list[0].windowId));
            return;
        }
        // No window list (a direct model caller): fall back to the app's most
        // recent window through the normal activation path.
        overflowPopover.hide();
        entryActivated(group);
    }

    // Open the T-14.7m chooser for a hidden group, anchored to the overflow
    // cell instead of the hidden group's (nonexistent) delegate.
    function openOverflowGroupChooser(group) {
        if (!group)
            return;
        closePopovers();
        hideTooltip();
        hideTimer.stop();
        var idx = indexOfItemId("__overflow__");
        chooserAnchorProxy.capture(idx >= 0 ? entryRepeater.itemAt(idx) : null);
        chooserAnchor = chooserAnchorProxy;
        chooserEntry = group;
        chooserHoverOpened = false;
        windowChooser.open = true;
    }

    // Open the Downloads stack popover (the default member).
    function openStack() {
        openStackFor(stackEntry);
    }

    // Capture/demo seam (T-14.7m): open the chooser for the first running app
    // with more than one window and force its first row's hover treatment, so
    // the live visual check captures the per-row actions without a synthetic
    // pointer. Never used in a normal session.
    function openChooserFixture() {
        for (var i = 0; i < items.length; ++i) {
            var e = items[i];
            if (e.running === true && e.kind !== "minimized"
                    && e.windowList !== undefined && e.windowList.length > 1) {
                openChooser(e);
                windowChooser.fixtureHoverIndex = 0;
                return true;
            }
        }
        return false;
    }

    // Capture/demo seam (T-14.7n): scroll the open chooser's bounded viewport
    // mid-list and highlight a visible row, so the live visual check shows a
    // long list scrolled with the per-row actions revealed. Never set in a
    // normal session.
    function scrollChooserFixture() {
        windowChooser.scrollToRow(4);
        windowChooser.fixtureHoverIndex = 6;
    }

    // Capture/demo seam (T-14.7p): open the hover chooser on the first grouped
    // entry without a synthetic pointer, or (`mode === "retarget"`) retarget it
    // to the second grouped entry with the magnification pointer between them.
    // Never set in a normal session.
    function hoverChooserFixture(mode) {
        var grouped = [];
        for (var i = 0; i < items.length; ++i) {
            if (chooserEligible(items[i]))
                grouped.push(items[i]);
        }
        if (grouped.length === 0)
            return false;
        var first = indexOfItemId(grouped[0].id);
        if (first >= 0) {
            entryHoverBegan(grouped[0], entryRepeater.itemAt(first));
            openChooserOnHover();
            windowChooser.fixtureHoverIndex = 0;
        }
        if (mode === "retarget" && grouped.length > 1) {
            var second = indexOfItemId(grouped[1].id);
            if (second >= 0) {
                var a = entryRepeater.itemAt(first);
                var b = entryRepeater.itemAt(second);
                pointerAlong = (a.x + b.x + b.width) / 2;
                smoothPointerAlong = pointerAlong;
                chooserHoverEntry = grouped[1];
                openChooserOnHover();
                windowChooser.fixtureHoverIndex = 0;
            }
        }
        return true;
    }

    // Capture/demo seam (T-14.7q): open the overflow cell's "More Windows" list
// once it exists, without a synthetic pointer. Never set in a normal session.
    function openOverflowFixture() {
        var idx = indexOfItemId("__overflow__");
        if (idx < 0)
            return false;
        openOverflow(items[idx]);
        return true;
    }

    // Capture/demo seam (T-14.7k): open the stack entry with `id` without a
    // synthetic pointer click. Never used in a normal session.
    function openStackById(id) {
        var idx = indexOfItemId(id);
        if (idx >= 0)
            openStackFor(items[idx]);
    }

    // Open a folder stack's popover and clear its new-items badge. The shell
    // owns the badge state; `downloadsViewed`/`folderViewed` ask it to mark the
    // folder seen. The open entry drives the popover's items and title
    // (T-10 section 17, T-14.7k).
    function openStackFor(entry) {
        if (!entry || entry.kind !== "stack")
            return;
        closePopovers();
        hideTooltip();
        hideTimer.stop();
        stackEntryRef = entry;
        var idx = indexOfItemId(entry.id);
        stackAnchor = idx >= 0 ? entryRepeater.itemAt(idx) : null;
        stackPopover.open = true;
        if (entry.id === stackEntry.id)
            downloadsViewed();
        else if (entry.path !== undefined)
            folderViewed(entry.path);
    }

    // Open the Add Application picker anchored to the divider (T-14.7e). The
    // signal lets the shell refresh the app-index corpus and push it into
    // `appPickerItems`; the picker opens immediately and shows the current
    // corpus while that refresh lands.
    function openAppPicker() {
        closePopovers();
        hideTooltip();
        hideTimer.stop();
        var idx = indexOfItemId(dividerEntry.id);
        appPickerAnchor = idx >= 0 ? entryRepeater.itemAt(idx) : null;
        appPicker.open = true;
        appPickerRequested();
    }

    // Capture/demo seam only: set the picker's filter text. The shell calls
    // this from the `DF_APP_PICKER_FIXTURE` path; normal sessions never do.
    function setAppPickerQuery(text) { appPicker.setQueryText(text); }

    // --- Drag rearrangement (T-10 section 12) ----------------------------
    function appIndexOfId(id) {
        for (var i = 0; i < appEntries.length; ++i) {
            if (appEntries[i].id === id)
                return i;
        }
        return -1;
    }

    function isDraggable(entry) {
        if (!entry)
            return false;
        // A user folder pin is removable by dragging it out (T-14.7k); the
        // built-in Downloads member is not.
        if (entry.kind === "stack")
            return entry.canRemove === true;
        return entry.kind !== "divider" && entry.kind !== "trash"
                && entry.kind !== "minimized" && entry.kind !== "external"
                && entry.kind !== "overflow";
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
        dragIsFolderPin = false;
        dragFolderPath = "";
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
        // A folder pin only supports drag-out removal, not reordering
        // (T-14.7k): there is no app-region slot to drop it into.
        if (entry.kind === "stack") {
            dragIsFolderPin = true;
            dragFolderPath = entry.path !== undefined ? entry.path : "";
            dragFromAppIndex = -1;
            dragOriginalPinnedIds = currentPinnedIds();
            dragTargetIndex = -1;
            dragPointerAlong = -1;
            dragOutside = false;
            dragOutOfDock = false;
            dragPromote = false;
            dragBaseCenters = [];
            return;
        }
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
        var start = axisIsX ? restingPlateRect.x : restingPlateRect.y;
        var end = axisIsX ? restingPlateRect.x + restingPlateRect.w
                          : restingPlateRect.y + restingPlateRect.h;
        var margin = iconSize;
        dragOutside = along < start - margin || along > end + margin;
        if (dragIsFolderPin) {
            // Dropping a folder pin off the Dock removes it (T-14.7k).
            dragOutOfDock = dragOutside;
            dragPromote = false;
            dragTargetIndex = -1;
            return;
        }
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
        if (dragIsFolderPin) {
            var removed = dragOutOfDock;
            var path = dragFolderPath;
            resetDrag();
            if (removed && path.length > 0)
                folderPinRemoved(path);
            return;
        }
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
        if (dragIsFolderPin) {
            // Leaving the surface with a folder pin removes it (T-14.7k).
            dragOutside = true;
            dragOutOfDock = true;
            dragPromote = false;
            finalizeDrag();
            return;
        }
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
        var alongStart = axisIsX ? restingPlateRect.x : restingPlateRect.y;
        var alongEnd = axisIsX ? restingPlateRect.x + restingPlateRect.w
                               : restingPlateRect.y + restingPlateRect.h;
        var crossStart = axisIsX ? restingPlateRect.y : restingPlateRect.x;
        var crossEnd = axisIsX ? restingPlateRect.y + restingPlateRect.h
                               : restingPlateRect.x + restingPlateRect.w;
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
            if (k === "external" || k === "divider")
                continue;
            if (k === "trash" || k === "minimized" || k === "stack" || k === "overflow")
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
        hideTooltip();
        hideTimer.stop();
        revealTimer.stop();
        externalDragActive = true;
        externalPayloadIsApp = payloadIsApp;
        externalPayloadIsFolder = false;
        externalPayloadCount = payloadCount;
        externalPayloadName = "";
        externalPayloadIconPath = "";
        externalPayloadFirstName = "";
        externalTargetId = "";
        externalInsertIndex = -1;
        springLoadTargetId = "";
        springLoadTimer.stop();
        externalDragChanged();
    }

    // The enter-time read resolved the payload (T-14.7f): update the ghost
    // identity and the file count/name. Called by the shell once per drag.
    function setExternalPayload(payloadIsApp, name, iconPath, count, firstName, folder) {
        externalPayloadIsApp = payloadIsApp;
        externalPayloadIsFolder = folder === true;
        externalPayloadName = name !== undefined ? name : "";
        externalPayloadIconPath = iconPath !== undefined ? iconPath : "";
        externalPayloadCount = count !== undefined ? count : 0;
        externalPayloadFirstName = firstName !== undefined ? firstName : "";
        externalDragChanged();
    }

    // A duplicate app-alias drop pulsed `desktopId`'s pinned entry so the
    // no-op is visible (T-14.7f). Test/introspection hook and shell entry.
    function flashPin(desktopId) {
        duplicateFlashId = desktopId !== undefined ? desktopId : "";
        duplicateFlashTimer.restart();
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
        if (externalPayloadIsApp || externalPayloadIsFolder) {
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

    // Hover the entry with `entryId` during an external drag (test/capture
    // hook); computes the entry centre and routes through `externalDragTo`.
    function externalHoverEntry(entryId) {
        if (!externalDragActive)
            return;
        var idx = indexOfItemId(entryId);
        if (idx < 0 || idx >= layout.length)
            return;
        var r = layout[idx];
        var scene = dock.mapToItem(null, r.x + r.w / 2, r.y + r.h / 2);
        externalDragTo(scene.x, scene.y);
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
        externalPayloadIsFolder = false;
        externalPayloadCount = 0;
        externalPayloadName = "";
        externalPayloadIconPath = "";
        externalPayloadFirstName = "";
        externalTargetId = "";
        externalInsertIndex = -1;
        springLoadTargetId = "";
        springLoadTimer.stop();
        externalDragChanged();
    }

    // The hover affordance for the current external-drag target (T-14.7f):
    // "Open with <app>", "Move to Trash"/"Trash unavailable", "Move to
    // Downloads", "Add to Dock", or "" for a no-op. Mirrors the pure
    // `dockDropAffordance` helper so the decision is testable headless.
    function externalAffordanceFor(targetKind, targetName) {
        if (externalPayloadIsApp) {
            if (targetKind === "trash" || targetKind === "stack"
                    || targetKind === "divider")
                return "";
            return qsTr("Add to Dock");
        }
        if (externalPayloadIsFolder) {
            // A single dropped folder pins on the app region and moves into a
            // stack or the Trash (T-14.7k).
            if (targetKind === "divider")
                return "";
            if (targetKind === "stack")
                return targetName.length > 0 ? qsTr("Move to %1").arg(targetName)
                                             : qsTr("Move to Folder");
            if (targetKind === "trash")
                return trashAvailable ? qsTr("Move to Trash") : qsTr("Trash unavailable");
            return qsTr("Pin Folder");
        }
        if (targetKind === "trash")
            return trashAvailable ? qsTr("Move to Trash") : qsTr("Trash unavailable");
        if (targetKind === "stack")
            return targetName.length > 0 ? qsTr("Move to %1").arg(targetName)
                                         : qsTr("Move to Folder");
        if (targetKind === "pinned" || targetKind === "temporary"
                || targetKind === "recent")
            return targetName.length > 0 ? qsTr("Open with %1").arg(targetName)
                                         : qsTr("Open with");
        return "";
    }

    readonly property int externalTargetIndex:
        externalDragActive && externalTargetId !== "" ? indexOfItemId(externalTargetId) : -1

    readonly property string externalAffordance: {
        if (!externalDragActive || externalTargetIndex < 0)
            return "";
        var e = items[externalTargetIndex];
        var nm = e.name !== undefined ? e.name : "";
        return externalAffordanceFor(e.kind, nm);
    }

    // The dragged identity shown while hovering (T-14.7f): an app alias's
    // name, or "N items"/the single file's name. Suppressed when a concrete
    // target affordance already names the action.
    readonly property string externalIdentityLabel: {
        if (!externalDragActive)
            return "";
        if (externalPayloadIsApp)
            return externalPayloadName;
        if (externalPayloadCount === 1)
            return externalPayloadFirstName;
        if (externalPayloadCount > 1)
            return qsTr("%1 items").arg(externalPayloadCount);
        return "";
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
                dock.openStackFor(dock.items[idx]);
        }
    }

    // A duplicate app-alias drop highlights the already-pinned entry briefly,
    // then the highlight returns (T-14.7f).
    Timer {
        id: duplicateFlashTimer
        interval: dock.duplicateFlashMs
        onTriggered: dock.duplicateFlashId = ""
    }

    // The lifted entry follows the pointer on the dock axis and stays on the
    // bar perpendicular to it.
    function draggedX(itemWidth) {
        if (axisIsX)
            return dragPointerAlong - itemWidth / 2;
        if (position === "right")
            return restingPlateRect.x + padding;
        return restingPlateRect.x + padding - indicatorSpace;
    }
    function draggedY(itemHeight) {
        if (axisIsX)
            return restingPlateRect.y + padding - dragLift;
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
            return stackMenuModel(e);
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
            type: "item", label: qsTr("Add Application…"),
            action: "add_application"
        });
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

    // A folder stack menu (T-10 section 17, T-14.7k): open the folder in Files;
// the folder listing itself is the click popover. A user pin (not the built-in
// Downloads member) also offers Remove from Dock, which writes the settings key.
    function stackMenuModel(e) {
        var out = [];
        if (e && e.id !== "__downloads__") {
            out.push({
                type: "item", label: qsTr("Open in Files"),
                action: "open_stack_folder",
                payload: { path: e.path }
            });
            out.push({ type: "separator" });
            out.push({
                type: "item", label: qsTr("Remove from Dock"),
                action: "remove_folder_pin",
                payload: { path: e.path }
            });
        } else {
            out.push({
                type: "item", label: qsTr("Open in Files"),
                action: "open_downloads_folder"
            });
        }
        return out;
    }

    // The open popover's rectangle in Dock-scene coordinates, or an empty
    // rect. The shell renders it into the Dock's `overlay` surface. A menu
    // with an open submenu reports the union of both panels so the nested
    // panel is not clipped (`ContextMenu.contentRect`).
    readonly property var activePopoverRect: {
        // `open` keeps the rect valid while the popover fades in/out (the
        // shell captures the animation); `visible` covers the close tail.
        var popup = (entryMenu.open || entryMenu.visible) ? entryMenu
                 : ((windowChooser.open || windowChooser.visible) ? windowChooser
                 : ((stackPopover.open || stackPopover.visible) ? stackPopover
                 : ((appPicker.open || appPicker.visible) ? appPicker
                 : ((overflowPopover.open || overflowPopover.visible) ? overflowPopover
                 : ((trashEmptyPopover.open || trashEmptyPopover.visible) ? trashEmptyPopover
                 : null)))));
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

    // The hover label's rectangle in Dock-scene coordinates, or an empty rect
    // (T-14.7i). It rides the same popover/headroom path as a menu so a bottom
    // Dock's label is committed (and never clipped) like every other overlay.
    readonly property var tooltipRect: {
        // `visible` covers both the open state and the close fade tail.
        if (!dockTooltip.visible
                || dockTooltip.width <= 0 || dockTooltip.height <= 0)
            return { x: 0, y: 0, w: 0, h: 0 };
        var topLeft = dockTooltip.mapToItem(dock, 0, 0);
        return { x: topLeft.x, y: topLeft.y,
                 w: dockTooltip.width, h: dockTooltip.height };
    }

    // The union of the open popover and the hover label; the shell commits it
    // to the Dock's overlay surface (T-10 section 5, T-14.7i). They never show
    // together, but the union keeps the close tail of either covered.
    readonly property var popoverRect: {
        var a = activePopoverRect;
        var b = tooltipRect;
        var aValid = a.w > 0 && a.h > 0;
        var bValid = b.w > 0 && b.h > 0;
        if (!aValid && !bValid)
            return { x: 0, y: 0, w: 0, h: 0 };
        if (!aValid)
            return b;
        if (!bValid)
            return a;
        var x0 = Math.min(a.x, b.x);
        var y0 = Math.min(a.y, b.y);
        var x1 = Math.max(a.x + a.w, b.x + b.w);
        var y1 = Math.max(a.y + a.h, b.y + b.h);
        return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }

    function scaledGap(a, b, aDivider, bDivider) {
        if (aDivider || bDivider)
            return dividerGap;
        return gap * (a + b) / (2 * iconSize);
    }

    // The popover placement clamps into the shell's pre-sized buffer budget
    // (T-14.7c); a zero budget (the QML default, and every standalone test)
    // leaves the legacy unbounded placement so a tall menu can still open
    // above the Dock.
    function clampPopoverX(px, w) {
        if (popoverGutter <= 0)
            return px;
        return Math.max(-popoverGutter, Math.min(width + popoverGutter - w, px));
    }
    function clampPopoverY(py, h) {
        if (popoverHeadroom <= 0)
            return py;
        // Keep the popover inside the pre-sized headroom; when the content is
        // taller than the budget it is pinned to the top of the buffer.
        var minY = -popoverHeadroom;
        var maxY = height - h;
        if (maxY < minY)
            return minY;
        return Math.max(minY, Math.min(maxY, py));
    }

    // The launch/attention phase for an entry. The shell publishes phases in
    // `bouncePhases` keyed by entry id so the entry model is never rebuilt for
    // a bounce; a phase embedded in the entry itself still wins so existing
    // direct-model callers keep working (T-14.7c).
    function entryPhase(entry) {
        if (!entry)
            return -1;
        if (entry.bounce !== undefined && entry.bounce >= 0)
            return entry.bounce;
        var info = bouncePhases[entry.id];
        return info !== undefined && info.phase !== undefined ? info.phase : -1;
    }

    function entryAttention(entry) {
        if (!entry)
            return false;
        if (entry.attention === true)
            return true;
        var info = bouncePhases[entry.id];
        return info !== undefined && info.attention === true;
    }

    // The minimize-reaction phase for an entry (T-14.7s): the shell publishes
    // it in the same `bouncePhases` map as a separate `minimize` field so a
    // minimize pulse has its own amplitude and never rebuilds the entry model.
    // -1 when the entry is not reacting; an entry-embedded `minimizeBounce`
    // wins so direct-model callers keep working.
    function entryMinimizePhase(entry) {
        if (!entry)
            return -1;
        if (entry.minimizeBounce !== undefined && entry.minimizeBounce >= 0)
            return entry.minimizeBounce;
        var info = bouncePhases[entry.id];
        return info !== undefined && info.minimize === true
               && info.minimizePhase !== undefined ? info.minimizePhase : -1;
    }

    function entryMinimizeActive(entry) {
        if (!entry)
            return false;
        if (entry.minimizeBounce !== undefined && entry.minimizeBounce >= 0)
            return true;
        var info = bouncePhases[entry.id];
        return info !== undefined && info.minimize === true;
    }

    // The launch/attention bounce translation for an entry (T-10 section
    // 8.1): a sinusoidal hop whose phase the shell drives from the
    // compositor-clock launch/attention clocks. Attention is taller than a
    // launch; reduced motion removes the translation and leaves the state
    // legible through the entry's own indicator/label.
    function entryBounce(entry) {
        if (Theme.reducedMotion)
            return 0;
        // The minimize reaction (T-14.7s) is its own one-hop bounce: opt-in,
        // amplitude from the token as a fraction of the icon size. The
        // reduced-motion gate above already removed the translation.
        if (minimizeReaction && entryMinimizeActive(entry)) {
            var minimize = entryMinimizePhase(entry);
            if (minimize >= 0)
                return Theme.controls.dock.minimizeReaction.amplitudeRatio
                       * iconSize * Math.sin(Math.PI * minimize);
        }
        var phase = entryPhase(entry);
        if (phase < 0)
            return 0;
        // `dock.animateOpening` off suppresses the launch hop; an attention
        // bounce is a notification and still plays (T-10 sections 8.1/19).
        if (!animateOpening && !entryAttention(entry))
            return 0;
        var amplitude = entryAttention(entry) ? barThickness / 2
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
        // model, so the active DragHandler's delegate survives. The app slots
        // are items indices; an inserted region divider shifts the tail, so
        // map through `appItemIndices`.
        if (dragging) {
            for (var a = 0; a < n; ++a) {
                var k = list[a].kind;
                if (k === "divider" || k === "trash" || k === "minimized")
                    continue;
                var appIdx = appItemIndices.indexOf(a);
                if (appIdx < 0)
                    continue;
                var targetItem = appItemIndices[appSlot(appIdx)];
                positions[a] = base.positions[targetItem];
                sizes[a] = base.sizes[targetItem];
            }
        }

        if (magnified) {
            var peak = iconSize * magnifyPeakFactor;
            var falloff = magnifyFalloff * iconSize;
            var along = Math.max(0, smoothPointerAlong);
            for (var i = 0; i < n; ++i) {
                if (list[i].kind === "divider" || list[i].kind === "external")
                    continue;
                var d = Math.abs(base.centers[i] - along);
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

        var out = [];
        for (var j = 0; j < n; ++j) {
            var isDivider = list[j].kind === "divider";
            var extra = isDivider ? 0 : indicatorSpace;
            var bounce = isDivider ? 0 : entryBounce(list[j]);
            if (axisIsX) {
                var h = sizes[j] + extra;
                // Cross-axis placement is relative to the floating plate. The
                // artwork keeps its `padding` inset from the plate's interior
                // edge and is anchored on the anchored-edge side, so under
                // magnification it grows *up* into the magnify band while the
                // running dot (in the anchored-edge padding, T-14.7u) stays
                // put. A bounce lifts it further into the band. The plate
                // already carries `hideY`. A divider instead spans the whole
                // plate cross-axis so its hairline can read near-plate-height
                // (T-14.7v); its slot is still one `dividerWidth` along the
                // axis.
                var y = isDivider ? restingPlateRect.y
                                  : restingPlateRect.y + barThickness - padding
                                    - sizes[j] - bounce;
                out.push({
                    x: positions[j],
                    y: y,
                    w: isDivider ? dividerWidth : sizes[j],
                    h: isDivider ? barThickness : h,
                    iconSize: sizes[j]
                });
            } else {
                var w = sizes[j] + extra;
                // A vertical plate floats off the anchored edge. The artwork
                // keeps its `padding` inset on both sides at rest; under
                // magnification it grows toward the interior (right for a left
                // Dock, left for a right Dock) while the running dot in the
                // anchored-edge padding stays put (T-14.7u). Bounce moves away
                // from the edge, into the magnify band (T-10 section 14). The
                // plate already carries `hideX`.
                var vx = position === "right"
                       ? restingPlateRect.x + padding + iconSize - sizes[j] - bounce
                       : restingPlateRect.x + padding - indicatorSpace + bounce;
                out.push({
                    x: isDivider ? restingPlateRect.x : vx,
                    y: positions[j],
                    w: isDivider ? barThickness : w,
                    h: isDivider ? dividerWidth : sizes[j],
                    iconSize: sizes[j]
                });
            }
        }
        return out;
    }

    // The *resting* plate: the baseline geometry with the auto-hide
    // translation and no magnification. It fixes the anchored edge the live
    // plate grows from and is what the entry cross-axis positions are laid out
    // against, so the geometry never feeds back into itself. `paddingAlong`
    // insets the plate's ends; `edgeMargin` floats its anchored edge off the
    // screen edge (T-14.7a).
    readonly property var restingPlateRect: {
        var base = _baseline;
        if (axisIsX)
            return {
                x: base.positions[0] - paddingAlong,
                y: height - edgeMargin - barThickness + hideY,
                w: base.total + 2 * paddingAlong,
                h: barThickness
            };
        // A vertical plate floats `edgeMargin` off its anchored edge, with the
        // transparent magnify band on the interior side (T-10 section 2).
        return {
            x: (position === "right" ? width - edgeMargin - barThickness : edgeMargin) + hideX,
            y: base.positions[0] - paddingAlong,
            w: barThickness,
            h: base.total + 2 * paddingAlong
        };
    }

    // The visible floating plate: the *live* union of the entry rects (their
    // magnified sizes included, launch/attention bounce overshoot excluded) plus
    // the cross and along padding. The anchored edge stays put — the plate
    // grows into the pre-reserved magnify band — and the result is clamped to
    // the surface. It is the single source for the plate drawing, the input
    // region, and the declared backdrop panel rect (T-14.7b).
    readonly property var plateRect: {
        var l = layout;
        var n = l.length;
        if (n === 0)
            return restingPlateRect;
        var i, r, b;
        if (axisIsX) {
            var minX = Number.MAX_VALUE;
            var maxX = -Number.MAX_VALUE;
            var minTop = Number.MAX_VALUE;
            for (i = 0; i < n; ++i) {
                // A divider is a full-cross-axis marker; it must not expand
                // the plate (T-14.7v). The real entries set the union.
                if (items[i].kind === "divider")
                    continue;
                r = l[i];
                if (r.w <= 0)
                    continue;
                minX = Math.min(minX, r.x);
                maxX = Math.max(maxX, r.x + r.w);
                // Add the bounce back so a launch/attention hop does not pump
                // the plate (T-14.7b).
                b = entryBounce(items[i]);
                minTop = Math.min(minTop, r.y + b);
            }
            if (minX > maxX)
                return restingPlateRect;
            var bottom = restingPlateRect.y + restingPlateRect.h;
            var px = minX - paddingAlong;
            var pw = (maxX - minX) + 2 * paddingAlong;
            if (px < 0) {
                pw += px;
                px = 0;
            }
            if (px + pw > width)
                pw = width - px;
            var py = Math.max(0, minTop - padding);
            return { x: px, y: py, w: pw, h: bottom - py };
        }
        // A vertical plate: the along axis is y; the anchored cross edge is
        // fixed at the screen edge and the plate grows into the interior band.
        var minY = Number.MAX_VALUE;
        var maxY = -Number.MAX_VALUE;
        for (i = 0; i < n; ++i) {
            if (items[i].kind === "divider")
                continue;
            r = l[i];
            if (r.h <= 0)
                continue;
            minY = Math.min(minY, r.y);
            maxY = Math.max(maxY, r.y + r.h);
        }
        if (minY > maxY)
            return restingPlateRect;
        var pyv = minY - paddingAlong;
        var ph = (maxY - minY) + 2 * paddingAlong;
        if (pyv < 0) {
            ph += pyv;
            pyv = 0;
        }
        if (pyv + ph > height)
            ph = height - pyv;
        if (position === "right") {
            var right = restingPlateRect.x + restingPlateRect.w;
            var minLeft = Number.MAX_VALUE;
            for (i = 0; i < n; ++i) {
                if (items[i].kind === "divider" || l[i].h <= 0)
                    continue;
                b = entryBounce(items[i]);
                minLeft = Math.min(minLeft, l[i].x + b);
            }
            var rx = Math.max(0, minLeft - padding);
            return { x: rx, y: pyv, w: right - rx, h: ph };
        }
        var left = restingPlateRect.x;
        var maxRight = -Number.MAX_VALUE;
        for (i = 0; i < n; ++i) {
            if (items[i].kind === "divider" || l[i].h <= 0)
                continue;
            b = entryBounce(items[i]);
            maxRight = Math.max(maxRight, l[i].x + l[i].w - b);
        }
        var leftRight = Math.min(width, maxRight + padding);
        return { x: left, y: pyv, w: leftRight - left, h: ph };
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
        var out = [plateRect];
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
    // The floating glass plate (T-14.7j): a translucent fill, a bright inner
    // top-edge highlight, a hairline border, and a soft shadow, layered over
    // the compositor's live frosted backdrop (T-14.7b). Every value is a
    // token. The group is clipped so the shadow can only fall toward the
    // anchored screen edge, never into the transparent magnify band on the
    // interior side (T-10 section 2). The rim is on the interior cross edge
    // for every Dock position: the top for a bottom Dock, the interior side
    // for a vertical one.
    Item {
        id: dockPlate
        objectName: "dockPlate"

        readonly property bool horizontal: dock.axisIsX
        // The plate's origin inside this group (the group is the plate plus
        // the edge room the shadow may use).
        readonly property real plateX: dock.plateRect.x - x
        readonly property real plateY: dock.plateRect.y - y
        readonly property real plateW: dock.plateRect.w
        readonly property real plateH: dock.plateRect.h
        readonly property real plateRadius: Theme.controls.dock.radius
        readonly property real rimInset: plateRadius * 0.6
        readonly property real rimThickness: Theme.controls.dock.plate.rimHeight

        x: dock.axisIsX || dock.position === "right" ? dock.plateRect.x : 0
        y: dock.plateRect.y
        width: dock.axisIsX ? dock.plateRect.w
               : dock.position === "right" ? dock.width - dock.plateRect.x
               : dock.plateRect.x + dock.plateRect.w
        height: dock.axisIsX ? Math.max(0, dock.height - dock.plateRect.y)
                             : dock.plateRect.h
        clip: true

        Shadow {
            objectName: "dockPlateShadow"
            x: dockPlate.plateX
            y: dockPlate.plateY
            width: dockPlate.plateW
            height: dockPlate.plateH
            radius: dockPlate.plateRadius
            blur: Theme.controls.dock.plate.shadowBlur
            shadowOpacity: Theme.controls.dock.plate.shadowOpacity
            offset: Qt.point(0, Theme.controls.dock.plate.shadowOffsetY)
        }

        Rectangle {
            objectName: "dockBar"
            x: dockPlate.plateX
            y: dockPlate.plateY
            width: dockPlate.plateW
            height: dockPlate.plateH
            radius: dockPlate.plateRadius
            color: Theme.color.dockFill
            opacity: Theme.controls.dock.plate.fillOpacity
        }

        // The bright inner highlight: a short hairline inset from the rounded
        // corners so it reads as a glass rim, not a lid.
        Rectangle {
            objectName: "dockRim"
            x: dockPlate.horizontal
               ? dockPlate.plateX + dockPlate.rimInset
               : (dock.position === "left"
                  ? dockPlate.plateX + dockPlate.plateW - dockPlate.rimThickness
                  : dockPlate.plateX)
            y: dockPlate.horizontal
               ? dockPlate.plateY
               : dockPlate.plateY + dockPlate.rimInset
            width: dockPlate.horizontal
                   ? Math.max(0, dockPlate.plateW - 2 * dockPlate.rimInset)
                   : dockPlate.rimThickness
            height: dockPlate.horizontal
                    ? dockPlate.rimThickness
                    : Math.max(0, dockPlate.plateH - 2 * dockPlate.rimInset)
            radius: dockPlate.rimThickness / 2
            color: Theme.color.dockRim
            opacity: Theme.controls.dock.plate.rimOpacity
        }

        // The hairline border around the whole plate.
        Rectangle {
            objectName: "dockBorder"
            x: dockPlate.plateX
            y: dockPlate.plateY
            width: dockPlate.plateW
            height: dockPlate.plateH
            radius: dockPlate.plateRadius
            color: "transparent"
            border.width: Theme.controls.dock.plate.borderWidth
            border.color: Theme.color.dockBorder
            opacity: Theme.controls.dock.plate.borderOpacity
        }
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
            // The launch/attention phase rides the shell's `bouncePhases` map
            // keyed by entry id; it is injected here instead of mutating the
            // model, so a bounce does not recreate this delegate (T-14.7c).
            bouncePhase: dock.entryPhase(modelData)
            bounceAttention: dock.entryAttention(modelData)
            iconSize: dock.layout.length > index ? dock.layout[index].iconSize : dock.iconSize
            indicatorEdge: dock.indicatorEdge
            showIndicator: dock.showIndicators
            keyboardFocused: dock.keyboardFocused && modelData.id === dock.focusedItemId
            reorderHint: dock.reorderHintFor(modelData)
            externalDropTarget: dock.externalDragActive
                                && modelData.id === dock.externalTargetId
            duplicateFlash: dock.duplicateFlashId !== ""
                            && modelData.desktopId !== undefined
                            && String(modelData.desktopId) === dock.duplicateFlashId
            dragging: dock.dragging
            lifted: isDragged
            z: isDragged ? 10 : 0
            width: dock.layout.length > index ? dock.layout[index].w : dock.iconSize
            height: dock.layout.length > index ? dock.layout[index].h : dock.iconSize
            x: isDragged ? dock.draggedX(width)
               : (dock.layout.length > index ? dock.layout[index].x : 0)
            y: isDragged ? dock.draggedY(height)
               : (dock.layout.length > index ? dock.layout[index].y : 0)

            // The gap left by a reorder springs open or closed while dragging
            // (and just after, so a cancelled drag's gap does not snap shut);
            // every other layout change is handled by the root `iconSize`
            // Behavior, and magnification is progress-based so it never passes
            // through a Behavior (T-14.7c). Reduced motion (duration 0) snaps.
            Behavior on x {
                enabled: !dock.magnifying && (dock.dragging || dock.dragSettling)
                         && !isDragged
                NumberAnimation {
                    duration: Theme.motion.dockMagnify.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.dockMagnify.curve
                }
            }
            Behavior on y {
                enabled: !dock.magnifying && (dock.dragging || dock.dragSettling)
                         && !isDragged
                NumberAnimation {
                    duration: Theme.motion.dockMagnify.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.dockMagnify.curve
                }
            }

            onDragBegan: (entry, sx, sy) => dock.beginDrag(entry)
            onDragMoved: (entry, sx, sy) => dock.updateDrag(entry, sx, sy)
            onDragEnded: (entry, sx, sy) => dock.endDrag(entry, sx, sy)

            onHoverBegan: (entryItem) => dock.entryHoverBegan(modelData, entryItem)
            onHoverEnded: (entryItem) => dock.entryHoverEnded(entryItem)

            onDividerResizeBegan: (sx, sy) => dock.beginDividerResize(sx, sy)
            onDividerResizeMoved: (sx, sy) => dock.updateDividerResize(sx, sy)
            onDividerResizeEnded: () => dock.endDividerResize()

            onActivated: (entry) => {
                // The click tree (T-10 section 8): a running app with more
                // than one window opens the chooser; everything else is a
                // shell activation (single window, minimized restore, launch).
                // A folder stack resolves single vs double click here.
                dock.handleEntryTap(entry);
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
        // clamped to the surface and to the pre-sized popover budget so the
        // offscreen render target never has to grow on open (T-14.7c).
        x: {
            if (!dock.menuAnchor)
                return 0;
            var px;
            if (dock.axisIsX)
                px = Math.max(0, Math.min(dock.width - width,
                    dock.menuAnchor.x + (dock.menuAnchor.width - width) / 2));
            else
                px = dock.position === "left"
                    ? dock.plateRect.x + dock.plateRect.w + 4
                    : dock.plateRect.x - width - 4;
            return dock.clampPopoverX(px, width);
        }
        y: {
            if (!dock.menuAnchor)
                return 0;
            // A bottom Dock's menu opens upward; negative y is covered by the
            // offscreen scene's pre-sized headroom (T-14.7c).
            if (dock.axisIsX)
                return dock.clampPopoverY(dock.menuAnchor.y - height - 4, height);
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
            if (item.action === "empty_trash") {
                // Confirmed: run the operation asynchronously and swap the
                // menu for the progress/result popover (T-14.7r). The shell
                // starts the work from the emitted action below.
                dock.beginTrashEmpty();
                dock.menuActionRequested(item.action, item.payload);
                return;
            }
            // The Add Application picker is a Dock-local surface; it opens
            // anchored to the divider without a shell round trip (T-14.7e).
            if (item.action === "add_application") {
                dock.openAppPicker();
                return;
            }
            // A pinned folder's menu is a Dock-local surface (T-14.7k): open
            // the folder through the one reveal path, or remove the pin.
            if (item.action === "open_stack_folder") {
                if (item.payload && item.payload.path !== undefined)
                    dock.folderOpenRequested(item.payload.path);
                return;
            }
            if (item.action === "remove_folder_pin") {
                if (item.payload && item.payload.path !== undefined)
                    dock.folderPinRemoved(item.payload.path);
                dock.closePopovers();
                return;
            }
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
            var px;
            if (dock.axisIsX)
                px = Math.max(0, Math.min(dock.width - width,
                    dock.chooserAnchor.x + (dock.chooserAnchor.width - width) / 2));
            else
                px = dock.position === "left"
                    ? dock.plateRect.x + dock.plateRect.w + 4
                    : dock.plateRect.x - width - 4;
            return dock.clampPopoverX(px, width);
        }
        y: {
            if (!dock.chooserAnchor)
                return 0;
            // A bottom Dock's chooser opens upward; negative y is covered by
            // the pre-sized headroom (T-14.7c).
            if (dock.axisIsX)
                return dock.clampPopoverY(dock.chooserAnchor.y - height - 4, height);
            return Math.max(0, Math.min(dock.height - height,
                dock.chooserAnchor.y + (dock.chooserAnchor.height - height) / 2));
        }
        onOpened: {
            dock.chooserOpen = true;
            dock.popoverChanged();
        }
        onClosed: {
            dock.chooserOpen = false;
            // A closed chooser releases every hover-open timer and target
            // (T-14.7p); a click-opened chooser has none to release.
            chooserHoverTimer.stop();
            chooserCloseTimer.stop();
            dock.chooserHoverEntry = null;
            dock.chooserHoverOpened = false;
            dock.popoverChanged();
            dock.scheduleHide();
        }
        onWindowActivated: (windowId) => dock.windowActivated(windowId)
        onWindowCloseRequested: (windowId) => dock.windowCloseRequested(windowId)
        onWindowMinimizeRequested: (windowId, minimized) =>
            dock.windowMinimizeRequested(windowId, minimized)
        onShowAllWindows: () => dock.menuActionRequested(
            "show_all_windows", { appId: dock.chooserEntry ? dock.chooserEntry.appId : "" })
    }

    // The folder stack popover (T-10 section 17, T-14.7k). It serves the
    // Downloads default member and every pinned folder through one widget; the
    // open entry decides the listing, title, and open action. It is anchored
    // and placed exactly like the window chooser; the shell renders it into
    // the Dock's overlay surface and performs the resolved open action.
    DockStackPopover {
        id: stackPopover
        objectName: "stackPopover"
        items: dock.stackPopoverItems
        title: dock.stackPopoverTitle
        anchorItem: dock.stackAnchor
        x: {
            if (!dock.stackAnchor)
                return 0;
            var px;
            if (dock.axisIsX)
                px = Math.max(0, Math.min(dock.width - width,
                    dock.stackAnchor.x + (dock.stackAnchor.width - width) / 2));
            else
                px = dock.position === "left"
                    ? dock.plateRect.x + dock.plateRect.w + 4
                    : dock.plateRect.x - width - 4;
            return dock.clampPopoverX(px, width);
        }
        y: {
            if (!dock.stackAnchor)
                return 0;
            if (dock.axisIsX)
                return dock.clampPopoverY(dock.stackAnchor.y - height - 4, height);
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
        onOpenFolder: () => {
            if (dock.stackEntryRef && dock.stackEntryRef.id !== dock.stackEntry.id
                    && dock.stackEntryRef.path !== undefined)
                dock.folderOpenRequested(dock.stackEntryRef.path);
            else
                dock.downloadsFolderRequested();
        }
    }

    // The Add Application picker (T-14.7e, ADR 0090). It is anchored to the
    // divider and placed exactly like the stack/chooser; the shell renders it
    // into the Dock's overlay surface. Filtering is local; a row toggle goes
    // back to the controller's single `dock.pinned` writer.
    DockAppPicker {
        id: appPicker
        objectName: "appPicker"
        items: dock.appPickerItems
        available: dock.appIndexAvailable
        anchorItem: dock.appPickerAnchor
        x: {
            if (!dock.appPickerAnchor)
                return 0;
            var px;
            if (dock.axisIsX)
                px = Math.max(0, Math.min(dock.width - width,
                    dock.appPickerAnchor.x + (dock.appPickerAnchor.width - width) / 2));
            else
                px = dock.position === "left"
                    ? dock.plateRect.x + dock.plateRect.w + 4
                    : dock.plateRect.x - width - 4;
            return dock.clampPopoverX(px, width);
        }
        y: {
            if (!dock.appPickerAnchor)
                return 0;
            if (dock.axisIsX)
                return dock.clampPopoverY(dock.appPickerAnchor.y - height - 4, height);
            return Math.max(0, Math.min(dock.height - height,
                dock.appPickerAnchor.y + (dock.appPickerAnchor.height - height) / 2));
        }
        onOpened: {
            dock.appPickerOpen = true;
            dock.popoverChanged();
        }
        onClosed: {
            dock.appPickerOpen = false;
            dock.popoverChanged();
            dock.scheduleHide();
        }
        onPinToggled: (desktopId, pinned) => dock.appPinToggled(desktopId, pinned)
    }

    // The "More Windows" overflow list (T-14.7q). It is anchored to the
    // terminal overflow cell and placed exactly like the chooser/stack; the
    // shell renders it into the Dock's overlay surface. A chosen group is
    // resolved by the Dock (activate one window, or open the chooser).
    DockOverflowPopover {
        id: overflowPopover
        objectName: "overflowPopover"
        groups: dock.overflowGroups
        anchorItem: dock.overflowAnchor
        x: {
            if (!dock.overflowAnchor)
                return 0;
            var px;
            if (dock.axisIsX)
                px = Math.max(0, Math.min(dock.width - width,
                    dock.overflowAnchor.x + (dock.overflowAnchor.width - width) / 2));
            else
                px = dock.position === "left"
                    ? dock.plateRect.x + dock.plateRect.w + 4
                    : dock.plateRect.x - width - 4;
            return dock.clampPopoverX(px, width);
        }
        y: {
            if (!dock.overflowAnchor)
                return 0;
            if (dock.axisIsX)
                return dock.clampPopoverY(dock.overflowAnchor.y - height - 4, height);
            return Math.max(0, Math.min(dock.height - height,
                dock.overflowAnchor.y + (dock.overflowAnchor.height - height) / 2));
        }
        onOpened: {
            dock.overflowOpen = true;
            dock.popoverChanged();
        }
        onClosed: {
            dock.overflowOpen = false;
            dock.popoverChanged();
            dock.scheduleHide();
        }
        onGroupActivated: (group) => dock.activateOverflowGroup(group)
    }

    // The Trash Empty progress/result popover (T-14.7r). It is anchored to the
    // Trash entry and placed exactly like the chooser/stack; the shell renders
    // it into the Dock's overlay surface. The Dock owns the visible phase and
    // raises Try Again; the operation itself runs in the shell's bridge.
    DockTrashEmptyPopover {
        id: trashEmptyPopover
        objectName: "trashEmptyPopover"
        phase: dock.trashEmptyPhase
        busyVisible: dock.trashBusyVisible
        removedCount: dock.trashEmptyRemoved
        errorMessage: dock.trashEmptyError
        anchorItem: dock.trashEmptyAnchor
        x: {
            if (!dock.trashEmptyAnchor)
                return 0;
            var px;
            if (dock.axisIsX)
                px = Math.max(0, Math.min(dock.width - width,
                    dock.trashEmptyAnchor.x + (dock.trashEmptyAnchor.width - width) / 2));
            else
                px = dock.position === "left"
                    ? dock.plateRect.x + dock.plateRect.w + 4
                    : dock.plateRect.x - width - 4;
            return dock.clampPopoverX(px, width);
        }
        y: {
            if (!dock.trashEmptyAnchor)
                return 0;
            if (dock.axisIsX)
                return dock.clampPopoverY(dock.trashEmptyAnchor.y - height - 4, height);
            return Math.max(0, Math.min(dock.height - height,
                dock.trashEmptyAnchor.y + (dock.trashEmptyAnchor.height - height) / 2));
        }
        onOpened: {
            dock.trashEmptyOpen = true;
            dock.popoverChanged();
        }
        onClosed: {
            dock.trashEmptyOpen = false;
            dock.resetTrashEmptyState();
            dock.popoverChanged();
            dock.scheduleHide();
        }
        onRetryRequested: () => dock.retryTrashEmpty()
    }

    // The busy indicator's short delay (T-14.7r): shown only once the empty has
    // run longer than `busyDelay`, so a fast empty never flickers it.
    Timer {
        id: trashBusyDelayTimer
        interval: Theme.controls.dock.trashEmpty.busyDelay
        repeat: false
        onTriggered: dock.trashBusyVisible = true
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

    // The external-drag affordance capsule (T-14.7f): the action the hovered
    // target will take ("Open with <app>", "Move to Trash", "Move to
    // Downloads", "Add to Dock"), or the dragged identity while no concrete
    // target is hovered (an app name, "N items", or the single file name).
    // It floats above the target on a bottom Dock and beside it otherwise.
    readonly property var externalAnchorRect: {
        if (!externalDragActive)
            return null;
        var idx = externalTargetIndex;
        if (idx < 0) {
            for (var i = 0; i < items.length; ++i) {
                if (items[i].kind === "external") {
                    idx = i;
                    break;
                }
            }
        }
        if (idx < 0 || idx >= layout.length)
            return null;
        return layout[idx];
    }

    Rectangle {
        id: externalAffordanceCapsule
        objectName: "externalAffordance"
        readonly property string label:
            dock.externalAffordance.length > 0 ? dock.externalAffordance
                                               : dock.externalIdentityLabel
        readonly property var anchorRect: dock.externalAnchorRect
        visible: dock.externalDragActive && label.length > 0 && anchorRect !== null
        width: affordanceText.implicitWidth + 16
        height: affordanceText.implicitHeight + 8
        radius: height / 2
        color: Theme.color.chrome
        border.width: 1
        border.color: Theme.color.border
        opacity: 0.95
        z: 3000
        x: {
            if (!anchorRect)
                return 0;
            if (dock.axisIsX) {
                var cx = anchorRect.x + anchorRect.w / 2;
                return Math.max(0, Math.min(dock.width - width, cx - width / 2));
            }
            return dock.position === "right"
                ? dock.plateRect.x - width - 8
                : dock.plateRect.x + dock.plateRect.w + 8;
        }
        y: {
            if (!anchorRect)
                return 0;
            if (dock.axisIsX) {
                // Lift the capsule clear of the identity ghost's own name
                // label, which sits just above the gap (T-14.7f).
                var lift = dock.externalPayloadIsApp ? 26 : 6;
                return Math.max(0, anchorRect.y - height - lift);
            }
            var cy = anchorRect.y + anchorRect.h / 2;
            return Math.max(0, Math.min(dock.height - height, cy - height / 2));
        }
        Text {
            id: affordanceText
            anchors.centerIn: parent
            text: externalAffordanceCapsule.label
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
        }
    }

    // The hover name label (T-14.7i): a design-system Tooltip anchored to the
    // hovered entry, so it follows magnification frame by frame. It is
    // presentational only; the entry's accessible name still carries the state.
    Tooltip {
        id: dockTooltip
        objectName: "dockTooltip"
        open: dock.tooltipOpen
        anchorItem: dock.tooltipAnchor
        text: dock.tooltipText
        placement: dock.tooltipPlacement
        bounds: dock
        // The fade finished: release the anchor so no stale delegate is held,
        // unless a new entry is already waiting on the dwell timer (otherwise
        // moving A -> B would clear B's anchor as A fades out).
        onVisibleChanged: {
            if (!visible && !dock.tooltipOpen && !tooltipDwellTimer.running) {
                dock.tooltipAnchor = null;
                dock.tooltipEntry = null;
            }
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
           : (dock.position === "right" ? dock.plateRect.x - width - 8
                                        : dock.plateRect.x + dock.barThickness + 8)
        y: dock.axisIsX
           ? Math.max(0, dock.plateRect.y - height - 4)
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
                dock.hideTooltip();
                if (dock.chooserOpen && dock.chooserHoverOpened)
                    chooserCloseTimer.restart();
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
    function tooltipItem() { return dockTooltip; }

    // Capture/demo seam (T-14.7i): show the hover label for the first entry of
    // a kind (`app`/`folder`/`trash`) without a real pointer, and place the
    // magnification pointer on it so the capture shows the label over a
    // magnified icon. Never set in a normal session.
    function showTooltipFor(kind) {
        var idx = -1;
        for (var i = 0; i < items.length; ++i) {
            var k = items[i].kind;
            if (kind === "app" && (k === "pinned" || k === "temporary" || k === "recent"))
                idx = i;
            else if (kind === "folder" && k === "stack")
                idx = i;
            else if (kind === "trash" && k === "trash")
                idx = i;
            if (idx >= 0)
                break;
        }
        if (idx < 0)
            return;
        var item = itemAt(idx);
        if (!item)
            return;
        if (magnification > 0) {
            pointerAlong = _baseline.centers[idx];
            smoothPointerAlong = pointerAlong;
        }
        entryHoverBegan(items[idx], item);
        showTooltip();
    }
}
