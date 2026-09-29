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
    }
}