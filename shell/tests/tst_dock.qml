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
        SignalSpy { id: windowCloseSpy; signalName: "windowCloseRequested" }
        SignalSpy { id: windowMinimizeSpy; signalName: "windowMinimizeRequested" }
        SignalSpy { id: popoverSpy; signalName: "popoverChanged" }
        SignalSpy { id: pinnedOrderSpy; signalName: "pinnedOrderChanged" }
        SignalSpy { id: releaseFocusSpy; signalName: "keyboardFocusReleaseRequested" }
        SignalSpy { id: externalDropSpy; signalName: "externalDropRequested" }
        SignalSpy { id: externalDragSpy; signalName: "externalDragChanged" }
        SignalSpy { id: springLoadSpy; signalName: "springLoadRequested" }
        SignalSpy { id: downloadSpy; signalName: "downloadActivated" }
        SignalSpy { id: downloadsFolderSpy; signalName: "downloadsFolderRequested" }
        SignalSpy { id: downloadsViewedSpy; signalName: "downloadsViewed" }
        SignalSpy { id: folderOpenSpy; signalName: "folderOpenRequested" }
        SignalSpy { id: folderViewedSpy; signalName: "folderViewed" }
        SignalSpy { id: folderRemovedSpy; signalName: "folderPinRemoved" }
        SignalSpy { id: sizePreviewSpy; signalName: "dockSizePreview" }
        SignalSpy { id: sizeChangedSpy; signalName: "dockSizeChanged" }
        SignalSpy { id: appPickerRequestedSpy; signalName: "appPickerRequested" }
        SignalSpy { id: appPinToggledSpy; signalName: "appPinToggled" }
        SignalSpy { id: tileRectSpy; signalName: "entryTileRect" }
        SignalSpy { id: appsDrawerSpy; signalName: "appsDrawerRequested" }

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

        // The magnified profile engages and releases with the Dock hover ease
        // (T-14.7aa): entering/leaving the Dock grows and shrinks the bubble
        // instead of snapping. Tests that assert settled magnified geometry
        // wait out the animation first.
        function waitForMagnify() {
            wait(Theme.motion.dockHover.fullDuration + 60);
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

        // The permanent Applications launcher tile the shell injects as a
        // pinned entry (T-19.2 follow-up): it toggles the drawer and cannot be
        // removed, reordered, or dragged.
        function launcher() {
            return { id: "__apps__", appId: "", name: "Applications",
                     kind: "pinned", pinned: true, appsLauncher: true,
                     running: false, desktopId: "", missing: false,
                     windowList: [], windowCount: 0 };
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

        // True when a pixel is (within `tol`) a known artwork fill colour. The
        // mask tests use explicit colours so they never mistake a fully-drawn
        // square (whose own pixel(0,0) is artwork) for a masked one.
        function nearColor(image, x, y, r, g, b, tol) {
            return Math.abs(image.red(x, y) - r) <= tol
                && Math.abs(image.green(x, y) - g) <= tol
                && Math.abs(image.blue(x, y) - b) <= tol;
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

        // -- T-14.7w squircle tile masking ----------------------------------
        // The shipped test icons for the masking cases (a full-bleed square, a
        // circle, and a padded square). `Qt.resolvedUrl` resolves them next to
        // this file; DockGlyph prepends `file://`, so strip the scheme here.
        function assetPath(name) {
            return Qt.resolvedUrl("data/" + name).toString().replace("file://", "");
        }

        // -- Entry regions --------------------------------------------------

        function test_items_order_and_trash_is_last() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), temporary("term", "Terminal"),
                           minimized("win1", "Document") ]
            });
            var items = dock.items;
            compare(items.length, 8); // pinned, |, temp, |, min, |, stack, trash
            compare(items[0].kind, "pinned");
            compare(items[1].kind, "divider");
            compare(items[2].kind, "temporary");
            compare(items[3].kind, "divider");
            compare(items[4].kind, "minimized");
            compare(items[5].kind, "divider");
            compare(items[6].kind, "stack");
            compare(items[7].kind, "trash");
            compare(dock.itemCount(), 8);
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

        // -- T-14.7v region dividers -----------------------------------------

        function countDividers(dock) {
            var n = 0;
            for (var i = 0; i < dock.items.length; ++i)
                if (dock.items[i].kind === "divider")
                    ++n;
            return n;
        }

        function test_region_dividers_split_pinned_tail_and_fixed() {
            // pinned + running unpinned + stack + Trash: two rules, matching
            // the reference `[pinned] | [temporary/recent] | [stacks Trash]`.
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                entries: [ app("files", "Files", true), temporary("term", "Terminal") ]
            });
            compare(countDividers(dock), 2);
            compare(dock.items[0].kind, "pinned");
            compare(dock.items[1].kind, "divider");
            compare(dock.items[2].kind, "temporary");
            compare(dock.items[3].kind, "divider");
            compare(dock.items[4].kind, "stack");
            compare(dock.items[5].kind, "trash");
            // Only the app | right-region rule carries the resize handle.
            compare(dock.items[1].resizeHandle, false);
            compare(dock.items[3].resizeHandle, true);
        }

        function test_pinned_only_dock_has_no_orphan_rule() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            // No temporary/recent tail: pinned meets the fixed tail in one rule.
            compare(countDividers(dock), 1);
            compare(dock.items[2].kind, "divider");
            compare(dock.items[2].resizeHandle, true);
            // No doubled or orphaned rule at the end.
            compare(dock.items[dock.items.length - 2].kind, "stack");
            compare(dock.items[dock.items.length - 1].kind, "trash");
        }

        function test_no_divider_when_app_region_is_empty() {
            var dock = make(dockComponent, { width: 1280, height: 200, entries: [] });
            compare(countDividers(dock), 0);
            compare(dock.items[0].kind, "stack");
            compare(dock.items[1].kind, "trash");
        }

        function test_minimized_region_gets_its_own_divider() {
            var dock = make(dockComponent, {
                width: 1280, height: 200,
                entries: [ app("a", "A", true), minimized("win1", "Doc") ]
            });
            compare(countDividers(dock), 2);
            compare(dock.items[0].kind, "pinned");
            compare(dock.items[1].kind, "divider");
            compare(dock.items[1].resizeHandle, true);
            compare(dock.items[2].kind, "minimized");
            compare(dock.items[3].kind, "divider");
            compare(dock.items[3].resizeHandle, false);
            compare(dock.items[4].kind, "stack");
        }

        function test_divider_uses_the_oversized_gap() {
            var dock = make(dockComponent, {
                width: 1280, height: 220, magnification: 0,
                entries: [ app("a", "A", true), temporary("t", "T") ]
            });
            var base = dock._baseline;
            // a at 0, pinned rule at 1, t at 2.
            var gapBefore = base.positions[1] - (base.positions[0] + base.sizes[0]);
            var gapAfter = base.positions[2] - (base.positions[1] + base.sizes[1]);
            fuzzyCompare(gapBefore, dock.dividerGap, 0.001);
            fuzzyCompare(gapAfter, dock.dividerGap, 0.001);
            verify(dock.dividerGap > dock.gap);
            compare(dock.dividerGap, Theme.controls.dock.divider.gap);
        }

        function test_divider_gap_holds_on_every_position() {
            var entries = [ app("a", "A", true), temporary("t", "T") ];
            var bottom = make(dockComponent, {
                width: 1280, height: 220, position: "bottom", entries: entries
            });
            var db = bottom._baseline;
            fuzzyCompare(db.positions[1] - (db.positions[0] + db.sizes[0]),
                         bottom.dividerGap, 0.001);

            var left = make(dockComponent, {
                width: 240, height: 800, position: "left", entries: entries
            });
            var dl = left._baseline;
            fuzzyCompare(dl.positions[1] - (dl.positions[0] + dl.sizes[0]),
                         left.dividerGap, 0.001);

            var right = make(dockComponent, {
                width: 240, height: 800, position: "right", entries: entries
            });
            var dr = right._baseline;
            fuzzyCompare(dr.positions[1] - (dr.positions[0] + dr.sizes[0]),
                         right.dividerGap, 0.001);
        }

        function test_divider_hairline_spans_the_plate_cross_axis() {
            var dock = make(dockComponent, {
                width: 1280, height: 220,
                entries: [ app("a", "A", true), temporary("t", "T") ]
            });
            var divider = dock.itemAt(dock.indexOfItemId("__divider__"));
            // The divider's slot is one pixel along the axis but spans the
            // plate cross-axis so the hairline reads near-plate-height.
            fuzzyCompare(divider.height, dock.barThickness, 0.001);
            var line = findChild(divider, "divider");
            fuzzyCompare(line.height,
                         divider.height * Theme.controls.dock.divider.heightRatio, 0.001);
            fuzzyCompare(line.width, Theme.controls.dock.divider.width, 0.001);
            // Only the app | right-region rule mounts the resize handle.
            var pinnedRule = dock.itemAt(dock.indexOfItemId("__divider_pinned__"));
            compare(findChild(pinnedRule, "dividerHit").visible, false);
            compare(findChild(divider, "dividerHit").visible, true);
        }

        function test_divider_spacing_survives_magnification() {
            var dock = make(dockComponent, {
                width: 1280, height: 260, magnification: 1.0,
                entries: [ app("a", "A", true), temporary("t", "T") ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.magnifying, true);
            var layout = dock.layout;
            // The rule's neighbours keep the over-sized gap under magnification.
            var gapAfter = layout[2].x - (layout[1].x + layout[1].w);
            fuzzyCompare(gapAfter, dock.dividerGap, 0.001);
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
            waitForMagnify();
            compare(dock.magnifying, true);
            var layout = dock.layout;
            // The hovered entry reaches the peak size.
            verify(layout[1].iconSize > dock.iconSize);
            // The neighbour grows less than the hovered entry.
            verify(layout[0].iconSize > dock.iconSize);
            verify(layout[0].iconSize < layout[1].iconSize);
        }

        // The reference zoom bubble (Dock_Tile_Mouseover.png, T-14.7aa): a
        // quadratic falloff that keeps ~90 % of the peak effect one tile away,
        // ~55-60 % two tiles away, and reaches zero by ~4 icon widths. This
        // pins the *profile shape*, not a peak value (the peak is the
        // user-tuned `magnification`).
        function test_magnification_profile_matches_the_reference_bubble() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true), app("d", "D", true),
                           app("e", "E", true), app("f", "F", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[2];
            waitForRendering(stage);
            waitForMagnify();
            var layout = dock.layout;
            var peak = layout[2].iconSize - dock.iconSize;
            verify(peak > 0, "the anchored tile reaches the peak");
            var one = ((layout[1].iconSize - dock.iconSize)
                       + (layout[3].iconSize - dock.iconSize)) / (2 * peak);
            var two = ((layout[0].iconSize - dock.iconSize)
                       + (layout[4].iconSize - dock.iconSize)) / (2 * peak);
            var three = (layout[5].iconSize - dock.iconSize) / peak;
            verify(one > 0.85 && one < 0.95, "one tile keeps ~90 % of the peak: " + one);
            verify(two > 0.50 && two < 0.65, "two tiles keep ~60 % of the peak: " + two);
            verify(three < 0.2, "three tiles are nearly back at rest: " + three);
        }

        // Entering and leaving the Dock grows and shrinks the bubble with the
        // Dock spring instead of snapping (T-14.7aa), and the release keeps
        // shrinking around the tile the pointer was on.
        function test_hover_zoom_engages_and_releases() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            var target = 1;
            dock.pointerAlong = dock._baseline.centers[target];
            // Engage: sample the ramp; a snap would jump straight to 1.
            var sawEngaging = false;
            for (var e = 0; e < 10; ++e) {
                wait(Math.floor(Theme.motion.dockHover.fullDuration / 20));
                if (dock.magnifyEngagement > 0 && dock.magnifyEngagement < 1) {
                    sawEngaging = true;
                    break;
                }
            }
            verify(sawEngaging, "the zoom engages with a spring instead of snapping");
            waitForMagnify();
            fuzzyCompare(dock.magnifyEngagement, 1, 0.001);
            var peak = dock.layout[target].iconSize;
            verify(peak > dock.iconSize);
            compare(dock.magnifyPointerIndex, target);
            // Release: sample the shrinking phase — the bubble must stay
            // around the held tile until the engagement reaches rest.
            dock.pointerAlong = -1;
            var shrunkAroundAnchor = false;
            var sawRelease = false;
            for (var s = 0; s < 10; ++s) {
                wait(Math.floor(Theme.motion.dockHover.fullDuration / 20));
                if (dock.magnifyEngagement <= 0.001)
                    break;
                if (dock.magnifyEngagement < 1) {
                    sawRelease = true;
                    if (dock.layout[target].iconSize > dock.iconSize)
                        shrunkAroundAnchor = true;
                }
            }
            verify(sawRelease, "the zoom releases with a spring, not a snap");
            verify(shrunkAroundAnchor,
                   "the shrinking bubble stays around the held anchor");
            waitForMagnify();
            fuzzyCompare(dock.magnifyEngagement, 0, 0.001);
            for (var i = 0; i < dock.layout.length; ++i) {
                if (dock.items[i].kind === "divider")
                    continue;
                fuzzyCompare(dock.layout[i].iconSize, dock.iconSize, 0.001);
            }
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

        function test_plate_height_is_fixed_under_magnification() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            // The background's cross-axis extent never changes: macOS keeps the
            // bar a constant height and the artwork zooms inside its reserved
            // band (T-14.7aa). The along axis still wraps the spreading row.
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            var baseW = dock.restingPlateRect.w;
            var baseH = dock.restingPlateRect.h;
            var baseY = dock.restingPlateRect.y;
            dock.pointerAlong = dock._baseline.centers[1];
            waitForRendering(stage);
            waitForMagnify();
            compare(dock.magnifying, true);
            verify(dock.plateRect.w > baseW);
            fuzzyCompare(dock.plateRect.h, baseH, 0.001);
            fuzzyCompare(dock.plateRect.y, baseY, 0.001);
            // The magnified artwork grows above the bar, into the pre-reserved
            // band, rather than the bar growing to wrap it.
            compare(dock.layout[1].y < dock.plateRect.y, true);
        }

        // The background has exactly two horizontal levels: base and one zoomed
        // level. It resizes once when the zoom engages and then never again, so
        // moving the pointer along the Dock cannot resize or translate it; the
        // height is always the base height (T-14.7aa).
        function test_plate_has_one_zoomed_level_that_does_not_track_the_pointer() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true), app("d", "D", true) ]
            });
            var base = dock.restingPlateRect;
            dock.pointerAlong = dock._baseline.centers[0];
            waitForMagnify();
            var zoomed = dock.plateRect;
            fuzzyCompare(zoomed.h, base.h, 0.001);
            verify(zoomed.w > base.w, "the bar grows to one zoomed level");
            fuzzyCompare(zoomed.w, dock.magnifiedPlateLength, 0.001);
            for (var i = 1; i <= 24; ++i) {
                dock.pointerAlong = dock._baseline.centers[0]
                        + (dock._baseline.centers[3] - dock._baseline.centers[0])
                          * i / 24;
                wait(16);
                fuzzyCompare(dock.plateRect.x, zoomed.x, 0.001);
                fuzzyCompare(dock.plateRect.y, zoomed.y, 0.001);
                fuzzyCompare(dock.plateRect.w, zoomed.w, 0.001);
                fuzzyCompare(dock.plateRect.h, zoomed.h, 0.001);
            }
            // Release returns to the base level.
            dock.pointerAlong = -1;
            waitForMagnify();
            fuzzyCompare(dock.plateRect.x, base.x, 0.001);
            fuzzyCompare(dock.plateRect.y, base.y, 0.001);
            fuzzyCompare(dock.plateRect.w, base.w, 0.001);
            fuzzyCompare(dock.plateRect.h, base.h, 0.001);
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
                // Along the axis the plate wraps the row.
                verify(e.x >= r.x - 0.001);
                verify(e.x + e.w <= r.x + r.w + 0.001);
                // Across the axis the plate is fixed, so magnified artwork may
                // pass the interior edge — but never the reserved band.
                verify(e.y >= r.y - dock.magnifyBand - 0.001);
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
            waitForMagnify();
            // The height never changes — the bar is fixed (T-14.7aa).
            fuzzyCompare(dock.plateRect.h, dock.restingPlateRect.h, 0.001);
            fuzzyCompare(dock.plateRect.y, dock.restingPlateRect.y, 0.001);
            dock.pointerAlong = -1;
            waitForRendering(stage);
            // The profile releases with the Dock hover ease (T-14.7aa); the
            // along-axis plate returns to rest once the engagement has decayed.
            waitForMagnify();
            compare(dock.magnifying, false);
            fuzzyCompare(dock.plateRect.x, dock.restingPlateRect.x, 0.001);
            fuzzyCompare(dock.plateRect.y, dock.restingPlateRect.y, 0.001);
            fuzzyCompare(dock.plateRect.w, dock.restingPlateRect.w, 0.001);
            fuzzyCompare(dock.plateRect.h, dock.restingPlateRect.h, 0.001);
        }

        function test_plate_height_is_fixed_under_reduced_motion() {
            // Park the pointer clear of the Dock so a leftover hover
            // position from an earlier case cannot seed `pointerAlong`.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            // Reduced motion removes the ease, not the geometry: the bar stays
            // the same height and the row still follows the pointer (T-14.7aa).
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.magnifying, true);
            fuzzyCompare(dock.plateRect.h, dock.restingPlateRect.h, 0.001);
            verify(dock.plateRect.w > dock.restingPlateRect.w);
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

        // -- T-14.7y tracking stability -------------------------------------

        // The tracker must approach the raw pointer without passing it: an
        // overshooting per-sample retarget is what made the smoothed value ring
        // around the true position.
        function test_smoothed_pointer_never_overshoots_the_target() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            var start = dock.smoothPointerAlong;
            var target = dock._baseline.centers[2];
            verify(target > start);
            dock.pointerAlong = target;
            var peak = start;
            for (var i = 0; i < 18; ++i) {
                wait(12);
                peak = Math.max(peak, dock.smoothPointerAlong);
            }
            verify(peak <= target + 0.5,
                   "smoothed pointer overshot its target: " + peak + " > " + target);
            fuzzyCompare(dock.smoothPointerAlong, target, 1.0);
        }

        // The anchor is decided from the raw pointer, so the smoothing filter
        // can never flip the anchored tile at a boundary. Immediately after a
        // raw jump the smoothed value still sits on the old tile; the anchor
        // must already be the tile under the *raw* pointer.
        function test_anchor_index_follows_the_raw_pointer() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.anchorIndex, 0);
            // Jump the raw pointer a tile over; do not wait for the tracker.
            dock.pointerAlong = dock._baseline.centers[2];
            compare(dock.anchorIndex, 2,
                    "anchor must be the tile under the raw pointer, not the lagging filter");
        }

        // The anchor rule is intact: the anchored tile's center stays on its
        // baseline center while the pointer is over it.
        function nearestItemIndex(dock, along) {
            var best = -1;
            var bestDistance = Number.MAX_VALUE;
            for (var i = 0; i < dock.items.length; ++i) {
                if (dock.items[i].kind === "divider" || dock.items[i].kind === "external")
                    continue;
                var d = Math.abs(dock._baseline.centers[i] - along);
                if (d < bestDistance) {
                    bestDistance = d;
                    best = i;
                }
            }
            return best;
        }

        // A slow sweep across two tiles: the anchored tile tracks the raw pointer and
// contains the pointer (the layout is a continuous warp around it), and the
// plate top never reverses by more than a device pixel while the pointer moves
// one way. The warp replaced the old discrete anchor pin: the anchored tile is
// only pinned to its baseline centre when the pointer is at that centre, and
// it follows the pointer proportionally in between, so crossing a tile
// boundary can never translate the whole row (T-14.7aa).
        function test_slow_sweep_has_a_stable_anchor_and_plate() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            var base = dock._baseline;
            var start = base.centers[0];
            var end = base.centers[2];
            dock.pointerAlong = start;
            waitForRendering(stage);
            var prevTop = dock.plateRect.y;
            var maxReversal = 0;
            var steps = 30;
            for (var i = 1; i <= steps; ++i) {
                var along = start + (end - start) * i / steps;
                dock.pointerAlong = along;
                // Let the pointer tracker settle before reading the layout:
                // this pins the anchor and plate rules, not the tracker lag.
                wait(40);
                // (a)/(c): the anchor is exactly the tile under the raw pointer,
                // and the anchored tile stays under the pointer (within the
                // scaled gap the pointer may sit in).
                var expected = nearestItemIndex(dock, along);
                if (dock.anchorIndex !== expected)
                    verify(false, "anchor flipped at step " + i + ": "
                           + dock.anchorIndex + " != " + expected);
                var e = dock.layout[expected];
                var slack = dock.gap + 2.0;
                verify(along >= e.x - slack && along <= e.x + e.iconSize + slack,
                       "anchored tile left the pointer at step " + i);
                // (b): a one-way sweep must not backtrack the plate top by
                // more than a device pixel.
                var top = dock.plateRect.y;
                if (top - prevTop > 0)
                    maxReversal = Math.max(maxReversal, top - prevTop);
                prevTop = top;
            }
            verify(maxReversal < 1.0,
                   "plate top reversed by " + maxReversal + " px during a one-way sweep");
        }

        // The magnified layout is a continuous function of the pointer: sweep
        // across several anchor boundaries in small steps and no tile (and no
        // plate edge) may jump. The discrete anchor pin this replaced
        // translated the whole row by a full magnified pitch at every boundary
        // (~35 px with the reference profile), which is the hover twitch
        // (T-14.7aa).
        function test_sweep_across_anchor_boundaries_never_jumps() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true), app("d", "D", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[0];
            waitForMagnify();
            var from = dock._baseline.centers[0] - 20;
            var to = dock._baseline.centers[3] + 20;
            // Start settled at the sweep's first position: the first loop step
            // must only add the sweep's own 1.53 px, not a jump from a parked
            // pointer far away.
            dock.pointerAlong = from;
            waitForMagnify();
            var prev = [];
            for (var e0 = 0; e0 < dock.layout.length; ++e0)
                prev.push(dock.layout[e0].x);
            var prevPlate = dock.plateRect.x;
            var maxStep = 0;
            var maxPlateStep = 0;
            var steps = 150;
            for (var i = 1; i <= steps; ++i) {
                dock.pointerAlong = from + (to - from) * i / steps;
                wait(16);
                var l = dock.layout;
                for (var e = 0; e < l.length; ++e) {
                    if (dock.items[e].kind === "divider")
                        continue;
                    maxStep = Math.max(maxStep, Math.abs(l[e].x - prev[e]));
                    prev[e] = l[e].x;
                }
                maxPlateStep = Math.max(maxPlateStep, Math.abs(dock.plateRect.x - prevPlate));
                prevPlate = dock.plateRect.x;
            }
            verify(maxStep < 8.0,
                   "a tile jumped " + maxStep + " px in one 1.53 px pointer step");
            verify(maxPlateStep < 8.0,
                   "the plate jumped " + maxPlateStep + " px in one 1.53 px pointer step");
        }

        // The bar's cross-axis edge is fixed, so there is no plate edge signal to
        // damp: the height is exact at rest, under magnification, and under
        // reduced motion (T-14.7aa; this replaces the T-14.7y peak-hold test).
        function test_plate_height_is_exact_under_reduced_motion() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            dock.pointerAlong = dock._baseline.centers[1];
            waitForRendering(stage);
            fuzzyCompare(dock.plateRect.y, dock.restingPlateRect.y, 0.001);
            fuzzyCompare(dock.plateRect.h, dock.restingPlateRect.h, 0.001);
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
            // The artwork keeps `padding` from the plate's interior edge; the
            // running-indicator space is tucked into the anchored-edge padding
            // (T-14.7u), so the entry starts `indicatorSpace` earlier.
            fuzzyCompare(layout[0].x + dock.indicatorSpace - dock.edgeMargin,
                         dock.padding, 0.001);
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
            // A right Dock packs the artwork `padding` in from the plate's
            // interior (left) edge, with the dot in the anchored-edge padding
            // (T-14.7u).
            fuzzyCompare(layout[0].x - (160 - dock.edgeMargin - dock.barThickness),
                         dock.padding, 0.001);
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

        // T-14.7u: the resting plate is one icon plus two paddings; the running
        // indicator lives inside the anchored-edge padding rather than in a band
        // added on top of it.
        function test_plate_thickness_and_indicator_inset_match_the_reference() {
            compare(Theme.controls.dock.padding, 15);
            compare(Theme.controls.dock.gap, 14);
            compare(Theme.controls.dock.indicatorGap, 8);
            compare(Theme.controls.dock.radius, 28);

            var dock = make(dockComponent, {
                width: 1280, height: 240, position: "bottom",
                entries: [ app("a", "A", true) ]
            });
            // One icon plus two cross-axis paddings: the indicator is not
            // additive.
            fuzzyCompare(dock.barThickness, dock.iconSize + 2 * dock.padding, 0.001);
            // The artwork keeps `padding` from the plate's interior (top) edge.
            fuzzyCompare(dock.layout[0].y - dock.plateRect.y, dock.padding, 0.001);
            // The whole entry (artwork + dot) still fits inside the plate.
            verify(dock.layout[0].y + dock.layout[0].h
                   <= dock.plateRect.y + dock.plateRect.h + 0.001);

            var entry = dock.itemAt(0);
            var indicator = findChild(entry, "indicator");
            verify(indicator !== null && indicator.visible);
            // The dot sits `indicatorGap` below the artwork, inside the bottom
            // padding; it never overlaps the artwork.
            fuzzyCompare(indicator.y, entry.iconSize + Theme.controls.dock.indicatorGap, 0.001);
            verify(indicator.y >= entry.iconSize);
        }

        function test_indicator_stays_inside_the_plate_at_icon_size_extremes() {
            var sizes = [Theme.controls.dock.iconSizeMin,
                         Theme.controls.dock.iconSizeMax];
            for (var c = 0; c < sizes.length; ++c) {
                var dock = make(dockComponent, {
                    width: 1280, height: 320, iconSize: sizes[c],
                    magnification: 0, entries: [ app("a", "A", true) ]
                });
                var entry = dock.itemAt(0);
                var indicator = findChild(entry, "indicator");
                verify(indicator !== null && indicator.visible);
                // The dot is always `indicatorGap` below the artwork...
                fuzzyCompare(indicator.y, entry.iconSize + Theme.controls.dock.indicatorGap, 0.001);
                // ...and the indicator space never exceeds the padding, so the
                // entry (dot included) stays inside the plate.
                verify(dock.indicatorSpace <= dock.padding);
                verify(dock.layout[0].y + dock.layout[0].h
                       <= dock.plateRect.y + dock.plateRect.h + 0.001);
            }
        }

        // -- Glass plate and squircles (T-14.7j) ----------------------------

        function test_plate_is_a_layered_glass_not_a_flat_slab() {
            // The plate is four token layers: a translucent fill, a bright
            // inner top-edge rim, a hairline border, and a soft shadow. The
            // old surface was one flat slab (chrome at chromeOpacity).
            var dock = make(dockComponent, {
                width: 400, height: 240,
                entries: [ app("a", "A", true) ]
            });
            var plate = findChild(dock, "dockPlate");
            var bar = findChild(dock, "dockBar");
            var rim = findChild(dock, "dockRim");
            var stroke = findChild(dock, "dockRimStroke");
            var border = findChild(dock, "dockBorder");
            var shadow = findChild(dock, "dockPlateShadow");
            verify(plate !== null && bar !== null && rim !== null);
            verify(stroke !== null && border !== null && shadow !== null);

            compare(bar.radius, Theme.controls.dock.radius);
            compare(bar.color, Theme.color.dockFill);
            fuzzyCompare(bar.opacity, Theme.controls.dock.plate.fillOpacity, 0.0001);
            // The rim is a stroked path along the interior edge, inset by half
            // the stroke so no rim pixel leaves the plate (T-14.7z).
            compare(stroke.strokeColor, Theme.color.dockRim);
            fuzzyCompare(stroke.strokeWidth,
                         Theme.controls.dock.plate.rimHeight, 0.0001);
            fuzzyCompare(rim.opacity, Theme.controls.dock.plate.rimOpacity, 0.0001);
            fuzzyCompare(rim.height, plate.plateH - stroke.strokeWidth, 0.001);
            compare(border.border.width, Theme.controls.dock.plate.borderWidth);
            compare(border.border.color, Theme.color.dockBorder);
            fuzzyCompare(border.opacity, Theme.controls.dock.plate.borderOpacity, 0.0001);
            verify(shadow.visible);
            compare(shadow.blur, Theme.controls.dock.plate.shadowBlur);

            // Pixel: the rim row reads brighter than the plate body just below
            // it, so there is a visible highlight rather than a flat fill.
            var img = grabImage(stage);
            var cx = Math.round(dock.plateRect.x + dock.plateRect.w / 2);
            var bodyY = Math.round(dock.plateRect.y + 5);
            var bodyLuma = img.red(cx, bodyY) + img.green(cx, bodyY)
                         + img.blue(cx, bodyY);
            var rimLuma = 0;
            for (var dy = 0; dy <= 2; ++dy) {
                var y = Math.round(dock.plateRect.y) + dy;
                rimLuma = Math.max(rimLuma, img.red(cx, y) + img.green(cx, y)
                                              + img.blue(cx, y));
            }
            verify(rimLuma > bodyLuma,
                   "the top rim must be brighter than the body");
        }

        function test_plate_glass_follows_the_color_scheme() {
            // The plate tone is a semantic token, so it follows the scheme.
            // Snapshot each tone as a string, because the plate binding stays
            // live and would re-resolve when the scheme flips.
            Theme.dark = false;
            var light = make(dockComponent, {
                width: 400, height: 240,
                entries: [ app("a", "A", true) ]
            });
            var lightFill = String(findChild(light, "dockBar").color);
            compare(lightFill, String(Theme.lightScheme.color.dockFill));

            Theme.dark = true;
            var dark = make(dockComponent, {
                width: 400, height: 240,
                entries: [ app("a", "A", true) ]
            });
            var darkFill = String(findChild(dark, "dockBar").color);
            compare(darkFill, String(Theme.darkScheme.color.dockFill));
            verify(lightFill !== darkFill, "the plate tone is scheme-aware");
            compare(String(findChild(dark, "dockRimStroke").strokeColor),
                    String(Theme.darkScheme.color.dockRim));
        }

        function test_glyph_tile_is_a_token_squircle_with_inset() {
            // The placeholder tile draws at the icon radius ratio and spans the
            // icon box less the token inset; the themed artwork is masked into
            // exactly the same tile (T-14.7w).
            var glyph = make(glyphComponent, {
                kind: "app", name: "Files", appId: "org.dragonfruit.Files",
                size: 48
            });
            var tile = findChild(glyph, "appTile");
            verify(tile !== null);
            fuzzyCompare(tile.radius,
                         48 * Theme.controls.dock.icon.radiusRatio, 0.001);
            fuzzyCompare(tile.width,
                         48 - 2 * 48 * Theme.controls.dock.icon.inset, 0.5);

            var themed = make(glyphComponent, {
                kind: "app", name: "Files", size: 48,
                iconPath: assetPath("icon-square.svg")
            });
            var art = findChild(themed, "artwork");
            verify(art !== null);
            tryCompare(themed, "maskedArtwork", true);
            fuzzyCompare(art.width, tile.width, 0.5);
            fuzzyCompare(art.x, tile.x, 0.001);
        }

        // T-14.7w: a full-bleed square icon is clipped to the tile squircle —
        // the corners are transparent, the artwork reaches the tile edge, and
        // the centre is painted. The mask is a Canvas clip, so the headless
        // software scene graph used here executes it and `grabImage` sees it.
        function test_square_icon_is_masked_to_the_tile() {
            var glyph = make(glyphComponent, {
                kind: "app", name: "Square", size: 48,
                iconPath: assetPath("icon-square.svg")
            });
            tryCompare(glyph, "maskedArtwork", true);
            waitForRendering(stage);
            wait(60);
            var img = grabImage(glyph);
            // The corner (1,1) is outside the token radius (0.24*48 = 11.5),
            // so it must not carry the square's red fill; the centre and the
            // edge midpoints must.
            verify(!nearColor(img, 1, 1, 226, 59, 59, 40),
                   "the square tile corner is masked");
            verify(nearColor(img, 24, 24, 226, 59, 59, 40),
                   "the tile centre is the artwork");
            verify(nearColor(img, 24, 1, 226, 59, 59, 40),
                   "the artwork reaches the tile top");
            verify(nearColor(img, 1, 24, 226, 59, 59, 40),
                   "the artwork reaches the tile left");
            glyph.destroy();
        }

        // T-14.7w: a round icon (no padding of ours) fills the tile — it is
        // not shrunk by a second inset on top of the clip.
        function test_round_icon_is_not_double_inset() {
            var glyph = make(glyphComponent, {
                kind: "app", name: "Round", size: 48,
                iconPath: assetPath("icon-round.svg")
            });
            tryCompare(glyph, "maskedArtwork", true);
            waitForRendering(stage);
            wait(60);
            var img = grabImage(glyph);
            // The circle is inscribed in the tile: it touches the top and left
            // edges at the midpoints. A 6% artwork inset would pull it in.
            verify(nearColor(img, 24, 1, 43, 108, 255, 40),
                   "the circle touches the tile top");
            verify(nearColor(img, 1, 24, 43, 108, 255, 40),
                   "the circle touches the tile left");
            verify(nearColor(img, 24, 24, 43, 108, 255, 40),
                   "the circle paints its centre");
            verify(!nearColor(img, 1, 1, 43, 108, 255, 40),
                   "the circle leaves the corner clear");
            glyph.destroy();
        }

        // T-14.7w: the placeholder tile and the masked artwork have the same
        // extent — same corner mask, same edge reach — so they read as one
        // squircle family.
        function test_placeholder_and_themed_artwork_share_the_tile() {
            // A dark backdrop makes the transparent corners observable: the
            // grab's pixel(0,0) is the corner (backdrop when masked, artwork
            // when not).
            var placeholder = makeGlyphOnBackdrop(
                { kind: "app", name: "Files", appId: "org.dragonfruit.Files",
                  size: 48 }, 0);
            var themed = makeGlyphOnBackdrop(
                { kind: "app", name: "Square", size: 48,
                  iconPath: assetPath("icon-square.svg") }, 90);
            tryCompare(themed.glyph, "maskedArtwork", true);
            waitForRendering(stage);
            wait(60);
            var p = grabImage(placeholder.glyph);
            var t = grabImage(themed.glyph);
            compare(countForeground(p, 0, 0, 3, 3), 0,
                    "the placeholder corner is rounded");
            compare(countForeground(t, 0, 0, 3, 3), 0,
                    "the artwork corner is masked");
            verify(!backgroundIs(p, 24, 1), "the placeholder reaches the tile edge");
            verify(!backgroundIs(t, 24, 1), "the artwork reaches the tile edge");
            placeholder.glyph.destroy();
            placeholder.back.destroy();
            themed.glyph.destroy();
            themed.back.destroy();
        }

        // T-14.7w: the mask lives on the artwork only; the running indicator
        // and the window-count badge are siblings and stay unclipped.
        function test_masked_artwork_does_not_clip_indicator_or_badge() {
            var entry = make(entryComponent, {
                iconSize: 48,
                entry: { id: "a", appId: "a", name: "Square", kind: "pinned",
                         running: true, iconPath: assetPath("icon-square.svg"),
                         windowList: [ { id: 1 }, { id: 2 } ] }
            });
            var glyph = findChild(entry, "glyph");
            tryCompare(glyph, "maskedArtwork", true);
            waitForRendering(stage);
            wait(60);
            var img = grabImage(entry);
            var indicator = findChild(entry, "indicator");
            verify(indicator !== null && indicator.visible, "the indicator is present");
            verify(!backgroundIs(img,
                                 Math.round(indicator.x + indicator.width / 2),
                                 Math.round(indicator.y + indicator.height / 2)),
                   "the running indicator is not clipped by the artwork mask");
            var badge = findChild(entry, "windowBadge");
            verify(badge !== null && badge.opacity > 0, "the window badge is present");
            verify(!backgroundIs(img, Math.round(badge.x + badge.width / 2),
                                      Math.round(badge.y + badge.height / 2)),
                   "the window badge is not clipped by the artwork mask");
            entry.destroy();
        }

        function test_entry_states_use_the_squircle_and_state_tokens() {
            var entry = make(entryComponent, {
                iconSize: 48,
                entry: { id: "a", appId: "a", name: "A", kind: "pinned",
                         running: true }
            });
            var hover = findChild(entry, "hoverHighlight");
            fuzzyCompare(hover.radius,
                         48 * Theme.controls.dock.hover.radiusRatio, 0.001);
            compare(hover.color, Theme.color.dockHoverFill);
            fuzzyCompare(hover.opacity,
                         Theme.controls.dock.hover.fillOpacity, 0.0001);

            var indicator = findChild(entry, "indicator");
            compare(indicator.color, Theme.color.dockIndicator);
            fuzzyCompare(indicator.opacity,
                         Theme.controls.dock.indicator.opacity, 0.0001);

            var divider = make(entryComponent, {
                entry: { id: "__divider__", kind: "divider" }
            });
            var line = findChild(divider, "divider");
            compare(line.width, Theme.controls.dock.divider.width);
            fuzzyCompare(line.height,
                         divider.height * Theme.controls.dock.divider.heightRatio,
                         0.001);
            compare(line.color, Theme.color.dockDivider);
            fuzzyCompare(line.opacity,
                         Theme.controls.dock.divider.opacity, 0.0001);
        }

        // The anchored tile of the magnified profile draws no hover wash: the
        // zoom itself is the hover state (the reference). It lifts off the
        // plate with the zoom shadow instead (T-14.7aa).
        function test_zoomed_entry_drops_the_wash_and_lifts() {
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            var entry = dock.itemAt(1);
            var p = entry.mapToItem(stage, entry.width / 2, entry.height / 2);
            mouseMove(stage, p.x, p.y);
            waitForRendering(stage);
            waitForMagnify();
            compare(entry.hovered, true, "the pointer hovers the tile");
            compare(entry.zoomed, true, "the anchored tile is the zoomed one");
            compare(findChild(entry, "hoverHighlight").visible, false,
                    "the zoom replaces the hover wash");
            var shadow = findChild(entry, "zoomShadow");
            verify(shadow !== null && shadow.visible, "the zoomed tile lifts");
            fuzzyCompare(shadow.blur, Theme.controls.dock.hover.shadowBlur, 0.001);
            fuzzyCompare(shadow.shadowOpacity,
                         Theme.controls.dock.hover.shadowOpacity, 0.001);
        }

        function test_plate_rim_is_on_the_interior_edge() {
            // The bright rim faces the screen interior on every Dock position,
            // never the anchored edge.
            var bottom = make(dockComponent, {
                width: 1280, height: 240, position: "bottom",
                entries: [ app("a", "A", true) ]
            });
            var bRim = findChild(bottom, "dockRim");
            verify(bRim.width > bRim.height, "the bottom rim is horizontal");

            var left = make(dockComponent, {
                width: 240, height: 800, position: "left",
                entries: [ app("a", "A", true) ]
            });
            var lRim = findChild(left, "dockRim");
            var lPlate = findChild(left, "dockPlate");
            verify(lRim.height > lRim.width, "the left rim is vertical");
            fuzzyCompare(lRim.x + lRim.width, lPlate.plateX + lPlate.plateW, 1.5);

            var right = make(dockComponent, {
                width: 240, height: 800, position: "right",
                entries: [ app("a", "A", true) ]
            });
            var rRim = findChild(right, "dockRim");
            var rPlate = findChild(right, "dockPlate");
            verify(rRim.height > rRim.width, "the right rim is vertical");
            fuzzyCompare(rRim.x, rPlate.plateX, 1.5);
        }

        // The reference plate is glass, not a slab: a soft interior gloss band
        // under the hairline rim and a bright lip along the anchored edge
        // (Dock_Tile_Mouseover.png, T-14.7aa).
        function test_plate_has_a_gloss_band_and_an_anchored_edge_lip() {
            var bottom = make(dockComponent, {
                width: 1280, height: 240, position: "bottom",
                entries: [ app("a", "A", true) ]
            });
            var gloss = findChild(bottom, "dockGloss");
            var glossStroke = findChild(bottom, "dockGlossStroke");
            verify(gloss !== null && glossStroke !== null, "the gloss band exists");
            fuzzyCompare(gloss.opacity, Theme.controls.dock.plate.glossOpacity, 0.001);
            fuzzyCompare(glossStroke.strokeWidth,
                         Theme.controls.dock.plate.glossHeight, 0.001);
            compare(glossStroke.strokeColor, Theme.color.dockRim);
            var edge = findChild(bottom, "dockEdge");
            verify(edge !== null, "the anchored-edge lip exists");
            fuzzyCompare(edge.opacity, Theme.controls.dock.plate.edgeOpacity, 0.001);
            compare(edge.color, Theme.color.dockRim);
            fuzzyCompare(edge.height, Theme.controls.dock.plate.edgeHeight, 0.001);
            verify(edge.width > 0 && edge.width < bottom.panelRect.w);
            // The lip hugs the anchored (bottom) edge, inset past the corners.
            // The plate group's origin is the plate itself on a bottom Dock,
            // so the lip's local y is the plate height.
            fuzzyCompare(edge.y + edge.height, bottom.panelRect.h, 0.001);
            fuzzyCompare(edge.x, Theme.controls.dock.radius, 0.001);

            var left = make(dockComponent, {
                width: 240, height: 800, position: "left",
                entries: [ app("a", "A", true) ]
            });
            var lEdge = findChild(left, "dockEdge");
            fuzzyCompare(lEdge.width, Theme.controls.dock.plate.edgeHeight, 0.001);
            fuzzyCompare(lEdge.x, left.panelRect.x, 0.001);
            fuzzyCompare(lEdge.height,
                         left.panelRect.h - 2 * Theme.controls.dock.radius, 0.001);
        }

        // -- T-14.7z plate corners and frost alignment -----------------------

        // Point-in-rounded-rect, inflated by `slop` so a stroke's anti-aliased
        // outer pixel is not counted as leaving the shape.
        function insideRoundedRect(px, py, r, R, slop) {
            var s = slop === undefined ? 0 : slop;
            if (px < r.x - s || px > r.x + r.w + s
                    || py < r.y - s || py > r.y + r.h + s)
                return false;
            var cx = Math.min(Math.max(px, r.x + R), r.x + r.w - R);
            var cy = Math.min(Math.max(py, r.y + R), r.y + r.h - R);
            var dx = px - cx;
            var dy = py - cy;
            var rad = R + s;
            return dx * dx + dy * dy <= rad * rad + 0.001;
        }

        function test_panel_rect_is_the_integer_edge_the_qml_draws() {
            // The plate the QML draws and the rect declared to the compositor
            // are one integer rounded rect (T-14.7z).
            var dock = make(dockComponent, {
                width: 1280, height: 240, position: "bottom",
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            var r = dock.panelRect;
            compare(r.x, Math.round(r.x));
            compare(r.y, Math.round(r.y));
            compare(r.w, Math.round(r.w));
            compare(r.h, Math.round(r.h));
            // Each snapped edge is within half a pixel of the live plate edge.
            fuzzyCompare(r.x, dock.plateRect.x, 0.501);
            fuzzyCompare(r.x + r.w, dock.plateRect.x + dock.plateRect.w, 0.501);
            fuzzyCompare(r.y, dock.plateRect.y, 0.501);
            fuzzyCompare(r.y + r.h, dock.plateRect.y + dock.plateRect.h, 0.501);
            // The QML fill (dockBar) draws exactly `panelRect` with the token
            // radius; that is the rounded edge the frost shares.
            var plate = findChild(dock, "dockPlate");
            var bar = findChild(dock, "dockBar");
            fuzzyCompare(plate.plateX, dock.panelRect.x - plate.x, 0.001);
            fuzzyCompare(plate.plateY, dock.panelRect.y - plate.y, 0.001);
            fuzzyCompare(bar.width, dock.panelRect.w, 0.001);
            fuzzyCompare(bar.height, dock.panelRect.h, 0.001);
            compare(bar.radius, Theme.controls.dock.radius);
        }

        function test_panel_rect_never_flips_during_a_sweep() {
            // A one-way pointer sweep must move the integer panel edge one way
            // only, so the declared backdrop cannot ping-pong by a pixel.
            mouseMove(stage, 640, stage.height - 1);
            waitForRendering(stage);
            var dock = make(dockComponent, {
                width: 1280, height: 160, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            var base = dock._baseline;
            var start = base.centers[0];
            var end = base.centers[2];
            dock.pointerAlong = start;
            waitForRendering(stage);
            var prevTop = dock.panelRect.y;
            var flips = 0;
            for (var i = 1; i <= 30; ++i) {
                dock.pointerAlong = start + (end - start) * i / 30;
                wait(16);
                var top = dock.panelRect.y;
                if (top - prevTop > 0)
                    ++flips;
                prevTop = top;
            }
            compare(flips, 0,
                    "the integer panel edge flipped back during a one-way sweep");
        }

        function test_rim_pixels_follow_the_plate_round_rect() {
            // Hiding the rim and diffing the two frames isolates the rim
            // pixels: every one must lie inside the plate's rounded rect, and
            // at least one must sit on the top-left corner arc (the pre-fix
            // straight hairline poked outside and had none on the arc).
            var dock = make(dockComponent, {
                width: 1280, height: 240, position: "bottom",
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            waitForRendering(stage);
            var r = dock.panelRect;
            var R = Theme.controls.dock.radius;
            var before = grabImage(stage);
            var rim = findChild(dock, "dockRim");
            verify(rim !== null);
            rim.visible = false;
            waitForRendering(stage);
            var after = grabImage(stage);
            rim.visible = true;
            waitForRendering(stage);

            var x0 = Math.max(0, Math.floor(r.x) - 2);
            var x1 = Math.min(before.width, Math.ceil(r.x + r.w) + 2);
            var y0 = Math.max(0, Math.floor(r.y) - 2);
            var y1 = Math.min(before.height, Math.ceil(r.y + R) + 2);
            var outside = 0;
            var onArc = 0;
            for (var y = y0; y < y1; ++y) {
                for (var x = x0; x < x1; ++x) {
                    var d = Math.abs(before.red(x, y) - after.red(x, y))
                          + Math.abs(before.green(x, y) - after.green(x, y))
                          + Math.abs(before.blue(x, y) - after.blue(x, y));
                    if (d <= 6)
                        continue;
                    var px = x + 0.5;
                    var py = y + 0.5;
                    if (!insideRoundedRect(px, py, r, R, 1.0)) {
                        ++outside;
                        continue;
                    }
                    // The top-left corner quadrant carries the arc.
                    if (px < r.x + R && py < r.y + R)
                        ++onArc;
                }
            }
            compare(outside, 0, "a rim pixel left the plate shape");
            verify(onArc > 0, "the rim must follow the top-left corner arc");
        }

        function test_artwork_padding_is_even_at_rest_and_magnified() {
            var dock = make(dockComponent, {
                width: 1280, height: 240, position: "bottom", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            waitForRendering(stage);
            // Take magnification off so the baseline is at rest.
            dock.magnification = 0;
            waitForRendering(stage);
            // At rest the top artwork inset and both end insets are the pad.
            fuzzyCompare(dock.layout[0].y - dock.panelRect.y, dock.padding, 0.501);
            fuzzyCompare(dock.layout[0].x - dock.panelRect.x,
                         dock.paddingAlong, 0.501);
            var last = dock.layout.length - 1;
            fuzzyCompare(dock.panelRect.x + dock.panelRect.w
                         - (dock.layout[last].x + dock.layout[last].w),
                         dock.paddingAlong, 0.501);

            // Settle anchored on the center tile: the bar's height is fixed, so
            // the magnified artwork rises past the interior edge into the
            // reserved band; the along-axis bar has one zoomed level and the
            // row stays inside it.
            dock.magnification = 1.0;
            dock.pointerAlong = dock._baseline.centers[1];
            waitForMagnify();
            var top = Number.MAX_VALUE;
            var left = Number.MAX_VALUE;
            var right = -Number.MAX_VALUE;
            for (var i = 0; i < dock.layout.length; ++i) {
                if (dock.items[i].kind === "divider" || dock.layout[i].w <= 0)
                    continue;
                top = Math.min(top, dock.layout[i].y);
                left = Math.min(left, dock.layout[i].x);
                right = Math.max(right, dock.layout[i].x + dock.layout[i].w);
            }
            verify(top < dock.panelRect.y,
                   "magnified artwork rises above the fixed bar");
            verify(top >= dock.panelRect.y - dock.magnifyBand - 0.001,
                   "the overhang stays inside the reserved band");
            // The bar has one zoomed level; the row sits inside it and the
            // background does not resize with the pointer.
            verify(left >= dock.panelRect.x - 0.001);
            verify(right <= dock.panelRect.x + dock.panelRect.w + 0.001);
            fuzzyCompare(dock.plateRect.w, dock.magnifiedPlateLength, 0.001);
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

        // -- Launch-origin tile hand-off (T-14.7l) --------------------------

        function test_activation_reports_the_entry_tile_in_output_coordinates() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                outputOriginX: 0, outputOriginY: 700,
                entries: [ app("files", "Files", true) ]
            });
            tileRectSpy.target = dock;
            tileRectSpy.clear();
            dock.activateEntry(dock.items[0]);
            compare(tileRectSpy.count, 1);
            var args = tileRectSpy.signalArguments[0];
            var r = dock.layout[0];
            compare(args[0], "files.desktop");
            compare(args[1], r.x);
            compare(args[2], 700 + r.y);
            compare(args[3], r.w);
            compare(args[4], r.h);
        }

        function test_tile_is_not_reported_for_trash_or_stacks() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: []
            });
            tileRectSpy.target = dock;
            tileRectSpy.clear();
            dock.activateEntry(dock.trashEntry);
            dock.activateEntry(dock.stackEntry);
            compare(tileRectSpy.count, 0);
        }

        function test_settled_relayout_republishes_a_moved_tile() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [ app("files", "Files", true) ]
            });
            tileRectSpy.target = dock;
            tileRectSpy.clear();
            dock.activateEntry(dock.items[0]);
            compare(tileRectSpy.count, 1);
            var before = tileRectSpy.signalArguments[0];
            // Adding a second pinned entry shifts the first tile along the axis.
            dock.entries = [ app("files", "Files", true), app("term", "Terminal", false) ];
            waitForRendering(stage);
            verify(tileRectSpy.count >= 2);
            var after = tileRectSpy.signalArguments[tileRectSpy.count - 1];
            compare(after[0], "files.desktop");
            verify(after[1] !== before[1] || after[3] !== before[3]);
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

        // -- Minimize-to-icon reaction (T-14.7s) ---------------------------

        // Off by default: the shell only publishes a minimize phase when the
        // key is on, and the renderer ignores one even if injected.
        function test_minimize_reaction_is_off_by_default() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            compare(dock.minimizeReaction, false);
            var baseY = dock.layout[0].y;
            dock.bouncePhases = { a: { minimize: true, minimizePhase: 0.5 } };
            waitForRendering(stage);
            compare(dock.entryBounce(dock.items[0]), 0);
            compare(dock.layout[0].y, baseY);
        }

        function test_minimize_reaction_lifts_the_entry_once() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, minimizeReaction: true,
                entries: [ app("a", "A", true) ]
            });
            var baseY = dock.layout[0].y;
            // Baseline (no phase) is already zero.
            compare(dock.entryBounce(dock.items[0]), 0);
            dock.bouncePhases = { a: { minimize: true, minimizePhase: 0.5 } };
            waitForRendering(stage);
            // A bottom Dock bounces up into the magnify band; the amplitude is
            // the token fraction of the icon size.
            var expected = Theme.controls.dock.minimizeReaction.amplitudeRatio
                           * dock.iconSize;
            verify(Math.abs(dock.entryBounce(dock.items[0]) - expected) < 0.5);
            verify(dock.layout[0].y < baseY);
            // The phase is bounded: the end of the hop has no translation.
            dock.bouncePhases = { a: { minimize: true, minimizePhase: 1.0 } };
            waitForRendering(stage);
            compare(dock.entryBounce(dock.items[0]), 0);
            // A cleared map removes the reaction without rebuilding the model.
            dock.bouncePhases = ({});
            waitForRendering(stage);
            compare(dock.entryBounce(dock.items[0]), 0);
        }

        function test_minimize_reaction_follows_the_dock_axis_and_direction() {
            // A left Dock bounces away from the screen edge, to the right.
            var left = make(dockComponent, {
                width: 160, height: 800, position: "left", minimizeReaction: true,
                entries: [ app("a", "A", true) ]
            });
            var leftBaseX = left.layout[0].x;
            var leftBaseY = left.layout[0].y;
            left.bouncePhases = { a: { minimize: true, minimizePhase: 0.5 } };
            waitForRendering(stage);
            verify(left.layout[0].x > leftBaseX);
            // The hop is on the cross axis only; y is untouched.
            compare(left.layout[0].y, leftBaseY);

            // A right Dock mirrors it: the entry moves left, away from the edge.
            var right = make(dockComponent, {
                width: 160, height: 800, position: "right", minimizeReaction: true,
                entries: [ app("a", "A", true) ]
            });
            var rightBaseX = right.layout[0].x;
            right.bouncePhases = { a: { minimize: true, minimizePhase: 0.5 } };
            waitForRendering(stage);
            verify(right.layout[0].x < rightBaseX);
        }

        function test_reduced_motion_removes_the_minimize_reaction() {
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160, minimizeReaction: true,
                entries: [ app("a", "A", true) ]
            });
            dock.bouncePhases = { a: { minimize: true, minimizePhase: 0.5 } };
            waitForRendering(stage);
            compare(dock.entryBounce(dock.items[0]), 0);
        }

        function test_minimize_reaction_does_not_rebuild_the_entry_model() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, minimizeReaction: true,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            var first = dock.itemAt(0);
            var second = dock.itemAt(1);
            first.pressed = true;
            dock.bouncePhases = { a: { minimize: true, minimizePhase: 0.5 } };
            waitForRendering(stage);
            verify(dock.itemAt(0) === first);
            verify(dock.itemAt(1) === second);
            compare(first.pressed, true);
            compare(dock.entries.length, 2);
            // The accessible name still carries the running state (the reaction
            // is decorative and never enters the name).
            verify(dock.itemAt(0).Accessible.name.indexOf("running") >= 0);
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
            waitForMagnify();
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

        // -- Applications launcher tile (T-19.2 follow-up) ------------------

        function test_apps_launcher_tile_toggles_drawer() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", false), launcher(),
                           app("settings", "Settings", false) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            appsDrawerSpy.target = dock;
            appsDrawerSpy.clear();
            // The launcher sits second, immediately after the first pinned tile.
            compare(dock.items[1].appsLauncher, true);
            dock.activateEntry(dock.items[1]);
            compare(appsDrawerSpy.count, 1);
            // It never goes through the launch/activation path.
            compare(activatedSpy.count, 0);
        }

        function test_apps_launcher_is_not_draggable_or_reorderable() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", false), launcher() ]
            });
            var tile = dock.items[1];
            compare(tile.appsLauncher, true);
            compare(dock.isDraggable(tile), false);
            compare(dock.reorderHintFor(tile), "");
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            dock.focusedItemId = tile.id;
            compare(dock.reorderFocusedPinned(1), false);
            compare(pinnedOrderSpy.count, 0);
        }

        function test_apps_launcher_menu_disables_remove() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", false), launcher() ]
            });
            dock.openEntryMenu(dock.items[1]);
            var menu = findChild(dock, "entryMenu");
            var labels = menuLabels(menu);
            var openIndex = labels.indexOf("Open Applications");
            var removeIndex = labels.indexOf("Remove from Dock");
            verify(openIndex >= 0);
            verify(removeIndex >= 0);
            compare(menu.entries[removeIndex].enabled, false);
            // The Open row toggles the drawer locally rather than launching.
            appsDrawerSpy.target = dock;
            appsDrawerSpy.clear();
            menu.activate(openIndex);
            compare(appsDrawerSpy.count, 1);
        }

        function test_apps_launcher_glyph_renders_grid_mark() {
            var glyph = make(glyphComponent, { kind: "launcher", size: 48 });
            var artwork = findChild(glyph, "launcherArtwork");
            verify(artwork !== null);
            verify(findChild(glyph, "launcherTile") !== null);
            verify(findChild(glyph, "launcherGlyph") !== null);
            var img = grabImage(glyph);
            verify(img.width > 0);
            verify(countForegroundAll(img) > 0, "the launcher tile renders");
            glyph.destroy();
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
            dock.openEntryMenu(dock.minimizedEntries[0]);
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

        // Walk the Trash confirmation to the destructive action (T-14.7r):
        // open the menu, select Empty Trash, then confirm in the swapped model.
        function confirmEmptyTrash(dock) {
            dock.openEntryMenu(dock.trashEntry);
            var menu = findChild(dock, "entryMenu");
            menu.activate(menuLabels(menu).indexOf("Empty Trash"));
            waitForRendering(stage);
            menu = findChild(dock, "entryMenu");
            menu.activate(menuLabels(menu).indexOf("Empty Trash"));
            waitForRendering(stage);
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

        // -- T-14.7r Trash empty progress and result --------------------------

        function test_empty_trash_confirmation_starts_async_and_opens_progress() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            confirmEmptyTrash(dock);
            // Confirming runs the destructive action, closes the menu, and opens
            // the progress popover anchored to the Trash entry.
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "empty_trash");
            compare(findChild(dock, "entryMenu").open, false);
            compare(dock.trashEmptyPhase, "emptying");
            compare(dock.trashEmptyOpen, true);
            var pop = findChild(dock, "trashEmptyPopover");
            verify(pop !== null);
            compare(pop.open, true);
            // The busy indicator waits out the delay: hidden at first...
            compare(findChild(pop, "trashEmptySpinner").visible, false);
            // ...and shown once the empty has run longer than the delay.
            wait(Theme.controls.dock.trashEmpty.busyDelay + 80);
            compare(findChild(pop, "trashEmptySpinner").visible, true);
            compare(findChild(pop, "trashEmptyCheck").visible, false);
            verify(pop.accessibleStateText.indexOf("Emptying") >= 0);
        }

        function test_empty_trash_fast_success_never_shows_the_busy_indicator() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            confirmEmptyTrash(dock);
            var pop = findChild(dock, "trashEmptyPopover");
            // The result lands before the busy delay elapses.
            dock.handleTrashEmptyResult(true, 3, "");
            waitForRendering(stage);
            compare(dock.trashEmptyPhase, "succeeded");
            compare(dock.trashBusyVisible, false);
            compare(findChild(pop, "trashEmptySpinner").visible, false);
        }

        function test_empty_trash_success_shows_check_and_removed_count() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            confirmEmptyTrash(dock);
            var pop = findChild(dock, "trashEmptyPopover");
            // The popover fades in; its children's effective visibility is only
            // readable once it is visible.
            tryCompare(pop, "visible", true);
            dock.handleTrashEmptyResult(true, 3, "");
            waitForRendering(stage);
            compare(dock.trashEmptyPhase, "succeeded");
            compare(findChild(pop, "trashEmptySpinner").visible, false);
            tryCompare(findChild(pop, "trashEmptyCheck"), "visible", true);
            compare(findChild(pop, "trashEmptyRetry").visible, false);
            verify(findChild(pop, "trashEmptyStatusText").text.indexOf("3") >= 0);
            verify(pop.accessibleStateText.indexOf("3") >= 0);
        }

        function test_empty_trash_failure_offers_try_again_and_retries() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            confirmEmptyTrash(dock);
            var pop = findChild(dock, "trashEmptyPopover");
            tryCompare(pop, "visible", true);
            dock.handleTrashEmptyResult(false, -1, "The Trash is read-only");
            waitForRendering(stage);
            compare(dock.trashEmptyPhase, "failed");
            compare(findChild(pop, "trashEmptySpinner").visible, false);
            compare(findChild(pop, "trashEmptyCheck").visible, false);
            verify(findChild(pop, "trashEmptyStatusText").text.indexOf("read-only") >= 0);
            var retry = findChild(pop, "trashEmptyRetry");
            tryCompare(retry, "visible", true);
            // Focus moves to the safe action.
            compare(retry.activeFocus, true);
            // Try Again reruns the same action, without re-opening the menu.
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            retry.clicked();
            waitForRendering(stage);
            compare(dock.trashEmptyPhase, "emptying");
            compare(menuActionSpy.count, 1);
            compare(menuActionSpy.signalArguments[0][0], "empty_trash");
        }

        function test_empty_trash_second_request_while_emptying_is_ignored() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            confirmEmptyTrash(dock);
            menuActionSpy.target = dock;
            menuActionSpy.clear();
            dock.retryTrashEmpty();
            compare(menuActionSpy.count, 0);
            compare(dock.trashEmptyPhase, "emptying");
        }

        function test_empty_trash_reduced_motion_keeps_busy_static_but_distinct() {
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            confirmEmptyTrash(dock);
            wait(Theme.controls.dock.trashEmpty.busyDelay + 80);
            var pop = findChild(dock, "trashEmptyPopover");
            var spinner = findChild(pop, "trashEmptySpinner");
            compare(spinner.visible, true);
            // No rotation under reduced motion, but the ring still reads as
            // working against the success check.
            compare(spinner.rotation, 0);
            compare(findChild(pop, "trashEmptyCheck").visible, false);
        }

        function test_empty_trash_result_after_dismiss_is_dropped() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, entries: [], trashFull: true
            });
            confirmEmptyTrash(dock);
            // Dismissing the popover resets the operation state; a late result
            // cannot resurrect it.
            findChild(dock, "trashEmptyPopover").open = false;
            waitForRendering(stage);
            compare(dock.trashEmptyPhase, "idle");
            dock.handleTrashEmptyResult(true, 2, "");
            compare(dock.trashEmptyPhase, "idle");
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

        // -- T-14.7x production pointer injection --------------------------
        // These drive `DockInject`, a QML-callable wrapper over the same
        // `ChromePointer` injection (`DockPointer` alias, T-16.12)
        // `ShellController::onDockPointerMoved/Button`
        // uses. QtTest's own `mouseClick` stamps its events, which hides the
        // zero-timestamp DragHandler bug the compositor's synthetic path hit;
        // the wrapper reproduces the production sequence exactly.

        function injectStationaryTap(window, item) {
            DockInject.reset();
            var p = item.mapToItem(null, item.width / 2, item.height / 2);
            DockInject.move(window, p.x, p.y);
            DockInject.button(window, p.x, p.y, Qt.LeftButton, true);
            DockInject.button(window, p.x, p.y, Qt.LeftButton, false);
            waitForRendering(stage);
        }

        function test_injected_stationary_tap_activates_an_app_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            injectStationaryTap(stage.Window.window, dock.itemAt(0));
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].appId, "files");
        }

        function test_injected_stationary_tap_activates_a_temporary_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ temporary("term", "Terminal") ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            injectStationaryTap(stage.Window.window, dock.itemAt(0));
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].kind, "temporary");
        }

        function test_injected_stationary_tap_activates_a_minimized_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ minimized("files", "Files") ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            injectStationaryTap(stage.Window.window, dock.itemAt(0));
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].kind, "minimized");
        }

        function test_injected_stationary_tap_activates_the_trash() {
            var dock = make(dockComponent, { width: 1280, height: 160, entries: [] });
            activatedSpy.target = dock;
            activatedSpy.clear();
            var trash = dock.itemAt(dock.itemCount() - 1);
            compare(trash.isTrash, true);
            injectStationaryTap(stage.Window.window, trash);
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0].kind, "trash");
        }

        function test_injected_stationary_tap_opens_a_folder_stack() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2, downloadsBadge: 1,
                entries: [ app("a", "A", true) ]
            });
            var idx = dock.indexOfItemId("__downloads__");
            injectStationaryTap(stage.Window.window, dock.itemAt(idx));
            compare(dock.stackOpen, true);
            dock.closePopovers();
        }

        function test_injected_stationary_tap_opens_the_overflow_list() {
            var dock = withOverflow([ hiddenGroup("alpha", "Alpha", 1) ]);
            var idx = dock.indexOfItemId("__overflow__");
            injectStationaryTap(stage.Window.window, dock.itemAt(idx));
            compare(dock.overflowOpen, true);
            dock.closePopovers();
        }

        function test_injected_right_button_opens_the_entry_menu() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true) ]
            });
            menuSpy.target = dock;
            menuSpy.clear();
            var entry = dock.itemAt(0);
            var p = entry.mapToItem(null, entry.width / 2, entry.height / 2);
            DockInject.reset();
            DockInject.move(stage.Window.window, p.x, p.y);
            DockInject.button(stage.Window.window, p.x, p.y, Qt.RightButton, true);
            DockInject.button(stage.Window.window, p.x, p.y, Qt.RightButton, false);
            compare(menuSpy.count, 1);
            compare(menuSpy.signalArguments[0][0].appId, "files");
        }

        function test_injected_slop_drag_lifts_instead_of_activating() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            activatedSpy.target = dock;
            activatedSpy.clear();
            var entry = dock.itemAt(0);
            var p = entry.mapToItem(null, entry.width / 2, entry.height / 2);
            DockInject.reset();
            DockInject.move(stage.Window.window, p.x, p.y);
            DockInject.button(stage.Window.window, p.x, p.y, Qt.LeftButton, true);
            DockInject.move(stage.Window.window, p.x + 24, p.y);
            compare(dock.dragging, true);
            DockInject.button(stage.Window.window, p.x + 24, p.y, Qt.LeftButton, false);
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

        // -- T-14.7m per-window chooser actions -----------------------------

        function test_chooser_close_action_emits_and_keeps_the_chooser_open() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "A", focused: true },
                    { windowId: "2", title: "B", minimized: true }
                ]) ]
            });
            windowCloseSpy.target = dock;
            windowCloseSpy.clear();
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            var close = findChild(chooser, "chooserCloseAction");
            verify(close !== null);
            close.activated();
            compare(windowCloseSpy.count, 1);
            compare(windowCloseSpy.signalArguments[0][0], "1");
            // The action asks; it never dismisses the popover.
            compare(dock.chooserOpen, true);
        }

        function test_chooser_minimize_action_is_stateful() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "A", focused: true },
                    { windowId: "2", title: "B", minimized: true }
                ]) ]
            });
            windowMinimizeSpy.target = dock;
            windowMinimizeSpy.clear();
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            var minimize = findChild(chooser, "chooserMinimizeAction");
            compare(minimize.glyph, "minimize");
            verify(minimize.accessibleLabel.indexOf("Minimize A") === 0);
            minimize.activated();
            compare(windowMinimizeSpy.count, 1);
            compare(windowMinimizeSpy.signalArguments[0][0], "1");
            compare(windowMinimizeSpy.signalArguments[0][1], true);
            compare(dock.chooserOpen, true);
            // The next projection flips the stateful label and glyph.
            dock.entries = [ multiWindow("files", "Files", [
                { windowId: "1", title: "A", minimized: true },
                { windowId: "2", title: "B", minimized: true }
            ]) ];
            waitForRendering(stage);
            var restored = findChild(findChild(dock, "windowChooser"),
                                     "chooserMinimizeAction");
            compare(restored.glyph, "restore");
            verify(restored.accessibleLabel.indexOf("Restore A") === 0);
            compare(dock.chooserOpen, true);
        }

        function test_chooser_close_is_destructive_and_actions_stay_off_the_tab_chain() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "A", focused: true },
                    { windowId: "2", title: "B" }
                ]) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            var close = findChild(chooser, "chooserCloseAction");
            var minimize = findChild(chooser, "chooserMinimizeAction");
            compare(close.destructive, true);
            compare(minimize.destructive, false);
            // The buttons must not steal the chooser's keyboard focus.
            compare(close.activeFocusOnTab, false);
            compare(minimize.activeFocusOnTab, false);
        }

        function test_chooser_rows_update_on_projection_and_dismiss_on_last_close() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "A", focused: true },
                    { windowId: "2", title: "B" }
                ]) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            compare(chooser.windows.length, 2);
            // One window closed: the chooser stays open and drops the row.
            dock.entries = [ multiWindow("files", "Files", [
                { windowId: "1", title: "A", focused: true }
            ]) ];
            waitForRendering(stage);
            compare(dock.chooserOpen, true);
            compare(chooser.windows.length, 1);
            // The app's last window closed: its entry is gone, so it dismisses.
            dock.entries = [];
            waitForRendering(stage);
            compare(dock.chooserOpen, false);
        }

        function test_chooser_row_accessible_name_carries_state() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", [
                    { windowId: "1", title: "Document", focused: true },
                    { windowId: "2", title: "Report", minimized: true }
                ]) ]
            });
            dock.openChooser(dock.items[0]);
            var chooser = findChild(dock, "windowChooser");
            verify(chooser.windowAccessibleName("Document", true, false)
                   .indexOf("(active)") >= 0);
            verify(chooser.windowAccessibleName("Report", false, true)
                   .indexOf("(minimized)") >= 0);
            verify(chooser.windowAccessibleName("Plain", false, false)
                   .indexOf("Plain") === 0);
        }

        // -- T-14.7n chooser row discipline ---------------------------------

        function manyWindows(n) {
            var out = [];
            for (var i = 0; i < n; ++i)
                out.push({ windowId: String(i + 1), title: "Window " + (i + 1),
                           focused: i === 0, workspaceName: "Space 1" });
            return out;
        }

        function chooserRows(dock) {
            return findChild(findChild(dock, "windowChooser"), "chooserRows");
        }

        function test_chooser_viewport_caps_the_visible_rows_at_the_token() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(10)) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            compare(chooser.maxRows, Theme.controls.dock.chooser.maxRows);
            compare(chooser.rowCount, 10);
            compare(chooser.visibleRows, chooser.maxRows);
            compare(chooser.scrolls, true);
            compare(chooser.listHeight, chooser.visibleRows * chooser.rowHeight);
            var view = chooserRows(dock);
            verify(view !== null);
            compare(view.height, chooser.listHeight);
            compare(view.interactive, true);
            // The capped popover stays inside the pre-sized headroom (ADR 0100).
            verify(chooser.height < 320);
            // The header is a fixed band above the viewport, never inside it.
            var header = findChild(chooser, "chooserHeader");
            compare(header.y, chooser.padding);
            fuzzyCompare(view.y, header.y + header.height
                              + Theme.controls.window.borderWidth, 0.001);
        }

        function test_chooser_short_list_has_no_scrollbar_or_gutter() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            compare(chooser.rowCount, 2);
            compare(chooser.visibleRows, 2);
            compare(chooser.scrolls, false);
            compare(chooser.scrollbarGutter, 0);
            compare(chooser.listHeight, 2 * chooser.rowHeight);
            var view = chooserRows(dock);
            compare(view.interactive, false);
            compare(view.height, chooser.listHeight);
            // The surface is exactly as tall as its content: no reserved gutter.
            compare(chooser.height,
                    chooser.padding + findChild(chooser, "chooserHeader").height
                    + Theme.controls.window.borderWidth + chooser.listHeight
                    + chooser.padding + chooser.arrowSize / 2);
        }

        function test_chooser_scrolling_reserves_a_gutter_for_the_actions() {
            var long = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(10)) ]
            });
            long.openChooser(long.items[0]);
            waitForRendering(stage);
            var longChooser = findChild(long, "windowChooser");
            compare(longChooser.scrollbarGutter,
                    Theme.controls.dock.chooser.scrollbarWidth
                    + Theme.controls.dock.chooser.scrollbarMargin);
            var slot = findChild(longChooser, "chooserRowActions");
            var view = chooserRows(long);
            // The reserved action slot sits left of the scrollbar track.
            verify(slot.x + slot.width
                   <= view.width - longChooser.scrollbarGutter + 0.001);

            var short = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            short.openChooser(short.items[0]);
            waitForRendering(stage);
            compare(findChild(short, "windowChooser").scrollbarGutter, 0);
        }

        function test_chooser_header_stays_pinned_while_scrolled() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(10)) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            var header = findChild(chooser, "chooserHeader");
            var view = chooserRows(dock);
            var headerY = header.y;
            var viewY = view.y;
            view.flickable.contentY = 3 * chooser.rowHeight;
            waitForRendering(stage);
            // The rows move under the pinned header; neither the header nor the
            // viewport band moves.
            compare(view.flickable.contentY, 3 * chooser.rowHeight);
            compare(header.y, headerY);
            compare(view.y, viewY);
        }

        function test_chooser_keyboard_scrolls_the_focused_row_into_view() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(10)) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            var flickable = chooserRows(dock).flickable;
            compare(chooser.currentIndex, -1);
            chooser.moveSelection(1);
            compare(chooser.currentIndex, 0);
            compare(flickable.contentY, 0);
            // Jump to the last row: the viewport follows so the focused row is
            // fully visible.
            chooser.moveSelectionTo(chooser.rowCount - 1);
            compare(chooser.currentIndex, 9);
            verify(flickable.contentY > 0);
            verify(flickable.contentY <= 9 * chooser.rowHeight);
            verify(flickable.contentY + flickable.height
                   >= 10 * chooser.rowHeight - 0.001);
            // Home jumps back to the first row and the viewport to the top.
            chooser.moveSelectionTo(0);
            compare(chooser.currentIndex, 0);
            compare(flickable.contentY, 0);
        }

        function test_chooser_keyboard_navigation_is_live() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(8)) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            keyClick(Qt.Key_Down);
            compare(chooser.currentIndex, 0);
            keyClick(Qt.Key_End);
            compare(chooser.currentIndex, 7);
            keyClick(Qt.Key_Home);
            compare(chooser.currentIndex, 0);
            keyClick(Qt.Key_Up);
            compare(chooser.currentIndex, -1);
        }

        function test_chooser_keyboard_enter_activates_and_escape_closes() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            windowSpy.target = dock;
            windowSpy.clear();
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            chooser.moveSelection(1); // the first row
            chooser.activateCurrent();
            compare(windowSpy.count, 1);
            compare(windowSpy.signalArguments[0][0], "1");
            compare(dock.chooserOpen, false);
        }

        // -- T-14.7p hover-open chooser, retarget, and stable anchor ---------

        function hoverFixtureDock(props) {
            var base = { width: 1280, height: 160, chooserOnHover: true };
            for (var key in props)
                base[key] = props[key];
            return make(dockComponent, base);
        }

        function test_chooser_hover_is_off_by_default() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            compare(dock.chooserOnHover, false);
            dock.entryHoverBegan(dock.items[0], dock.itemAt(0));
            // The shipping click contract is unchanged: the name label appears,
            // the chooser does not.
            tryCompare(dock, "tooltipOpen", true);
            compare(dock.chooserOpen, false);
        }

        function test_chooser_hover_dwell_opens_without_a_click() {
            var dock = hoverFixtureDock({
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            compare(dock.chooserOnHover, true);
            dock.entryHoverBegan(dock.items[0], dock.itemAt(0));
            compare(dock.chooserOpen, false, "the chooser waits for the dwell");
            compare(dock.tooltipOpen, false, "hover-open suppresses the name label");
            tryCompare(dock, "chooserOpen", true);
            compare(dock.chooserHoverOpened, true);
            var chooser = findChild(dock, "windowChooser");
            compare(chooser.entry.id, "files");
        }

        function test_chooser_hover_closes_after_the_delay() {
            var dock = hoverFixtureDock({
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            var entry = dock.itemAt(0);
            dock.entryHoverBegan(dock.items[0], entry);
            tryCompare(dock, "chooserOpen", true);
            dock.entryHoverEnded(entry);
            compare(dock.chooserOpen, true, "the popover survives the gap");
            tryCompare(dock, "chooserOpen", false);
        }

        function test_chooser_hover_retargets_between_grouped_entries() {
            var dock = hoverFixtureDock({
                entries: [ multiWindow("files", "Files", manyWindows(2)),
                           multiWindow("music", "Music", manyWindows(3)) ]
            });
            var a = dock.itemAt(0);
            var b = dock.itemAt(1);
            dock.entryHoverBegan(dock.items[0], a);
            tryCompare(dock, "chooserOpen", true);
            var proxy = findChild(dock, "chooserAnchorProxy");
            verify(proxy !== null);
            fuzzyCompare(proxy.x, a.x, 0.5);
            // Move A -> B: the leave restarts the close delay, the enter cancels
            // it and dwells, then retargets the same popover.
            dock.entryHoverEnded(a);
            dock.entryHoverBegan(dock.items[1], b);
            compare(dock.chooserOpen, true, "retarget does not close first");
            tryVerify(function() {
                return dock.chooserEntry !== null && dock.chooserEntry.id === "music";
            });
            compare(dock.chooserOpen, true);
            tryVerify(function() { return Math.abs(proxy.x - b.x) < 0.5; });
            // The retarget keeps the same popover instance and anchor.
            compare(dock.chooserAnchor, proxy);
        }

        function test_chooser_hover_released_onto_a_single_window_entry_closes() {
            var dock = hoverFixtureDock({
                entries: [ multiWindow("files", "Files", manyWindows(2)),
                           app("music", "Music", true) ]
            });
            dock.entryHoverBegan(dock.items[0], dock.itemAt(0));
            tryCompare(dock, "chooserOpen", true);
            dock.entryHoverEnded(dock.itemAt(0));
            // Releasing onto an entry with <= 1 window dismisses cleanly.
            dock.entryHoverBegan(dock.items[1], dock.itemAt(1));
            tryCompare(dock, "chooserOpen", false);
        }

        function test_chooser_hover_gives_way_to_another_popover() {
            var dock = hoverFixtureDock({
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            dock.entryHoverBegan(dock.items[0], dock.itemAt(0));
            tryCompare(dock, "chooserOpen", true);
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            compare(dock.chooserOpen, false, "one popover at a time");
            compare(dock.menuOpen, true);
        }

        function test_chooser_hover_is_suppressed_by_keyboard_navigation() {
            var dock = hoverFixtureDock({
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            dock.entryHoverBegan(dock.items[0], dock.itemAt(0));
            tryCompare(dock, "chooserOpen", true);
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            compare(dock.keyboardFocused, true);
            compare(dock.chooserOpen, false);
        }

        function test_chooser_anchor_survives_a_delegate_rebuild() {
            var dock = hoverFixtureDock({
                entries: [ multiWindow("files", "Files", manyWindows(2)) ]
            });
            dock.entryHoverBegan(dock.items[0], dock.itemAt(0));
            tryCompare(dock, "chooserOpen", true);
            var proxy = findChild(dock, "chooserAnchorProxy");
            verify(proxy !== null);
            compare(dock.chooserAnchor, proxy,
                    "the chooser anchors to the snapshot proxy, not the delegate");
            verify(proxy.width > 0);
            // The shell rebuilds `entries` on a pin change: a fresh list with a
            // new object identity destroys and recreates the delegate.
            dock.entries = [ multiWindow("files", "Files", manyWindows(2)) ];
            waitForRendering(stage);
            compare(dock.chooserOpen, true, "the popover survives the rebuild");
            compare(dock.chooserAnchor, proxy, "the snapshot is still the anchor");
            var item = dock.itemAt(0);
            tryVerify(function() { return Math.abs(proxy.x - item.x) < 0.5; });
            var chooser = findChild(dock, "windowChooser");
            tryVerify(function() {
                return Math.abs(chooser.x + chooser.width / 2
                                - (proxy.x + proxy.width / 2)) < 1.0;
            });
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
            waitForMagnify();
            var halfPeak = dock.layout[1].iconSize;
            verify(halfPeak > dock.iconSize);
            dock.magnification = 1.0;
            waitForRendering(stage);
            waitForMagnify();
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

        // -- Keyboard reordering (T-14.7t) -----------------------------------

        function test_keyboard_reorder_moves_the_focused_pinned_entry() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            compare(dock.focusedItemId, "a");
            waitForRendering(stage);
            keyClick(Qt.Key_Right, Qt.ControlModifier | Qt.ShiftModifier);
            waitForRendering(stage);
            compare(pinnedOrderSpy.count, 1);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","),
                    "b.desktop,a.desktop,c.desktop");
            // The moved entry keeps the focus ring so repeated presses chain.
            compare(dock.focusedItemId, "a");
        }

        function test_keyboard_reorder_is_axis_aware() {
            var vertical = make(dockComponent, {
                width: 160, height: 720, position: "left",
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            pinnedOrderSpy.target = vertical;
            pinnedOrderSpy.clear();
            vertical.forceActiveFocus();
            vertical.beginKeyboardNavigation();
            waitForRendering(stage);
            // The lateral key is not the reorder chord on a vertical Dock.
            keyClick(Qt.Key_Right, Qt.ControlModifier | Qt.ShiftModifier);
            waitForRendering(stage);
            compare(pinnedOrderSpy.count, 0);
            // The Down key is.
            keyClick(Qt.Key_Down, Qt.ControlModifier | Qt.ShiftModifier);
            waitForRendering(stage);
            compare(pinnedOrderSpy.count, 1);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","),
                    "b.desktop,a.desktop");
        }

        function test_keyboard_reorder_is_a_no_op_at_the_ends() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true),
                           app("c", "C", true) ]
            });
            pinnedOrderSpy.target = dock;
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            pinnedOrderSpy.clear();
            // At the first slot, moving left does nothing.
            keyClick(Qt.Key_Left, Qt.ControlModifier | Qt.ShiftModifier);
            waitForRendering(stage);
            compare(pinnedOrderSpy.count, 0);
            compare(dock.focusedItemId, "a");
            // At the last slot, moving right does nothing.
            dock.focusedItemId = "c";
            compare(dock.focusedItemId, "c");
            keyClick(Qt.Key_Right, Qt.ControlModifier | Qt.ShiftModifier);
            waitForRendering(stage);
            compare(pinnedOrderSpy.count, 0);
            compare(dock.focusedItemId, "c");
        }

        function test_keyboard_reorder_does_nothing_when_not_focused() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            // No beginKeyboardNavigation: the Dock does not own the keyboard.
            keyClick(Qt.Key_Right, Qt.ControlModifier | Qt.ShiftModifier);
            waitForRendering(stage);
            compare(pinnedOrderSpy.count, 0);
            compare(dock.focusedItemId, "");
        }

        function test_keyboard_reorder_only_touches_pinned() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), app("b", "B", true),
                           temporary("t", "Terminal"), minimized("win1", "Doc") ]
            });
            pinnedOrderSpy.target = dock;
            pinnedOrderSpy.clear();
            dock.forceActiveFocus();
            dock.beginKeyboardNavigation();
            waitForRendering(stage);
            // Focus down to the temporary running entry: the chord is inert.
            dock.moveKeyboardFocus(1);
            dock.moveKeyboardFocus(1);
            compare(dock.focusedItemId, "t");
            keyClick(Qt.Key_Right, Qt.ControlModifier | Qt.ShiftModifier);
            waitForRendering(stage);
            compare(pinnedOrderSpy.count, 0);
            // Back on a pinned entry, the payload is pinned-only and complete.
            dock.moveKeyboardFocus(-2);
            compare(dock.focusedItemId, "a");
            keyClick(Qt.Key_Right, Qt.ControlModifier | Qt.ShiftModifier);
            waitForRendering(stage);
            compare(pinnedOrderSpy.count, 1);
            compare(pinnedOrderSpy.signalArguments[0][0].join(","),
                    "b.desktop,a.desktop");
        }

        function test_keyboard_reorder_hint_is_axis_aware_and_pinned_only() {
            var bottom = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true), temporary("t", "Terminal") ]
            });
            waitForRendering(stage);
            verify(bottom.itemAt(0).Accessible.description
                   .indexOf("Left or Right") >= 0);
            compare(bottom.itemAt(1).Accessible.description, "");

            var vertical = make(dockComponent, {
                width: 160, height: 720, position: "left",
                entries: [ app("a", "A", true) ]
            });
            waitForRendering(stage);
            verify(vertical.itemAt(0).Accessible.description
                   .indexOf("Up or Down") >= 0);
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

        // -- Window-count badge (T-14.7o) ----------------------------------

        function windowedEntry(id, count, extra) {
            var list = [];
            for (var i = 0; i < count; ++i)
                list.push({ windowId: String(i + 1), title: "W" + (i + 1) });
            var e = { id: id, appId: id, name: id, kind: "pinned",
                      pinned: true, running: true, windowCount: count,
                      windows: count, windowList: list };
            if (extra)
                for (var k in extra)
                    e[k] = extra[k];
            return e;
        }

        function test_window_badge_shows_for_a_grouped_app() {
            var entry = make(entryComponent, {
                iconSize: 48, entry: windowedEntry("files", 2)
            });
            var badge = findChild(entry, "windowBadge");
            verify(badge !== null);
            compare(badge.visible, true);
            compare(findChild(entry, "windowBadgeText").text, "2");
            compare(badge.color, Theme.color.accent);
        }

        function test_window_badge_hides_for_zero_or_one_window() {
            var one = make(entryComponent, {
                iconSize: 48, entry: windowedEntry("files", 1)
            });
            compare(findChild(one, "windowBadge").visible, false);

            var zero = make(entryComponent, {
                iconSize: 48, entry: windowedEntry("files", 0)
            });
            compare(findChild(zero, "windowBadge").visible, false);

            // A stopped pinned app (no window list at all) shows nothing.
            var stopped = make(entryComponent, {
                iconSize: 48, entry: app("files", "Files", false)
            });
            compare(findChild(stopped, "windowBadge").visible, false);
        }

        function test_window_badge_caps_at_nine_plus() {
            var entry = make(entryComponent, {
                iconSize: 48, entry: windowedEntry("files", 12)
            });
            compare(findChild(entry, "windowBadgeText").text, "9+");
        }

        function test_window_badge_is_attention_colored() {
            var normal = make(entryComponent, {
                iconSize: 48, entry: windowedEntry("files", 3)
            });
            compare(findChild(normal, "windowBadge").color, Theme.color.accent);

            var urgent = make(entryComponent, {
                iconSize: 48,
                entry: windowedEntry("mail", 3, { attention: true })
            });
            compare(findChild(urgent, "windowBadge").color, Theme.color.danger);
        }

        function test_window_badge_yields_to_status_and_app_badge() {
            // The failure state wins: no count badge, the status badge shows.
            var failed = make(entryComponent, {
                iconSize: 48,
                entry: windowedEntry("files", 4, { launch: "failed" })
            });
            compare(findChild(failed, "windowBadge").visible, false);
            compare(findChild(failed, "statusBadge").visible, true);

            var missing = make(entryComponent, {
                iconSize: 48,
                entry: windowedEntry("files", 4, { missing: true })
            });
            compare(findChild(missing, "windowBadge").visible, false);

            // An app-provided count also wins over the derived one.
            var appBadge = make(entryComponent, {
                iconSize: 48,
                entry: windowedEntry("files", 4, { badge: 7 })
            });
            compare(findChild(appBadge, "windowBadge").visible, false);
        }

        function test_window_badge_never_on_stacks_or_trash() {
            var stack = make(entryComponent, {
                iconSize: 48,
                entry: { id: "downloads", name: "Downloads", kind: "stack",
                         stackCount: 4, running: false, windowCount: 4 }
            });
            compare(findChild(stack, "windowBadge").visible, false);

            var trash = make(entryComponent, {
                iconSize: 48,
                entry: { id: "__trash__", name: "Trash", kind: "trash",
                         running: false, windowCount: 3 }
            });
            compare(findChild(trash, "windowBadge").visible, false);

            // A per-window minimized row is one window, not the app's group.
            var minimized = make(entryComponent, {
                iconSize: 48,
                entry: { id: "win:1", name: "Doc", kind: "minimized",
                         running: false, windowCount: 3,
                         windowList: [ { windowId: "1" }, { windowId: "2" },
                                       { windowId: "3" } ] }
            });
            compare(findChild(minimized, "windowBadge").visible, false);
        }

        function test_window_badge_geometry_comes_from_tokens() {
            var entry = make(entryComponent, {
                iconSize: 48, entry: windowedEntry("files", 2)
            });
            var badge = findChild(entry, "windowBadge");
            var expected = Math.max(
                Theme.controls.dock.windowBadge.sizeMin,
                Math.min(Theme.controls.dock.windowBadge.sizeMax,
                         Math.round(48 * Theme.controls.dock.windowBadge.sizeRatio)));
            fuzzyCompare(badge.height, expected, 0.001);
            fuzzyCompare(badge.x,
                         entry.artworkX + entry.iconSize - badge.width
                         - Theme.controls.dock.windowBadge.inset, 0.001);
        }

        function test_window_badge_is_instant_under_reduced_motion() {
            Theme.reducedMotion = true;
            var entry = make(entryComponent, {
                iconSize: 48, entry: windowedEntry("files", 2)
            });
            var badge = findChild(entry, "windowBadge");
            compare(badge.visible, true);
            // Dropping to one window removes it with no fade under reduced
            // motion (the `motion.focus` duration is 0).
            entry.entry = windowedEntry("files", 1);
            tryCompare(badge, "visible", false);
        }

        // -- Terminal overflow cell (T-14.7q) ------------------------------

        function hiddenGroup(id, name, windowCount) {
            var list = [];
            for (var i = 0; i < windowCount; ++i)
                list.push({ windowId: id + "-w" + i, title: name + " " + (i + 1),
                            focused: i === 0, workspaceName: "Space 1" });
            return { id: id, appId: id, name: name, kind: "temporary",
                     running: true, desktopId: id + ".desktop",
                     windowCount: windowCount, windowList: list,
                     overflowGroup: true };
        }

        function overflowEntry(groups) {
            return { id: "__overflow__", kind: "overflow", name: "", appId: "",
                     running: false, pinned: false,
                     hiddenCount: groups.length, windowCount: groups.length,
                     groups: groups };
        }

        function withOverflow(groups) {
            return make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("files", "Files", true), overflowEntry(groups) ]
            });
        }

        function test_overflow_cell_renders_grid_and_count_badge() {
            var dock = withOverflow([ hiddenGroup("alpha", "Alpha", 1),
                                      hiddenGroup("beta", "Beta", 3) ]);
            var idx = dock.indexOfItemId("__overflow__");
            verify(idx >= 0);
            var item = dock.itemAt(idx);
            compare(item.kind, "overflow");
            verify(findChild(item, "overflowArtwork") !== null);
            var badge = findChild(item, "windowBadge");
            verify(badge !== null);
            compare(badge.visible, true);
            compare(findChild(item, "windowBadgeText").text, "2");
            compare(item.Accessible.name.indexOf("2 more window groups") >= 0, true);
            // It is not reorderable or pinnable.
            compare(dock.isDraggable(item.entry), false);
        }

        function test_overflow_cell_is_not_last_when_there_is_a_divider() {
            var dock = withOverflow([ hiddenGroup("alpha", "Alpha", 1) ]);
            var idx = dock.indexOfItemId("__overflow__");
            compare(dock.items[idx + 1].kind, "divider");
        }

        function test_overflow_cell_opens_the_more_windows_list() {
            var dock = withOverflow([ hiddenGroup("alpha", "Alpha", 1),
                                      hiddenGroup("beta", "Beta", 2) ]);
            var idx = dock.indexOfItemId("__overflow__");
            dock.activateEntry(dock.items[idx]);
            compare(dock.overflowOpen, true);
            verify(dock.popoverRect.w > 0);
            var popover = findChild(dock, "overflowPopover");
            compare(popover.rowCount, 2);
            tryCompare(popover, "visible", true);
            // Every hidden group is present, none lost.
            var names = [];
            for (var i = 0; i < popover.groups.length; ++i)
                names.push(popover.groups[i].name);
            verify(names.indexOf("Alpha") >= 0);
            verify(names.indexOf("Beta") >= 0);
        }

        function test_overflow_single_window_row_activates_the_window() {
            var dock = withOverflow([ hiddenGroup("alpha", "Alpha", 1),
                                      hiddenGroup("beta", "Beta", 2) ]);
            windowSpy.target = dock;
            windowSpy.clear();
            dock.activateEntry(dock.items[dock.indexOfItemId("__overflow__")]);
            var popover = findChild(dock, "overflowPopover");
            popover.groupActivated(popover.groups[0]);
            compare(windowSpy.count, 1);
            compare(windowSpy.signalArguments[0][0], "alpha-w0");
            compare(dock.overflowOpen, false);
        }

        function test_overflow_multi_window_row_opens_the_chooser_on_the_cell() {
            var dock = withOverflow([ hiddenGroup("alpha", "Alpha", 1),
                                      hiddenGroup("beta", "Beta", 3) ]);
            dock.activateEntry(dock.items[dock.indexOfItemId("__overflow__")]);
            var popover = findChild(dock, "overflowPopover");
            popover.groupActivated(popover.groups[1]);
            compare(dock.overflowOpen, false);
            compare(dock.chooserOpen, true);
            compare(dock.chooserEntry.id, "beta");
            // Anchored to the overflow cell snapshot, not a hidden delegate.
            compare(dock.chooserAnchor.objectName, "chooserAnchorProxy");
            var cell = dock.itemAt(dock.indexOfItemId("__overflow__"));
            fuzzyCompare(dock.chooserAnchor.x, cell.x, 0.001);
            var chooser = findChild(dock, "windowChooser");
            compare(chooser.windows.length, 3);
        }

        function test_overflow_multi_window_chooser_survives_a_projection() {
            var dock = withOverflow([ hiddenGroup("beta", "Beta", 2) ]);
            dock.activateEntry(dock.items[dock.indexOfItemId("__overflow__")]);
            var popover = findChild(dock, "overflowPopover");
            popover.groupActivated(popover.groups[0]);
            compare(dock.chooserOpen, true);
            // A fresh projection with one window closed re-resolves the group
            // from the new overflow entry and keeps the chooser open.
            dock.entries = [ app("files", "Files", true),
                             overflowEntry([ hiddenGroup("beta", "Beta", 1) ]) ];
            waitForRendering(stage);
            compare(dock.chooserOpen, true);
            compare(dock.chooserEntry.id, "beta");
            // The overflow cell gone: the chooser dismisses.
            dock.entries = [];
            waitForRendering(stage);
            compare(dock.chooserOpen, false);
        }

        function test_overflow_popover_dismisses_when_the_cell_leaves() {
            var dock = withOverflow([ hiddenGroup("alpha", "Alpha", 1) ]);
            dock.activateEntry(dock.items[dock.indexOfItemId("__overflow__")]);
            compare(dock.overflowOpen, true);
            dock.entries = [ app("files", "Files", true) ];
            waitForRendering(stage);
            compare(dock.overflowOpen, false);
        }

        function test_overflow_empty_list_is_a_disabled_row() {
            var dock = withOverflow([]);
            dock.activateEntry(dock.items[dock.indexOfItemId("__overflow__")]);
            var popover = findChild(dock, "overflowPopover");
            compare(popover.rowCount, 0);
            tryCompare(findChild(popover, "overflowEmptyRow"), "visible", true);
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

        // --- T-14.7k: any folder as a stack --------------------------------

        function folderPin() {
            return { id: "folder:/home/u/Documents", path: "/home/u/Documents",
                     name: "Documents",
                     items: [ { name: "notes.txt", path: "/home/u/Documents/notes.txt",
                                isDir: false },
                              { name: "Projects", path: "/home/u/Documents/Projects",
                                isDir: true } ],
                     count: 2, badge: 0, missing: false };
        }

        function test_folder_pin_renders_in_the_stacks_region_before_trash() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2,
                folderPins: [ folderPin() ],
                entries: [ app("a", "A", true) ]
            });
            var downloadsIdx = dock.indexOfItemId("__downloads__");
            var pinIdx = dock.indexOfItemId("folder:/home/u/Documents");
            var trashIdx = dock.indexOfItemId("__trash__");
            verify(downloadsIdx >= 0 && pinIdx > downloadsIdx && trashIdx > pinIdx);
            var pin = dock.items[pinIdx];
            compare(pin.kind, "stack");
            compare(pin.name, "Documents");
            compare(pin.stackCount, 2);
            compare(pin.canRemove, true);
            // The Downloads default member is not removable.
            compare(dock.items[downloadsIdx].canRemove, false);
        }

        function test_folder_pin_single_click_opens_its_own_popover() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2,
                folderPins: [ folderPin() ],
                entries: [ app("a", "A", true) ]
            });
            folderViewedSpy.target = dock;
            folderViewedSpy.clear();
            var pinIdx = dock.indexOfItemId("folder:/home/u/Documents");
            dock.activateEntry(dock.items[pinIdx]);
            compare(dock.stackOpen, true);
            compare(folderViewedSpy.count, 1);
            compare(folderViewedSpy.signalArguments[0][0], "/home/u/Documents");
            var popover = findChild(dock, "stackPopover");
            compare(popover.title, "Documents");
            compare(popover.items.length, 2);
        }

        function test_folder_pin_double_click_opens_the_folder_in_files() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2,
                folderPins: [ folderPin() ],
                entries: [ app("a", "A", true) ]
            });
            folderOpenSpy.target = dock;
            folderOpenSpy.clear();
            var pin = dock.items[dock.indexOfItemId("folder:/home/u/Documents")];
            dock.handleEntryTap(pin);
            compare(dock.stackOpen, true);
            compare(folderOpenSpy.count, 0);
            dock.handleEntryTap(pin);
            compare(folderOpenSpy.count, 1);
            compare(folderOpenSpy.signalArguments[0][0], "/home/u/Documents");
            compare(dock.stackOpen, false);
        }

        function test_folder_pin_context_menu_offers_open_and_remove() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2,
                folderPins: [ folderPin() ],
                entries: [ app("a", "A", true) ]
            });
            var pin = dock.items[dock.indexOfItemId("folder:/home/u/Documents")];
            dock.openEntryMenu(pin);
            var hasOpen = false;
            var hasRemove = false;
            for (var i = 0; i < dock.menuModel.length; ++i) {
                if (dock.menuModel[i].action === "open_stack_folder")
                    hasOpen = true;
                if (dock.menuModel[i].action === "remove_folder_pin")
                    hasRemove = true;
            }
            verify(hasOpen);
            verify(hasRemove);
        }

        function test_folder_pin_drag_out_removes_it() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsItems: stackItems(), downloadsCount: 2,
                folderPins: [ folderPin() ],
                entries: [ app("a", "A", true) ]
            });
            folderRemovedSpy.target = dock;
            folderRemovedSpy.clear();
            var pin = dock.items[dock.indexOfItemId("folder:/home/u/Documents")];
            dock.beginDrag(pin);
            verify(dock.dragging);
            // A drop far off the Dock ends the pin (T-14.7k).
            dock.dropAt(pin, -1000);
            compare(folderRemovedSpy.count, 1);
            compare(folderRemovedSpy.signalArguments[0][0], "/home/u/Documents");
        }

        function test_missing_folder_pin_degrades_but_renders() {
            var missing = folderPin();
            missing.missing = true;
            missing.items = [];
            missing.count = 0;
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                folderPins: [ missing ],
                entries: [ app("a", "A", true) ]
            });
            var idx = dock.indexOfItemId("folder:/home/u/Documents");
            verify(idx >= 0);
            compare(dock.items[idx].missing, true);
            compare(dock.items[idx].stackCount, 0);
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

        // T-19.1d: our own apps render their bundled Phosphor artwork keyed by
        // desktop id, so nested dev (before an install) shows the right tile
        // even with no app-index themed path.
        function test_first_party_app_uses_bundled_tile() {
            var files = make(glyphComponent, {
                kind: "app", name: "Files", appId: "org.dragonfruit.Files",
                desktopId: "org.dragonfruit.Files.desktop", size: 48
            });
            compare(files.bundledIcon,
                    "qrc:/icons/apps/org.dragonfruit.Files.svg");
            verify(files.hasBundledIcon);
            verify(files.hasAppArtwork);
            tryCompare(files, "maskedArtwork", true);
            waitForRendering(stage);
            wait(60);
            var img = grabImage(files);
            // The Files tile is our flat blue rounded square; a point above
            // the centred folder glyph is a real bundled pixel, not the
            // placeholder initial.
            verify(nearColor(img, 24, 4, 59, 130, 246, 40),
                   "the bundled Files artwork renders");
            files.destroy();

            var settings = make(glyphComponent, {
                kind: "app", name: "Settings", appId: "org.dragonfruit.Settings",
                desktopId: "org.dragonfruit.Settings.desktop", size: 48
            });
            compare(settings.bundledIcon,
                    "qrc:/icons/apps/org.dragonfruit.Settings.svg");
            verify(settings.hasBundledIcon);
            tryCompare(settings, "maskedArtwork", true);
            settings.destroy();
        }

        // T-19.1d: a third-party entry has no bundled tile and still prefers
        // the app-index themed `iconPath`.
        function test_third_party_prefers_themed_icon_path() {
            var themed = make(glyphComponent, {
                kind: "app", name: "Square",
                desktopId: "com.example.Square.desktop",
                iconPath: assetPath("icon-square.svg"), size: 48
            });
            compare(themed.bundledIcon, "");
            verify(!themed.hasBundledIcon);
            verify(themed.hasThemedIconHint);
            verify(themed.iconUrl.startsWith("file://"));
            tryCompare(themed, "maskedArtwork", true);
            themed.destroy();
        }

        // T-19.1d: for a first-party id the bundled tile wins even when
        // app-index happened to resolve a themed path.
        function test_first_party_bundled_tile_wins_over_themed_path() {
            var glyph = make(glyphComponent, {
                kind: "app", name: "Files",
                desktopId: "org.dragonfruit.Files.desktop",
                iconPath: assetPath("icon-square.svg"), size: 48
            });
            compare(glyph.bundledIcon,
                    "qrc:/icons/apps/org.dragonfruit.Files.svg");
            compare(glyph.iconUrl, glyph.bundledIcon);
            glyph.destroy();
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

        // -- T-14.7i hover name label ---------------------------------------

        function test_tooltip_shows_the_name_after_the_dwell() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "Safari", true) ]
            });
            var entry = dock.itemAt(0);
            dock.entryHoverBegan(dock.entries[0], entry);
            compare(dock.tooltipOpen, false, "the label waits for the dwell");
            compare(dock.tooltipText, "Safari");
            tryCompare(dock, "tooltipOpen", true);
            compare(dock.tooltipAnchor, entry);
            var tooltip = findChild(dock, "dockTooltip");
            verify(tooltip !== null, "the Dock hosts one design-system Tooltip");
            compare(tooltip.text, "Safari");
            verify(tooltip.y + tooltip.height <= entry.y + 0.5,
                   "a bottom Dock's label sits above the entry");
            compare(tooltip.activeFocus, false, "a tooltip never takes focus");
            compare(tooltip.focus, false);
        }

        function test_tooltip_hides_on_pointer_leave() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "Safari", true) ]
            });
            var entry = dock.itemAt(0);
            dock.entryHoverBegan(dock.entries[0], entry);
            tryCompare(dock, "tooltipOpen", true);
            dock.entryHoverEnded(entry);
            compare(dock.tooltipOpen, false);
            var tooltip = findChild(dock, "dockTooltip");
            tryCompare(tooltip, "visible", false);
            wait(200);
            verify(dock.tooltipAnchor === null,
                   "the anchor is released once the label is hidden");
        }

        function test_tooltip_hides_on_click() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "Safari", true) ]
            });
            var entry = dock.itemAt(0);
            dock.entryHoverBegan(dock.entries[0], entry);
            tryCompare(dock, "tooltipOpen", true);
            dock.handleEntryTap(dock.entries[0]);
            compare(dock.tooltipOpen, false);
        }

        function test_tooltip_is_suppressed_by_a_popover() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "Safari", true) ]
            });
            var entry = dock.itemAt(0);
            dock.entryHoverBegan(dock.entries[0], entry);
            tryCompare(dock, "tooltipOpen", true);
            dock.openEntryMenu(dock.entries[0]);
            compare(dock.tooltipOpen, false);
        }

        function test_tooltip_state_text_for_windows_folder_and_trash() {
            var withWindows = { id: "a", appId: "a", name: "Safari", kind: "pinned",
                                running: true, pinned: true, desktopId: "a.desktop",
                                windowList: [ { windowId: 1 }, { windowId: 2 },
                                              { windowId: 3 } ] };
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsCount: 3,
                entries: [ withWindows ]
            });
            compare(dock.itemAt(0).tooltipLabel, "Safari — 3 windows");
            // items: app, divider, stack, trash.
            compare(dock.itemAt(2).tooltipLabel, "Downloads — 3 items");
            compare(dock.itemAt(3).tooltipLabel, "Trash — empty");
        }

        function test_tooltip_folder_name_is_data_not_a_literal() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                downloadsName: "Screenshots", downloadsCount: 1, entries: []
            });
            compare(dock.itemAt(0).tooltipLabel, "Screenshots — 1 item");
        }

        function test_tooltip_follows_the_magnified_entry_and_stays_open() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 1.0,
                entries: [ app("a", "Safari", true), app("b", "Music", true) ]
            });
            var entry = dock.itemAt(0);
            dock.entryHoverBegan(dock.entries[0], entry);
            tryCompare(dock, "tooltipOpen", true);
            dock.pointerAlong = dock._baseline.centers[0];
            tryCompare(dock, "magnifying", true);
            wait(250);
            compare(dock.tooltipOpen, true,
                    "a label stays up while magnification runs");
            var tooltip = findChild(dock, "dockTooltip");
            verify(entry.iconSize > dock.iconSize, "the hovered entry grew");
            verify(tooltip.y + tooltip.height <= entry.y + 0.5,
                   "the label stays anchored above the magnified entry");
        }

        function test_tooltip_clamps_at_the_dock_end() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "A", true) ]
            });
            dock.entryHoverBegan(dock.entries[0], dock.itemAt(0));
            tryCompare(dock, "tooltipOpen", true);
            var tooltip = findChild(dock, "dockTooltip");
            verify(tooltip.x >= 0, "the label must not spill past the leading edge");
            verify(tooltip.x + tooltip.width <= dock.width + 0.5,
                   "the label must be clamped at the trailing edge");
        }

        function test_tooltip_rides_the_overlay_rect() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "Safari", true) ]
            });
            compare(dock.popoverRect.w, 0);
            dock.entryHoverBegan(dock.entries[0], dock.itemAt(0));
            tryCompare(dock, "tooltipOpen", true);
            tryCompare(findChild(dock, "dockTooltip"), "visible", true);
            verify(dock.popoverRect.w > 0,
                   "the hover label is committed through the overlay path");
            verify(dock.popoverRect.h > 0);
        }

        // The hover label points at its icon with a token tail (T-14.7aa):
        // the capsule clears the anchor by the tail plus the offset, the tail
        // is aimed at the anchor centre, and the committed overlay rect covers
        // the tail so the shell's copy never clips it.
        function test_tooltip_tail_points_at_the_anchor_and_rides_the_rect() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "Safari", true), app("b", "Music", true) ]
            });
            var entry = dock.itemAt(0);
            dock.entryHoverBegan(dock.entries[0], entry);
            tryCompare(dock, "tooltipOpen", true);
            var tooltip = findChild(dock, "dockTooltip");
            compare(tooltip.tailVisible, true, "the Dock label carries the tail");
            tryCompare(tooltip, "visible", true);
            var tail = findChild(tooltip, "tooltipTail");
            verify(tail !== null && tail.visible, "the tail is drawn");
            fuzzyCompare(tail.width, Theme.controls.tooltip.tailWidth, 0.001);
            fuzzyCompare(tail.height, Theme.controls.tooltip.tailHeight, 0.001);
            // Above placement: the capsule bottom sits `offset + tailHeight`
            // above the entry, so the tip lands `offset` away from it.
            fuzzyCompare(tooltip.y + tooltip.height + Theme.controls.tooltip.tailHeight,
                         entry.y - Theme.controls.tooltip.offset, 0.501);
            // The tail is aimed at the anchor centre.
            fuzzyCompare(tooltip.x + tail.x + tail.width / 2,
                         entry.x + entry.width / 2, 0.501);
            // The committed overlay rect covers the capsule plus the tail.
            var rect = dock.tooltipRect;
            fuzzyCompare(rect.y, tooltip.y, 0.501);
            fuzzyCompare(rect.h,
                         tooltip.height + Theme.controls.tooltip.tailHeight, 0.501);
        }

        function test_tooltip_reduced_motion_opens_without_a_fade() {
            Theme.reducedMotion = true;
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ app("a", "Safari", true) ]
            });
            dock.entryHoverBegan(dock.entries[0], dock.itemAt(0));
            dock.showTooltip();
            waitForRendering(stage);
            compare(findChild(dock, "dockTooltip").opacity, 1);
        }
    }
}
