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