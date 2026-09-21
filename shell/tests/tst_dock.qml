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

        // Reduced motion is a global singleton; reset it before every test so
        // a failure mid-test cannot leak into the next one.
        function init() {
            Theme.reducedMotion = false;
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

        function test_bar_does_not_grow_with_magnification() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, magnification: 0,
                entries: [ app("a", "A", true), app("b", "B", true) ]
            });
            var baseBar = dock.barRect.w;
            dock.magnification = 1.0;
            dock.pointerAlong = dock._baseline.centers[0];
            waitForRendering(stage);
            compare(dock.barRect.w, baseBar);
            compare(dock.barRect.h, dock.barThickness);
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
            // Items stack on the y axis and the bar hugs the left edge.
            var layout = dock.layout;
            verify(layout[1].y > layout[0].y);
            compare(dock.barRect.x, 0);
            compare(dock.barRect.w, dock.barThickness);
            // An entry starts at the bar padding on the anchored edge side.
            fuzzyCompare(layout[0].x, dock.padding, 0.001);
            verify(dock.barRect.x + dock.barThickness > layout[0].x);
        }

        function test_right_dock_indicator_edge() {
            var dock = make(dockComponent, {
                width: 160, height: 800, position: "right",
                entries: [ app("a", "A", true) ]
            });
            compare(dock.indicatorEdge, "right");
            // The bar hugs the right edge and entries pack from the right so
            // the running indicator is against the screen edge.
            compare(dock.barRect.x, 160 - dock.barThickness);
            var layout = dock.layout;
            fuzzyCompare(layout[0].x + layout[0].w, 160 - dock.padding, 0.001);
        }

        function test_vertical_entries_stay_inside_the_surface() {
            // A vertical Dock's magnified artwork grows into the transparent
            // band on the interior side; it must never be clipped by the
            // (thickness + band)-wide surface.
            var left = make(dockComponent, {
                width: 124, height: 720, position: "left", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            left.pointerAlong = left._baseline.centers[0];
            waitForRendering(stage);
            verify(left.barRect.x >= 0);
            verify(left.barRect.x + left.barRect.w <= left.width);
            for (var i = 0; i < left.layout.length; ++i) {
                verify(left.layout[i].x >= 0);
                verify(left.layout[i].x + left.layout[i].w <= left.width + 0.001);
            }

            var right = make(dockComponent, {
                width: 124, height: 720, position: "right", magnification: 1.0,
                entries: [ app("a", "A", true), app("b", "B", true), app("c", "C", true) ]
            });
            right.pointerAlong = right._baseline.centers[0];
            waitForRendering(stage);
            verify(right.barRect.x >= 0);
            verify(right.barRect.x + right.barRect.w <= right.width);
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
            verify(left.barRect.x < 0);

            var right = make(dockComponent, {
                width: 160, height: 800, position: "right",
                autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            right.hide();
            wait(Theme.motion.dockReveal.duration + 40);
            verify(right.hideX > 0);
            verify(right.barRect.x > 160 - right.barThickness);
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
            verify(dock.barRect.y > dock.magnifyBand);
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
            fuzzyCompare(dock.inputRects[0].y, dock.barRect.y, 0.001);
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
            verify(left.popoverRect.x >= left.barRect.x + left.barRect.w);

            var right = make(dockComponent, {
                width: 124, height: 720, position: "right",
                entries: [ app("files", "Files", true) ]
            });
            right.openEntryMenu(right.items[0]);
            waitForRendering(stage);
            verify(right.popoverRect.w > 0);
            verify(right.popoverRect.x + right.popoverRect.w <= right.barRect.x);
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
            var far = dock.barRect.x - dock.iconSize - 50;
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
            dock.dropAt(t, dock.barRect.x - dock.iconSize - 50);
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
            var bandY = Math.max(0, Math.floor(dock.barRect.y) - 4);
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
    }
}
