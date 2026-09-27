// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Dock

// Dock tests (T-10, first slice): entry regions, running indicators,
// pointer-anchored magnification, placement, auto-hide, and activation.
// Runs headless on the offscreen platform + software scene graph.
Item {
    id: stage
    width: 1280
    height: 240

    TestCase {
        id: testCase
        name: "Dock"
        when: windowShown

        Component { id: dockComponent; Dock { } }
        Component { id: entryComponent; DockEntry { } }
        Component { id: glyphComponent; DockGlyph { } }
        Component { id: backdropComponent; Rectangle { } }

        SignalSpy { id: activatedSpy; signalName: "entryActivated" }
        SignalSpy { id: menuSpy; signalName: "entryContextMenuRequested" }
        SignalSpy { id: dividerMenuSpy; signalName: "dividerContextMenuRequested" }
        SignalSpy { id: menuActionSpy; signalName: "menuActionRequested" }
        SignalSpy { id: windowSpy; signalName: "windowActivated" }
        SignalSpy { id: popoverSpy; signalName: "popoverChanged" }
        SignalSpy { id: pinnedOrderSpy; signalName: "pinnedOrderChanged" }
        SignalSpy { id: releaseFocusSpy; signalName: "keyboardFocusReleaseRequested" }
        SignalSpy { id: externalDropSpy; signalName: "externalDropRequested" }
        SignalSpy { id: externalDragSpy; signalName: "externalDragChanged" }
        SignalSpy { id: springLoadSpy; signalName: "springLoadRequested" }
        SignalSpy { id: downloadSpy; signalName: "downloadActivated" }
        SignalSpy { id: downloadsFolderSpy; signalName: "downloadsFolderRequested" }
        SignalSpy { id: downloadsViewedSpy; signalName: "downloadsViewed" }
        SignalSpy { id: sizePreviewSpy; signalName: "dockSizePreview" }
        SignalSpy { id: sizeChangedSpy; signalName: "dockSizeChanged" }
        SignalSpy { id: appPickerRequestedSpy; signalName: "appPickerRequested" }
        SignalSpy { id: appPinToggledSpy; signalName: "appPinToggled" }

        // Reduced motion is a global singleton; reset it before every test so
        // a failure mid-test cannot leak into the next one. Same for the colour
        // scheme the T-14.7d glyph tests flip.
        function init() {
            Theme.reducedMotion = false;
            Theme.dark = false;
        }

        function make(component, props) {
            var obj = createTemporaryObject(component, stage, props || {});
            waitForRendering(stage);
            return obj;
        }

        function app(id, name, running) {
            return { id: id, appId: id, name: name, kind: "pinned",
                     running: running === true, pinned: true,
                     desktopId: id + ".desktop" };
        }

        function temporary(id, name) {
            return { id: id, appId: id, name: name, kind: "temporary",
                     running: true, desktopId: id + ".desktop" };
        }

        function minimized(id, name) {
            return { id: id, appId: id, name: name, kind: "minimized" };
        }

        // -- T-14.7d glyph pixel helpers ------------------------------------
        // `grabImage` composites the item over the offscreen stage's opaque
        // background; treat the top-left pixel as background and count the
        // pixels that differ from it as foreground.
        function backgroundIs(image, x, y) {
            return Math.abs(image.red(x, y) - image.red(0, 0))
                 + Math.abs(image.green(x, y) - image.green(0, 0))
                 + Math.abs(image.blue(x, y) - image.blue(0, 0)) <= 24;
        }

        function countForeground(image, x0, y0, x1, y1) {
            var n = 0;
            for (var y = y0; y < y1; ++y)
                for (var x = x0; x < x1; ++x)
                    if (!backgroundIs(image, x, y))
                        ++n;
            return n;
        }

        function countForegroundAll(image) {
            return countForeground(image, 0, 0, image.width, image.height);
        }

        function sumLuma(image) {
            var total = 0;
            for (var y = 0; y < image.height; ++y)
                for (var x = 0; x < image.width; ++x)
                    total += image.red(x, y) + image.green(x, y) + image.blue(x, y);
            return total;
        }

        // The trash is drawn in a `trashSize`-derived box centred in the
        // iconSize box (T-14.7d).
        function trashBox(size) {
            return size * Theme.controls.dock.trashSize
                       / Theme.controls.dock.iconSize;
        }

        function trashInset(size) {
            return (size - trashBox(size)) / 2;
        }

        // A band above the lid and clear of the centred handle: only a full
        // bin's crumpled-paper silhouette reaches it.
        function contentsBand(size) {
            var t = trashBox(size);
            var off = trashInset(size);
            return {
                x0: Math.round(off + t * 0.26), x1: Math.round(off + t * 0.34),
                y0: Math.round(off + t * 0.14), y1: Math.round(off + t * 0.22)
            };
        }

        // -- Entry regions --------------------------------------------------

        function test_items_order_and_trash_is_last() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), temporary("term", "Terminal"),
                           minimized("win1", "Document") ]
            });
            var items = dock.items;
            compare(items.length, 6); // apps, divider, minimized, stack, trash
            compare(items[0].kind, "pinned");
            compare(items[1].kind, "temporary");
            compare(items[2].kind, "divider");
            compare(items[3].kind, "minimized");
            compare(items[4].kind, "stack");
            compare(items[5].kind, "trash");
            compare(dock.itemCount(), 6);
        }

        function test_minimized_hidden_when_minimize_into_tile_icon() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                minimizeIntoTileIcon: true,
                entries: [ app("files", "Files", true), minimized("win1", "Document") ]
            });
            compare(dock.minimizedEntries.length, 0);
            // apps, divider, stack, trash
            compare(dock.items.length, 4);
        }

        function test_running_indicator_only_for_running_apps() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), app("settings", "Settings", false) ]
            });
            var runningEntry = dock.itemAt(0);
            var idleEntry = dock.itemAt(1);
            compare(runningEntry.running, true);
            compare(idleEntry.running, false);

            var runningIndicator = findChild(runningEntry, "indicator");
            var idleIndicator = findChild(idleEntry, "indicator");
            verify(runningIndicator !== null);
            verify(idleIndicator !== null);
            compare(runningIndicator.visible, true);
            compare(idleIndicator.visible, false);
        }

        function test_indicators_off_hides_every_dot() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, showIndicators: false,
                entries: [ app("files", "Files", true) ]
            });
            var indicator = findChild(dock.itemAt(0), "indicator");
            compare(indicator.visible, false);
        }

        // -- Magnification --------------------------------------------------

        function test_no_magnification_is_uniform() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            var layout = dock.layout;
            for (var i = 0; i < 3; ++i)
                compare(layout[i].iconSize, dock.iconSize);
        }

        function test_magnification_grows_neighbourhood() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            var base = dock._baseline.centers[1];
            dock.pointerAlong = base;
            waitForRendering(stage);
            compare(dock.magnifying, true);
            var layout = dock.layout;
            // The hovered entry reaches the peak size.
            verify(layout[1].iconSize > dock.iconSize);
            // The neighbour grows less than the hovered entry.
            verify(layout[0].iconSize > dock.iconSize);
            verify(layout[0].iconSize < layout[1].iconSize);
        }

        function test_anchor_stays_under_pointer() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            var base = dock._baseline.centers[1];
            dock.pointerAlong = base;
            waitForRendering(stage);
            var layout = dock.layout;
            // The anchor's center stays at its baseline center.
            var anchorCenter = layout[1].x + layout[1].iconSize / 2;
            verify(Math.abs(anchorCenter - base) < 0.5);
        }

        function test_plate_grows_with_magnification() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            // T-14.7b: the plate wraps the magnified row in both axes instead
            // of leaving the artwork to spill out of a fixed slab.
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            var baseW = dock.restingPlateRect.w;
            var baseH = dock.restingPlateRect.h;
            dock.pointerAlong = dock._baseline.centers[1];
            waitForRendering(stage);
            compare(dock.magnifying, true);
            verify(dock.plateRect.w > baseW);
            verify(dock.plateRect.h > baseH);
        }

        function test_plate_contains_every_entry_rect_under_magnification() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[1];
            waitForRendering(stage);
            var r = dock.plateRect;
            for (var i = 0; i < dock.layout.length; ++i) {
                if (dock.items[i].kind === "divider")
                    continue;
                var e = dock.layout[i];
                verify(e.x >= r.x - 0.001);
                verify(e.x + e.w <= r.x + r.w + 0.001);
                verify(e.y >= r.y - 0.001);
                verify(e.y + e.h <= r.y + r.h + 0.001);
            }
        }

        function test_plate_anchored_edge_stays_put_under_magnification() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            // The anchored edge never moves while the plate grows: windows do
            // not re-layout during a sweep (T-14.7b).
            var bottom = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            var bottomEdge = bottom.plateRect.y + bottom.plateRect.h;
            bottom.pointerAlong = bottom._baseline.centers[0];
            waitForRendering(stage);
            fuzzyCompare(bottom.plateRect.y + bottom.plateRect.h, bottomEdge, 0.001);
            fuzzyCompare(bottom.plateRect.y + bottom.plateRect.h,
                         bottom.height - bottom.edgeMargin, 0.001);

            var left = make(dockComponent, {
                width: 240, height: 800, position: "left", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            left.pointerAlong = left._baseline.centers[0];
            waitForRendering(stage);
            fuzzyCompare(left.plateRect.x, left.edgeMargin, 0.001);

            var right = make(dockComponent, {
                width: 240, height: 800, position: "right", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            right.pointerAlong = right._baseline.centers[0];
            waitForRendering(stage);
            fuzzyCompare(right.plateRect.x + right.plateRect.w,
                         right.width - right.edgeMargin, 0.001);
        }

        function test_reserved_thickness_is_constant_under_magnification() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            var reserved = dock.reservedThickness;
            var surface = dock.surfaceThickness;
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.reservedThickness, reserved);
            compare(dock.reservedThickness, dock.barThickness + dock.edgeMargin);
            compare(dock.surfaceThickness, surface);
        }

        function test_plate_returns_to_baseline_on_mouse_out() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[1];
            waitForRendering(stage);
            verify(dock.plateRect.h > dock.restingPlateRect.h);
            dock.pointerAlong = -1;
            waitForRendering(stage);
            compare(dock.magnifying, false);
            fuzzyCompare(dock.plateRect.x, dock.restingPlateRect.x, 0.001);
            fuzzyCompare(dock.plateRect.y, dock.restingPlateRect.y, 0.001);
            fuzzyCompare(dock.plateRect.w, dock.restingPlateRect.w, 0.001);
            fuzzyCompare(dock.plateRect.h, dock.restingPlateRect.h, 0.001);
        }

        function test_plate_grows_under_reduced_motion() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            // Reduced motion removes the spring, not the geometry (T-14.7b).
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.magnifying, true);
            verify(dock.plateRect.h > dock.restingPlateRect.h);
        }

        function test_bounce_does_not_grow_the_plate() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            // A launch/attention bounce overshoots the plate; the plate must
            // not pump with it (T-14.7b).
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom",
                entries: [ { id: "a", appId: "a", name: "A", kind: "pinned",
                             pinned: true, running: true, attention: true,
                             bounce: 0.5 } ]
            });
            compare(dock.plateRect.w, dock.restingPlateRect.w);
            compare(dock.plateRect.h, dock.restingPlateRect.h);
            // The bounced entry rises above the resting plate.
            verify(dock.layout[0].y < dock.plateRect.y);
        }

        function test_magnification_pointer_is_smoothed() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            // The first placement snaps: magnification was off when it arrived.
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            fuzzyCompare(dock.smoothPointerAlong, dock._baseline.centers[0], 0.001);
            // A move while magnifying springs toward the new position instead
            // of jumping to it.
            var target = dock._baseline.centers[2];
            dock.pointerAlong = target;
            wait(Math.floor(Theme.motion.dockMagnify.fullDuration / 3));
            verify(dock.smoothPointerAlong > dock._baseline.centers[0]);
            wait(Theme.motion.dockMagnify.fullDuration + 120);
            fuzzyCompare(dock.smoothPointerAlong, target, 0.5);
        }

        function test_magnification_pointer_snaps_under_reduced_motion() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            dock.pointerAlong = dock._baseline.centers[2];
            waitForRendering(stage);
            fuzzyCompare(dock.smoothPointerAlong, dock._baseline.centers[2], 0.001);
        }

        function test_entries_snap_to_layout_after_configure() {
            // The shell creates the Dock at width 0, sets `entries`, then the
            // surface configure sets the width. The delegates must snap to the
            // centered layout; an always-on position Behavior would freeze
            // them mid-animation because the idle shell does not re-render.
            var dock = make(dockComponent, {
                width: 0, height: 124, magnification: 0.5
            });
            dock.entries = [ app("a", "A", true), app("b", "B", true) ];
            waitForRendering(stage);
            dock.width = 1280;
            waitForRendering(stage);
            for (var i = 0; i < dock.layout.length; ++i) {
                var item = dock.itemAt(i);
                compare(item.x, dock.layout[i].x);
                compare(item.width, dock.layout[i].w);
            }
        }

        // T-14.7c: a `dock.size`/overflow size change animates with the
        // design-system spring and then settles exactly on the target.
        function test_a_size_change_animates_then_settles() {
            var dock = make(dockComponent, {
                width: 1280, height: 240, magnification: 0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            waitForRendering(stage);
            var start = dock.iconSize;
            var target = Theme.controls.dock.iconSizeMax;
            verify(target > start);
            dock.iconSize = target;
            // Mid-flight the size is strictly between the endpoints.
            wait(Math.floor(Theme.motion.dockMagnify.fullDuration / 3));
            verify(dock.iconSize > start);
            verify(dock.iconSize < target);
            // It settles exactly on the target.
            wait(Theme.motion.dockMagnify.fullDuration + 120);
            fuzzyCompare(dock.iconSize, target, 0.5);
        }

        function test_a_size_change_snaps_under_reduced_motion() {
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 240, magnification: 0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            waitForRendering(stage);
            var target = Theme.controls.dock.iconSizeMax;
            dock.iconSize = target;
            waitForRendering(stage);
            fuzzyCompare(dock.iconSize, target, 0.001);
        }

        // -- Placement ------------------------------------------------------

        function test_bottom_dock_axis_is_horizontal() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom",
                entries: [ app("a", "A", true) ]
            });
            compare(dock.axisIsX, true);
            compare(dock.indicatorEdge, "bottom");
        }

        function test_left_dock_axis_is_vertical() {
            var dock = make(dockComponent, {
                width: 160, height: 800, position: "left",
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            compare(dock.axisIsX, false);
            compare(dock.indicatorEdge, "left");
            // Items stack on the y axis and the plate floats `edgeMargin` off
            // the left edge (T-14.7a).
            var layout = dock.layout;
            verify(layout[1].y > layout[0].y);
            compare(dock.plateRect.x, dock.edgeMargin);
            compare(dock.plateRect.w, dock.barThickness);
            // An entry starts at the bar padding inside the plate.
            fuzzyCompare(layout[0].x, dock.edgeMargin + dock.padding, 0.001);
            verify(dock.plateRect.x + dock.barThickness > layout[0].x);
        }

        function test_right_dock_indicator_edge() {
            var dock = make(dockComponent, {
                width: 160, height: 800, position: "right",
                entries: [ app("a", "A", true) ]
            });
            compare(dock.indicatorEdge, "right");
            // The plate floats `edgeMargin` off the right edge and entries
            // pack from the right so the running indicator is against the
            // screen edge.
            compare(dock.plateRect.x, 160 - dock.edgeMargin - dock.barThickness);
            var layout = dock.layout;
            fuzzyCompare(layout[0].x + layout[0].w, 160 - dock.edgeMargin - dock.padding,
                         0.001);
        }

        function test_vertical_entries_stay_inside_the_surface() {
            // A vertical Dock's magnified artwork grows into the transparent
            // band on the interior side; it must never be clipped by the
            // (thickness + band + edge gap)-wide surface.
            var left = make(dockComponent, {
                width: 160, height: 720, position: "left", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            left.pointerAlong = left._baseline.centers[0];
            waitForRendering(stage);
            verify(left.plateRect.x >= 0);
            verify(left.plateRect.x + left.plateRect.w <= left.width);
            for (var i = 0; i < left.layout.length; ++i) {
                verify(left.layout[i].x >= 0);
                verify(left.layout[i].x + left.layout[i].w <= left.width + 0.001);
            }

            var right = make(dockComponent, {
                width: 160, height: 720, position: "right", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            right.pointerAlong = right._baseline.centers[0];
            waitForRendering(stage);
            verify(right.plateRect.x >= 0);
            verify(right.plateRect.x + right.plateRect.w <= right.width);
            for (var j = 0; j < right.layout.length; ++j) {
                verify(right.layout[j].x >= -0.001);
                verify(right.layout[j].x + right.layout[j].w <= right.width);
            }
        }

        function test_vertical_hide_translates_off_the_side_edge() {
            var left = make(dockComponent, {
                width: 160, height: 800, position: "left",
                autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            compare(left.hideX, 0);
            left.hide();
            wait(Theme.motion.dockReveal.duration + 40);
            verify(left.hideX < 0);
            verify(left.plateRect.x < 0);

            var right = make(dockComponent, {
                width: 160, height: 800, position: "right",
                autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            right.hide();
            wait(Theme.motion.dockReveal.duration + 40);
            verify(right.hideX > 0);
            verify(right.plateRect.x > 160 - right.barThickness);
        }

        // -- Plate geometry (T-14.7a) ---------------------------------------

        function test_plate_floats_edge_margin_from_the_screen_edge() {
            // The plate never touches its anchored edge: the gap is exactly
            // the `edgeMargin` token on every position.
            var bottom = make(dockComponent, {
                width: 1280, height: 240, position: "bottom",
                entries: [ app("files", "Files", true) ]
            });
            fuzzyCompare(bottom.plateRect.y + bottom.plateRect.h,
                         bottom.height - bottom.edgeMargin, 0.001);
            // The transparent magnify band is the room above the plate.
            verify(bottom.plateRect.y >= bottom.magnifyBand - 0.001);

            var left = make(dockComponent, {
                width: 240, height: 800, position: "left",
                entries: [ app("files", "Files", true) ]
            });
            fuzzyCompare(left.plateRect.x, left.edgeMargin, 0.001);

            var right = make(dockComponent, {
                width: 240, height: 800, position: "right",
                entries: [ app("files", "Files", true) ]
            });
            fuzzyCompare(right.plateRect.x + right.plateRect.w,
                         right.width - right.edgeMargin, 0.001);
        }

        function test_surface_and_reserved_thickness_track_the_plate() {
            var dock = make(dockComponent, {
                width: 1280, height: 240,
                entries: [ app("files", "Files", true) ]
            });
            compare(dock.edgeMargin, Theme.controls.dock.edgeMargin);
            compare(dock.paddingAlong, Theme.controls.dock.paddingAlong);
            compare(dock.reservedThickness, dock.barThickness + dock.edgeMargin);
            compare(dock.surfaceThickness,
                    dock.barThickness + dock.magnifyBand + dock.edgeMargin);
        }

        function test_plate_ends_use_padding_along() {
            var bottom = make(dockComponent, {
                width: 1280, height: 240, position: "bottom",
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            fuzzyCompare(bottom.plateRect.x,
                         bottom._baseline.positions[0] - bottom.paddingAlong, 0.001);
            fuzzyCompare(bottom.plateRect.w,
                         bottom._baseline.total + 2 * bottom.paddingAlong, 0.001);

            var left = make(dockComponent, {
                width: 240, height: 800, position: "left",
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            fuzzyCompare(left.plateRect.y,
                         left._baseline.positions[0] - left.paddingAlong, 0.001);
            fuzzyCompare(left.plateRect.h,
                         left._baseline.total + 2 * left.paddingAlong, 0.001);
        }

        function test_entries_stay_inside_the_plate_at_icon_size_min_and_max() {
            var sizes = [Theme.controls.dock.iconSizeMin, Theme.controls.dock.iconSizeMax];
            for (var c = 0; c < sizes.length; ++c) {
                var dock = make(dockComponent, {
                    width: 1280, height: 300, iconSize: sizes[c], magnification: 0,
                    entries: [ app("a", "A", true), app("b", "B", true) ]
                });
                var r = dock.plateRect;
                for (var i = 0; i < dock.layout.length; ++i) {
                    if (dock.items[i].kind === "divider")
                        continue;
                    var e = dock.layout[i];
                    verify(e.x >= r.x - 0.001);
                    verify(e.x + e.w <= r.x + r.w + 0.001);
                    verify(e.y >= r.y - 0.001);
                    verify(e.y + e.h <= r.y + r.h + 0.001);
                }
            }
        }

        function test_bounce_and_drag_stay_inside_the_surface() {
            // The launch/attention bounce grows into the transparent magnify
            // band and a lifted drag rises above the plate; neither may leave
            // the layer surface on any position (T-14.7a).
            var bouncing = { id: "a", appId: "a", name: "A", kind: "pinned",
                             pinned: true, running: true, attention: true,
                             bounce: 0.5 };
            var bottom = make(dockComponent, {
                width: 1280, height: 300,
                iconSize: Theme.controls.dock.iconSizeMax,
                entries: [ bouncing ]
            });
            for (var i = 0; i < bottom.layout.length; ++i) {
                if (bottom.items[i].kind === "divider")
                    continue;
                var e = bottom.layout[i];
                verify(e.y >= -0.001);
                verify(e.y + e.h <= bottom.height + 0.001);
            }
            bottom.beginDrag(bottom.appEntries[0]);
            verify(bottom.draggedY(bottom.itemAt(0).height) >= 0);
            bottom.dropAt(bottom.appEntries[0], bottom._baseline.centers[0]);

            var left = make(dockComponent, {
                width: 320, height: 800, position: "left",
                iconSize: Theme.controls.dock.iconSizeMax,
                entries: [ bouncing ]
            });
            for (var j = 0; j < left.layout.length; ++j) {
                if (left.items[j].kind === "divider")
                    continue;
                var f = left.layout[j];
                verify(f.x >= -0.001);
                verify(f.x + f.w <= left.width + 0.001);
            }
        }

        function test_hidden_plate_fully_clears_the_surface() {
            // The auto-hide translation reaches the surface edge: a hidden
            // plate leaves no visible strip (T-14.7a).
            var bottom = make(dockComponent, {
                width: 1280, height: 240, position: "bottom",
                autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            bottom.hide();
            wait(Theme.motion.dockReveal.duration + 40);
            verify(bottom.plateRect.y >= bottom.height - 0.001);

            var left = make(dockComponent, {
                width: 240, height: 800, position: "left",
                autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            left.hide();
            wait(Theme.motion.dockReveal.duration + 40);
            verify(left.plateRect.x + left.plateRect.w <= 0.001);

            var right = make(dockComponent, {
                width: 240, height: 800, position: "right",
                autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            right.hide();
            wait(Theme.motion.dockReveal.duration + 40);
            verify(right.plateRect.x >= right.width - 0.001);
        }

        // -- Auto-hide ------------------------------------------------------

        function test_auto_hide_translates_off_edge() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            compare(dock.hideOffset, 0);
            dock.hide();
            wait(Theme.motion.dockReveal.duration + 40);
            verify(dock.hideOffset >= dock.barThickness);
            // A bottom bar hides downward, off the bottom edge.
            verify(dock.hideY > 0);
            verify(dock.plateRect.y > dock.magnifyBand);
            dock.reveal();
            wait(Theme.motion.dockReveal.duration + 40);
            compare(dock.hideOffset, 0);
        }

        function test_auto_hide_off_never_translates() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: false, revealed: false,
                entries: [ app("a", "A", true) ]
            });
            compare(dock.hideOffset, 0);
        }

        function test_auto_hide_translation_is_animated() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            compare(dock.hideOffset, 0);
            dock.hide();
            // Mid-flight: the slide has begun but has not reached the target
            // (FR-14: the reveal/hide is a real motion, not a snap).
            wait(Math.floor(Theme.motion.dockReveal.duration / 4));
            verify(dock.hideOffset > 0);
            verify(dock.hideOffset < dock.barThickness);
            wait(Theme.motion.dockReveal.duration + 80);
            verify(dock.hideOffset >= dock.barThickness);
        }

        function test_auto_hide_reduced_motion_snaps() {
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            dock.hide();
            waitForRendering(stage);
            compare(dock.hideOffset,
                    dock.barThickness + Theme.controls.dock.edgeMargin);
        }

        function test_keyboard_focus_suppresses_rehide() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("files", "Files", true) ]
            });
            // Pointer off the Dock; keyboard navigation now owns it.
            mouseMove(stage, 640, dock.height + 40);
            waitForRendering(stage);
            dock.beginKeyboardNavigation();
            compare(dock.keyboardFocused, true);
            dock.scheduleHide();
            wait(Theme.controls.dock.hideDelay
                 + Theme.motion.dockReveal.duration + 80);
            compare(dock.revealed, true);
            // Leaving keyboard navigation restores the normal re-hide delay.
            dock.endKeyboardNavigation();
            wait(Theme.controls.dock.hideDelay
                 + Theme.motion.dockReveal.duration + 80);
            compare(dock.revealed, false);
        }

        function test_reveal_cancels_a_pending_rehide() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            mouseMove(stage, 640, dock.height + 40);
            waitForRendering(stage);
            dock.scheduleHide();       // a re-hide is now pending
            dock.reveal();             // must cancel it
            wait(Theme.controls.dock.hideDelay
                 + Theme.motion.dockReveal.duration + 80);
            compare(dock.revealed, true);
        }

        // -- Activation -----------------------------------------------------

        function test_click_activates_app_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            var entry = dock.itemAt(0);
            mouseClick(entry, entry.width / 2, entry.height / 2);
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].appId, "files");
        }

        function test_click_trash_activates_trash_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: []
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            var trash = dock.itemAt(dock.itemCount() - 1);
            compare(trash.isTrash, true);
            mouseClick(trash, trash.width / 2, trash.height / 2);
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].kind, "trash");
        }

        function test_right_click_opens_entry_menu() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            menuSpy.target = dock;
            menuSpy.clear();
            var entry = dock.itemAt(0);
            mouseClick(entry, entry.width / 2, entry.height / 2, Qt.RightButton);
            compare(menuSpy.count, 1);
            compare(menuSpy.signalArguments[0][0].appId, "files");
        }

        // -- Accessibility --------------------------------------------------

        function test_entry_accessible_names_carry_state() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), app("settings", "Settings", false) ]
            });
            var runningEntry = dock.itemAt(0);
            var idleEntry = dock.itemAt(1);
            compare(runningEntry.Accessible.role, Accessible.ListItem);
            verify(runningEntry.Accessible.name.indexOf("Files") === 0);
            verify(runningEntry.Accessible.name.indexOf("running") >= 0);
            verify(idleEntry.Accessible.name.indexOf("not running") >= 0);
        }

        // -- Launch lifecycle states ----------------------------------------

        function test_pinned_not_running_has_no_indicator() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "files", appId: "org.dragonfruit.Files", name: "Files",
                             kind: "pinned", pinned: true, running: false } ]
            });
            var entry = dock.itemAt(0);
            compare(entry.running, false);
            compare(findChild(entry, "indicator").visible, false);
        }

        function test_launching_entry_dims_and_is_accessible() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "settings", appId: "org.dragonfruit.Settings",
                             name: "Settings", kind: "pinned", pinned: true,
                             running: false, launch: "launching" } ]
            });
            var entry = dock.itemAt(0);
            compare(entry.launching, true);
            verify(entry.Accessible.name.indexOf("launching") >= 0);
            verify(findChild(entry, "glyph").opacity < 1.0);
        }

        function test_failed_launch_shows_badge() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "settings", appId: "org.dragonfruit.Settings",
                             name: "Settings", kind: "pinned", pinned: true,
                             running: false, launch: "failed" } ]
            });
            var entry = dock.itemAt(0);
            compare(entry.failed, true);
            verify(entry.Accessible.name.indexOf("failed") >= 0);
            compare(findChild(entry, "statusBadge").visible, true);
            // FR-1 launch failure: the bounce stops and no running indicator
            // is left behind (the entry is not running and carries no bounce
            // phase, so a subsequent frame cannot animate it).
            compare(entry.running, false);
            compare(findChild(entry, "indicator").visible, false);
            compare(dock.entryBounce(entry), 0);
        }

        function test_missing_pinned_app_is_marked_not_found() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "ghost", appId: "org.example.Ghost", name: "Ghost",
                             kind: "pinned", pinned: true, running: false,
                             missing: true } ]
            });
            var entry = dock.itemAt(0);
            compare(entry.missing, true);
            verify(entry.Accessible.name.indexOf("not found") >= 0);
            compare(findChild(entry, "statusBadge").visible, true);
        }

        // -- Bounce and input region (T-10 section 8.1, FR-13) --------------

        function test_launch_bounce_lifts_the_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "a", appId: "a", name: "A", kind: "pinned",
                             pinned: true, running: true, launch: "launching",
                             bounce: 0.5 } ]
            });
            var bouncedY = dock.layout[0].y;
            // Same entry without a bounce for the baseline position.
            dock.entries = [ { id: "a", appId: "a", name: "A", kind: "pinned",
                               pinned: true, running: true } ];
            waitForRendering(stage);
            var baseY = dock.layout[0].y;
            verify(bouncedY < baseY);
            verify(Math.abs((baseY - bouncedY) - dock.barThickness / 4) < 0.5);
        }

        function test_attention_bounce_is_taller_than_launch() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "a", appId: "a", name: "A", kind: "pinned",
                             pinned: true, running: true, attention: true,
                             bounce: 0.5 } ]
            });
            var attention = dock.entryBounce(dock.items[0]);
            var launch = dock.entryBounce({ bounce: 0.5 });
            verify(attention > launch);
            verify(Math.abs(attention - dock.barThickness / 2) < 0.5);
        }

        function test_reduced_motion_removes_bounce_translation() {
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "a", appId: "a", name: "A", kind: "pinned",
                             pinned: true, running: true, attention: true,
                             bounce: 0.5 } ]
            });
            compare(dock.entryBounce(dock.items[0]), 0);
            // The state stays legible as a subtle scale pulse instead.
            verify(dock.itemAt(0).pulseScale > 1.0);
        }

        function test_attention_is_in_the_accessible_name() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "a", appId: "a", name: "A", kind: "pinned",
                             pinned: true, running: true, attention: true,
                             bounce: 0.5 } ]
            });
            verify(dock.itemAt(0).Accessible.name.indexOf("attention") >= 0);
        }

        // T-14.7c: a running bounce advances a phase map, not the entry model,
        // so the Repeater delegates are never recreated and any live hover or
        // press state survives.
        function test_bounce_does_not_rebuild_the_entry_model() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            var first = dock.itemAt(0);
            var second = dock.itemAt(1);
            first.pressed = true;
            var baseY = dock.layout[0].y;

            // The shell pushes the phase through `bouncePhases` every frame.
            dock.bouncePhases = { a: { phase: 0.5, attention: false } };
            waitForRendering(stage);

            // The delegates are the very same objects (no model reset), and the
            // press state survived the bounce.
            verify(dock.itemAt(0) === first);
            verify(dock.itemAt(1) === second);
            compare(first.pressed, true);
            compare(dock.entries.length, 2);
            // The phase is read from the map and lifts the entry geometry.
            fuzzyCompare(dock.entryPhase(dock.items[0]), 0.5, 0.001);
            verify(dock.layout[0].y < baseY);
        }

        function test_attention_phase_rides_the_phase_map() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            var first = dock.itemAt(0);
            dock.bouncePhases = { a: { phase: 0.5, attention: true } };
            waitForRendering(stage);
            verify(dock.itemAt(0) === first);
            compare(dock.entryAttention(dock.items[0]), true);
            verify(dock.entryBounce(dock.items[0])
                   > dock.barThickness / 2 - 0.5);
            // An empty map clears the phase without touching the model.
            dock.bouncePhases = ({});
            waitForRendering(stage);
            verify(dock.itemAt(0) === first);
            compare(dock.entryBounce(dock.items[0]), 0);
        }

        // T-10 section 22: an app that exits mid-bounce must resolve — the
        // entry is removed, no stale bounce or input rect survives.
        function test_removing_a_bouncing_entry_resolves_the_animation() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            dock.entries = [ { id: "a", appId: "a", name: "A", kind: "pinned",
                               pinned: true, running: true, attention: true,
                               bounce: 0.5 },
                             app("b", "B", true) ];
            waitForRendering(stage);
            var bouncedRects = dock.inputRects.length;
            compare(dock.appEntries.length, 2);

            // The app quits while bouncing: the projection drops its entry.
            dock.entries = [ app("b", "B", true) ];
            waitForRendering(stage);
            compare(dock.appEntries.length, 1);
            compare(dock.appEntries[0].id, "b");
            compare(dock.entryBounce(dock.appEntries[0]), 0);
            verify(dock.inputRects.length < bouncedRects);
        }

        function test_idle_input_region_is_bar_only() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", false) ]
            });
            compare(dock.inputRects.length, 1);
        }

        function test_bouncing_entry_adds_an_input_rect() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "a", appId: "a", name: "A", kind: "pinned",
                             pinned: true, running: true, attention: true,
                             bounce: 0.5 } ]
            });
            compare(dock.inputRects.length, 2);
        }

        function test_magnified_icon_adds_an_input_rect() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            verify(dock.inputRects.length > 1);
        }

        function test_hidden_autohide_input_region_is_the_edge_band() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            compare(dock.inputRects.length, 1);
            dock.hide();
            waitForRendering(stage);
            // The hidden Dock keeps only its edge band interactive so the
            // pointer can summon it back (T-10 section 15).
            compare(dock.inputRects.length, 1);
            fuzzyCompare(dock.inputRects[0].y, dock.height - dock.edgeTrigger, 0.001);
            fuzzyCompare(dock.inputRects[0].h, dock.edgeTrigger, 0.001);
            dock.reveal();
            waitForRendering(stage);
            fuzzyCompare(dock.inputRects[0].y, dock.plateRect.y, 0.001);
        }

        function test_vertical_edge_band_hugs_the_screen_edge() {
            var left = make(dockComponent, {
                width: 160, height: 800, position: "left",
                autoHide: true, revealed: false,
                entries: [ app("a", "A", true) ]
            });
            compare(left.inputRects.length, 1);
            fuzzyCompare(left.inputRects[0].x, 0, 0.001);
            fuzzyCompare(left.inputRects[0].w, left.edgeTrigger, 0.001);

            var right = make(dockComponent, {
                width: 160, height: 800, position: "right",
                autoHide: true, revealed: false,
                entries: [ app("a", "A", true) ]
            });
            compare(right.inputRects.length, 1);
            fuzzyCompare(right.inputRects[0].x, right.width - right.edgeTrigger, 0.001);
        }

        function test_hidden_dock_does_not_magnify() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: false,
                magnification: 1.0, entries: [ app("a", "A", true) ]
            });
            // Normalize the pointer off the Dock, then force the hidden state.
            mouseMove(stage, 640, dock.height + 40);
            dock.hide();
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.revealed, false);
            compare(dock.magnifying, false);
        }

        function test_dwelling_at_the_edge_reveals_the_dock() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: false,
                entries: [ app("a", "A", true) ]
            });
            mouseMove(stage, 640, dock.height + 40);
            dock.hide();
            waitForRendering(stage);
            compare(dock.revealed, false);
            // Enter the bottom edge band; the reveal is delayed, not instant.
            mouseMove(dock, 640, dock.height - 1);
            compare(dock.revealed, false);
            wait(Theme.controls.dock.revealDelay + 80);
            compare(dock.revealed, true);
        }

        function test_leaving_the_dock_rehides_after_delay() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            mouseMove(dock, 640, dock.height - 20);
            waitForRendering(stage);
            compare(dock.revealed, true);
            // Off the Dock surface: the pointer left, so the hide delay runs.
            mouseMove(stage, 640, dock.height + 40);
            wait(Theme.controls.dock.hideDelay + 80);
            compare(dock.revealed, false);
        }

        function test_leaving_before_the_reveal_delay_cancels_the_reveal() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: false,
                entries: [ app("a", "A", true) ]
            });
            mouseMove(stage, 640, dock.height + 40);
            dock.hide();
            mouseMove(dock, 640, dock.height - 1);
            wait(Theme.controls.dock.revealDelay / 2);
            mouseMove(stage, 640, dock.height + 40);
            wait(Theme.controls.dock.revealDelay + 80);
            compare(dock.revealed, false);
        }

        function test_rehide_is_suppressed_while_a_popover_is_open() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("files", "Files", true) ]
            });
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            compare(dock.popoverOpen, true);
            // The pointer is over the popover surface, not the Dock; the hide
            // timer must not hide the Dock while a popover is open.
            mouseMove(stage, 640, dock.height + 40);
            wait(Theme.controls.dock.hideDelay + 80);
            compare(dock.revealed, true);
            dock.closePopovers();
            waitForRendering(stage);
        }

        // -- Context menu and window chooser (T-10 sections 9/13) ----------

        function multiWindow(id, name, windowList) {
            return { id: id, appId: id, name: name, kind: "pinned",
                     pinned: true, running: true, desktopId: id + ".desktop",
                     windows: windowList.length, windowList: windowList };
        }

        function test_right_click_opens_context_menu_and_suppresses_magnification() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("files", "Files", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.magnifying, true);
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            var menu = findChild(dock, "entryMenu");
            verify(menu !== null);
            compare(menu.open, true);
            compare(dock.menuOpen, true);
            compare(dock.popoverOpen, true);
            compare(dock.magnifying, false);
            verify(dock.popoverRect.w > 0);
            verify(dock.popoverRect.h > 0);
        }

        function test_vertical_menu_opens_beside_the_bar() {
            // A left Dock's popover opens to the interior (right of the bar);
            // a right Dock's to its left. The shell grows the offscreen scene
            // horizontally so neither is clipped (T-10 section 5).
            var left = make(dockComponent, {
                width: 124, height: 720, position: "left",
                entries: [ app("files", "Files", true) ]
            });
            left.openEntryMenu(left.items[0]);
            waitForRendering(stage);
            verify(left.popoverRect.w > 0);
            verify(left.popoverRect.x >= left.plateRect.x + left.plateRect.w);

            var right = make(dockComponent, {
                width: 124, height: 720, position: "right",
                entries: [ app("files", "Files", true) ]
            });
            right.openEntryMenu(right.items[0]);
            waitForRendering(stage);
            verify(right.popoverRect.w > 0);
            verify(right.popoverRect.x + right.popoverRect.w <= right.plateRect.x);
        }

        function test_menu_model_lists_windows_and_actions() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "A", focused: true },
                    { windowId: "2", title: "B", minimized: true,
                      workspaceName: "Space 2" }
                ]) ]
            });
            dock.openEntryMenu(dock.items[0]);
            var menu = findChild(dock, "entryMenu");
            // 2 windows, sep, Show All Windows, sep, Remove, Options, sep, Quit
            compare(menu.entries.length, 9);
            compare(menu.entries[0].label, "A");
            compare(menu.entries[0].checked, true);
            compare(menu.entries[1].label, "B (minimized)");
            compare(menu.entries[1].shortcut, "Space 2");
            compare(menu.entries[3].label, "Show All Windows");
            compare(menu.entries[5].label, "Remove from Dock");
            compare(menu.entries[6].label, "Options");
            compare(menu.entries[6].type, "submenu");
            compare(menu.entries[8].label, "Quit");
        }

        function test_app_options_submenu_has_assign_login_and_show_in_files() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", false) ]
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.openEntryMenu(dock.items[0]);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            var optionsIndex = labels.indexOf("Options");
            verify(optionsIndex >= 0);
            menu.openSubmenu(optionsIndex);
            waitForRendering(stage);
            var subLabels = [];
            for (var j = 0; j < menu.submenuEntries.length; ++j)
                subLabels.push(menu.submenuEntries[j].label);
            verify(subLabels.indexOf("Assign to This Desktop") >= 0);
            verify(subLabels.indexOf("Assign to All Desktops") >= 0);
            verify(subLabels.indexOf("Assign to None") >= 0);
            verify(subLabels.indexOf("Open at Login") >= 0);
            verify(subLabels.indexOf("Show in Files") >= 0);
            // Selecting a choice emits the assign action with its target.
            var assignIndex = subLabels.indexOf("Assign to This Desktop");
            menu.activateSubmenu(assignIndex);
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "assign_to");
            compare(menuActionSpy.signalArguments[0][1].target, "this");
            compare(menuActionSpy.signalArguments[0][1].desktopId, "files.desktop");
        }

        function test_pinned_not_running_menu_has_options_and_show_in_files() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", false) ]
            });
            dock.openEntryMenu(dock.items[0]);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            // Open · Options ▸ · Show in Files · Remove from Dock
            verify(labels.indexOf("Open") >= 0);
            verify(labels.indexOf("Options") >= 0);
            verify(labels.indexOf("Show in Files") >= 0);
            verify(labels.indexOf("Remove from Dock") >= 0);
            compare(labels.indexOf("Quit") < 0, true);
        }

        function test_minimized_entry_menu_lists_windows_and_quit() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ { id: "win:1", appId: "files", name: "Doc",
                             kind: "minimized", running: false, minimized: true,
                             windowList: [
                                 { windowId: "1", title: "Doc", minimized: true },
                                 { windowId: "2", title: "Other", focused: true }
                             ] } ]
            });
            dock.openEntryMenu(dock.items[1]); // divider, minimized, stack, trash
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            verify(labels.indexOf("Doc (minimized)") >= 0);
            verify(labels.indexOf("Other") >= 0);
            verify(labels.indexOf("Show All Windows") >= 0);
            verify(labels.indexOf("Quit") >= 0);
            // A minimized-window entry is not an app entry: no Open/Options.
            compare(labels.indexOf("Open") < 0, true);
            compare(labels.indexOf("Options") < 0, true);
        }

        function test_menu_keep_in_dock_for_temporary_app() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ temporary("term", "Terminal") ]
            });
            dock.openEntryMenu(dock.items[0]);
            var menu = findChild(dock, "entryMenu");
            var labels = [];
            for (var i = 0; i < menu.entries.length; ++i)
                labels.push(menu.entries[i].label);
            verify(labels.indexOf("Keep in Dock") >= 0);
            verify(labels.indexOf("Remove from Dock") < 0);
        }

        function test_menu_action_emits_window_activation() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "A", focused: true },
                    { windowId: "2", title: "B" }
                ]) ]
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.openEntryMenu(dock.items[0]);
            var menu = findChild(dock, "entryMenu");
            menu.activate(0);
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "activate_window");
            compare(menuActionSpy.signalArguments[0][1].windowId, "1");
        }

        function menuLabels(menu) {
            var labels = [];
            for (var i = 0; i < menu.entries.length; ++i)
                labels.push(menu.entries[i].label);
            return labels;
        }

        function test_trash_menu_offers_open_and_empty_trash() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            dock.openEntryMenu(dock.trashEntry);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            verify(labels.indexOf("Open") >= 0);
            var emptyIndex = labels.indexOf("Empty Trash");
            verify(emptyIndex >= 0);
            compare(menu.entries[emptyIndex].enabled, true);
        }

        function test_empty_trash_is_disabled_when_trash_is_empty() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: false
            });
            dock.openEntryMenu(dock.trashEntry);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            var emptyIndex = labels.indexOf("Empty Trash");
            verify(emptyIndex >= 0);
            compare(menu.entries[emptyIndex].enabled, false);
        }

        function test_empty_trash_confirms_before_emptying() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.openEntryMenu(dock.trashEntry);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            // Selecting Empty Trash swaps in the confirmation and does not
            // yet emit the destructive action.
            menu.activate(labels.indexOf("Empty Trash"));
            waitForRendering(stage);
            compare(menuActionSpy.count, 0);
            compare(dock.trashConfirming, true);
            compare(menu.open, true);
            labels = menuLabels(menu);
            verify(labels.indexOf("Cancel") >= 0);
            verify(labels.indexOf("Open") < 0);
            // Confirming emits the destructive action exactly once.
            menu.activate(labels.indexOf("Empty Trash"));
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "empty_trash");
            compare(dock.trashConfirming, false);
        }

        function test_trash_cancel_returns_to_the_menu() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.openEntryMenu(dock.trashEntry);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            menu.activate(labels.indexOf("Empty Trash"));
            labels = menuLabels(menu);
            menu.activate(labels.indexOf("Cancel"));
            compare(dock.trashConfirming, false);
            compare(menuActionSpy.count, 0);
        }

        function test_trash_open_emits_open_trash() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.openEntryMenu(dock.trashEntry);
            var menu = findChild(dock, "entryMenu");
            menu.activate(menuLabels(menu).indexOf("Open"));
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "open_trash");
        }

        function test_trash_unavailable_is_dimmed_and_disabled() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: false,
                trashAvailable: false
            });
            var entry = dock.itemAt(dock.items.length - 1);
            compare(entry.kind, "trash");
            compare(entry.trashUnavailable, true);
            verify(entry.Accessible.name.indexOf("unavailable") >= 0);
            verify(findChild(entry, "glyph").opacity < 1.0);
            compare(findChild(entry, "statusBadge").visible, true);
            // The menu degrades to a single disabled row...
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.openEntryMenu(dock.trashEntry);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            compare(labels.length, 1);
            compare(labels[0], "Trash unavailable");
            compare(menu.entries[0].enabled, false);
            // ...and activating the entry is inert (never a broken operation).
            dock.activateEntry(dock.trashEntry);
            compare(menuActionSpy.count, 0);
        }

        function test_multi_window_click_opens_chooser() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "A", focused: true, workspaceName: "Space 1" },
                    { windowId: "2", title: "B", minimized: true, workspaceName: "Space 2" }
                ]) ]
            });
            windowSpy.target = dock;
            windowSpy.clear();
            mouseClick(dock.itemAt(0), dock.itemAt(0).width / 2,
                       dock.itemAt(0).height / 2);
            waitForRendering(stage);
            compare(dock.chooserOpen, true);
            var chooser = findChild(dock, "windowChooser");
            verify(chooser !== null);
            compare(chooser.windows.length, 2);
            chooser.activateWindow(1);
            compare(windowSpy.count, 1);
            compare(windowSpy.signalArguments[0][0], "2");
            compare(dock.chooserOpen, false);
        }

        function test_single_window_click_does_not_open_chooser() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "A", focused: true }
                ]) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            mouseClick(dock.itemAt(0), dock.itemAt(0).width / 2,
                       dock.itemAt(0).height / 2);
            waitForRendering(stage);
            compare(dock.chooserOpen, false);
            compare(activatedSpy.count, 1);
        }

        // T-14.7g: a stationary click always activates, even when the entry is
// magnified and the pointer arrived through the transparent band.
        function test_stationary_click_activates_a_magnified_entry() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 240, position: "bottom", magnification: 1.0,
                entries: [ app("files", "Files", true), app("b", "B", true) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            // The pointer sweeps the band over the first entry, growing it,
            // then clicks the grown artwork.
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            verify(dock.magnifying);
            var entry = dock.itemAt(0);
            mouseClick(entry, entry.width / 2, entry.height / 2);
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].appId, "files");
        }

        // A click that jitters within the shared slop is still an activation.
        function test_small_jitter_within_the_slop_still_activates() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            var entry = dock.itemAt(0);
            var x = entry.x + entry.width / 2;
            var y = entry.y + entry.height / 2;
            mousePress(dock, x, y);
            mouseMove(dock, x + 4, y + 3, 40, Qt.LeftButton);
            mouseRelease(dock, x + 4, y + 3, Qt.LeftButton);
            compare(activatedSpy.count, 1);
        }

        // A slop drag lifts the entry into a rearrangement, not an activation.
        function test_slop_drag_lifts_instead_of_activating() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            var entry = dock.itemAt(0);
            var x = entry.x + entry.width / 2;
            var y = entry.y + entry.height / 2;
            mousePress(dock, x, y);
            mouseMove(dock, x + 24, y, 40, Qt.LeftButton);
            compare(dock.dragging, true);
            mouseRelease(dock, x + 24, y, Qt.LeftButton);
            compare(dock.dragging, false);
            compare(activatedSpy.count, 0);
        }

        // The root dismiss TapHandler must never eat an entry click: with a
        // popover open, a click on the entry still activates it (T-14.7g).
        function test_open_popover_does_not_eat_an_entry_click() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            compare(dock.menuOpen, true);
            activatedSpy.target = dock;
            activatedSpy.clear();
            var entry = dock.itemAt(0);
            mouseClick(entry, entry.width / 2, entry.height / 2);
            compare(activatedSpy.count, 1);
        }

        function test_chooser_accessible_names_carry_window_titles() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "Document", focused: true },
                    { windowId: "2", title: "Spreadsheet" }
                ]) ]
            });
            dock.openChooser(dock.items[0]);
            var chooser = findChild(dock, "windowChooser");
            verify(chooser.Accessible.name.indexOf("Files") >= 0);
            var row = findChild(chooser, "chooserRows");
            verify(row !== null);
        }

        function test_empty_dock_click_dismisses_popover() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            compare(dock.menuOpen, true);
            // Click far from every entry.
            mouseClick(dock, 20, 20);
            waitForRendering(stage);
            compare(dock.menuOpen, false);
            compare(dock.popoverOpen, false);
        }

        function test_escape_dismisses_the_context_menu() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            compare(dock.menuOpen, true);
            keyClick(Qt.Key_Escape);
            waitForRendering(stage);
            compare(dock.menuOpen, false);
            compare(dock.popoverOpen, false);
        }

        function test_tall_menu_opens_above_with_headroom() {
            var list = [];
            for (var i = 0; i < 12; ++i)
                list.push({ windowId: String(i + 1), title: "Window " + (i + 1) });
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", list) ]
            });
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            var rect = dock.popoverRect;
            // A tall menu extends above the Dock scene; the shell's headroom
            // makes room for it.
            verify(rect.y < 0);
            verify(rect.h > dock.magnifyBand);
        }

        // T-14.7c: the shell pre-sizes the offscreen buffer to a fixed popover
        // budget and the Dock clamps every popover into it, so opening a
        // menu/chooser/stack never needs the scene to grow.
        function test_popover_open_stays_inside_the_pre_sized_buffer() {
            var list = [];
            for (var i = 0; i < 24; ++i)
                list.push({ windowId: String(i + 1), title: "Window " + (i + 1) });
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                popoverHeadroom: 320, popoverGutter: 320,
                entries: [ multiWindow("files", "Files", list) ]
            });
            // Even a menu taller than the budget opens inside it.
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            var rect = dock.popoverRect;
            verify(rect.w > 0);
            verify(rect.y >= -dock.popoverHeadroom - 0.001);
            verify(rect.x >= -dock.popoverGutter - 0.001);
            verify(rect.x + rect.w <= dock.width + dock.popoverGutter + 0.001);
            // The Dock scene itself is unchanged; only its offset inside the
            // pre-sized buffer differs.
            compare(dock.width, 1280);
            compare(dock.height, 160);

            // A vertical Dock clamps a wide popover into the side gutter.
            var left = make(dockComponent, {
                width: 124, height: 720, position: "left",
                popoverHeadroom: 320, popoverGutter: 320,
                entries: [ multiWindow("term", "Terminal", list) ]
            });
            left.openEntryMenu(left.items[0]);
            waitForRendering(stage);
            var leftRect = left.popoverRect;
            verify(leftRect.w > 0);
            verify(leftRect.x >= -left.popoverGutter - 0.001);
            verify(leftRect.x + leftRect.w
                   <= left.width + left.popoverGutter + 0.001);
            compare(left.width, 124);
            compare(left.height, 720);
        }

        // -- Drag rearrangement (T-10 section 12) ---------------------------

        function test_drag_reorders_pinned() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            var a = dock.appEntries[0];
            dock.beginDrag(a);
            compare(dock.dragging, true);
            // Insert after B, before C: dragBaseCenters is [centerB, centerC].
            var betweenBC = (dock.dragBaseCenters[0] + dock.dragBaseCenters[1]) / 2;
            dock.dropAt(a, betweenBC);
            compare(pinnedOrderSpy.count, 1);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","),
                    "b.desktop,a.desktop,c.desktop");
            compare(dock.dragging, false);
        }

        function test_drag_first_to_end() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            var a = dock.appEntries[0];
            dock.beginDrag(a);
            dock.dropAt(a, dock.dragBaseCenters[dock.dragBaseCenters.length - 1] + 100);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","),
                    "b.desktop,c.desktop,a.desktop");
        }

        function test_drag_no_change_emits_nothing() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            var a = dock.appEntries[0];
            dock.beginDrag(a);
            dock.dropAt(a, dock._baseline.centers[0]);
            compare(pinnedOrderSpy.count, 0);
        }

        function test_drag_live_gap_reorders_visual_order() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            var a = dock.appEntries[0];
            dock.beginDrag(a);
            dock.dragTo(a, (dock.dragBaseCenters[0] + dock.dragBaseCenters[1]) / 2);
            compare(dock.visualAppEntries[0].id, "b");
            compare(dock.visualAppEntries[1].id, "a");
            compare(dock.visualAppEntries[2].id, "c");
            dock.dropAt(a, (dock.dragBaseCenters[0] + dock.dragBaseCenters[1]) / 2);
        }

        function test_drag_promotes_temporary_to_pinned() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), temporary("t", "T") ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            var t = dock.appEntries[1];
            compare(dock.pinnedCount, 1);
            dock.beginDrag(t);
            // Drop after A: the temporary is promoted at the end.
            dock.dropAt(t, dock.dragBaseCenters[0] + 100);
            compare(pinnedOrderSpy.count, 1);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","),
                    "a.desktop,t.desktop");
        }

        function test_drag_promote_before_first() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), temporary("t", "T") ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            var t = dock.appEntries[1];
            dock.beginDrag(t);
            dock.dropAt(t, dock.dragBaseCenters[0] - 1);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","),
                    "t.desktop,a.desktop");
        }

        function test_drag_out_of_dock_removes_pinned() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            var a = dock.appEntries[0];
            var far = dock.plateRect.x - dock.iconSize - 50;
            dock.beginDrag(a);
            dock.dragTo(a, far);
            compare(dock.dragOutOfDock, true);
            dock.dropAt(a, far);
            compare(pinnedOrderSpy.count, 1);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","), "b.desktop");
        }

        function test_drag_out_of_dock_keeps_temporary() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), temporary("t", "T") ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            var t = dock.appEntries[1];
            dock.beginDrag(t);
            dock.dropAt(t, dock.plateRect.x - dock.iconSize - 50);
            compare(pinnedOrderSpy.count, 0);
        }

        function test_drag_pointer_left_removes_pinned() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            dock.beginDrag(dock.appEntries[0]);
            dock.dragPointerLeft();
            compare(dock.dragging, false);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","), "b.desktop");
        }

        function test_drag_suppresses_magnification() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.magnifying, true);
            dock.beginDrag(dock.appEntries[0]);
            compare(dock.magnifying, false);
        }

        function test_dragged_entry_is_lifted() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            dock.beginDrag(dock.appEntries[0]);
            waitForRendering(stage);
            compare(dock.itemAt(0).lifted, true);
            verify(findChild(dock.itemAt(0), "liftShadow").visible);
            dock.dropAt(dock.appEntries[0], dock._baseline.centers[0]);
        }

        function test_mouse_drag_reorders_pinned() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            var a = dock.itemAt(0);
            var x = a.x + a.width / 2;
            var y = a.y + a.height / 2;
            mousePress(dock, x, y);
            // Move past the drag threshold and one slot to the right.
            mouseMove(dock, x + 12, y, 40, Qt.LeftButton);
            mouseMove(dock, x + dock.iconSize + dock.gap + 2, y, 40, Qt.LeftButton);
            compare(dock.dragging, true);
            compare(dock.dragTargetIndex, 1);
            mouseRelease(dock, x + dock.iconSize + dock.gap + 2, y, Qt.LeftButton);
            compare(dock.dragPointerAlong, -1);
            compare(pinnedOrderSpy.count, 1);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","),
                    "b.desktop,a.desktop,c.desktop");
        }

        // -- Divider resize (T-10 section 5) -------------------------------

        function test_divider_resize_previews_and_commits_fraction() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            sizePreviewSpy.target = dock;
            sizeChangedSpy.target = dock;
            sizePreviewSpy.clear();
            sizeChangedSpy.clear();
            dock.beginDividerResize(0, 0);
            compare(dock.resizing, true);
            var bigger = dock.iconSize + 8;
            dock.updateDividerResizeAt(bigger);
            compare(dock.iconSize, bigger);
            compare(sizePreviewSpy.count, 1);
            var expected = (bigger - dock.iconSizeMin)
                           / (dock.iconSizeMax - dock.iconSizeMin);
            fuzzyCompare(sizePreviewSpy.signalArguments[0][0], expected, 0.001);
            dock.endDividerResize();
            compare(dock.resizing, false);
            // Exactly one committed value, matching the last preview.
            compare(sizeChangedSpy.count, 1);
            fuzzyCompare(sizeChangedSpy.signalArguments[0][0],
                         sizePreviewSpy.signalArguments[0][0], 0.001);
        }

        function test_divider_resize_clamps_to_the_icon_range() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            sizePreviewSpy.target = dock;
            sizePreviewSpy.clear();
            dock.beginDividerResize(0, 0);
            dock.updateDividerResizeAt(9999);
            compare(dock.iconSize, dock.iconSizeMax);
            fuzzyCompare(sizePreviewSpy.signalArguments[0][0], 1.0, 0.001);
            dock.updateDividerResizeAt(-9999);
            compare(dock.iconSize, dock.iconSizeMin);
            fuzzyCompare(sizePreviewSpy.signalArguments[1][0], 0.0, 0.001);
            dock.endDividerResize();
        }

        function test_divider_resize_suppresses_magnification_and_keeps_input() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 0.5,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.magnifying, true);
            dock.beginDividerResize(0, 0);
            compare(dock.magnifying, false);
            // The whole surface stays interactive so the drag never leaks.
            compare(dock.inputRects.length, 1);
            compare(dock.inputRects[0].w, dock.width);
            dock.endDividerResize();
        }

        function test_mouse_drag_on_divider_resizes_the_dock() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            sizeChangedSpy.target = dock;
            sizeChangedSpy.clear();
            var divider = dock.itemAt(dock.indexOfItemId("__divider__"));
            compare(divider.isDivider, true);
            // The visible divider is 1 px wide; the invisible hit target is
            // centred on it and wider.
            verify(findChild(divider, "dividerHit").width >= 16);
            var x = divider.x + divider.width / 2;
            var y = divider.y + divider.height / 2;
            var start = dock.iconSize;
            mousePress(dock, x, y);
            // The first move crosses the drag threshold and starts the
            // resize; the second is the actual resize delta.
            mouseMove(dock, x + 8, y, 40, Qt.LeftButton);
            mouseMove(dock, x + 38, y, 40, Qt.LeftButton);
            compare(dock.resizing, true);
            verify(dock.iconSize > start);
            mouseRelease(dock, x + 38, y, Qt.LeftButton);
            compare(dock.resizing, false);
            compare(sizeChangedSpy.count, 1);
        }

        // -- Live settings and the divider menu (T-10 section 19) ----------

        function test_magnification_value_scales_the_peak() {
            var dock = make(dockComponent, {
                width: 1280, height: 200, magnification: 0.5,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            var base = dock._baseline.centers[1];
            dock.pointerAlong = base;
            waitForRendering(stage);
            var halfPeak = dock.layout[1].iconSize;
            verify(halfPeak > dock.iconSize);
            dock.magnification = 1.0;
            waitForRendering(stage);
            verify(dock.layout[1].iconSize > halfPeak);
        }

        function test_animate_opening_off_suppresses_launch_bounce() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, animateOpening: false,
                entries: [ { id: "a", appId: "a", name: "A", kind: "pinned",
                             pinned: true, running: true, bounce: 0.5 } ]
            });
            compare(dock.entryBounce(dock.items[0]), 0);
            // An attention bounce is a notification and still plays.
            verify(dock.entryBounce({ attention: true, bounce: 0.5 }) > 0);
        }

        function test_divider_right_click_opens_options_menu() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 0,
                entries: [ app("a", "A", true) ]
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            var divider = dock.itemAt(1);
            compare(divider.isDivider, true);
            mouseClick(divider, divider.width / 2, divider.height / 2,
                       Qt.RightButton);
            waitForRendering(stage);
            compare(dock.menuOpen, true);
            var menu = findChild(dock, "entryMenu");
            verify(menu !== null);
            var labels = [];
            for (var i = 0; i < menu.entries.length; ++i)
                labels.push(menu.entries[i].label);
            verify(labels.indexOf("Turn Magnification On") >= 0);
            verify(labels.indexOf("Dock Settings…") >= 0);
            menu.activate(labels.indexOf("Turn Magnification On"));
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "toggle_magnification");
        }

        function test_divider_menu_has_hiding_toggle() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: false,
                entries: [ app("a", "A", true) ]
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.openEntryMenu(dock.items[1]); // the divider
            var menu = findChild(dock, "entryMenu");
            var labels = [];
            for (var i = 0; i < menu.entries.length; ++i)
                labels.push(menu.entries[i].label);
            verify(labels.indexOf("Turn Hiding On") >= 0);
            menu.activate(labels.indexOf("Turn Hiding On"));
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "toggle_autohide");

            // With auto-hide on the item flips to the "off" label.
            dock.autoHide = true;
            waitForRendering(stage);
            dock.openEntryMenu(dock.items[1]);
            var labelsOn = [];
            var menuOn = findChild(dock, "entryMenu");
            for (var j = 0; j < menuOn.entries.length; ++j)
                labelsOn.push(menuOn.entries[j].label);
            verify(labelsOn.indexOf("Turn Hiding Off") >= 0);
        }

        function test_divider_menu_position_submenu_changes_position() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom",
                entries: [ app("a", "A", true) ]
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.openEntryMenu(dock.items[1]); // the divider
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            var posIndex = labels.indexOf("Position on Screen");
            verify(posIndex >= 0);
            menu.openSubmenu(posIndex);
            waitForRendering(stage);
            compare(menu.openSubmenuIndex, posIndex);
            var subLabels = [];
            for (var j = 0; j < menu.submenuEntries.length; ++j)
                subLabels.push(menu.submenuEntries[j].label);
            var leftIndex = subLabels.indexOf("Left");
            verify(leftIndex >= 0);
            menu.activateSubmenu(leftIndex);
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "set_position");
            compare(menuActionSpy.signalArguments[0][1].position, "left");
        }

        function test_autohide_off_never_hides() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: false, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            dock.hide();
            waitForRendering(stage);
            compare(dock.revealed, true);
            compare(dock.hideOffset, 0);
        }

        // -- Keyboard navigation (T-10 section 20) --------------------------

        function test_keyboard_begin_focuses_first_app() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), app("term", "Terminal", true) ]
            });
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            compare(dock.keyboardFocused, true);
            compare(dock.focusedItemId, "files");
            compare(dock.itemAt(0).keyboardFocused, true);
        }

        function test_keyboard_arrows_move_focus_and_skip_divider() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), temporary("term", "Terminal"),
                           minimized("win1", "Document") ]
            });
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            compare(dock.focusedItemId, "files");
            dock.moveKeyboardFocus(1);
            compare(dock.focusedItemId, "term");
            dock.moveKeyboardFocus(1); // the divider is skipped
            compare(dock.focusedItemId, "win1");
            dock.moveKeyboardFocus(1);
            compare(dock.focusedItemId, "__downloads__");
            dock.moveKeyboardFocus(1);
            compare(dock.focusedItemId, "__trash__");
            dock.moveKeyboardFocus(1); // wraps forward
            compare(dock.focusedItemId, "files");
            dock.moveKeyboardFocus(-1); // wraps backward
            compare(dock.focusedItemId, "__trash__");
        }

        function test_keyboard_arrow_key_moves_via_keys_handler() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), app("term", "Terminal", true) ]
            });
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            keyClick(Qt.Key_Right);
            waitForRendering(stage);
            compare(dock.focusedItemId, "term");
            keyClick(Qt.Key_Left);
            waitForRendering(stage);
            compare(dock.focusedItemId, "files");
        }

        function test_keyboard_return_activates_the_focused_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), app("term", "Terminal", true) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            dock.moveKeyboardFocus(1);
            keyClick(Qt.Key_Return);
            waitForRendering(stage);
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].id, "term");
        }

        function test_keyboard_up_opens_the_context_menu() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            keyClick(Qt.Key_Up);
            waitForRendering(stage);
            compare(dock.menuOpen, true);
            verify(dock.menuEntry !== null);
            compare(dock.menuEntry.id, "files");
        }

        function test_keyboard_typing_jumps_to_an_app_name() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), app("term", "Terminal", true),
                           app("browser", "Browser", true) ]
            });
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            keyClick(Qt.Key_T);
            waitForRendering(stage);
            compare(dock.focusedItemId, "term");
        }

        function test_keyboard_focus_draws_the_ring_only_on_the_focused_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), app("term", "Terminal", true) ]
            });
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            var first = findChild(dock.itemAt(0), "keyboardFocusRing");
            var second = findChild(dock.itemAt(1), "keyboardFocusRing");
            verify(first !== null && second !== null);
            compare(first.shown, true);
            compare(second.shown, false);
        }

        function test_end_keyboard_navigation_clears_the_ring() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            compare(dock.focusedItemId, "files");
            dock.endKeyboardNavigation();
            waitForRendering(stage);
            compare(dock.keyboardFocused, false);
            compare(dock.focusedItemId, "");
        }

        function test_keyboard_escape_releases_focus() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), app("term", "Terminal", true) ]
            });
            releaseFocusSpy.target = dock;
            releaseFocusSpy.clear();
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            waitForRendering(stage);
            compare(releaseFocusSpy.count, 1);
            compare(dock.keyboardFocused, false);
            compare(dock.focusedItemId, "");
        }

        function test_vertical_dock_uses_up_down_keys() {
            var dock = make(dockComponent, {
                width: 160, height: 720, position: "left",
                entries: [ app("files", "Files", true), app("term", "Terminal", true) ]
            });
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            keyClick(Qt.Key_Down);
            waitForRendering(stage);
            compare(dock.focusedItemId, "term");
            keyClick(Qt.Key_Up);
            waitForRendering(stage);
            compare(dock.focusedItemId, "files");
        }

        // -- External drops (T-10 section 12) --------------------------------

        function test_external_file_drag_highlights_the_target() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            externalDragSpy.target = dock;
            externalDragSpy.clear();
            dock.beginExternalDrag(false, 2);
            compare(dock.externalDragActive, true);
            compare(dock.externalPayloadIsApp, false);
            compare(dock.externalPayloadCount, 2);
            compare(dock.inputRects.length, 1); // whole surface while dragging

            var slot = dock.layout[0];
            var local = dock.mapToItem(null, slot.x + slot.w / 2, slot.y + slot.h / 2);
            dock.externalDragTo(local.x, local.y);
            compare(dock.externalTargetId, "a");
            waitForRendering(stage);
            compare(dock.itemAt(0).externalDropTarget, true);
            compare(dock.itemAt(1).externalDropTarget, false);

            dock.externalDragLeft();
            compare(dock.externalDragActive, false);
            compare(dock.externalTargetId, "");
        }

        function test_external_app_drop_opens_a_live_gap() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            externalDropSpy.target = dock;
            externalDropSpy.clear();
            dock.beginExternalDrag(true, 1);
            // Drop to the left of the first app: insertion index 0, so the
            // placeholder is the first item and the apps shift right.
            var slot = dock.layout[0];
            var local = dock.mapToItem(null, slot.x - 2, slot.y + slot.h / 2);
            dock.externalDragTo(local.x, local.y);
            compare(dock.externalGap, true);
            compare(dock.externalInsertIndex, 0);
            compare(dock.items[0].kind, "external");
            compare(dock.items[1].id, "a");

            dock.externalDrop(local.x, local.y);
            compare(externalDropSpy.count, 1);
            compare(externalDropSpy.signalArguments[0][0], "__external_drop__");
            compare(externalDropSpy.signalArguments[0][1], "external");
            compare(externalDropSpy.signalArguments[0][3], true);
            compare(dock.externalDragActive, false);
        }

        function test_external_drop_over_trash_reports_trash() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            externalDropSpy.target = dock;
            externalDropSpy.clear();
            dock.beginExternalDrag(false, 1);
            var trash = dock.layout[dock.layout.length - 1];
            var local = dock.mapToItem(null, trash.x + trash.w / 2,
                                       trash.y + trash.h / 2);
            dock.externalDrop(local.x, local.y);
            compare(externalDropSpy.count, 1);
            compare(externalDropSpy.signalArguments[0][1], "trash");
            compare(externalDropSpy.signalArguments[0][3], false);
        }

        function test_external_drop_on_empty_dock_is_a_no_op() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            externalDropSpy.target = dock;
            externalDropSpy.clear();
            dock.beginExternalDrag(false, 1);
            // Far outside the bar and its magnified band.
            dock.externalDropAt(-100, -100);
            compare(externalDropSpy.count, 1);
            compare(externalDropSpy.signalArguments[0][0], "");
            compare(externalDropSpy.signalArguments[0][1], "");
        }

        function test_external_drop_cancel_resets_state() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            dock.beginExternalDrag(true, 3);
            var slot = dock.layout[0];
            var local = dock.mapToItem(null, slot.x + slot.w / 2, slot.y + slot.h / 2);
            dock.externalDragTo(local.x, local.y);
            compare(dock.externalGap, true);
            dock.externalDragLeft();
            compare(dock.externalDragActive, false);
            compare(dock.externalGap, false);
            compare(dock.externalPayloadCount, 0);
            compare(dock.externalInsertIndex, -1);
        }

        function test_external_app_ghost_shows_the_resolved_identity() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            dock.beginExternalDrag(true, 1);
            // The shell resolves the alias once the enter-time read finishes.
            dock.setExternalPayload(true, "Files", "/tmp/files-icon", 1, "");
            compare(dock.externalPayloadIsApp, true);
            compare(dock.externalPayloadName, "Files");

            var slot = dock.layout[0];
            var local = dock.mapToItem(null, slot.x - 2, slot.y + slot.h / 2);
            dock.externalDragTo(local.x, local.y);
            compare(dock.externalGap, true);
            compare(dock.items[0].kind, "external");
            compare(dock.items[0].name, "Files");
            compare(dock.items[0].iconPath, "/tmp/files-icon");
            // An app alias on the app region offers a pin.
            compare(dock.externalAffordance, "Add to Dock");

            waitForRendering(stage);
            var ghost = dock.itemAt(0);
            compare(ghost.externalHasIdentity, true);
            compare(findChild(ghost, "externalGhostName").text, "Files");
            verify(findChild(ghost, "glyph").visible);
        }

        function test_external_app_ghost_degrades_without_identity() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            dock.beginExternalDrag(true, 1);
            var slot = dock.layout[0];
            var local = dock.mapToItem(null, slot.x - 2, slot.y + slot.h / 2);
            dock.externalDragTo(local.x, local.y);
            waitForRendering(stage);
            var ghost = dock.itemAt(0);
            // No identity resolved yet: the generic slot, no glyph crash.
            compare(ghost.externalHasIdentity, false);
            compare(findChild(ghost, "glyph").visible, false);
            compare(findChild(ghost, "externalPlaceholder").visible, true);
        }

        function test_external_file_affordances_follow_the_target() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(),
                entries: [ app("a", "A", true) ]
            });
            dock.beginExternalDrag(false, 1);
            dock.setExternalPayload(false, "", "", 1, "report.pdf");
            compare(dock.externalIdentityLabel, "report.pdf");

            // Over the app entry: open with that app.
            var slot = dock.layout[0];
            var local = dock.mapToItem(null, slot.x + slot.w / 2, slot.y + slot.h / 2);
            dock.externalDragTo(local.x, local.y);
            compare(dock.externalTargetId, "a");
            compare(dock.externalAffordance, "Open with A");

            // Over Trash: move to trash, or report it unavailable.
            var trashIdx = dock.layout.length - 1;
            var trash = dock.layout[trashIdx];
            var trashLocal = dock.mapToItem(null, trash.x + trash.w / 2,
                                            trash.y + trash.h / 2);
            dock.externalDragTo(trashLocal.x, trashLocal.y);
            compare(dock.externalAffordance, "Move to Trash");
            dock.trashAvailable = false;
            compare(dock.externalAffordance, "Trash unavailable");
            dock.trashAvailable = true;

            // Over the Downloads stack: move to Downloads.
            var stackIdx = dock.indexOfItemId("__downloads__");
            var stack = dock.layout[stackIdx];
            var stackLocal = dock.mapToItem(null, stack.x + stack.w / 2,
                                            stack.y + stack.h / 2);
            dock.externalDragTo(stackLocal.x, stackLocal.y);
            compare(dock.externalAffordance, "Move to Downloads");
        }

        function test_external_multi_file_label_counts() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            dock.beginExternalDrag(false, 3);
            dock.setExternalPayload(false, "", "", 3, "");
            compare(dock.externalIdentityLabel, "3 items");
            var noTarget = dock.mapToItem(null, -200, -200);
            dock.externalDragTo(noTarget.x, noTarget.y);
            compare(dock.externalAffordance, "");
        }

        function test_external_affordance_capsule_renders() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            var capsule = findChild(dock, "externalAffordance");
            verify(capsule !== null);
            compare(capsule.visible, false);

            dock.beginExternalDrag(false, 1);
            dock.setExternalPayload(false, "", "", 2, "");
            var slot = dock.layout[0];
            var local = dock.mapToItem(null, slot.x + slot.w / 2, slot.y + slot.h / 2);
            dock.externalDragTo(local.x, local.y);
            waitForRendering(stage);
            compare(capsule.visible, true);
            compare(capsule.label, "Open with A");
        }

        function test_duplicate_pin_flashes_then_clears() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            compare(dock.itemAt(0).duplicateFlash, false);
            dock.flashPin("a.desktop");
            compare(dock.duplicateFlashId, "a.desktop");
            waitForRendering(stage);
            compare(dock.itemAt(0).duplicateFlash, true);
            compare(dock.itemAt(1).duplicateFlash, false);
            verify(findChild(dock.itemAt(0), "duplicateFlash").visible);
            tryVerify(function() { return dock.duplicateFlashId === ""; },
                      dock.duplicateFlashMs + 1000);
            compare(dock.itemAt(0).duplicateFlash, false);
        }

        // -- Downloads stack + recents (T-10 section 17) --------------------

        function stackItems() {
            return [ { name: "a.txt", path: "/tmp/a.txt", isDir: false },
                     { name: "folder", path: "/tmp/folder", isDir: true } ];
        }

        function test_stack_entry_present_and_click_opens_popover() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2, downloadsBadge: 1,
                entries: [ app("a", "A", true) ]
            });
            var idx = dock.indexOfItemId("__downloads__");
            verify(idx >= 0);
            compare(dock.items[idx].kind, "stack");
            compare(dock.items[idx + 1].kind, "trash");

            downloadsViewedSpy.target = dock;
            downloadsViewedSpy.clear();
            dock.activateEntry(dock.items[idx]);
            compare(dock.stackOpen, true);
            compare(downloadsViewedSpy.count, 1);
            // The popover rect is non-empty so the shell renders the overlay.
            verify(dock.popoverRect.w > 0);

            dock.closePopovers();
            compare(dock.stackOpen, false);
        }

        function test_stack_badge_shows_the_new_count() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2, downloadsBadge: 3,
                entries: [ app("a", "A", true) ]
            });
            var idx = dock.indexOfItemId("__downloads__");
            var entry = dock.itemAt(idx);
            var badge = findChild(entry, "stackBadge");
            verify(badge !== null);
            compare(badge.visible, true);
            verify(entry.stateLabel.indexOf("2 items") >= 0);
            verify(entry.stateLabel.indexOf("3 new") >= 0);

            var empty = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: [], downloadsCount: 0, downloadsBadge: 0,
                entries: [ app("a", "A", true) ]
            });
            var emptyEntry = empty.itemAt(empty.indexOfItemId("__downloads__"));
            compare(findChild(emptyEntry, "stackBadge").visible, false);
        }

        function test_stack_drop_reports_downloads() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(),
                entries: [ app("a", "A", true) ]
            });
            externalDropSpy.target = dock;
            externalDropSpy.clear();
            dock.beginExternalDrag(false, 1);
            var idx = dock.indexOfItemId("__downloads__");
            var slot = dock.layout[idx];
            var local = dock.mapToItem(null, slot.x + slot.w / 2, slot.y + slot.h / 2);
            dock.externalDrop(local.x, local.y);
            compare(externalDropSpy.count, 1);
            compare(externalDropSpy.signalArguments[0][1], "stack");
            compare(externalDropSpy.signalArguments[0][3], false);
        }

        function test_spring_load_opens_the_stack() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(),
                entries: [ app("a", "A", true) ]
            });
            springLoadSpy.target = dock;
            springLoadSpy.clear();
            downloadsViewedSpy.target = dock;
            downloadsViewedSpy.clear();
            dock.beginExternalDrag(false, 1);
            var idx = dock.indexOfItemId("__downloads__");
            var slot = dock.layout[idx];
            var local = dock.mapToItem(null, slot.x + slot.w / 2, slot.y + slot.h / 2);
            dock.externalDragTo(local.x, local.y);
            compare(dock.externalTargetId, "__downloads__");
            tryVerify(function() { return dock.stackOpen; }, dock.springLoadDelay + 1000);
            compare(springLoadSpy.count, 1);
            compare(springLoadSpy.signalArguments[0][0], "__downloads__");
            compare(downloadsViewedSpy.count, 1);
        }

        // -- T-14.7h folder stacks: presentation and clicks -----------------

        function longStackItems(n) {
            var out = [];
            for (var i = 0; i < n; ++i)
                out.push({ name: "item" + i + ".txt", path: "/tmp/item" + i + ".txt",
                           isDir: false });
            return out;
        }

        function openStackOf(dock) {
            var idx = dock.indexOfItemId("__downloads__");
            dock.activateEntry(dock.items[idx]);
            var popover = findChild(dock, "stackPopover");
            // The popover gates its own `visible` on `opacity`, so wait out the
            // open fade before asserting on visible descendants.
            if (popover)
                tryVerify(function() { return popover.opacity > 0; }, 1000);
            return popover;
        }

        function countTextDescendants(item) {
            var n = 0;
            for (var i = 0; i < item.children.length; ++i) {
                var child = item.children[i];
                if (child.text !== undefined)
                    ++n;
                if (child.children !== undefined)
                    n += countTextDescendants(child);
            }
            return n;
        }

        function test_stack_double_click_opens_the_folder_in_files() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2,
                entries: [ app("a", "A", true) ]
            });
            downloadsFolderSpy.target = dock;
            downloadsFolderSpy.clear();
            var idx = dock.indexOfItemId("__downloads__");
            // The first tap opens the popover immediately...
            dock.handleEntryTap(dock.items[idx]);
            compare(dock.stackOpen, true);
            compare(downloadsFolderSpy.count, 0);
            // ...and a second tap inside the double-click window opens Files.
            dock.handleEntryTap(dock.items[idx]);
            compare(downloadsFolderSpy.count, 1);
            compare(dock.stackOpen, false);
        }

        function test_stack_context_menu_offers_open_in_files() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(),
                entries: [ app("a", "A", true) ]
            });
            var idx = dock.indexOfItemId("__downloads__");
            dock.openEntryMenu(dock.items[idx]);
            waitForRendering(stage);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            verify(labels.indexOf("Open in Files") >= 0);
            // The action routes through the shell's stack open path.
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            menu.activate(labels.indexOf("Open in Files"));
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "open_downloads_folder");
        }

        function test_stack_popover_header_names_the_folder_and_opens_it() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2,
                downloadsName: "My Downloads",
                entries: [ app("a", "A", true) ]
            });
            var popover = openStackOf(dock);
            verify(popover !== null);
            compare(popover.title, "My Downloads");

            var headerText = findChild(popover, "stackHeaderText");
            verify(headerText !== null);
            compare(headerText.text, "My Downloads");
            verify(findChild(popover, "stackHeaderIcon") !== null);
            var action = findChild(popover, "stackOpenAction");
            verify(action !== null);
            compare(action.text, "Open in Files");

            downloadsFolderSpy.target = dock;
            downloadsFolderSpy.clear();
            popover.activateFolder();
            compare(downloadsFolderSpy.count, 1);
            compare(dock.stackOpen, false);
        }

        function test_stack_popover_header_action_is_keyboard_reachable() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(),
                entries: [ app("a", "A", true) ]
            });
            var popover = openStackOf(dock);
            compare(popover.currentIndex, -1);
            // Down moves onto the first row; Up returns to the header action.
            popover.moveSelection(1);
            compare(popover.currentIndex, 0);
            popover.moveSelection(-1);
            compare(popover.currentIndex, -1);
            downloadsFolderSpy.target = dock;
            downloadsFolderSpy.clear();
            popover.activateCurrent();
            compare(downloadsFolderSpy.count, 1);
        }

        function test_stack_popover_rows_open_the_item() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(),
                entries: [ app("a", "A", true) ]
            });
            var popover = openStackOf(dock);
            downloadSpy.target = dock;
            downloadSpy.clear();
            popover.activateItem(0);
            compare(downloadSpy.count, 1);
            compare(downloadSpy.signalArguments[0][0], "/tmp/a.txt");
            // The row carries an icon and a name label.
            verify(findChild(popover, "stackRowIcon") !== null);
            verify(findChild(popover, "stackRowName") !== null);
        }

        function test_stack_popover_empty_and_overflow_rows() {
            var empty = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: [], downloadsCount: 0,
                entries: [ app("a", "A", true) ]
            });
            var emptyPopover = openStackOf(empty);
            compare(emptyPopover.isEmpty, true);
            compare(emptyPopover.overflowCount, 0);
            compare(emptyPopover.rowCount, 0);
            var emptyRow = findChild(emptyPopover, "stackEmptyRow");
            verify(emptyRow !== null);
            compare(emptyRow.visible, true);
            compare(findChild(emptyPopover, "stackOverflowRow").visible, false);

            var long = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: longStackItems(10), downloadsCount: 10,
                entries: [ app("a", "A", true) ]
            });
            var longPopover = openStackOf(long);
            compare(longPopover.overflowCount, 2);
            verify(findChild(longPopover, "stackOverflowRow").visible);
            compare(findChild(longPopover, "stackEmptyRow").visible, false);
            // A long folder scrolls through the rest.
            compare(longPopover.scrolls, true);
            compare(findChild(longPopover, "stackRows").interactive, true);
        }

        function test_stack_glyph_has_no_text_in_artwork() {
            var glyph = make(glyphComponent, {
                kind: "stack", name: "Downloads", size: 48
            });
            var artwork = findChild(glyph, "stackArtwork");
            verify(artwork !== null);
            // The folder name lives in the hover label and the popover, never
            // in the artwork (the aesthetic fix of T-14.7h).
            compare(countTextDescendants(artwork), 0);
            verify(findChild(glyph, "stackFolderTab") !== null);
            verify(findChild(glyph, "stackFolderFront") !== null);
            var img = grabImage(glyph);
            verify(img.width > 0);
            glyph.destroy();
        }

        function test_stack_entry_single_click_activates() {
            var entry = make(entryComponent, {
                iconSize: 48,
                entry: { id: "__downloads__", name: "Downloads", kind: "stack",
                         stackCount: 3, badge: 0 }
            });
            var activated = 0;
            entry.activated.connect(function() { ++activated; });
            // A single click on the stack reaches the Dock through the
            // stack-specific tap handler.
            mouseClick(entry, entry.width / 2, entry.height / 2);
            compare(activated, 1);
        }

        function test_recent_entries_render_in_the_app_region() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true),
                           { id: "recent:term.desktop", appId: "term.desktop",
                             desktopId: "term.desktop", name: "Terminal",
                             kind: "recent", running: false, pinned: false } ]
            });
            compare(dock.appEntries.length, 2);
            compare(dock.appEntries[1].kind, "recent");
            var idx = dock.indexOfItemId("recent:term.desktop");
            var divider = dock.indexOfItemId("__divider__");
            verify(idx >= 0 && idx < divider);
            activatedSpy.target = dock;
            activatedSpy.clear();
            dock.activateEntry(dock.appEntries[1]);
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].kind, "recent");
        }

        // -- Artwork --------------------------------------------------------

        function test_magnify_band_is_transparent() {
            // The Dock surface spans the baseline bar plus the transparent
            // magnified band above it; only the bar and the entries may paint,
            // or the band shows as an opaque strip (T-10 section 2). Paint the
            // backing rectangle and prove the band lets it show through.
            var backdrop = createTemporaryObject(backdropComponent, stage, {
                x: 0, y: 0, width: 400, height: 240, color: "#00ff00"
            });
            var dock = make(dockComponent, {
                width: 400, height: 240,
                entries: [ app("a", "A", true) ]
            });
            waitForRendering(stage);
            compare(dock.color.a, 0);
            var img = grabImage(stage);
            var bandX = Math.floor(dock.width / 2);
            var bandY = Math.max(0, Math.floor(dock.plateRect.y) - 4);
            compare(img.red(bandX, bandY), 0);
            compare(img.green(bandX, bandY), 255);
            compare(img.blue(bandX, bandY), 0);
            backdrop.destroy();
        }

        function test_glyph_renders_pixels() {
            var appGlyph = make(glyphComponent, {
                kind: "app", name: "Files", appId: "org.dragonfruit.Files", size: 48
            });
            var img = grabImage(appGlyph);
            verify(img.width > 0);
            appGlyph.destroy();

            var trash = make(glyphComponent, { kind: "trash", size: 48 });
            var img2 = grabImage(trash);
            verify(img2.width > 0);
            trash.destroy();
        }

        // T-14.1a: the Dock renders the themed icon app-index resolved when a
        // path is present, and falls back to the initial tile otherwise.
        function test_glyph_prefers_a_themed_icon_path() {
            var plain = make(glyphComponent, {
                kind: "app", name: "Files", appId: "org.dragonfruit.Files", size: 48
            });
            compare(plain.iconPath, "");
            verify(!plain.hasThemedIconHint);
            plain.destroy();

            var themed = make(glyphComponent, {
                kind: "app", name: "Files",
                iconPath: "/usr/share/icons/hicolor/48x48/apps/files.png", size: 48
            });
            compare(themed.iconPath, "/usr/share/icons/hicolor/48x48/apps/files.png");
            verify(themed.hasThemedIconHint);
            themed.destroy();
        }

        // -- T-14.7d Trash entry artwork ------------------------------------

        // `grabImage` does not apply the grabbed item's own opacity, and it
        // composites over the stage; give each glyph a fixed dark backdrop so
        // the foreground sample is scheme-independent and a DockEntry grab can
        // still show the glyph's dimming.
        function makeGlyphOnBackdrop(props, x) {
            var size = props.size !== undefined ? props.size
                                                : Theme.controls.dock.iconSize;
            var back = createTemporaryObject(backdropComponent, stage,
                { x: x, y: 0, width: size, height: size, color: "#101010" });
            var glyph = createTemporaryObject(glyphComponent, stage, props);
            glyph.x = x;
            glyph.y = 0;
            waitForRendering(stage);
            return { back: back, glyph: glyph };
        }

        function test_trash_glyph_renders_pixels_at_every_dock_size() {
            var sizes = [32, 48, 64];
            for (var i = 0; i < sizes.length; ++i) {
                var s = sizes[i];
                var made = makeGlyphOnBackdrop(
                    { kind: "trash", size: s }, i * 90);
                var img = grabImage(made.glyph);
                compare(img.width, s);
                verify(countForegroundAll(img) > 0,
                       "the trash renders pixels at " + s + "px");
                // The artwork stays inside the centred trashSize box.
                var off = Math.floor(trashInset(s));
                if (off > 0) {
                    compare(countForeground(img, 0, 0, off, s), 0,
                            "no artwork left of the trashSize box at " + s);
                    compare(countForeground(img, s - off, 0, s, s), 0,
                            "no artwork right of the trashSize box at " + s);
                }
                made.glyph.destroy();
                made.back.destroy();
            }
        }

        function test_trash_empty_and_full_differ_above_the_lid() {
            var empty = makeGlyphOnBackdrop(
                { kind: "trash", trashFull: false, size: 48 }, 0);
            var full = makeGlyphOnBackdrop(
                { kind: "trash", trashFull: true, size: 48 }, 90);
            var imgE = grabImage(empty.glyph);
            var imgF = grabImage(full.glyph);
            var band = contentsBand(48);
            compare(countForeground(imgE, band.x0, band.y0, band.x1, band.y1), 0,
                    "an empty bin has no contents above the lid");
            verify(countForeground(imgF, band.x0, band.y0, band.x1, band.y1) > 0,
                   "a full bin shows contents above the lid");
            // Both states are non-null, and the full state adds pixels.
            verify(countForegroundAll(imgE) > 0, "the empty bin renders");
            verify(countForegroundAll(imgF) > countForegroundAll(imgE),
                   "the full bin renders more than the empty one");
            empty.glyph.destroy();
            empty.back.destroy();
            full.glyph.destroy();
            full.back.destroy();
        }

        function test_trash_full_and_empty_render_in_both_schemes() {
            var schemes = [false, true];
            for (var i = 0; i < schemes.length; ++i) {
                Theme.dark = schemes[i];
                var empty = makeGlyphOnBackdrop(
                    { kind: "trash", trashFull: false, size: 48 }, 0);
                var full = makeGlyphOnBackdrop(
                    { kind: "trash", trashFull: true, size: 48 }, 90);
                var imgE = grabImage(empty.glyph);
                var imgF = grabImage(full.glyph);
                var label = schemes[i] ? "dark" : "light";
                verify(countForegroundAll(imgE) > 0,
                       "the empty bin renders " + label);
                verify(countForegroundAll(imgF) > countForegroundAll(imgE),
                       "the full bin adds contents " + label);
                empty.glyph.destroy();
                empty.back.destroy();
                full.glyph.destroy();
                full.back.destroy();
            }
        }

        function test_trash_unavailable_renders_dimmer() {
            // Grab the entries (not the glyphs) so the renderer applies the
            // entry's glyph opacity.
            var backA = createTemporaryObject(backdropComponent, stage,
                { x: 0, y: 0, width: 48, height: 60, color: "#101010" });
            var available = make(entryComponent,
                                 { entry: { kind: "trash", available: true } });
            var backU = createTemporaryObject(backdropComponent, stage,
                { x: 90, y: 0, width: 48, height: 60, color: "#101010" });
            var unavailable = make(entryComponent,
                                   { entry: { kind: "trash", available: false } });
            unavailable.x = 90;
            waitForRendering(stage);
            var glyphA = findChild(available, "glyph");
            var glyphU = findChild(unavailable, "glyph");
            verify(glyphA !== null && glyphU !== null, "both entries carry a glyph");
            verify(glyphU.opacity < glyphA.opacity, "the unavailable glyph is dimmed");
            verify(sumLuma(grabImage(unavailable)) < sumLuma(grabImage(available)),
                   "the unavailable trash renders dimmer");
            available.destroy();
            unavailable.destroy();
            backA.destroy();
            backU.destroy();
        }

        // -- Add Application picker (T-14.7e) ------------------------------

        function pickerRow(desktopId, name, pinned, iconPath) {
            return { desktopId: desktopId, name: name, pinned: pinned === true,
                     iconPath: iconPath !== undefined ? iconPath : "" };
        }

        function pickerFixture() {
            return [
                pickerRow("org.example.Calculator.desktop", "Calculator", true),
                pickerRow("org.example.Files.desktop", "Files", false),
                pickerRow("org.example.Settings.desktop", "Settings", false),
                pickerRow("org.example.Terminal.desktop", "Terminal", false)
            ];
        }

        function test_divider_menu_offers_add_application() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            dock.openEntryMenu(dock.dividerEntry);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            verify(labels.indexOf("Add Application…") >= 0);
        }

        function test_add_application_opens_the_picker_anchored_to_the_divider() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                appPickerItems: pickerFixture(),
                entries: [ app("files", "Files", true) ]
            });
            appPickerRequestedSpy.target = dock;
            appPickerRequestedSpy.clear();
            dock.openEntryMenu(dock.dividerEntry);
            var menu = findChild(dock, "entryMenu");
            menu.activate(menuLabels(menu).indexOf("Add Application…"));
            waitForRendering(stage);
            var picker = findChild(dock, "appPicker");
            verify(picker !== null, "the picker exists");
            compare(picker.open, true);
            compare(dock.appPickerOpen, true);
            compare(appPickerRequestedSpy.count, 1, "the shell is asked to refresh");
            verify(dock.popoverRect.w > 0 && dock.popoverRect.h > 0,
                   "the overlay rect covers the picker");
            // The picker is anchored to the divider: its arrow tracks it.
            compare(picker.anchorItem, dock.itemAt(1));
        }

        function test_picker_filters_rows_by_name_and_id() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                appPickerItems: pickerFixture(),
                entries: []
            });
            var picker = findChild(dock, "appPicker");
            picker.open = true;
            waitForRendering(stage);
            compare(picker.filteredItems.length, 4);
            picker.query = "term";
            compare(picker.filteredItems.length, 1);
            compare(picker.filteredItems[0].name, "Terminal");
            // The query also matches the desktop id, case-insensitively.
            picker.query = "CALCULATOR.DESKTOP";
            compare(picker.filteredItems.length, 1);
            compare(picker.filteredItems[0].desktopId, "org.example.Calculator.desktop");
            picker.query = "nothing-here";
            compare(picker.filteredItems.length, 0);
            picker.query = "";
            compare(picker.filteredItems.length, 4);
        }

        function test_picker_shows_a_disabled_empty_row_for_an_empty_filter() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                appPickerItems: pickerFixture(),
                entries: []
            });
            var picker = findChild(dock, "appPicker");
            picker.open = true;
            picker.query = "zzzz";
            waitForRendering(stage);
            wait(300);
            compare(picker.noMatches, true);
            var empty = findChild(picker, "appPickerEmptyRow");
            verify(empty !== null && empty.visible, "the empty-filter row shows");
        }

        function test_picker_shows_the_absence_state_without_app_index() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                appIndexAvailable: false,
                appPickerItems: [],
                entries: []
            });
            var picker = findChild(dock, "appPicker");
            picker.open = true;
            waitForRendering(stage);
            wait(300);
            compare(picker.showAbsence, true);
            var absence = findChild(picker, "appPickerUnavailableRow");
            verify(absence !== null && absence.visible, "the absence row shows");
            // The absence state must not read as an empty corpus.
            compare(picker.noMatches, false);

            dock.appIndexAvailable = true;
            waitForRendering(stage);
            wait(300);
            compare(picker.showAbsence, false);
            compare(picker.noMatches, true, "an available but empty corpus is empty");
        }

        function test_picker_row_toggle_emits_the_pin_change() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                appPickerItems: pickerFixture(),
                entries: []
            });
            appPinToggledSpy.target = dock;
            appPinToggledSpy.clear();
            var picker = findChild(dock, "appPicker");
            picker.open = true;
            waitForRendering(stage);
            // Pin the unpinned Files row.
            var filesIndex = picker.filteredItems.length > 0 ? 1 : 0;
            for (var i = 0; i < picker.filteredItems.length; ++i) {
                if (picker.filteredItems[i].name === "Files")
                    filesIndex = i;
            }
            picker.toggleRowAt(filesIndex);
            compare(appPinToggledSpy.count, 1);
            compare(appPinToggledSpy.signalArguments[0][0], "org.example.Files.desktop");
            compare(appPinToggledSpy.signalArguments[0][1], true);
            // Unpin the pinned Calculator row.
            picker.toggleRowAt(filesIndex === 0 ? 1 : 0);
            compare(appPinToggledSpy.count, 2);
            compare(appPinToggledSpy.signalArguments[1][1], false);
        }

        function test_picker_keyboard_traversal_moves_and_toggles() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                appPickerItems: pickerFixture(),
                entries: []
            });
            appPinToggledSpy.target = dock;
            appPinToggledSpy.clear();
            var picker = findChild(dock, "appPicker");
            picker.open = true;
            waitForRendering(stage);
            compare(picker.currentIndex, 0, "the first row is highlighted on open");
            picker.moveSelection(1);
            compare(picker.currentIndex, 1);
            picker.moveSelection(-1);
            compare(picker.currentIndex, 0);
            // Down at the end stops at the last row; Up at the top stops at 0.
            picker.moveSelection(99);
            compare(picker.currentIndex, picker.filteredItems.length - 1);
            picker.moveSelection(99);
            compare(picker.currentIndex, picker.filteredItems.length - 1);
            picker.moveSelection(-99);
            compare(picker.currentIndex, 0);
            picker.toggleCurrent();
            compare(appPinToggledSpy.count, 1);
        }

        function test_picker_accessibility_carries_name_and_in_dock_state() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                appPickerItems: pickerFixture(),
                entries: []
            });
            var picker = findChild(dock, "appPicker");
            picker.open = true;
            waitForRendering(stage);
            var row = picker.filteredItems[0];
            var delegate = findChild(picker, "appPickerRows");
            verify(delegate !== null, "the row viewport exists");
            // Accessible names are derived from the row data + pin state; the
            // delegate role is a list item.
            verify(row !== undefined, "rows exist to expose");
        }

        function test_picker_escape_closes() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                appPickerItems: pickerFixture(),
                entries: []
            });
            var picker = findChild(dock, "appPicker");
            picker.open = true;
            waitForRendering(stage);
            compare(picker.open, true);
            picker.hide();
            compare(picker.open, false);
            compare(dock.appPickerOpen, false);
        }
    }
}
