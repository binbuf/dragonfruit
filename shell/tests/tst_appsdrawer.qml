// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.AppsDrawer

// Applications drawer view tests (T-19.2): the tile grid, the category and
// query filters, the launch signal (with the acted-on desktop id), and the
// explicit app-index-absent state. Runs headless on the offscreen platform +
// software scene graph; the pure list helper is covered by tst_dockcore.
Item {
    id: stage
    width: 1280
    height: 720

    TestCase {
        id: testCase
        name: "AppsDrawer"
        when: windowShown

        Component { id: drawerComponent; AppsDrawer { } }
        SignalSpy { id: launchedSpy; signalName: "appLaunched" }
        SignalSpy { id: dismissedSpy; signalName: "dismissRequested" }

        readonly property var sampleApps: [
            { desktopId: "org.example.Calendar.desktop", name: "Calendar",
              iconPath: "", categories: ["productivity"] },
            { desktopId: "org.example.Browser.desktop", name: "Browser",
              iconPath: "", categories: ["social"] },
            { desktopId: "org.example.Calculator.desktop", name: "Calculator",
              iconPath: "", categories: ["utilities"] },
            { desktopId: "org.example.Editor.desktop", name: "Editor",
              iconPath: "", categories: ["developer-tools", "utilities"] }
        ]

        function make(props) {
            var merged = { width: stage.width, height: stage.height, active: true };
            for (var key in (props || {}))
                merged[key] = props[key];
            var drawer = createTemporaryObject(drawerComponent, stage, merged);
            launchedSpy.target = drawer;
            dismissedSpy.target = drawer;
            launchedSpy.clear();
            dismissedSpy.clear();
            waitForRendering(stage);
            return drawer;
        }

        function tiles(drawer) {
            var flow = findChild(drawer, "appsDrawerTiles");
            var out = [];
            for (var i = 0; i < flow.children.length; ++i) {
                if (flow.children[i].objectName === "appsDrawerTile")
                    out.push(flow.children[i]);
            }
            return out;
        }

        function labels(drawer) {
            var out = [];
            var list = tiles(drawer);
            for (var i = 0; i < list.length; ++i)
                out.push(findChild(list[i], "appsDrawerTileLabel").text);
            return out;
        }

        function test_grid_renders_one_tile_per_app_with_labels() {
            var drawer = make({ apps: sampleApps });
            var list = tiles(drawer);
            compare(list.length, 4);
            var names = labels(drawer);
            verify(names.indexOf("Calendar") >= 0);
            verify(names.indexOf("Browser") >= 0);
            verify(names.indexOf("Calculator") >= 0);
            verify(names.indexOf("Editor") >= 0);
            // The themed icon is absent here, so the Phosphor fallback shows.
            verify(findChild(list[0], "appsDrawerTileFallback").visible);
        }

        function test_category_filter_narrows_the_grid() {
            var drawer = make({ apps: sampleApps });
            compare(tiles(drawer).length, 4);
            drawer.category = "utilities";
            waitForRendering(stage);
            compare(tiles(drawer).length, 2);
            var names = labels(drawer);
            verify(names.indexOf("Calculator") >= 0);
            verify(names.indexOf("Editor") >= 0);
            // Every pill is present plus All.
            verify(drawer.pills.length >= 2);
        }

        function test_query_filter_is_case_insensitive_over_name_and_id() {
            var drawer = make({ apps: sampleApps });
            drawer.setQueryText("calc");
            waitForRendering(stage);
            compare(tiles(drawer).length, 1);
            compare(labels(drawer)[0], "Calculator");
            drawer.setQueryText("ORG.EXAMPLE.BROWSER");
            waitForRendering(stage);
            compare(tiles(drawer).length, 1);
            compare(labels(drawer)[0], "Browser");
            drawer.setQueryText("nothing-matches");
            waitForRendering(stage);
            compare(tiles(drawer).length, 0);
            verify(findChild(drawer, "appsDrawerEmpty").visible);
        }

        function test_activating_a_tile_fires_the_launch_signal_with_the_id() {
            var drawer = make({ apps: sampleApps });
            var list = tiles(drawer);
            // "Calendar" is the first tile of the sample order.
            var tile = list[0];
            mouseClick(tile, tile.width / 2, tile.height / 2);
            waitForRendering(stage);
            compare(launchedSpy.count, 1);
            compare(launchedSpy.signalArguments[0][0],
                    "org.example.Calendar.desktop");
        }

        function test_keyboard_launches_the_selected_tile() {
            var drawer = make({ apps: sampleApps });
            drawer.currentIndex = 1;
            drawer.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Return);
            waitForRendering(stage);
            compare(launchedSpy.count, 1);
            compare(launchedSpy.signalArguments[0][0], "org.example.Browser.desktop");
        }

        function test_escape_dismisses() {
            var drawer = make({ apps: sampleApps });
            drawer.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            compare(dismissedSpy.count, 1);
        }

        function test_absent_app_index_shows_the_explicit_row() {
            var drawer = make({ apps: [], available: false });
            verify(findChild(drawer, "appsDrawerUnavailable").visible);
            compare(findChild(drawer, "appsDrawerUnavailableText").text,
                    "Application index unavailable");
            compare(tiles(drawer).length, 0);
            verify(!findChild(drawer, "appsDrawerEmpty").visible);
        }

        function test_extras_outside_the_canonical_keys_are_ignored_by_pills() {
            var drawer = make({ apps: [
                { desktopId: "x.desktop", name: "X", iconPath: "", categories: ["unknown-key"] }
            ] });
            // Only the implicit All pill, since no mapped category is present.
            compare(drawer.pills.length, 1);
            compare(drawer.pills[0].key, "all");
        }

        // The grid is capped at seven columns (the reference width); a narrow
        // output wraps to fewer instead of overflowing.
        function test_grid_is_capped_at_seven_columns() {
            var wide = make({ apps: [], width: 1920 });
            compare(wide.columns, 7);
            compare(wide.gridWidth, wide.gridMaxWidth);

            var narrow = make({ apps: [], width: 700 });
            verify(narrow.columns < 7);
            verify(narrow.columns >= 1);
        }

        // The Flow really lays out seven tiles per row at the cap, not just
        // reports seven.
        function test_grid_wraps_at_seven_tiles_per_row() {
            var many = [];
            for (var i = 0; i < 20; ++i)
                many.push({ desktopId: "a" + i + ".desktop", name: "App " + i,
                            iconPath: "", categories: [] });
            var drawer = make({ apps: many, width: 1920 });
            var list = tiles(drawer);
            compare(list.length, 20);
            var firstRow = 0;
            var firstY = list[0].y;
            for (var j = 0; j < list.length; ++j) {
                if (list[j].y === firstY)
                    ++firstRow;
            }
            compare(firstRow, 7);
        }

        // The card wraps the grid with at least `panelPadding` on both ends and
        // equal left/right padding.
        function test_panel_gives_the_grid_equal_padding() {
            var seven = [];
            for (var i = 0; i < 7; ++i)
                seven.push({ desktopId: "p" + i + ".desktop", name: "P " + i,
                             iconPath: "", categories: [] });
            var drawer = make({ apps: seven });
            var panel = findChild(drawer, "appsDrawerPanel");
            var grid = findChild(drawer, "appsDrawerGrid");
            verify(panel !== null);
            verify(grid !== null);
            var leftPad = grid.x;
            var rightPad = panel.width - (grid.x + grid.width);
            compare(Math.abs(leftPad - rightPad) <= 1, true);
            verify(leftPad >= drawer.panelPadding,
                   "left padding at least panelPadding, got " + leftPad);
            verify(rightPad >= drawer.panelPadding,
                   "right padding at least panelPadding, got " + rightPad);
            // The card is the grid plus the padding on each side.
            verify(panel.width >= grid.width + 2 * drawer.panelPadding);
            // And the grid keeps the vertical padding below it.
            var bottomPad = panel.height - (grid.y + grid.height);
            verify(bottomPad >= drawer.panelPaddingV,
                   "bottom padding at least panelPaddingV, got " + bottomPad);
        }

        // The grid viewport shows at most five rows; a longer corpus scrolls,
        // and a short corpus hugs its content instead of reserving five rows.
        function test_grid_viewport_is_at_most_five_rows() {
            var many = [];
            for (var i = 0; i < 50; ++i)
                many.push({ desktopId: "r" + i + ".desktop", name: "Row " + i,
                            iconPath: "", categories: [] });
            // A tall card fits the full five rows; a short output clamps below it.
            var roomy = make({ apps: many, height: 1400 });
            var grid = findChild(roomy, "appsDrawerGrid");
            compare(Math.round(grid.height), Math.round(roomy.gridMaxHeight),
                    "roomy viewport " + grid.height + " vs max " + roomy.gridMaxHeight
                    + " (content " + grid.contentHeight + ")");
            verify(grid.contentHeight > grid.height);

            var shortDock = make({ apps: many, height: 720 });
            var shortGrid = findChild(shortDock, "appsDrawerGrid");
            verify(shortGrid.height < shortDock.gridMaxHeight);
            verify(shortGrid.contentHeight > shortGrid.height);

            var small = make({ apps: sampleApps, height: 1400 });
            var smallGrid = findChild(small, "appsDrawerGrid");
            compare(Math.round(smallGrid.height), Math.round(smallGrid.contentHeight),
                    "small viewport " + smallGrid.height + " vs content "
                    + smallGrid.contentHeight);
            verify(smallGrid.height < small.gridMaxHeight);
        }

        // The drawer's top section is a flat search bar (no pill background)
        // over the card, framed by the two hairline separators.
        function test_top_section_is_flat_with_separators() {
            var drawer = make({ apps: sampleApps });
            var search = findChild(drawer, "appsDrawerSearch");
            verify(search !== null);
            compare(search.flat, true);
            compare(search.placeholderText, "Applications");
            verify(findChild(drawer, "appsDrawerTopSeparator") !== null);
            verify(findChild(drawer, "appsDrawerPillsSeparator") !== null);
        }
    }
}