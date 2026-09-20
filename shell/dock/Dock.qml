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
// GVfs-backed Trash state are follow-ups (see PROGRESS.md).
Rectangle {
    id: dock

    // --- Injected data --------------------------------------------------
    // [{ id, appId, name, kind, running, minimized, launch, attention,
    //    badge, pinned, trashFull }]
    property var entries: []
    property string position: "bottom"          // bottom | left | right
    property real iconSize: Theme.controls.dock.iconSize
    property real magnification: 0.0            // 0..1, 0 = off
    property bool showIndicators: true
    property bool minimizeIntoTileIcon: false
    property bool autoHide: false
    property bool revealed: true
    property bool trashFull: false

    // --- Interaction state ----------------------------------------------
    // Pointer position along the dock axis in local coordinates, -1 when the
    // pointer is not over the Dock.
    property real pointerAlong: -1
    property bool dragging: false

    signal entryActivated(var entry)
    signal entryContextMenuRequested(var entry, real globalX, real globalY)
    signal dividerContextMenuRequested(real globalX, real globalY)
    signal settingsRequested()

    // --- Geometry constants ---------------------------------------------
    readonly property real padding: Theme.controls.dock.padding
    readonly property real gap: Theme.controls.dock.gap
    readonly property real dividerWidth: 1
    readonly property real barThickness: iconSize + 2 * padding
    readonly property real magnifyPeak: Theme.controls.dock.magnifyPeak
    readonly property real magnifyFalloff: Theme.controls.dock.magnifyFalloff
    // Transparent room above/beside the bar that magnified artwork grows
    // into. The reserved zone is `barThickness` only (section 2).
    readonly property real magnifyBand:
        Math.ceil((magnifyPeak - 1) * iconSize) + padding
    readonly property bool axisIsX: position === "bottom"
    readonly property string indicatorEdge:
        position === "bottom" ? "bottom" : (position === "left" ? "left" : "right")
    readonly property real axisLength: axisIsX ? width : height

    // --- Auto-hide translation ------------------------------------------
    readonly property real hideOffset:
        autoHide && !revealed ? barThickness + Theme.controls.dock.edgeMargin : 0

    function reveal() { revealed = true; }
    function hide() { if (autoHide) revealed = false; }

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
        running: false, trashFull: dock.trashFull
    })
    readonly property var dividerEntry: ({ id: "__divider__", kind: "divider" })

    // Ordered items: apps, divider, minimized windows, Trash.
    readonly property var items: {
        var out = appEntries.slice();
        out.push(dividerEntry);
        for (var i = 0; i < minimizedEntries.length; ++i)
            out.push(minimizedEntries[i]);
        out.push(trashEntry);
        return out;
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

        if (magnified) {
            var peak = iconSize * magnifyPeak;
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
                    y: band + barThickness - padding - h - hideOffset - bounce,
                    w: isDivider ? dividerWidth : sizes[j],
                    h: isDivider ? barThickness - 2 * padding : h,
                    iconSize: sizes[j]
                });
            } else {
                var w = sizes[j] + extra;
                var vx = position === "right" ? band + padding - bounce
                                              : band + padding + bounce;
                out.push({
                    x: vx,
                    y: positions[j] - hideOffset,
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
                y: magnifyBand - hideOffset,
                w: base.total + 2 * padding,
                h: barThickness
            };
        return {
            x: magnifyBand - hideOffset,
            y: base.positions[0] - padding,
            w: barThickness,
            h: base.total + 2 * padding
        };
    }

    // The surface input region: the visible bar plus the currently magnified
    // or bouncing icon rectangles. The transparent magnified band passes
    // clicks through to the windows beneath, and the hidden Dock is fully
    // transparent to input (T-10 FR-13). The shell commits this every frame.
    readonly property var inputRects: {
        if (autoHide && !revealed)
            return [];
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

            entry: modelData
            iconSize: dock.layout.length > index ? dock.layout[index].iconSize : dock.iconSize
            indicatorEdge: dock.indicatorEdge
            showIndicator: dock.showIndicators
            dragging: dock.dragging
            width: dock.layout.length > index ? dock.layout[index].w : dock.iconSize
            height: dock.layout.length > index ? dock.layout[index].h : dock.iconSize
            x: dock.layout.length > index ? dock.layout[index].x : 0
            y: dock.layout.length > index ? dock.layout[index].y : 0

            onActivated: (entry) => dock.entryActivated(entry)
            onContextMenuRequested: (entry, gx, gy) => {
                if (entry.kind === "divider")
                    dock.dividerContextMenuRequested(gx, gy);
                else
                    dock.entryContextMenuRequested(entry, gx, gy);
            }
        }
    }

    // Pointer tracking for magnification. HoverHandlers do not consume
    // events, so the per-entry handlers still work.
    HoverHandler {
        id: dockHover
        onPointChanged: dock.pointerAlong =
            dock.axisIsX ? point.position.x : point.position.y
        onHoveredChanged: {
            if (!hovered)
                dock.pointerAlong = -1;
        }
    }

    // Test/introspection hooks.
    function itemAt(index) { return entryRepeater.itemAt(index); }
    function itemCount() { return entryRepeater.count; }
}
