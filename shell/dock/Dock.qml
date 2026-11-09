// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
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
    // Effective chrome material from the compositor (P1). `materialBlurActive`
    // is true only when the sampled backdrop pass actually runs; when false
    // the plate switches to the milky fallback instead of relying on a
    // compositor blur that is not there. `materialTier` is 0=full, 1=reduced,
    // 2=minimal; `materialFallback` is 0=none, 1=no-sampled-blur, 2=software,
    // 3=multi-gpu. QML must not re-derive the degrade policy.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    // T198: one resolved Dock optical preset from the same policy the
    // compositor consumes. On the sampled path the compositor paints the flat
    // band, rim and corner lens, so the QML layers below are hidden to avoid a
    // second tint/rim/shadow. Unsupported paths and Reduce Transparency keep
    // the coherent shell-painted plate.
    readonly property var glassOptics: Glass.resolve("dock", {
        scheme: Theme.dark ? "dark" : "light",
        rollout: Glass.rollout,
        slider: Glass.slider,
        reduceTransparency: Glass.reduceTransparency,
        increaseContrast: Glass.increaseContrast,
        backend: materialFallback === 2 ? "software"
               : materialBlurActive ? "gpu-sampled" : "gpu-no-sampled-blur",
        tier: materialTier === 2 ? "minimal" : materialTier === 1 ? "reduced" : "full"
    })
    readonly property bool sampledPlate:
        materialBlurActive && glassOptics.renderMode === "sampled-glass"
    readonly property bool replacementPlate: Glass.rollout === "replacement"
    readonly property real contrastEdgeBoost: Glass.increaseContrast
        ? Theme.glass.accessibility.increaseContrast.edgeBoost : 0
    // T-21.2c: the transient Popup role state for every Dock popover. The
    // shell applies it through the same MaterialPolicy as the Dock role above;
    // the popovers bind to these, so QML never re-derives the tier.
    property bool popupMaterialBlurActive: true
    property int popupMaterialTier: 0
    property int popupMaterialFallback: 0
    readonly property string popupMaterialFallbackReason:
        Material.fallbackReason(popupMaterialFallback, popupMaterialBlurActive,
                                popupMaterialTier)
    // [{ id, appId, name, kind, running, active, minimized, launch, attention,
    //    badge, pinned, trashFull }]
    property var entries: []
    // A new projection arrived (T-14.7m): interaction state that cannot
    // survive a delegate rebuild is reconciled immediately, before a stale
    // DragHandler/divider can leave `dragging`/`resizing` latched on. The
    // popover anchors are refreshed from the `items` change below, which also
    // covers non-`entries` rebuilds (a badge clear, Trash state, folder pins).
    onEntriesChanged: reconcileInteractionAfterProjection()
    // The ordered item model was rebuilt. The dependent Repeater settles after
    // this signal, so re-resolve every open popover on the next event-loop turn.
    // Coalesce a burst (an external-drag insertion index changes per pointer
    // sample) into one refresh.
    property bool popoverRefreshPending: false
    onItemsChanged: {
        // A non-`entries` rebuild (a folder listing, Trash state, external
        // gap) also recreates delegates, so reconcile the gesture state here
        // as well as on a projection change.
        reconcileInteractionAfterProjection()
        dock.schedulePopoverRefresh()
    }

    function schedulePopoverRefresh() {
        if (popoverRefreshPending)
            return;
        popoverRefreshPending = true;
        Qt.callLater(function() {
            dock.popoverRefreshPending = false;
            dock.refreshPopoversAfterProjection();
        });
    }
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

    // The Dock clamps overlays into a fixed budget so moving or fading one
    // does not resize its render target on every frame.
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
    // Follow the latest sample once per frame. The shell samples on its
    // output-paced render path; standalone QML uses Qt's animation clock.
    property real smoothPointerAlong: -1
    property bool pointerTrackingFromShell: false
    readonly property int pointerTrackingDuration: Theme.motion.dockMagnifyTrack.duration
    readonly property bool pointerTrackingActive:
        magnifying && pointerTrackingDuration > 0 && smoothPointerAlong >= 0
        && pointerAlong !== smoothPointerAlong
    onPointerTrackingDurationChanged: {
        if (pointerTrackingDuration === 0)
            smoothPointerAlong = pointerAlong;
    }
    FrameAnimation {
        running: !dock.pointerTrackingFromShell && dock.pointerTrackingActive
        onTriggered: dock.advancePointerTracking(frameTime * 1000)
    }
    function advancePointerTracking(elapsedMs: real) {
        if (!pointerTrackingActive)
            return;
        // Close 99% of the gap over the tracking duration at any refresh rate.
        var remaining = (pointerAlong - smoothPointerAlong)
                        * Math.pow(0.01, elapsedMs / pointerTrackingDuration);
        smoothPointerAlong = Math.abs(remaining) < 0.25
                             ? pointerAlong : pointerAlong - remaining;
    }
    onPointerAlongChanged: {
        if (!dock.magnifying || dock.pointerTrackingDuration === 0
                || dock.smoothPointerAlong < 0)
            dock.smoothPointerAlong = dock.pointerAlong;
    }

    // The pointer and anchor the magnified profile is evaluated at (T-14.7aa).
    // They hold the last on-Dock value after the pointer leaves, so the
    // engagement below releases the bubble around the tile the pointer was on
    // instead of snapping back to the baseline.
    property real magnifyPointer: -1
    property int magnifyPointerIndex: -1
    // The magnified profile's amplitude, 0..1 (T-14.7aa). The pointer drives
    // the profile *shape* (above); this drives its *engagement*: entering and
    // leaving the Dock grows and shrinks the bubble with a short, gentle ease
    // (`motion.dockHover`) rather than a snap. The overshooting magnet spring
    // (`motion.dockMagnify`) is deliberately not used here: its curve reaches
    // most of the target in the first frame, which reads as a pop when the
    // whole row translates with the bubble. Reduced motion collapses the
    // duration to zero in the token, which keeps the state change legible with
    // no translation.
    property real magnifyEngagement: magnifying ? 1.0 : 0.0
    Behavior on magnifyEngagement {
        NumberAnimation {
            duration: Theme.motion.dockHover.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: Theme.motion.dockHover.curve
        }
    }
    onSmoothPointerAlongChanged: {
        // Only follow the tracker while the pointer is on the Dock. When the
        // pointer leaves, the raw value snaps to -1 and this tracker animates
        // toward it; the held `magnifyPointer` must not follow that exit slide,
        // or the release bubble would collapse toward the Dock edge instead of
        // shrinking around the tile the pointer left.
        if (dock.pointerAlong >= 0 && dock.smoothPointerAlong >= 0)
            dock.magnifyPointer = dock.smoothPointerAlong;
    }
    onAnchorIndexChanged: {
        if (dock.anchorIndex >= 0)
            dock.magnifyPointerIndex = dock.anchorIndex;
    }
    // True while the magnified profile is drawn: while the pointer is on the
    // Dock, and through the engagement release after it leaves. `layout` reads
    // this instead of `magnifying` so the release frames still shrink the
    // bubble around the held pointer.
    readonly property bool magnifyProfile:
        magnification > 0 && magnifyEngagement > 0.001
        && magnifyPointer >= 0 && magnifyPointerIndex >= 0
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
            // A cancelled or completed drag no longer holds the Dock open.
            if (dock.autoHide)
                dock.scheduleHide();
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
    // opens the popover immediately; a second tap on the *same* stack inside
    // this window opens the folder in Files. The id is tracked with the time so
    // tapping stack A then stack B within the window cannot open B in Files.
    readonly property int stackDoubleClickMs: 400
    property real lastStackTapTime: 0
    property string lastStackTapId: ""

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
            return stackEntryRef.items !== undefined && stackEntryRef.items !== null
                   ? stackEntryRef.items : [];
        return downloadsItems !== undefined && downloadsItems !== null
               ? downloadsItems : [];
    }
    readonly property string stackPopoverTitle: {
        if (stackEntryRef !== null && stackEntryRef.id !== undefined
                && stackEntryRef.id !== "__downloads__"
                && stackEntryRef.name !== undefined && stackEntryRef.name !== null
                && String(stackEntryRef.name).length > 0)
            return String(stackEntryRef.name);
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
            && overflowEntryRef.groups !== null
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
    // The reference label sits 8 pt above the Dock *plate*, not above the
    // resting icon box: the plate is the stable anchor while the artwork
    // grows up into the magnify band. Its x/width still follow the magnified
    // delegate, so the capsule tracks the pointed tile frame by frame.
    readonly property var tooltipAnchorRect: {
        if (tooltipAnchor === null)
            return null;
        // A bottom Dock's label is plate-anchored so it clears the artwork
        // growing into the magnify band. A vertical Dock's label sits beside
        // the entry, so it keeps the delegate's cross-axis centre.
        if (tooltipPlacement === "above" || tooltipPlacement === "below")
            return { x: tooltipAnchor.x, y: plateRect.y,
                     w: tooltipAnchor.width, h: 0 };
        return { x: tooltipAnchor.x, y: tooltipAnchor.y,
                 w: tooltipAnchor.width, h: tooltipAnchor.height };
    }

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

    // Geometry snapshots for the popovers that anchor to a live entry delegate
    // (T-14.7c stability). The shell rebuilds `entries` whenever the compositor
    // projection changes (a window opens/closes, a badge clears, a pin toggles),
    // which destroys and recreates every Repeater delegate. A popover that
    // held the old delegate would lose its anchor and jump to the surface
    // origin, so each popover snapshots the delegate's rect into one of these
    // invisible Items (the chooser/Trash pattern) and re-captures it after the
    // projection settles. They are geometry proxies only: no surface, no input.
    Item {
        id: menuAnchorProxy
        objectName: "menuAnchorProxy"
        visible: false
        width: 0
        height: 0

        function capture(entryItem) {
            if (!entryItem)
                return;
            var topLeft = entryItem.mapToItem(dock, 0, 0);
            menuAnchorProxy.x = topLeft.x;
            menuAnchorProxy.y = topLeft.y;
            menuAnchorProxy.width = entryItem.width;
            menuAnchorProxy.height = entryItem.height;
        }

        function reposition() {
            var previous = menuAnchorProxy.parent;
            menuAnchorProxy.parent = null;
            menuAnchorProxy.parent = previous;
        }
    }

    Item {
        id: stackAnchorProxy
        objectName: "stackAnchorProxy"
        visible: false
        width: 0
        height: 0

        function capture(entryItem) {
            if (!entryItem)
                return;
            var topLeft = entryItem.mapToItem(dock, 0, 0);
            stackAnchorProxy.x = topLeft.x;
            stackAnchorProxy.y = topLeft.y;
            stackAnchorProxy.width = entryItem.width;
            stackAnchorProxy.height = entryItem.height;
        }

        function reposition() {
            var previous = stackAnchorProxy.parent;
            stackAnchorProxy.parent = null;
            stackAnchorProxy.parent = previous;
        }
    }

    Item {
        id: appPickerAnchorProxy
        objectName: "appPickerAnchorProxy"
        visible: false
        width: 0
        height: 0

        function capture(entryItem) {
            if (!entryItem)
                return;
            var topLeft = entryItem.mapToItem(dock, 0, 0);
            appPickerAnchorProxy.x = topLeft.x;
            appPickerAnchorProxy.y = topLeft.y;
            appPickerAnchorProxy.width = entryItem.width;
            appPickerAnchorProxy.height = entryItem.height;
        }

        function reposition() {
            var previous = appPickerAnchorProxy.parent;
            appPickerAnchorProxy.parent = null;
            appPickerAnchorProxy.parent = previous;
        }
    }

    Item {
        id: overflowAnchorProxy
        objectName: "overflowAnchorProxy"
        visible: false
        width: 0
        height: 0

        function capture(entryItem) {
            if (!entryItem)
                return;
            var topLeft = entryItem.mapToItem(dock, 0, 0);
            overflowAnchorProxy.x = topLeft.x;
            overflowAnchorProxy.y = topLeft.y;
            overflowAnchorProxy.width = entryItem.width;
            overflowAnchorProxy.height = entryItem.height;
        }

        function reposition() {
            var previous = overflowAnchorProxy.parent;
            overflowAnchorProxy.parent = null;
            overflowAnchorProxy.parent = previous;
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
            && entry.windowList !== undefined && entry.windowList !== null
            && entry.windowList.length > 1;
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
        // The same entry's delegate may be recreated by a projection while the
        // pointer never moved. Keep the label and re-bind it instead of
        // restarting the dwell and flashing the capsule off.
        if (tooltipEntry && entry && tooltipEntry.id !== undefined
                && tooltipEntry.id === entry.id
                && (tooltipOpen || tooltipDwellTimer.running)) {
            tooltipEntry = entry;
            tooltipAnchor = entryItem;
            return;
        }
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
        // A Repeater rebuild reports hover-ended as the old delegate dies. If
        // the entry still exists under a different delegate, keep the label;
        // `refreshTooltipAfterProjection` re-binds it to the fresh delegate.
        if (tooltipEntry && tooltipEntry.id !== undefined) {
            var idx = indexOfItemId(tooltipEntry.id);
            if (idx >= 0 && entryRepeater.itemAt(idx) !== entryItem)
                return;
        }
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
        if (idx < 0)
            return;
        chooserAnchorProxy.capture(entryRepeater.itemAt(idx));
        chooserAnchorProxy.reposition();
        chooserEntry = entry;
        chooserHoverEntry = entry;
    }

    // The label may only open for the entry that is actually under the pointer
    // *now*. A projection rebuild can move a different entry under a
    // stationary pointer (reorder, pin change, overflow); the dwell must not
    // then fire the old entry's label. This is checked at fire time as well as
    // after a rebuild, so a queued timer cannot resurrect a stale identity.
    function tooltipIdentityValid() {
        if (!tooltipEntry || tooltipEntry.id === undefined
                || tooltipEntry.id === null)
            return false;
        var idx = indexOfItemId(String(tooltipEntry.id));
        if (idx < 0 || idx >= items.length)
            return false;
        if (tooltipAnchor !== entryRepeater.itemAt(idx))
            return false;
        if (pointerAlong >= 0 && anchorIndex >= 0 && anchorIndex !== idx)
            return false;
        return true;
    }

    function showTooltip() {
        if (!tooltipAnchor || tooltipText.length === 0 || tooltipOpen)
            return;
        if (dragging || externalDragActive || resizing || popoverOpen)
            return;
        if (!tooltipIdentityValid()) {
            hideTooltip();
            return;
        }
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
    // The permanent Applications launcher tile was activated (T-19.2
    // follow-up): the Dock owns the tile and asks the shell to toggle the
    // apps drawer. The shell injects the tile with `appsLauncher: true`.
    signal appsDrawerRequested()

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
    // `dock.magnification` (0..1, 0 = off) maps onto the peak icon factor.
    // The controlled reference has two calibration anchors: the default
    // fixture (0.5) peaks at 1.217x and the diagnostic fixture (1.0) at
    // 2.047x. A smoothstep segment through (0, 1), (0.5, peak) and
    // (1, peakMax) keeps the mapping monotone and C1 at both anchors, so a
    // live magnification change never kinks or overshoots.
    readonly property real magnifyPeak: Theme.controls.dock.magnifyPeak
    readonly property real magnifyPeakMax: Theme.controls.dock.magnifyPeakMax
    readonly property real magnifyPeakFactor: {
        if (magnification <= 0)
            return 1.0;
        var m = Math.min(1.0, magnification);
        if (m <= 0.5)
            return 1 + (magnifyPeak - 1) * smoothstep01(m / 0.5);
        return magnifyPeak
             + (magnifyPeakMax - magnifyPeak) * smoothstep01((m - 0.5) / 0.5);
    }
    // The controlled reference zoom profile is a raised cosine over absolute
    // logical distance from the pointer (docs/dock.md §5): S(d) = 1 +
    // (peak - 1) * 0.5 * (1 + cos(pi * d / R)). R is the measured 125.1 pt
    // influence radius, not a pitch multiplier, so the curve is bounded and
    // continuous for every live icon size.
    readonly property real magnifyInfluence: Theme.controls.dock.magnifyInfluence
    readonly property real restingPitch: iconSize + gap
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
    // while magnifying (the continuous follow would be a stream) and while the
    // entry is bouncing (a launch/attention/minimize hop also moves the layout
    // every committed frame); the settled state republishes once motion ends.
    function republishEntryTile() {
        if (tileEntryId === "" || lastTileRect === null || magnifying || dragging)
            return;
        var idx = indexOfItemId(tileEntryId);
        if (idx < 0 || idx >= layout.length)
            return;
        if (entryBounce(items[idx]) > 0)
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
        if (!entries)
            return out;
        for (var i = 0; i < entries.length; ++i) {
            if (!entries[i])
                continue;
            var k = entries[i].kind;
            if (k === "pinned" || k === "temporary" || k === "recent" || k === "overflow")
                out.push(entries[i]);
        }
        return out;
    }
    readonly property var minimizedEntries: {
        var out = [];
        if (minimizeIntoTileIcon || !entries)
            return out;
        for (var i = 0; i < entries.length; ++i) {
            if (entries[i] && entries[i].kind === "minimized")
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
        if (!folderPins)
            return out;
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
    // anchor decision is made from the **raw** `pointerAlong`, so the filter
    // that smooths the geometry can never flip the anchored tile back and
    // forth at a boundary (T-14.7y). The raw value also gates whether the
    // pointer is over the Dock at all.
    readonly property int anchorIndex: {
        if (pointerAlong < 0 || items.length === 0)
            return -1;
        var along = Math.max(0, pointerAlong);
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
    // Closing the last popover must return active focus to the Dock when the
    // compositor handed it the keyboard: every popover takes focus while open,
    // and a hidden FocusScope does not give it back, which would silently stop
    // the Dock's arrow/Return handling after Escape (T-10 section 20).
    onPopoverOpenChanged: {
        if (popoverOpen) {
            hideTooltip();
        } else if (keyboardFocused) {
            dock.forceActiveFocus()
        }
    }

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
        return entry && entry.kind === "pinned" && entry.appsLauncher !== true
                ? reorderChordHint : "";
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
        if (!entry || entry.kind !== "pinned" || entry.appsLauncher === true)
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
        // Dividers and drop placeholders are layout only, never activatable.
        if (entry.kind === "divider" || entry.kind === "external")
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
        // The permanent Applications launcher toggles the drawer instead of
        // launching an app (T-19.2 follow-up).
        if (entry.appsLauncher === true) {
            hideTooltip();
            appsDrawerRequested();
            return;
        }
        // Report this entry's tile before the click resolves, so the shell can
        // hand it to the compositor if the action launches or restores the app
        // (T-14.7l).
        if (entry.kind !== "trash")
            publishEntryTile(entry);
        if (entry.running === true && entry.kind !== "minimized"
                && entry.windowList !== undefined && entry.windowList !== null
                && entry.windowList.length > 1) {
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
        // goes away as the action commits (T-14.7i). An entry click also
        // dismisses any other open popover, so a menu cannot outlive the action
        // it was covering.
        chooserHoverTimer.stop();
        closePopovers();
        hideTooltip();
        if (entry.kind !== "stack") {
            // Any other entry ends a pending stack double-click.
            lastStackTapTime = 0;
            lastStackTapId = "";
            activateEntry(entry);
            return;
        }
        var now = Date.now();
        if (lastStackTapTime > 0 && now - lastStackTapTime < stackDoubleClickMs
                && lastStackTapId === entry.id) {
            lastStackTapTime = 0;
            lastStackTapId = "";
            doubleActivateEntry(entry);
        } else {
            lastStackTapTime = now;
            lastStackTapId = entry.id;
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
            var name = list[i].name !== undefined && list[i].name !== null
                       ? String(list[i].name) : "";
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
        // The open popover's `onPhaseChanged` announces the restart; do not
        // announce the same state twice.
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
        // The entry can vanish between the press and this call (a projection
        // rebuild). Never open a menu with no anchor: it would sit at (0, 0).
        if (idx < 0)
            return;
        menuAnchorProxy.capture(entryRepeater.itemAt(idx));
        menuAnchor = menuAnchorProxy;
        menuEntry = entry;
        entryMenu.open = true;
    }

    // Capture/demo seam (T202): open the first app entry's context menu
    // without a synthetic pointer so the menu-family capture is deterministic.
    function openEntryMenuFixture() {
        for (var i = 0; i < entries.length; ++i) {
            var entry = entries[i];
            if (!entry || entry.kind === "divider" || entry.kind === "overflow")
                continue;
            entryContextMenuRequested(entry, 0, 0);
            openEntryMenu(entry);
            return;
        }
    }

    // Open the window chooser on `entry`. A click passes no second argument;
    // the hover path passes `true` so a pointer leave may close it again
    // (T-14.7p). The anchor is the snapshot proxy, captured from the live
    // delegate, so a later Repeater rebuild cannot orphan the popover.
    function openChooser(entry, fromHover) {
        if (!entry)
            return;
        closePopovers();
        hideTooltip();
        hideTimer.stop();
        var idx = indexOfItemId(entry.id);
        if (idx < 0)
            return;
        chooserAnchorProxy.capture(entryRepeater.itemAt(idx));
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
            var groups = items[oidx].groups !== undefined
                         && items[oidx].groups !== null
                         ? items[oidx].groups : [];
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
        var id = chooserEntry && chooserEntry.id !== undefined
                 && chooserEntry.id !== null ? chooserEntry.id : "";
        var idx = id !== "" ? indexOfItemId(id) : -1;
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

    // Keep the open context menu bound to its fresh entry (and its delegate's
    // geometry), or dismiss it when the entry is gone. Without this a window
    // open/close while the menu is up rebuilds the delegate and the menu jumps
    // to the surface origin (T-14.7c stability).
    function refreshMenuAfterProjection() {
        if (!menuOpen)
            return;
        var id = menuEntry && menuEntry.id !== undefined && menuEntry.id !== null
                 ? menuEntry.id : "";
        var idx = id !== "" ? indexOfItemId(id) : -1;
        if (idx < 0) {
            entryMenu.hide();
            return;
        }
        menuEntry = items[idx];
        menuAnchorProxy.capture(entryRepeater.itemAt(idx));
        menuAnchorProxy.reposition();
    }

    // Keep the open folder stack bound to its fresh listing, or dismiss it when
    // the stack is gone. Opening a stack clears its badge, which rebuilds the
    // entry model on the very next projection; the proxy is what keeps the
    // popover anchored across that rebuild (T-14.7c stability).
    function refreshStackAfterProjection() {
        if (!stackOpen)
            return;
        var id = stackEntryRef && stackEntryRef.id !== undefined
                 && stackEntryRef.id !== null ? stackEntryRef.id : "";
        var idx = id !== "" ? indexOfItemId(id) : -1;
        if (idx < 0) {
            stackPopover.hide();
            return;
        }
        stackEntryRef = items[idx];
        stackAnchorProxy.capture(entryRepeater.itemAt(idx));
        stackAnchorProxy.reposition();
    }

    // Keep the Add Application picker anchored while pinning rebuilds the
    // entry model. The picker is a management surface, so a row toggle is the
    // common case, not an edge case (T-14.7e stability).
    function refreshAppPickerAfterProjection() {
        if (!appPickerOpen)
            return;
        var idx = indexOfItemId(dividerEntry.id);
        if (idx < 0) {
            appPicker.hide();
            return;
        }
        appPickerAnchorProxy.capture(entryRepeater.itemAt(idx));
        appPickerAnchorProxy.reposition();
    }

    // Keep the hover label on the fresh delegate after a rebuild, or dismiss it
    // when the entry is gone. The Tooltip reads the delegate geometry directly
    // (so it follows magnification), which is exactly why it must be re-bound
    // rather than held across a Repeater reset.
    function refreshTooltipAfterProjection() {
        if (!tooltipOpen && !tooltipDwellTimer.running)
            return;
        // The pointer may have left while the rebuild was queued; a re-bound
        // anchor must not resurrect a label for a pointer that is gone.
        if (pointerAlong < 0) {
            hideTooltip();
            tooltipAnchor = null;
            tooltipEntry = null;
            return;
        }
        var id = tooltipEntry && tooltipEntry.id !== undefined
                 && tooltipEntry.id !== null ? tooltipEntry.id : "";
        var idx = id !== "" ? indexOfItemId(id) : -1;
        // A rebuild that moved a different tile under the pointer must dismiss
        // the label instead of re-binding it to the old identity's new slot.
        if (idx < 0 || (anchorIndex >= 0 && idx !== anchorIndex)) {
            hideTooltip();
            tooltipAnchor = null;
            tooltipEntry = null;
            return;
        }
        tooltipEntry = items[idx];
        tooltipAnchor = entryRepeater.itemAt(idx);
    }

    // Keep the Empty Trash popover anchored while the right region reflows
    // (a folder pin or stack is added/removed). The Trash entry is permanent,
    // but its slot can move; the proxy follows the fresh delegate.
    function refreshTrashEmptyAfterProjection() {
        if (!trashEmptyOpen)
            return;
        var idx = indexOfItemId("__trash__");
        if (idx < 0) {
            trashEmptyPopover.hide();
            return;
        }
        trashEmptyAnchorProxy.capture(entryRepeater.itemAt(idx));
        trashEmptyAnchorProxy.reposition();
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
        overflowAnchorProxy.capture(entryRepeater.itemAt(idx));
        overflowAnchorProxy.reposition();
    }

    function refreshPopoversAfterProjection() {
        refreshChooserAfterProjection();
        refreshMenuAfterProjection();
        refreshStackAfterProjection();
        refreshAppPickerAfterProjection();
        refreshOverflowAfterProjection();
        refreshTrashEmptyAfterProjection();
        refreshTooltipAfterProjection();
    }

    // The entry model is rebuilt whenever the compositor projection changes.
    // A live DragHandler/divider delegate can be destroyed mid-gesture, which
    // would otherwise leave `dragging` or `resizing` latched on forever and
    // disable magnification/auto-hide for the rest of the session. Cancel the
    // gesture before that state can stick. An external drag belongs to the
    // shell, so only its resolved target is cleared.
    function reconcileInteractionAfterProjection() {
        if (dragging)
            resetDrag();
        if (resizing)
            endDividerResize();
        if (externalDragActive) {
            var externalChanged = false;
            if (externalTargetId !== "" && indexOfItemId(externalTargetId) < 0) {
                externalTargetId = "";
                springLoadTargetId = "";
                springLoadTimer.stop();
                externalChanged = true;
            }
            if (externalInsertIndex > appEntries.length) {
                externalInsertIndex = appEntries.length;
                externalChanged = true;
            }
            if (externalChanged)
                externalDragChanged();
        }
        if (keyboardFocused && focusedItemId !== ""
                && indexOfItemId(focusedItemId) < 0) {
            var list = navigableItems;
            focusedItemId = list.length > 0 ? list[0].id : "";
        }
        // A cancelled gesture must not leave an auto-hidden Dock stuck open.
        if (autoHide)
            scheduleHide();
    }

    // Open the "More Windows" list anchored to the overflow cell (T-14.7q).
    function openOverflow(entry) {
        if (!entry || entry.kind !== "overflow")
            return;
        closePopovers();
        hideTooltip();
        hideTimer.stop();
        var idx = indexOfItemId(entry.id);
        if (idx < 0)
            return;
        overflowEntryRef = entry;
        overflowAnchorProxy.capture(entryRepeater.itemAt(idx));
        overflowAnchor = overflowAnchorProxy;
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
        if (idx < 0)
            return;
        chooserAnchorProxy.capture(entryRepeater.itemAt(idx));
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
                    && e.windowList !== undefined && e.windowList !== null
                    && e.windowList.length > 1) {
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
        var idx = indexOfItemId(entry.id);
        if (idx < 0)
            return;
        stackEntryRef = entry;
        stackAnchorProxy.capture(entryRepeater.itemAt(idx));
        stackAnchor = stackAnchorProxy;
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
        if (idx < 0)
            return;
        appPickerAnchorProxy.capture(entryRepeater.itemAt(idx));
        appPickerAnchor = appPickerAnchorProxy;
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
        // The permanent Applications launcher is not draggable (no removal and
        // no reorder): it is a fixed affordance like the Trash.
        if (entry.appsLauncher === true)
            return false;
        return entry.kind !== "divider" && entry.kind !== "trash"
                && entry.kind !== "minimized" && entry.kind !== "external"
                && entry.kind !== "overflow";
    }

    function currentPinnedIds() {
        var out = [];
        for (var i = 0; i < appEntries.length; ++i) {
            if (appEntries[i].kind === "pinned"
                    && appEntries[i].appsLauncher !== true)
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
                if (pe.kind === "pinned" && pe.appsLauncher !== true
                        && pe.id !== dragEntryId)
                    out.push(pe.desktopId);
            }
            return out;
        }
        var vis = visualAppEntries;
        for (var j = 0; j < vis.length; ++j) {
            var e = vis[j];
            if (e.kind === "pinned" && e.appsLauncher !== true)
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
        hideTooltip();
        hideTimer.stop();
        revealTimer.stop();
        dragSettleTimer.stop();
        dragSettling = false;
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
        if (autoHide)
            scheduleHide();
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
            // The model and its computed layout can be observed on opposite
            // sides of an entry change for one evaluation (see `inputRects`);
            // skip the frame instead of aborting the whole drag with a
            // TypeError.
            if (i >= l.length || !items[i])
                continue;
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
            // See `itemIndexAtLocal`: a model/layout mismatch for one
            // evaluation must not abort the drag.
            if (i >= l.length)
                continue;
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
        dragSettleTimer.stop();
        dragSettling = false;
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
        if (autoHide)
            scheduleHide();
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
        var nm = e.name !== undefined && e.name !== null ? String(e.name) : "";
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
        if (e.appsLauncher === true)
            return appsLauncherMenuModel();
        var running = e.running === true;
        var minimized = e.kind === "minimized";
        var list = e.windowList !== undefined && e.windowList !== null
                   ? e.windowList : [];
        // The window list (T-10 sections 9/13): shown for any entry that
        // carries an app's windows, including a minimized-window entry (its
        // owning app's full list). The frontmost window is checked and each
        // minimized row is marked.
        if (list.length > 0) {
            for (var i = 0; i < list.length; ++i) {
                var w = list[i] || {};
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

    // The permanent Applications launcher's menu (T-19.2 follow-up): it opens
    // the drawer, and its Remove from Dock row is permanently disabled because
    // the tile is not removable. It owns no app identity, so it carries no
    // Keep/Remove, Options, or Show in Files rows.
    function appsLauncherMenuModel() {
        var out = [];
        out.push({
            type: "item", label: qsTr("Open Applications"),
            action: "open_apps_drawer"
        });
        out.push({ type: "separator" });
        out.push({
            type: "item", label: qsTr("Remove from Dock"),
            action: "remove_from_dock", enabled: false
        });
        return out;
    }

    // The open popover's rectangle in Dock-scene coordinates, or an empty
    // rect. The shell renders it into the Dock's `overlay` surface. A menu
    // with an open submenu reports the union of both panels so the nested
    // panel is not clipped (`ContextMenu.contentRect`).
    readonly property var activePopoverRect: {
        // An `open` popover owns the overlay. A closing popover can still be
        // `visible` for its fade tail; if it were preferred, opening a second
        // popover would keep committing the old one's rect until the fade
        // ended, leaving the new popover invisible. Fall back to the visible
        // tail only when nothing is open (so the close animation is captured).
        var popup = entryMenu.open ? entryMenu
                 : (windowChooser.open ? windowChooser
                 : (stackPopover.open ? stackPopover
                 : (appPicker.open ? appPicker
                 : (overflowPopover.open ? overflowPopover
                 : (trashEmptyPopover.open ? trashEmptyPopover
                 : (entryMenu.visible ? entryMenu
                 : (windowChooser.visible ? windowChooser
                 : (stackPopover.visible ? stackPopover
                 : (appPicker.visible ? appPicker
                 : (overflowPopover.visible ? overflowPopover
                 : (trashEmptyPopover.visible ? trashEmptyPopover
                 : null)))))))))));
        if (!popup || popup.width <= 0 || popup.height <= 0)
            return { x: 0, y: 0, w: 0, h: 0 };
        // Use the untransformed rect: the popover scales *down* while opening
        // (0.97 -> 1.0) and closing (1.0 -> 0.97) around its bottom edge, so the
        // untransformed box always covers the drawn content. Mapping through
        // `mapToItem` would bake the scale at evaluation time but would not
        // re-evaluate as the Behavior animates, leaving a stale overlay rect.
        if (popup.contentRect !== undefined) {
            var content = popup.contentRect;
            return { x: popup.x + content.x, y: popup.y + content.y,
                     w: content.w, h: content.h };
        }
        return { x: popup.x, y: popup.y, w: popup.width, h: popup.height };
    }

    // T202: the Dock popover overlay is a detached menu family. The shell
    // declares the active card with this preset so the compositor and QML
    // resolve the same optics.
    readonly property string activePopoverPreset: "menuDetached"
    readonly property real activePopoverRadius: Theme.controls.contextMenu.radius
    // A popover accepts pointer input only while logically open; the visible
    // close tail must not intercept clicks (T202).
    readonly property bool popoverInputActive:
        entryMenu.open || windowChooser.open || stackPopover.open
        || appPicker.open || overflowPopover.open || trashEmptyPopover.open

    // The hover label's rectangle in Dock-scene coordinates, or an empty rect
    // (T-14.7i). It rides the same popover/headroom path as a menu so a bottom
    // Dock's label is committed (and never clipped) like every other overlay.
    readonly property var tooltipRect: {
        // `visible` covers both the open state and the close fade tail.
        if (!dockTooltip.visible
                || dockTooltip.width <= 0 || dockTooltip.height <= 0)
            return { x: 0, y: 0, w: 0, h: 0 };
        // Direct child coordinates: the capsule scales down while opening and
        // closing around its centre, so the untransformed box covers the drawn
        // content and does not go stale as the scale Behavior animates.
        var x = dockTooltip.x;
        var y = dockTooltip.y;
        var w = dockTooltip.width;
        var h = dockTooltip.height;
        // The pointer tail paints beyond the capsule on the anchor side
        // (T-14.7aa); the committed overlay rect must cover it or the shell's
        // copy clips it off the overlay pass.
        if (dockTooltip.tailVisible) {
            var tail = dockTooltip.tailHeight;
            if (dock.tooltipPlacement === "above")
                h += tail;
            else if (dock.tooltipPlacement === "below") {
                y -= tail;
                h += tail;
            } else if (dock.tooltipPlacement === "left")
                w += tail;
            else {
                x -= tail;
                w += tail;
            }
        }
        return { x: x, y: y, w: w, h: h };
    }

    // The union of the open popover and the hover label; the shell commits it
    // to the Dock's overlay surface (T-10 section 5, T-14.7i). They never show
    // together, but the union keeps the close tail of either covered. The rect
    // is clamped into the pre-sized buffer's real image bounds: a menu taller
    // than the headroom is pinned to the buffer top, and reporting its full
    // height would make the shell's `QImage::copy` return a null image (no
    // overlay at all). The visible part is committed instead.
    readonly property var popoverRect: {
        var a = activePopoverRect;
        var b = tooltipRect;
        var aValid = a.w > 0 && a.h > 0;
        var bValid = b.w > 0 && b.h > 0;
        if (!aValid && !bValid)
            return { x: 0, y: 0, w: 0, h: 0 };
        if (!aValid)
            return clampPopoverRect(b);
        if (!bValid)
            return clampPopoverRect(a);
        var x0 = Math.min(a.x, b.x);
        var y0 = Math.min(a.y, b.y);
        var x1 = Math.max(a.x + a.w, b.x + b.w);
        var y1 = Math.max(a.y + a.h, b.y + b.h);
        return clampPopoverRect({ x: x0, y: y0, w: x1 - x0, h: y1 - y0 });
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

    // Clamp a popover rect to the part of the offscreen image that actually
    // exists. `clampPopoverX/Y` place the popover; this bounds its *size* for
    // the overlay commit, so a content-taller-than-headroom menu still produces
    // a valid copy instead of a null image (the scene clips the rest anyway).
    function clampPopoverRect(r) {
        if (popoverGutter <= 0 && popoverHeadroom <= 0)
            return r;
        var minX = popoverGutter > 0 ? -popoverGutter : r.x;
        var maxX = popoverGutter > 0 ? width + popoverGutter : r.x + r.w;
        var minY = popoverHeadroom > 0 ? -popoverHeadroom : r.y;
        var maxY = popoverHeadroom > 0 ? height : r.y + r.h;
        var x0 = Math.max(minX, Math.min(maxX, r.x));
        var x1 = Math.max(x0, Math.min(maxX, r.x + r.w));
        var y0 = Math.max(minY, Math.min(maxY, r.y));
        var y1 = Math.max(y0, Math.min(maxY, r.y + r.h));
        return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
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
        var info = bouncePhases ? bouncePhases[entry.id] : undefined;
        return info !== undefined && info.phase !== undefined ? info.phase : -1;
    }

    function entryAttention(entry) {
        if (!entry)
            return false;
        if (entry.attention === true)
            return true;
        var info = bouncePhases ? bouncePhases[entry.id] : undefined;
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
        var info = bouncePhases ? bouncePhases[entry.id] : undefined;
        return info !== undefined && info.minimize === true
               && info.minimizePhase !== undefined ? info.minimizePhase : -1;
    }

    function entryMinimizeActive(entry) {
        if (!entry)
            return false;
        if (entry.minimizeBounce !== undefined && entry.minimizeBounce >= 0)
            return true;
        var info = bouncePhases ? bouncePhases[entry.id] : undefined;
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
        // The reference launch lift is 15 pt (partial 60 fps evidence); the
        // attention bounce keeps its taller bar-relative amplitude. The
        // multi-hop shape is the existing adaptation, not a new curve.
        var amplitude = entryAttention(entry) ? barThickness / 2
                                              : Theme.controls.dock.launchLift;
        return amplitude * Math.sin(Math.PI * phase);
    }

    // --- Per-frame layout ------------------------------------------------
    // The magnified row for a pointer at `along` with the profile scaled by
    // `engaged`: the icon sizes, the leading-edge positions laid out from the
    // resting leading edge, and the continuous pointer warp pin (T-14.7aa).
    // `layout` calls this every frame; `magnifiedPlateLength` calls it for the
    // candidate pointers whose union sizes the fixed zoomed bar. One source of
    // truth for the magnification geometry.
    // The reference temporal envelope: g(u) = 3u^2 - 2u^3.
    function smoothstep01(t) {
        var u = Math.max(0, Math.min(1, t));
        return u * u * (3 - 2 * u);
    }

    // The continuous magnification weight for one entry (docs/dock.md §5).
    // `distance` is the horizontal distance from the pointer to the entry's
    // *baseline* centre in logical points. The raised cosine is flat at the
    // pointer, falls to zero exactly at the measured influence radius, and
    // has zero slope at both ends, so neither the peak nor the release kinks.
    function magnifyWeightAt(distance) {
        var r = magnifyInfluence;
        var d = Math.abs(distance);
        if (r <= 0)
            return d < 0.0001 ? 1 : 0;
        if (d >= r)
            return 0;
        return 0.5 * (1 + Math.cos(Math.PI * d / r));
    }

    function magnifiedRowAt(along, engaged, peakFactor) {
        var list = items;
        var n = list.length;
        var base = _baseline;
        var peak = iconSize * peakFactor;
        var sizes = [];
        var positions = [];
        var magnifiedCenters = [];
        var i;
        for (i = 0; i < n; ++i) {
            var s = base.sizes[i];
            if (list[i].kind !== "divider" && list[i].kind !== "external") {
                s = iconSize + (peak - iconSize) * engaged
                    * magnifyWeightAt(base.centers[i] - along);
            }
            sizes.push(s);
        }
        var cursor = base.positions[0];
        for (i = 0; i < n; ++i) {
            positions.push(cursor);
            magnifiedCenters.push(cursor + sizes[i] / 2);
            if (i < n - 1)
                cursor += sizes[i]
                        + scaledGap(sizes[i], sizes[i + 1],
                                    list[i].kind === "divider",
                                    list[i + 1].kind === "divider");
        }
        // Pin the pointer with a *continuous* warp: map the pointer through the
        // piecewise-linear resting-centre -> magnified-centre map and translate
        // the whole row so the warped pointer returns to the real one. Every
        // position is then a continuous function of the pointer, so crossing a
        // tile can never translate the row. The previous discrete anchor pin
        // did exactly that: the magnified pitch is wider than the resting
        // pitch, so the moment the anchor flipped the next tile snapped from
        // its pushed-out magnified position back to its resting centre — a
        // whole-pitch row jump on every boundary (~35 px with this profile).
        var warped = along;
        if (along <= base.centers[0]) {
            warped = magnifiedCenters[0] + (along - base.centers[0]);
        } else if (along >= base.centers[n - 1]) {
            warped = magnifiedCenters[n - 1] + (along - base.centers[n - 1]);
        } else {
            for (var w = 0; w < n - 1; ++w) {
                var c0 = base.centers[w];
                var c1 = base.centers[w + 1];
                if (along > c0 && along <= c1) {
                    var f = c1 > c0 ? (along - c0) / (c1 - c0) : 0;
                    warped = magnifiedCenters[w]
                           + f * (magnifiedCenters[w + 1] - magnifiedCenters[w]);
                    break;
                }
            }
        }
        var shift = along - warped;
        for (i = 0; i < n; ++i)
            positions[i] += shift;
        return { sizes: sizes, positions: positions };
    }

    // The swept union of the fully magnified row at a given peak factor. This
    // is the same measurement `magnifiedPlateLength` uses, but parameterized
    // so the fit below can search for a peak that fits the output.
    function magnifiedPlateLengthFor(peakFactor) {
        var list = items;
        var n = list.length;
        if (n === 0 || peakFactor <= 1)
            return axisIsX ? restingPlateRect.w : restingPlateRect.h;
        var base = _baseline;
        var candidates = [axisCenter];
        for (var c = 0; c < n; ++c)
            candidates.push(base.centers[c]);
        var half = 0;
        for (var k = 0; k < candidates.length; ++k) {
            var along = Math.max(0, Math.min(axisLength, candidates[k]));
            var row = magnifiedRowAt(along, 1.0, peakFactor);
            for (var i = 0; i < n; ++i) {
                if (list[i].kind === "divider" || row.sizes[i] <= 0)
                    continue;
                half = Math.max(half,
                                axisCenter - row.positions[i],
                                row.positions[i] + row.sizes[i] - axisCenter);
            }
        }
        return 2 * half + 2 * paddingAlong;
    }

    // Near a screen edge a full Dock's zoomed row can be wider than the
    // output. Instead of clipping the outermost tiles (docs/dock.md §5), lower
    // the peak uniformly until the swept union fits. The fit is recomputed only
    // when the resting geometry, icon size, or magnification changes — never
    // per pointer sample — so the sweep stays cheap. A Dock whose resting row
    // already exceeds the axis cannot be helped by zoom: keep it at rest.
    readonly property real magnifyFit: {
        if (magnification <= 0 || magnifyPeakFactor <= 1 || axisLength <= 0)
            return 1;
        var resting = axisIsX ? restingPlateRect.w : restingPlateRect.h;
        if (resting >= axisLength)
            return 0;
        if (magnifiedPlateLengthFor(magnifyPeakFactor) <= axisLength)
            return 1;
        // The union is monotone in the peak, so a short bisection lands well
        // inside a device pixel.
        var lo = 0;
        var hi = 1;
        for (var i = 0; i < 10; ++i) {
            var mid = (lo + hi) / 2;
            var peak = 1 + (magnifyPeakFactor - 1) * mid;
            if (magnifiedPlateLengthFor(peak) <= axisLength)
                lo = mid;
            else
                hi = mid;
        }
        return lo;
    }
    readonly property real effectiveMagnifyPeakFactor:
        1 + (magnifyPeakFactor - 1) * magnifyFit

    readonly property var layout: {
        var list = items;
        var n = list.length;
        var base = _baseline;
        var sizes = base.sizes.slice();
        var positions = base.positions.slice();
        var magnified = magnifyProfile;

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
                // A model rebuild can shrink the app region before the
                // reconcile handler cancels the drag; skip the stale slot for
                // this frame instead of writing undefined geometry.
                if (targetItem === undefined
                        || targetItem >= base.positions.length)
                    continue;
                positions[a] = base.positions[targetItem];
                sizes[a] = base.sizes[targetItem];
            }
        }

        if (magnified) {
            // The macOS reference profile (Dock_Tile_Mouseover.png, measured
            // 2026-09-28) is a quadratic bubble: ~90 % of the peak effect one
            // tile away, ~55 % two tiles away, and zero by ~4 icon widths.
            // `magnifyEngagement` scales the whole bubble so the zoom grows and
            // shrinks once as the pointer enters and leaves (T-14.7aa).
            var along = Math.max(0, magnifyPointer);
            var engaged = Math.max(0, magnifyEngagement);
            var row = magnifiedRowAt(along, engaged,
                                     effectiveMagnifyPeakFactor);
            sizes = row.sizes;
            positions = row.positions;
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

    // The bar's single magnified along-axis length (T-14.7aa). macOS resizes the
    // dock background horizontally between exactly two levels — base and one
    // zoomed level — and never varies that size while the pointer moves along
    // the Dock. The zoomed level is the widest union the magnified row can
    // reach, measured from the surface centre across the candidate pointers at
    // every resting centre (the row translates under the pointer, so the fixed
    // bar must cover the swept extent, not just one position), plus the end
    // padding. It is centred on the surface, so the base and zoomed levels
    // share a centre and the bar cannot shift sideways between them.
    readonly property real magnifiedPlateLength:
        magnifiedPlateLengthFor(effectiveMagnifyPeakFactor)

    // The visible floating plate. Its cross-axis extent — the height on a
    // bottom Dock — is fixed at the resting bar, and its along-axis extent has
    // only two levels, base and `magnifiedPlateLength`, blended by the hover
    // engagement: the background grows once when the zoom engages and shrinks
    // once when it releases, but never tracks the pointer, so moving along the
    // Dock cannot resize or move the background (T-14.7aa). The plate stays
    // centred on the surface and is the single source for the plate drawing,
    // the input region, and the declared backdrop panel rect.
    readonly property var plateRect: {
        var engagement = Math.max(0, Math.min(1, magnifyEngagement));
        if (axisIsX) {
            var restingW = restingPlateRect.w;
            var zoomedW = Math.max(restingW, magnifiedPlateLength);
            var w = restingW + (zoomedW - restingW) * engagement;
            if (w >= width)
                return { x: 0, y: restingPlateRect.y, w: width,
                         h: restingPlateRect.h };
            return { x: (width - w) / 2, y: restingPlateRect.y, w: w,
                     h: restingPlateRect.h };
        }
        var restingH = restingPlateRect.h;
        var zoomedH = Math.max(restingH, magnifiedPlateLength);
        var h = restingH + (zoomedH - restingH) * engagement;
        if (h >= height)
            return { x: restingPlateRect.x, y: 0, w: restingPlateRect.w,
                     h: height };
        return { x: restingPlateRect.x, y: (height - h) / 2, w: restingPlateRect.w,
                 h: h };
    }

    // The plate's rounded edge snapped to the integer pixel grid (T-14.7z).
    // The QML plate draws this rect and the shell declares the same rect to the
    // compositor as the backdrop panel, so the translucent fill and the frost
    // share one rounded edge instead of two independently rasterized ones (no
    // 1 px fringe as the plate grows). Each edge is snapped on its own so the
    // anchored edge never wobbles when the magnify edge moves by a fraction.
    readonly property var panelRect: {
        var r = plateRect;
        var left = Math.round(r.x);
        var top = Math.round(r.y);
        var right = Math.round(r.x + r.w);
        var bottom = Math.round(r.y + r.h);
        return { x: left, y: top,
                 w: Math.max(0, right - left), h: Math.max(0, bottom - top) };
    }

    // The interior-edge rim outline as an SVG path for the plate's rounded
    // rectangle, inset by half the stroke so every rim pixel lies inside the
    // plate shape (T-14.7z). The path traces the flat interior edge and both
    // interior corner arcs, so the highlight follows the curve instead of
    // poking a straight hairline across the corner. `w`/`h` are the already
    // inset box (the Shape's own size). `strokeWidth` defaults to the rim
    // hairline; the interior gloss band passes its own width so both strokes
    // share one path definition.
    function rimOutlinePath(w, h, strokeWidth) {
        if (strokeWidth === undefined)
            strokeWidth = Theme.controls.dock.plate.rimHeight;
        var d = strokeWidth / 2;
        var r = Math.max(0, Math.min(Theme.controls.dock.radius - d,
                                      Math.min(w, h) / 2));
        if (position === "right")
            return "M " + r + " 0 A " + r + " " + r + " 0 0 0 0 " + r
                 + " L 0 " + (h - r) + " A " + r + " " + r + " 0 0 0 " + r + " " + h;
        if (position === "left")
            return "M " + (w - r) + " 0 A " + r + " " + r + " 0 0 1 " + w + " " + r
                 + " L " + w + " " + (h - r) + " A " + r + " " + r + " 0 0 1 "
                 + (w - r) + " " + h;
        // bottom (default): the interior edge is the top.
        return "M 0 " + r + " A " + r + " " + r + " 0 0 1 " + r + " 0"
             + " L " + (w - r) + " 0 A " + r + " " + r + " 0 0 1 " + w + " " + r;
    }

    // The full plate silhouette as an SVG path. The fallback fill gradients
    // are clipped to this same rounded rect, so the top/bottom lifts cannot
    // leak past the corner arcs.
    function platePath(w, h) {
        var r = Math.max(0, Math.min(Theme.controls.dock.radius, Math.min(w, h) / 2));
        return "M " + r + " 0"
             + " L " + (w - r) + " 0 A " + r + " " + r + " 0 0 1 " + w + " " + r
             + " L " + w + " " + (h - r) + " A " + r + " " + r + " 0 0 1 "
             + (w - r) + " " + h
             + " L " + r + " " + h + " A " + r + " " + r + " 0 0 1 0 " + (h - r)
             + " L 0 " + r + " A " + r + " " + r + " 0 0 1 " + r + " 0 Z";
    }

    // The bottom edge plus both bottom corner arcs, inset by half the stroke.
    // Used for the bottom brightening rim on a bottom Dock; the straight
    // middle is already painted by `dockEdge`, so this path adds the corner
    // arcs without changing the anchored-edge lip.
    function bottomOutlinePath(w, h, strokeWidth) {
        if (strokeWidth === undefined)
            strokeWidth = Theme.controls.dock.plate.rimHeight;
        var d = strokeWidth / 2;
        var r = Math.max(0, Math.min(Theme.controls.dock.radius - d,
                                      Math.min(w, h) / 2));
        return "M 0 " + (h - r) + " A " + r + " " + r + " 0 0 1 " + r + " " + h
             + " L " + (w - r) + " " + h + " A " + r + " " + r + " 0 0 1 "
             + w + " " + (h - r);
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
            // The model and its computed layout can be observed for one
            // binding evaluation on opposite sides of an entry change (the
            // startup pin seed, a folder/trash update). A missing delegate is
            // not a fatal QML error; skip it for this frame instead of
            // aborting the whole expression and leaving inputRects undefined.
            if (i >= items.length || !items[i])
                continue;
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

        // The plate's origin inside this group (the group is the plate plus
        // the edge room the shadow may use). The group draws the integer
        // `panelRect`, the same rounded rect the shell declares to the
        // compositor, so the fill and the frost share one edge (T-14.7z).
        readonly property real plateX: dock.panelRect.x - x
        readonly property real plateY: dock.panelRect.y - y
        readonly property real plateW: dock.panelRect.w
        readonly property real plateH: dock.panelRect.h
        readonly property real plateRadius: Theme.controls.dock.radius

        x: dock.axisIsX || dock.position === "right" ? dock.panelRect.x : 0
        y: dock.panelRect.y
        width: dock.axisIsX ? dock.panelRect.w
               : dock.position === "right" ? dock.width - dock.panelRect.x
               : dock.panelRect.x + dock.panelRect.w
        height: dock.axisIsX ? Math.max(0, dock.height - dock.panelRect.y)
                             : dock.panelRect.h
        clip: true

        Shadow {
            objectName: "dockPlateShadow"
            visible: !dock.sampledPlate
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
            visible: !dock.sampledPlate
            x: dockPlate.plateX
            y: dockPlate.plateY
            width: dockPlate.plateW
            height: dockPlate.plateH
            radius: dockPlate.plateRadius
            // With the sampled pass active the compositor paints the flat
            // band/rim, so this layer is hidden. Unsupported paths and the
            // accessibility override keep one coherent shell-painted plate:
            // the replacement look uses the resolved neutral alpha/tint, the
            // legacy look keeps its milky fallback.
            color: dock.replacementPlate
                   ? Qt.rgba(dock.glassOptics.tint, dock.glassOptics.tint,
                             dock.glassOptics.tint, 1)
                   : (dock.materialBlurActive && !Glass.reduceTransparency
                      ? Theme.color.dockFill : Theme.color.dockFallbackFill)
            opacity: dock.replacementPlate
                     ? dock.glassOptics.alpha
                     : (dock.materialBlurActive && !Glass.reduceTransparency
                        ? Theme.controls.dock.plate.fillOpacity
                        : Theme.material.dockFallbackOpacity)
        }

        // The interior-edge gloss: a soft band just inside the rim that fades
        // the plate's interior edge into its body, so the material reads as
        // glass rather than a flat slab (T-14.7aa). It shares the rim's
        // rounded path, inset by half its stroke, so it follows the corner
        // arcs exactly like the hairline above it.
        Shape {
            id: dockGloss
            objectName: "dockGloss"
            visible: !dock.sampledPlate
            readonly property real inset: Theme.controls.dock.plate.glossHeight / 2
            x: dockPlate.plateX + inset
            y: dockPlate.plateY + inset
            width: Math.max(0, dockPlate.plateW - 2 * inset)
            height: Math.max(0, dockPlate.plateH - 2 * inset)
            preferredRendererType: Shape.GeometryRenderer
            antialiasing: true
            opacity: Theme.controls.dock.plate.glossOpacity
            ShapePath {
                objectName: "dockGlossStroke"
                strokeColor: Theme.color.dockRim
                strokeWidth: Theme.controls.dock.plate.glossHeight
                fillColor: "transparent"
                capStyle: ShapePath.FlatCap
                PathSvg {
                    path: dock.rimOutlinePath(dockGloss.width, dockGloss.height,
                                              Theme.controls.dock.plate.glossHeight)
                }
            }
        }

        // The top and bottom interior lifts. The reference plate is flat in
        // the centre but brightens over ~10 px at the top and ~12 px at the
        // bottom; these gradients are clipped to the same silhouette as the
        // fill, so they follow the corner arcs.
        Shape {
            id: dockTopGlow
            objectName: "dockTopGlow"
            visible: !dock.sampledPlate && dock.position === "bottom"
            x: dockPlate.plateX
            y: dockPlate.plateY
            width: dockPlate.plateW
            height: dockPlate.plateH
            preferredRendererType: Shape.GeometryRenderer
            antialiasing: true
            ShapePath {
                strokeWidth: 0
                fillGradient: LinearGradient {
                    x1: 0
                    y1: 0
                    x2: 0
                    y2: dockPlate.plateH
                    GradientStop {
                        position: 0.0
                        color: Qt.rgba(1, 1, 1,
                                       Theme.controls.dock.plate.topGlowOpacity)
                    }
                    GradientStop {
                        position: Math.min(1.0,
                            Theme.controls.dock.plate.topGlowHeight
                            / Math.max(1, dockPlate.plateH))
                        color: "transparent"
                    }
                    GradientStop { position: 1.0; color: "transparent" }
                }
                PathSvg { path: dock.platePath(dockPlate.plateW, dockPlate.plateH) }
            }
        }

        Shape {
            id: dockBottomGlow
            objectName: "dockBottomGlow"
            visible: !dock.sampledPlate && dock.position === "bottom"
            x: dockPlate.plateX
            y: dockPlate.plateY
            width: dockPlate.plateW
            height: dockPlate.plateH
            preferredRendererType: Shape.GeometryRenderer
            antialiasing: true
            ShapePath {
                strokeWidth: 0
                fillGradient: LinearGradient {
                    x1: 0
                    y1: 0
                    x2: 0
                    y2: dockPlate.plateH
                    GradientStop { position: 0.0; color: "transparent" }
                    GradientStop {
                        position: Math.max(0.0,
                            1.0 - Theme.controls.dock.plate.bottomGlowHeight
                            / Math.max(1, dockPlate.plateH))
                        color: "transparent"
                    }
                    GradientStop {
                        position: 1.0
                        color: Qt.rgba(1, 1, 1,
                                       Theme.controls.dock.plate.bottomGlowOpacity)
                    }
                }
                PathSvg { path: dock.platePath(dockPlate.plateW, dockPlate.plateH) }
            }
        }

        // The bottom edge highlight follows the two bottom corner arcs; the
        // straight middle is the anchored-edge lip below.
        Shape {
            id: dockBottomRim
            objectName: "dockBottomRim"
            z: 1
            visible: !dock.sampledPlate && dock.position === "bottom"
            readonly property real inset: Theme.controls.dock.plate.rimHeight / 2
            x: dockPlate.plateX + inset
            y: dockPlate.plateY + inset
            width: Math.max(0, dockPlate.plateW - 2 * inset)
            height: Math.max(0, dockPlate.plateH - 2 * inset)
            preferredRendererType: Shape.GeometryRenderer
            antialiasing: true
            opacity: Theme.controls.dock.plate.edgeOpacity
            ShapePath {
                objectName: "dockBottomRimStroke"
                strokeColor: Theme.color.dockRim
                strokeWidth: Theme.controls.dock.plate.rimHeight
                fillColor: "transparent"
                capStyle: ShapePath.FlatCap
                PathSvg {
                    path: dock.bottomOutlinePath(dockBottomRim.width,
                                                 dockBottomRim.height)
                }
            }
        }

        // The bright inner highlight: a stroked path along the interior edge
        // that follows the plate's corner arcs instead of a straight hairline
        // poking into them (T-14.7z). It is inset by half the stroke so every
        // rim pixel lies inside the plate shape. Only the interior edge is
        // traced: the top for a bottom Dock, the interior side for a vertical
        // one.
        Shape {
            id: dockRim
            objectName: "dockRim"
            z: 1
            visible: !dock.sampledPlate
            readonly property real inset: Theme.controls.dock.plate.rimHeight / 2
            x: dockPlate.plateX + inset
            y: dockPlate.plateY + inset
            width: Math.max(0, dockPlate.plateW - 2 * inset)
            height: Math.max(0, dockPlate.plateH - 2 * inset)
            preferredRendererType: Shape.GeometryRenderer
            antialiasing: true
            opacity: Math.min(1, Theme.controls.dock.plate.rimOpacity
                                 + dock.contrastEdgeBoost)
            ShapePath {
                objectName: "dockRimStroke"
                strokeColor: Theme.color.dockRim
                strokeWidth: Theme.controls.dock.plate.rimHeight
                fillColor: "transparent"
                capStyle: ShapePath.FlatCap
                PathSvg { path: dock.rimOutlinePath(dockRim.width, dockRim.height) }
            }
        }

        // The anchored-edge highlight: a short bright lip along the flat
        // middle of the plate's screen-edge side (the reference's lower edge,
        // Dock_Tile_Mouseover.png). It is inset past the corner arcs on the
        // along axis, so it never pokes outside the rounded shape, and it is
        // orientation-aware like the interior rim.
        Rectangle {
            objectName: "dockEdge"
            z: 1
            readonly property real corner: Theme.controls.dock.radius
            readonly property bool horizontal: dock.position === "bottom"
            color: Theme.color.dockRim
            opacity: Math.min(1, Theme.controls.dock.plate.edgeOpacity
                                 + dock.contrastEdgeBoost)
            visible: !dock.sampledPlate && width > 0 && height > 0
            x: dock.position === "left" ? dockPlate.plateX
               : dock.position === "right" ? dockPlate.plateX + dockPlate.plateW - width
               : dockPlate.plateX + corner
            y: dock.position === "bottom" ? dockPlate.plateY + dockPlate.plateH - height
               : dockPlate.plateY + corner
            width: horizontal
                   ? Math.max(0, dockPlate.plateW - 2 * corner)
                   : Theme.controls.dock.plate.edgeHeight
            height: horizontal
                    ? Theme.controls.dock.plate.edgeHeight
                    : Math.max(0, dockPlate.plateH - 2 * corner)
        }

        // The hairline border around the whole plate.
        Rectangle {
            objectName: "dockBorder"
            visible: !dock.sampledPlate
            x: dockPlate.plateX
            y: dockPlate.plateY
            width: dockPlate.plateW
            height: dockPlate.plateH
            radius: dockPlate.plateRadius
            color: "transparent"
            border.width: Theme.controls.dock.plate.borderWidth
            border.color: Theme.color.dockBorder
            opacity: Math.min(1, Theme.controls.dock.plate.borderOpacity
                                 + dock.contrastEdgeBoost)
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
            artworkRasterSize: Math.ceil(dock.iconSize * dock.effectiveMagnifyPeakFactor)
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
            // The anchored tile of the magnified profile: it drops the static
            // hover wash and draws the zoom lift shadow instead, because the
            // reference hover state is the zoom itself (T-14.7aa).
            zoomed: dock.magnifyProfile && dock.magnifyPointerIndex === index
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
        materialPreset: "menuDetached"
        materialBackdrop: "trusted"
        materialBlurActive: dock.popupMaterialBlurActive
        materialTier: dock.popupMaterialTier
        materialFallback: dock.popupMaterialFallback
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
            if (!item)
                return;
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
            // The Applications launcher's Open row toggles the drawer locally
            // (T-19.2 follow-up).
            if (item.action === "open_apps_drawer") {
                dock.closePopovers();
                dock.appsDrawerRequested();
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
        materialBlurActive: dock.popupMaterialBlurActive
        materialTier: dock.popupMaterialTier
        materialFallback: dock.popupMaterialFallback
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
        materialBlurActive: dock.popupMaterialBlurActive
        materialTier: dock.popupMaterialTier
        materialFallback: dock.popupMaterialFallback
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
        materialBlurActive: dock.popupMaterialBlurActive
        materialTier: dock.popupMaterialTier
        materialFallback: dock.popupMaterialFallback
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
        materialBlurActive: dock.popupMaterialBlurActive
        materialTier: dock.popupMaterialTier
        materialFallback: dock.popupMaterialFallback
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
        materialBlurActive: dock.popupMaterialBlurActive
        materialTier: dock.popupMaterialTier
        materialFallback: dock.popupMaterialFallback
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
        // The controlled reference capsule sits 8 pt above the plate; its
        // width is still label-driven and it has no pointer tail.
        anchorRect: dock.tooltipAnchorRect
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
