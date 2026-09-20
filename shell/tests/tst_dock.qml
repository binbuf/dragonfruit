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

        SignalSpy { id: activatedSpy; signalName: "entryActivated" }
        SignalSpy { id: menuSpy; signalName: "entryContextMenuRequested" }
        SignalSpy { id: dividerMenuSpy; signalName: "dividerContextMenuRequested" }

        function make(component, props) {
            var obj = createTemporaryObject(component, stage, props || {});
            waitForRendering(stage);
            return obj;
        }

        function app(id, name, running) {
            return { id: id, appId: id, name: name, kind: "pinned",
                     running: running === true, pinned: true };
        }

        function temporary(id, name) {
            return { id: id, appId: id, name: name, kind: "temporary",
                     running: true };
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
            compare(items.length, 5); // apps, divider, minimized, trash
            compare(items[0].kind, "pinned");
            compare(items[1].kind, "temporary");
            compare(items[2].kind, "divider");
            compare(items[3].kind, "minimized");
            compare(items[4].kind, "trash");
            compare(dock.itemCount(), 5);
        }

        function test_minimized_hidden_when_minimize_into_tile_icon() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                minimizeIntoTileIcon: true,
                entries: [ app("files", "Files", true), minimized("win1", "Document") ]
            });
            compare(dock.minimizedEntries.length, 0);
            // apps, divider, trash
            compare(dock.items.length, 3);
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
            // Items stack on the y axis and keep the bar on the left edge.
            var layout = dock.layout;
            verify(layout[1].y > layout[0].y);
            verify(dock.barRect.x < layout[0].x);
        }

        function test_right_dock_indicator_edge() {
            var dock = make(dockComponent, {
                width: 160, height: 800, position: "right",
                entries: [ app("a", "A", true) ]
            });
            compare(dock.indicatorEdge, "right");
        }

        // -- Auto-hide ------------------------------------------------------

        function test_auto_hide_translates_off_edge() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: true, revealed: true,
                entries: [ app("a", "A", true) ]
            });
            compare(dock.hideOffset, 0);
            dock.hide();
            waitForRendering(stage);
            verify(dock.hideOffset >= dock.barThickness);
            dock.reveal();
            waitForRendering(stage);
            compare(dock.hideOffset, 0);
        }

        function test_auto_hide_off_never_translates() {
            var dock = make(dockComponent, {
                width: 1280, height: 160, autoHide: false, revealed: false,
                entries: [ app("a", "A", true) ]
            });
            compare(dock.hideOffset, 0);
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

        // -- Artwork --------------------------------------------------------

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
