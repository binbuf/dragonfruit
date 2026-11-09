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
        SignalSpy { id: resizeSpy; signalName: "resizeRequested" }

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
                    "game-controllers");
            // Only panes whose content has landed are advertised (no half panes).
            compare(SettingsPanes.shippedPanes.length, 28);
        }

        // -- Category tiles (T-19.1b) -------------------------------------------

        function test_category_tiles_resolve_and_fall_back() {
            // Every mapping names a vendored fill glyph and carries a gradient.
            var styles = SettingsPanes.categoryStyles;
            var checked = 0;
            for (var icon in styles) {
                var style = styles[icon];
                verify(PhosphorGlyphs.has("fill", style.source),
                       "category " + icon + " -> " + style.source + " must be vendored");
                verify(String(style.gradientStart).length > 0,
                       "category " + icon + " needs a gradientStart");
                verify(String(style.gradientEnd).length > 0,
                       "category " + icon + " needs a gradientEnd");
                checked += 1;
            }
            verify(checked >= 20,
                   "the shipped categories must be mapped (got " + checked + ")");

            // A pane with no Phosphor mapping keeps the Tabler-backed Icon fallback.
            compare(SettingsPanes.categoryStyle(SettingsPanes.paneById("trackpad")), null);
            verify(SettingsPanes.categoryStyle(SettingsPanes.paneById("general")) !== null);
            verify(SettingsPanes.categoryStyle(null) === null);
        }

        function test_sidebar_rows_and_header_adopt_the_category_tile() {
            var shell = make();
            // The pane list is driven by SettingsPanes.filter; each item carries
            // the resolved category style (or null) for the sidebar delegate.
            var visible = shell.visiblePanes;
            verify(SettingsPanes.categoryStyle(visible[0]) !== null,
                   "Bluetooth must carry a category tile");
            // The Appearance pane has no header card but its icon is mapped, so
            // the header (when a header pane is current) resolves the tile too.
            shell.selectPane("general");
            waitForRendering(stage);
            verify(shell.header.visible);
            verify(shell.header.category !== null,
                   "the General header must resolve a category tile");
            compare(shell.header.category.source, "gear");
        }

        function test_header_card_is_only_where_the_reference_has_one() {
            // The Wave-1 captures (Desktop & Dock, Displays, Wallpaper) have
            // no header card; Appearance is uncaptured but follows the same
            // settings-list shape. Only these panes have one.
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("appearance")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("desktop-dock")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("displays")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("wallpaper")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("general")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("accessibility")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("notifications")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("privacy")), true);

            var shell = make();
            verify(!shell.header.visible,
                   "the Appearance pane must not draw a header card");
        }

        // -- Shell opens --------------------------------------------------------

        function test_shell_opens_with_sidebar_and_search() {
            var shell = make();
            compare(shell.currentPaneId, "appearance");
            compare(shell.visiblePanes.length, 28);
            verify(shell.searchField !== null);
            verify(shell.sidebar !== null);
            verify(shell.titleBar !== null);
            compare(shell.titleBar.style, "unified",
                    "Settings must declare the unified window style");
            compare(shell.titleBar.height, WindowStyle.height("unified"),
                    "the titlebar height must come from the style table");
            compare(shell.titleBar.trafficLights.width,
                    WindowStyle.trafficLightSize * 3
                    + WindowStyle.trafficLightGap() * 2);
            compare(shell.titleBar.width, shell.width,
                    "the titlebar must span the window so the whole bar drags");
            compare(shell.sidebarPane.width, Theme.controls.sidebar.width,
                    "the sidebar uses the design-system width token");
            compare(shell.header.pane.id, "appearance");
            // At the start of history both history affordances are dimmed.
            verify(!shell.backButton.enabled);
            verify(!shell.forwardButton.enabled);
        }

        // -- T206 app-local chrome ---------------------------------------------

        function test_first_party_chrome_uses_measured_app_material() {
            var savedScheme = Settings.values["appearance.colorScheme"];
            var savedReduce = Settings.values["accessibility.reduceTransparency"];
            var savedContrast = Settings.values["accessibility.increaseContrast"];
            var shell = make();
            try {
                shell.glassRollout = "replacement";
                shell.appChromeSupported = true;
                Settings.set("appearance.colorScheme", "light");
                Settings.set("accessibility.reduceTransparency", false);
                Settings.set("accessibility.increaseContrast", false);
                tryCompare(Theme, "dark", false);
                waitForRendering(stage);

                compare(shell.appWindow.chromeTransparent, true);
                fuzzyCompare(shell.sidebarPane.color.a, 0.986, 1e-4);
                fuzzyCompare(shell.sidebarPane.color.r, 0.857, 1e-4);
                fuzzyCompare(shell.toolbar.fillColor.a, 0.961, 1e-4);
                fuzzyCompare(shell.toolbar.fillColor.r, 0.965, 1e-4);
                fuzzyCompare(shell.detailSheet.color.a, 1.0, 1e-6);
                compare(shell.detailSheet.bottomRightRadius,
                        Theme.controls.window.radius);

                // Dark Settings is the explicit Finder-derived adaptation.
                Settings.set("appearance.colorScheme", "dark");
                tryCompare(Theme, "dark", true);
                waitForRendering(stage);
                fuzzyCompare(shell.sidebarPane.color.a, 0.935, 1e-4);
                fuzzyCompare(shell.sidebarPane.color.r, 0.218, 1e-4);
                fuzzyCompare(shell.toolbar.fillColor.a, 0.900, 1e-4);
                fuzzyCompare(shell.toolbar.fillColor.r, 0.247, 1e-4);

                // Reduce Transparency takes the shared opaque plate.
                Settings.set("accessibility.reduceTransparency", true);
                waitForRendering(stage);
                fuzzyCompare(shell.sidebarPane.color.a, 0.96, 1e-4);
                fuzzyCompare(shell.sidebarPane.color.r, 0.1, 1e-4);

                // A non-composited client keeps the opaque source-list plate.
                shell.appChromeSupported = false;
                waitForRendering(stage);
                fuzzyCompare(shell.sidebarPane.color.a, 1.0, 1e-6);
                fuzzyCompare(shell.sidebarPane.color.r,
                             Theme.color.surfaceMuted.r, 1e-6);
                fuzzyCompare(shell.toolbar.fillColor.a, 1.0, 1e-6);
                fuzzyCompare(shell.toolbar.fillColor.r, Theme.color.chrome.r, 1e-6);

                // Focus is a controls/text state, not a material change.
                shell.windowActive = false;
                compare(shell.titleBar.active, false);
                shell.appChromeSupported = true;
                Settings.set("accessibility.reduceTransparency", false);
                waitForRendering(stage);
                fuzzyCompare(shell.sidebarPane.color.a, 0.935, 1e-4);
            } finally {
                Settings.set("appearance.colorScheme", savedScheme);
                Settings.set("accessibility.reduceTransparency", savedReduce);
                Settings.set("accessibility.increaseContrast", savedContrast);
                shell.glassRollout = "legacy";
                shell.appChromeSupported = true;
            }
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
                ["wifi", "bluetooth", "network", "battery", "general", "storage",
                 "appearance", "accessibility", "control-center", "menu-bar",
                 "search", "privacy", "desktop-dock", "mission-control",
                 "displays", "wallpaper", "screen-saver", "notifications",
                 "sound", "focus", "lock-screen", "biometrics", "users-groups", "printers",
                 "keyboard", "mouse", "trackpad", "game-controllers"]));
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
            // The default pane is Appearance, the seventh shipped row (Wi-Fi,
            // Bluetooth, Network, Battery, General, and Storage come first in
            // the map catalog order).
            compare(shell.sidebar.currentIndex, 6);
            keyClick(Qt.Key_Down);
            compare(shell.sidebar.currentIndex, 7);
            keyClick(Qt.Key_Return);
            compare(shell.currentPaneId, "accessibility");
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

        // -- Window-edge resize intent ------------------------------------------

        function test_edges_and_corners_report_resize_intent() {
            var shell = make();
            resizeSpy.target = shell;
            resizeSpy.clear();
            verify(shell.resizeEdges.thickness > 0);
            verify(shell.resizeEdges.corner > 0);

            // All four edges report their matching Qt.Edges flag.
            mouseClick(shell, shell.width / 2, 1);
            compare(resizeSpy.count, 1);
            compare(resizeSpy.signalArguments[0][0], Qt.TopEdge);

            mouseClick(shell, shell.width / 2, shell.height - 1);
            compare(resizeSpy.count, 2);
            compare(resizeSpy.signalArguments[1][0], Qt.BottomEdge);

            mouseClick(shell, 1, shell.height / 2);
            compare(resizeSpy.count, 3);
            compare(resizeSpy.signalArguments[2][0], Qt.LeftEdge);

            mouseClick(shell, shell.width - 1, shell.height / 2);
            compare(resizeSpy.count, 4);
            compare(resizeSpy.signalArguments[3][0], Qt.RightEdge);

            // A corner resizes both axes at once.
            mouseClick(shell, shell.width - 1, shell.height - 1);
            compare(resizeSpy.count, 5);
            compare(resizeSpy.signalArguments[4][0],
                    Qt.BottomEdge | Qt.RightEdge);

            // A press in the middle of the content is not a resize.
            mouseClick(shell, shell.width / 2, shell.height / 2);
            compare(resizeSpy.count, 5);
        }
    }
}