// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Settings app-shell tests (T-09.1a): the window opens with the sidebar and
// local search, search filters the pane list, keyboard navigation and history
// work, the window controls forward to the host, and the AT-SPI roles are
// present. Headless on the offscreen platform + software scene graph.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsShell"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        SignalSpy { id: closeSpy; signalName: "closeRequested" }

        function make(props) {
            var merged = { width: stage.width, height: stage.height };
            for (var key in (props || {}))
                merged[key] = props[key];
            var obj = createTemporaryObject(shellComponent, stage, merged);
            waitForRendering(stage);
            return obj;
        }

        function ids(shell) {
            return JSON.stringify(shell.visiblePanes.map(function(pane) {
                return pane.id;
            }));
        }

        // -- Catalog ------------------------------------------------------------

        function test_catalog_is_the_reference_ia() {
            compare(SettingsPanes.catalog[0].id, "wifi");
            compare(SettingsPanes.catalog[SettingsPanes.catalog.length - 1].id,
                    "printers");
            // Only panes whose content has landed are advertised (no half panes).
            compare(SettingsPanes.shippedPanes.length, 4);
        }

        // -- Shell opens --------------------------------------------------------

        function test_shell_opens_with_sidebar_and_search() {
            var shell = make();
            compare(shell.currentPaneId, "appearance");
            compare(shell.visiblePanes.length, 4);
            verify(shell.searchField !== null);
            verify(shell.sidebar !== null);
            verify(shell.titleBar !== null);
            compare(shell.header.pane.id, "appearance");
            // At the start of history both history affordances are dimmed.
            verify(!shell.backButton.enabled);
            verify(!shell.forwardButton.enabled);
        }

        // -- Search -------------------------------------------------------------

        function test_search_filters_panes() {
            var shell = make();
            shell.searchText = "wall";
            compare(ids(shell), JSON.stringify(["wallpaper"]));

            shell.searchText = "dock";
            compare(ids(shell), JSON.stringify(["desktop-dock"]));

            shell.searchText = "DISPLAY";
            compare(ids(shell), JSON.stringify(["displays"]));

            shell.searchText = "no-such-pane";
            compare(ids(shell), JSON.stringify([]));

            shell.searchText = "";
            compare(ids(shell), JSON.stringify(
                ["appearance", "desktop-dock", "displays", "wallpaper"]));
        }

        function test_search_keeps_the_current_pane() {
            var shell = make();
            shell.searchText = "wall";
            compare(shell.currentPaneId, "appearance");
            compare(shell.header.pane.id, "appearance");
        }

        // -- Keyboard navigation + history -------------------------------------

        function test_keyboard_navigates_and_activates() {
            var shell = make();
            shell.sidebar.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Down);
            compare(shell.sidebar.currentIndex, 1);
            keyClick(Qt.Key_Return);
            compare(shell.currentPaneId, "desktop-dock");
            verify(shell.canGoBack);
        }

        function test_history_back_and_forward() {
            var shell = make();
            shell.selectPane("displays");
            shell.selectPane("wallpaper");
            compare(shell.currentPaneId, "wallpaper");
            verify(!shell.canGoForward);

            shell.back();
            compare(shell.currentPaneId, "displays");
            verify(shell.canGoBack);
            verify(shell.canGoForward);

            shell.forward();
            compare(shell.currentPaneId, "wallpaper");
            verify(!shell.canGoForward);

            // Forward is dimmed (disabled) at the end of history.
            verify(!shell.forwardButton.enabled);
            verify(shell.backButton.enabled);
        }

        // -- Selection highlight ------------------------------------------------

        function test_selected_sidebar_row_is_accent_tinted() {
            var shell = make();
            // The row highlight animates in over the hover duration; let it
            // settle before sampling pixels.
            wait(Theme.motion.hover.duration + 100);
            waitForRendering(stage);
            var img = grabImage(shell);
            var accent = Theme.color.accent;
            var count = 0;
            for (var y = 0; y < img.height; y += 2) {
                for (var x = 0; x < 240; x += 2) {
                    var c = img.pixel(x, y);
                    if (Math.abs(c.r - accent.r) < 0.02
                            && Math.abs(c.g - accent.g) < 0.02
                            && Math.abs(c.b - accent.b) < 0.02)
                        count += 1;
                }
            }
            verify(count > 20,
                   "the selected pane row must carry the accent highlight (count="
                   + count + " index=" + shell.sidebar.currentIndex + ")");
        }

        // -- Accessibility ------------------------------------------------------

        function test_accessibility_roles_are_present() {
            var shell = make();
            compare(shell.appWindow.Accessible.role, Accessible.Window);
            compare(shell.titleBar.Accessible.role, Accessible.TitleBar);
            compare(shell.searchField.Accessible.role, Accessible.EditableText);
            compare(shell.searchField.Accessible.searchEdit, true);
            compare(shell.sidebar.Accessible.role, Accessible.List);
            compare(shell.header.Accessible.role, Accessible.Grouping);
            compare(shell.header.Accessible.name, "Appearance");
            compare(shell.backButton.Accessible.role, Accessible.Button);
            compare(shell.backButton.Accessible.name, "Back");
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