// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Files

// Files app-shell tests (T-10.4a): the window opens on Home with the
// Favorites/Locations sidebar, sidebar selection changes the browsing model,
// toolbar history works, the view switch persists per location, and the
// AT-SPI roles are present. `DF_FILES_FIXTURE=1` makes the location set
// deterministic. Headless on the offscreen platform + software scene graph.
Item {
    id: stage
    width: 1100
    height: 760

    TestCase {
        id: testCase
        name: "FilesShell"
        when: windowShown

        Component { id: shellComponent; FilesShell { } }

        SignalSpy { id: closeSpy; signalName: "closeRequested" }

        function make(props) {
            var merged = { width: stage.width, height: stage.height };
            for (var key in (props || {}))
                merged[key] = props[key];
            var obj = createTemporaryObject(shellComponent, stage, merged);
            waitForRendering(stage);
            return obj;
        }

        function labels(shell) {
            return shell.sidebar.entries.map(function(entry) {
                return entry.label;
            });
        }

        // -- Platform seam ------------------------------------------------------

        function test_platform_locations_are_deterministic() {
            compare(Files.displayName(Files.homeUri), "tester");
            compare(Files.displayName(Files.computerUri), "Computer");
            compare(Files.displayName(Files.trashUri), "Trash");
            verify(Files.favorites.length >= 3);
            // Home is a Location, not a Favorite (09-files.md#sidebar).
            verify(Files.favorites[0].uri !== Files.homeUri);
            compare(Files.volumes.length, 1);
            compare(Files.volumes[0].label, "Data");
            verify(Files.isBrowsable(Files.homeUri));
            verify(Files.isBrowsable(Files.trashUri));
            verify(!Files.isBrowsable("https://example.com"));
        }

        function test_breadcrumb_runs_from_computer_to_location() {
            var crumbs = Files.breadcrumb(Files.favorites[1].uri); // Documents
            compare(crumbs.length, 3);
            compare(crumbs[0].label, "Computer");
            compare(crumbs[1].label, "tester");
            compare(crumbs[2].label, "Documents");
            compare(crumbs[2].uri, Files.favorites[1].uri);

            var computer = Files.breadcrumb(Files.computerUri);
            compare(computer.length, 1);
            compare(computer[0].label, "Computer");

            var trash = Files.breadcrumb(Files.trashUri);
            compare(trash.length, 1);
            compare(trash[0].label, "Trash");
        }

        // -- Shell opens --------------------------------------------------------

        function test_shell_opens_at_home_with_sections() {
            var shell = make();
            compare(shell.browser.currentUri, Files.homeUri);
            compare(shell.locationTitle, "tester");
            compare(shell.titleBar.title, "tester");
            compare(shell.titleBar.width, shell.width,
                    "the titlebar must span the window so the whole bar drags");
            verify(shell.sidebar !== null);
            verify(shell.toolbar !== null);
            verify(shell.searchField !== null);

            var all = labels(shell);
            verify(all.indexOf("Favorites") >= 0);
            verify(all.indexOf("Locations") >= 0);
            compare(all[all.length - 1], "Trash");
            compare(shell.sidebar.currentIndex,
                    shell.entryIndexForUri(Files.homeUri));

            // At the start of history both affordances are dimmed.
            verify(!shell.backButton.enabled);
            verify(!shell.forwardButton.enabled);
        }

        // -- Sidebar changes the browsing model ---------------------------------

        function test_sidebar_selection_changes_the_model() {
            var shell = make();
            var documents = Files.favorites[1].uri;
            shell.sidebar.activateIndex(shell.entryIndexForUri(documents));
            compare(shell.browser.currentUri, documents);
            compare(shell.locationTitle, "Documents");
            verify(shell.backButton.enabled);
            compare(shell.pathBar.model[shell.pathBar.model.length - 1].label,
                    "Documents");
        }

        function test_sidebar_keyboard_navigates() {
            var shell = make();
            shell.sidebar.forceActiveFocus();
            waitForRendering(stage);
            // The default selection is Home; Down steps to Computer.
            keyClick(Qt.Key_Down);
            compare(shell.browser.currentUri, Files.computerUri);
            keyClick(Qt.Key_Up);
            compare(shell.browser.currentUri, Files.homeUri);
            verify(shell.backButton.enabled);
        }

        // -- Toolbar history ----------------------------------------------------

        function test_toolbar_history_back_and_forward() {
            var shell = make();
            var documents = Files.favorites[1].uri;
            var downloads = Files.favorites[2].uri;
            shell.browser.navigate(documents);
            shell.browser.navigate(downloads);
            compare(shell.browser.currentUri, downloads);
            verify(!shell.browser.canGoForward);

            mouseClick(shell.backButton);
            waitForRendering(stage);
            compare(shell.browser.currentUri, documents);
            verify(shell.backButton.enabled);
            verify(shell.forwardButton.enabled);

            mouseClick(shell.forwardButton);
            waitForRendering(stage);
            compare(shell.browser.currentUri, downloads);
            verify(!shell.forwardButton.enabled);
        }

        function test_history_does_not_grow_on_same_location() {
            var shell = make();
            shell.sidebar.activateIndex(shell.entryIndexForUri(Files.homeUri));
            compare(shell.browser.history.length, 1);
            shell.sidebar.activateIndex(shell.entryIndexForUri(Files.homeUri));
            compare(shell.browser.history.length, 1);
        }

        // -- View switch + per-location persistence -----------------------------

        function test_view_switch_persists_per_location() {
            var shell = make();
            compare(shell.viewControl.entries[0].icon, "icon-view");
            compare(shell.viewControl.entries[1].icon, "list-view");
            compare(shell.browser.currentView, "icon");
            compare(shell.viewControl.currentIndex, 0);

            shell.viewControl.activateIndex(1);
            compare(shell.browser.currentView, "list");
            waitForRendering(stage);
            compare(shell.viewControl.currentIndex, 1);

            // Another location starts on the default view...
            var documents = Files.favorites[1].uri;
            shell.browser.navigate(documents);
            waitForRendering(stage);
            compare(shell.browser.currentView, "icon");
            compare(shell.viewControl.currentIndex, 0);

            // ...and the first location remembers its choice.
            shell.browser.back();
            waitForRendering(stage);
            compare(shell.browser.currentUri, Files.homeUri);
            compare(shell.browser.currentView, "list");
            compare(shell.viewControl.currentIndex, 1);
        }

        // -- Views over the files-core listing (T-10.4b) -------------------------

        function test_icon_view_renders_the_listing() {
            var shell = make();
            verify(Files.viewFixtureUri.length > 0);
            shell.browser.navigate(Files.viewFixtureUri);
            tryCompare(shell.directory, "state", "complete");
            compare(shell.directory.count, 5);
            compare(shell.browser.currentView, "icon");
            compare(shell.iconView.visible, true);
            compare(shell.listView.visible, false);
        }

        function test_view_toggle_swaps_the_delegate() {
            var shell = make();
            shell.browser.navigate(Files.viewFixtureUri);
            tryCompare(shell.directory, "count", 5);
            compare(shell.iconView.visible, true);

            shell.viewControl.activateIndex(1);
            waitForRendering(stage);
            compare(shell.browser.currentView, "list");
            compare(shell.listView.visible, true);
            compare(shell.iconView.visible, false);

            shell.viewControl.activateIndex(0);
            waitForRendering(stage);
            compare(shell.browser.currentView, "icon");
            compare(shell.iconView.visible, true);
            compare(shell.listView.visible, false);
        }

        function test_selection_survives_the_view_switch() {
            var shell = make();
            shell.browser.navigate(Files.viewFixtureUri);
            tryCompare(shell.directory, "count", 5);
            waitForRendering(stage);

            // Click the first tile; the views only report the id and the shell
            // remembers it.
            mouseClick(shell.iconView, 60, 60);
            waitForRendering(stage);
            verify(shell.selectedId > 0);
            var chosen = shell.selectedId;
            compare(shell.iconView.selectedId, chosen);

            shell.viewControl.activateIndex(1);
            waitForRendering(stage);
            compare(shell.selectedId, chosen);
            compare(shell.listView.selectedId, chosen);
            compare(shell.iconView.selectedId, chosen);
        }

        function test_list_view_shows_the_columns() {
            var shell = make();
            shell.browser.navigate(Files.viewFixtureUri);
            tryCompare(shell.directory, "count", 5);
            // The rows carry the list-view columns (name + metadata).
            var index = shell.directory.index(0, 0);
            verify(shell.directory.data(index, 257).toString().length > 0); // nodeId
            verify(shell.directory.data(index, 258).toString().length > 0); // name
        }

        // -- Multi-select and context menus (T-10.4c) ---------------------------

        function test_multi_select_command_and_shift() {
            var shell = make();
            shell.browser.navigate(Files.viewFixtureUri);
            tryCompare(shell.directory, "state", "complete");
            var ids = shell.directory.allNodeIds();
            compare(ids.length, 5);

            shell.selectNode(ids[0], 0);
            compare(shell.selectedIds.length, 1);

            shell.selectNode(ids[2], Qt.ControlModifier);
            compare(shell.selectedIds.length, 2);

            shell.selectNode(ids[2], Qt.ControlModifier);
            compare(shell.selectedIds.length, 1);

            // Shift-click ranges from the anchor (the last plain click).
            shell.selectNode(ids[3], Qt.ShiftModifier);
            compare(shell.selectedIds.length, 2); // rows 0..3? anchor re-set
            verify(shell.selectedIds.indexOf(ids[3]) >= 0);

            shell.selectAll();
            compare(shell.selectedIds.length, 5);
        }

        function test_context_menu_targets_the_pointer() {
            var shell = make();
            shell.browser.navigate(Files.viewFixtureUri);
            tryCompare(shell.directory, "state", "complete");
            var ids = shell.directory.allNodeIds();

            shell.selectNode(ids[0], 0);
            shell.openNodeMenu(ids[0], "file:///fixture/alpha.txt", false, 12, 12);
            verify(shell.contextMenu.open);
            var actions = shell.contextMenu.model.map(function(e) { return e.action; });
            verify(actions.indexOf("rename") >= 0);
            verify(actions.indexOf("trash") >= 0);

            // Multiple selected: a shorter menu.
            shell.selectNode(ids[1], Qt.ControlModifier);
            shell.openNodeMenu(ids[0], "file:///fixture/alpha.txt", false, 12, 12);
            compare(shell.contextMenu.model.length, 1);
            compare(shell.contextMenu.model[0].action, "trash");
            shell.contextMenu.hide();

            shell.openBackgroundMenu(12, 12);
            compare(shell.contextMenu.model[0].action, "newFolder");
            compare(shell.contextMenu.model[shell.contextMenu.model.length - 1].action,
                    "selectAll");
            shell.contextMenu.hide();
        }

        function test_right_click_opens_the_item_menu() {
            var shell = make();
            shell.browser.navigate(Files.viewFixtureUri);
            tryCompare(shell.directory, "state", "complete");
            waitForRendering(stage);
            // A real pointer right-click on a tile routes through the view's
            // delegate to the shell's one context menu.
            mouseClick(shell.iconView, 60, 60, Qt.RightButton);
            tryCompare(shell.contextMenu, "open", true);
            verify(shell.contextMenu.model.length >= 3);
            shell.contextMenu.hide();
        }

        function test_capture_seams_drive_the_menus_and_rename() {
            var shell = make();
            shell.browser.navigate(Files.viewFixtureUri);
            tryCompare(shell.directory, "state", "complete");

            shell.startMenu = "item";
            shell.captureApplied = false;
            shell.applyCaptureSeam();
            compare(shell.contextMenu.open, true);
            verify(shell.contextMenu.width > 0);
            verify(shell.contextMenu.height > 0);
            tryCompare(shell.contextMenu, "visible", true);
            shell.contextMenu.hide();

            shell.startMenu = "background";
            shell.captureApplied = false;
            shell.applyCaptureSeam();
            compare(shell.contextMenu.open, true);
            shell.contextMenu.hide();

            shell.startMenu = "";
            shell.startRename = true;
            shell.captureApplied = false;
            shell.applyCaptureSeam();
            verify(shell.renamingId > 0);
            shell.cancelRename();
        }

        // -- Optimistic operations (T-10.4c) ------------------------------------

        function test_optimistic_rename_paints_immediately() {
            var shell = make();
            shell.browser.navigate(Files.mutationFixtureUri + "/Rename");
            tryCompare(shell.directory, "state", "complete");
            tryCompare(shell.directory, "count", 1);
            var id = shell.directory.allNodeIds()[0];

            verify(shell.directory.rename(id, "renamed.txt"));
            // No wait: the row already carries the new name.
            var row = shell.directory.rowForNodeId(id);
            verify(row >= 0);
            compare(shell.directory.data(shell.directory.index(row, 0), 258).toString(),
                    "renamed.txt");
            verify(shell.directory.pendingOps > 0);

            // The worker confirms and no error is recorded.
            tryCompare(shell.directory, "pendingOps", 0);
            compare(shell.directory.lastError, "");
        }

        function test_optimistic_rename_reverts_on_conflict() {
            var shell = make();
            shell.browser.navigate(Files.mutationFixtureUri + "/Revert");
            tryCompare(shell.directory, "state", "complete");
            tryCompare(shell.directory, "count", 2);
            var ids = shell.directory.allNodeIds(); // a.txt, b.txt
            var a = ids[0];
            var row = shell.directory.rowForNodeId(a);
            compare(shell.directory.data(shell.directory.index(row, 0), 258).toString(),
                    "a.txt");

            verify(shell.directory.rename(a, "b.txt"));
            // Optimistically painted as the colliding name...
            row = shell.directory.rowForNodeId(a);
            compare(shell.directory.data(shell.directory.index(row, 0), 258).toString(),
                    "b.txt");

            // ...then snapped back when the real rename reported AlreadyExists.
            tryCompare(shell.directory, "pendingOps", 0);
            verify(shell.directory.lastError.length > 0);
            row = shell.directory.rowForNodeId(a);
            compare(shell.directory.data(shell.directory.index(row, 0), 258).toString(),
                    "a.txt");
        }

        function test_optimistic_new_folder_paints_immediately() {
            var shell = make();
            shell.browser.navigate(Files.mutationFixtureUri + "/New");
            tryCompare(shell.directory, "state", "complete");
            compare(shell.directory.count, 0);

            verify(shell.directory.newFolder(shell.browser.currentUri));
            compare(shell.directory.count, 1); // visible before confirmation
            tryCompare(shell.directory, "pendingOps", 0);
            compare(shell.directory.lastError, "");
        }

        function test_optimistic_trash_paints_immediately() {
            var shell = make();
            shell.browser.navigate(Files.mutationFixtureUri + "/Trash");
            tryCompare(shell.directory, "state", "complete");
            tryCompare(shell.directory, "count", 1);
            var id = shell.directory.allNodeIds()[0];

            verify(shell.directory.trash(id));
            compare(shell.directory.count, 0); // gone within the same call
            tryCompare(shell.directory, "pendingOps", 0);
            compare(shell.directory.lastError, "");
        }

        // -- Trash location and Empty Trash (T-10.6b) ---------------------------

        function test_trash_location_lists_and_empty_clears_the_model() {
            var shell = make();

            // Put two real files into the (temp) home Trash through files-core.
            shell.browser.navigate(Files.mutationFixtureUri + "/Empty");
            tryCompare(shell.directory, "state", "complete");
            tryCompare(shell.directory, "count", 2);
            var ids = shell.directory.allNodeIds();
            verify(shell.directory.trash(ids[0]));
            verify(shell.directory.trash(ids[1]));
            tryCompare(shell.directory, "pendingOps", 0);
            compare(shell.directory.lastError, "");

            // Clicking Trash opens the same store Files lists (trash://).
            shell.browser.navigate(Files.trashUri);
            tryCompare(shell.directory, "state", "complete");
            verify(shell.directory.count >= 2);
            compare(shell.locationTitle, "Trash");
            compare(shell.pathBar.model[0].label, "Trash");

            // The Trash background menu owns Empty Trash, confirmation first.
            shell.openBackgroundMenu(20, 20);
            var labels = shell.contextMenu.model.map(function(e) { return e.label; });
            verify(labels.indexOf("Empty Trash") >= 0);
            shell.contextMenu.hide();

            var before = shell.directory.count;
            verify(before > 0);

            // The live-capture seam opens the same confirmation.
            shell.startEmptyTrash = true;
            shell.captureApplied = false;
            shell.applyCaptureSeam();
            compare(shell.emptyTrashDialog.open, true);
            shell.emptyTrashDialog.reject();

            shell.confirmEmptyTrash();
            compare(shell.emptyTrashDialog.open, true);
            shell.emptyTrashDialog.accept();
            compare(shell.directory.count, 0); // cleared within the same call

            tryCompare(shell.directory, "pendingOps", 0);
            compare(shell.directory.lastError, "");
            compare(shell.directory.count, 0);
        }

        // -- Performance (T-10.5) -----------------------------------------------

        // The 100k listing is served from the synthetic source behind the same
        // bridge (`DF_FILES_SYNTHETIC_COUNT`, set by the runner), so this is
        // the real windowed-rendering path with no 100k inodes on disk.
        function test_large_list_paints_fast_and_stays_windowed() {
            var shell = make();
            var start = Date.now();
            shell.browser.navigate("file:///synthetic");
            while (shell.directory.count === 0 && Date.now() - start < 5000)
                wait(1);
            var firstFrame = Date.now() - start;
            verify(firstFrame < 50,
                   "first frame " + firstFrame + " ms (budget < 50 ms)");
            verify(shell.directory.count > 0);

            tryCompare(shell.directory, "state", "complete", 60000);
            compare(shell.directory.count, 100000);

            // The icon view is active by default. Only the visible window of
            // delegates is instantiated, so memory stays flat with the listing.
            waitForRendering(stage);
            var grid = shell.iconView.gridView;
            verify(grid !== null);
            verify(grid.contentItem !== null);
            var delegates = grid.contentItem.children.length;
            console.log("T-10.5 qt: first_frame=" + firstFrame
                        + " ms, rows=" + shell.directory.count
                        + ", icon_delegates=" + delegates);
            verify(delegates > 0 && delegates < 600,
                   "icon delegates " + delegates + " for " + shell.directory.count
                   + " rows");

            // Scroll to the far end and back; the window does not grow.
            grid.positionViewAtIndex(99999, GridView.End);
            waitForRendering(stage);
            delegates = grid.contentItem.children.length;
            verify(delegates < 600, "after scrolling to the end: " + delegates);
            grid.positionViewAtBeginning();
            waitForRendering(stage);
            delegates = grid.contentItem.children.length;
            verify(delegates < 600, "after scrolling home: " + delegates);
        }

        function test_large_list_scroll_sweep() {
            var shell = make();
            shell.browser.navigate("file:///synthetic");
            tryCompare(shell.directory, "state", "complete", 60000);
            compare(shell.directory.count, 100000);

            shell.viewControl.activateIndex(1); // list view
            waitForRendering(stage);
            var list = shell.listView.listView;
            verify(list !== null);

            var start = Date.now();
            for (var step = 0; step < 100; ++step) {
                list.positionViewAtIndex(step * 997, ListView.Beginning);
                waitForRendering(stage);
            }
            var elapsed = Date.now() - start;
            var delegates = list.contentItem.children.length;
            console.log("T-10.5 qt scroll: 100 list jumps=" + elapsed
                        + " ms, list_delegates=" + delegates);
            verify(delegates < 200, "list delegates " + delegates);
            verify(elapsed < 10000, "100 scroll jumps took " + elapsed + " ms");
        }

        // -- Search -------------------------------------------------------------

        function test_search_field_is_wired() {
            var shell = make();
            verify(!shell.hasSearch);
            shell.searchField.text = "report";
            compare(shell.searchText, "report");
            verify(shell.hasSearch);
            shell.searchField.clear();
            compare(shell.searchText, "");
        }

        // -- Path bar navigates -------------------------------------------------

        function test_path_bar_segment_navigates() {
            var shell = make();
            shell.browser.navigate(Files.favorites[1].uri); // Documents
            waitForRendering(stage);
            // Computer, tester, Documents.
            compare(shell.pathBar.model.length, 3);
            shell.pathBar.navigated(Files.computerUri);
            compare(shell.browser.currentUri, Files.computerUri);
            compare(shell.locationTitle, "Computer");
        }

        // -- Accessibility ------------------------------------------------------

        function test_accessibility_roles_are_present() {
            var shell = make();
            compare(shell.appWindow.Accessible.role, Accessible.Window);
            compare(shell.titleBar.Accessible.role, Accessible.TitleBar);
            compare(shell.searchField.Accessible.role, Accessible.EditableText);
            compare(shell.searchField.Accessible.searchEdit, true);
            compare(shell.sidebar.Accessible.role, Accessible.List);
            compare(shell.backButton.Accessible.role, Accessible.Button);
            compare(shell.backButton.Accessible.name, "Back");
            compare(shell.pathBar.Accessible.role, Accessible.Pane);
        }

        // -- Window-control intent forwarding -----------------------------------

        function test_traffic_lights_forward_close() {
            var shell = make();
            closeSpy.target = shell;
            closeSpy.clear();
            var d = Theme.controls.trafficLights.diameter;
            var lights = shell.titleBar.trafficLights;
            mouseClick(lights, d / 2, d / 2);
            compare(closeSpy.count, 1);
        }
    }
}