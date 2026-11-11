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
            // The themed icon is absent here, so the Tabler fallback shows.
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

        function test_outside_press_dismisses_for_every_mouse_button() {
            var drawer = make({ apps: sampleApps });
            for (var button of [Qt.LeftButton, Qt.RightButton, Qt.MiddleButton]) {
                dismissedSpy.clear();
                mousePress(drawer, 2, 2, button);
                compare(dismissedSpy.count, 1);
                mouseRelease(drawer, 2, 2, button);
            }
            dismissedSpy.clear();
            var panel = findChild(drawer, "appsDrawerPanel");
            mouseClick(panel, 2, panel.height / 2);
            compare(dismissedSpy.count, 0, "empty card space is not outside");
        }

        function test_wheel_scrolls_grid_and_stays_open() {
            var apps = [];
            for (var i = 0; i < 100; ++i)
                apps.push({ desktopId: "app" + i, name: "App " + i, categories: [] });
            var drawer = make({ apps: apps });
            var grid = findChild(drawer, "appsDrawerGrid");
            verify(grid.contentHeight > grid.height);
            mouseWheel(grid, grid.width / 2, grid.height / 2, 0, -120);
            verify(grid.contentY > 0);
            compare(dismissedSpy.count, 0);
            mouseWheel(grid, grid.width / 2, grid.height / 2, 0, 120);
            compare(grid.contentY, 0);
        }

        // A wheel over the search field, the chip row, or the card's side
        // gutter must still scroll the grid. The reported bug was that
        // vertical scrolling only worked when the pointer was exactly over
        // the narrow grid band.
        function test_wheel_scrolls_from_the_card_chrome() {
            var apps = [];
            for (var i = 0; i < 100; ++i)
                apps.push({ desktopId: "app" + i, name: "App " + i,
                            iconPath: "", categories: [] });
            var drawer = make({ apps: apps });
            var grid = findChild(drawer, "appsDrawerGrid");
            var panel = findChild(drawer, "appsDrawerPanel");
            var search = findChild(drawer, "appsDrawerSearch");
            verify(grid.contentHeight > grid.height);

            mouseWheel(search, search.width / 2, search.height / 2, 0, -120);
            verify(grid.contentY > 0, "wheel over the search field did not scroll");

            grid.contentY = 0;
            mouseWheel(panel, panel.width - 4, panel.height / 2, 0, -120);
            verify(grid.contentY > 0, "wheel over the card gutter did not scroll");

            grid.contentY = 0;
            var pills = findChild(drawer, "appsDrawerPills");
            mouseWheel(pills, pills.width / 2, pills.height / 2, 0, -120);
            verify(grid.contentY > 0, "wheel over the chip row did not scroll");
        }

        // The More button opens the measured options menu and the Back button
        // is a real dismiss affordance.
        function test_more_menu_opens_and_back_dismisses() {
            var drawer = make({ apps: sampleApps });
            var more = findChild(drawer, "appsDrawerMoreButton");
            var back = findChild(drawer, "appsDrawerBackButton");
            verify(more !== null);
            verify(back !== null);

            mouseClick(more, more.width / 2, more.height / 2);
            waitForRendering(stage);
            var menu = findChild(drawer, "appsDrawerMoreMenu");
            verify(menu.open);
            menu.hide();
            waitForRendering(stage);

            dismissedSpy.clear();
            mouseClick(back, back.width / 2, back.height / 2);
            compare(dismissedSpy.count, 1);
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

        // The card wraps the measured grid with equal left/right padding.
        // The Flickable spans the card so a wheel anywhere over the column
        // band scrolls; the Grid itself keeps the measured width and is
        // centred inside it.
        function test_panel_gives_the_grid_equal_padding() {
            var seven = [];
            for (var i = 0; i < 7; ++i)
                seven.push({ desktopId: "p" + i + ".desktop", name: "P " + i,
                             iconPath: "", categories: [] });
            var drawer = make({ apps: seven });
            var panel = findChild(drawer, "appsDrawerPanel");
            var flick = findChild(drawer, "appsDrawerGrid");
            var grid = findChild(drawer, "appsDrawerTiles");
            verify(panel !== null);
            verify(flick !== null);
            verify(grid !== null);
            var origin = grid.mapToItem(panel, 0, 0);
            var leftPad = origin.x;
            var rightPad = panel.width - (origin.x + grid.width);
            compare(Math.abs(leftPad - rightPad) <= 1, true);
            verify(leftPad >= drawer.panelPadding,
                   "left padding at least panelPadding, got " + leftPad);
            verify(rightPad >= drawer.panelPadding,
                   "right padding at least panelPadding, got " + rightPad);
            // The card is the grid plus the padding on each side.
            verify(panel.width >= grid.width + 2 * drawer.panelPadding);
            // And the grid keeps the vertical padding below it.
            var bottomPad = panel.height - (flick.y + flick.height);
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

            var shortDock = make({ apps: many, height: 640 });
            var shortGrid = findChild(shortDock, "appsDrawerGrid");
            verify(shortGrid.height < shortDock.gridMaxHeight);
            verify(shortGrid.contentHeight > shortGrid.height);

            var small = make({ apps: sampleApps, height: 1400 });
            var smallGrid = findChild(small, "appsDrawerGrid");
            compare(Math.round(smallGrid.height), Math.round(smallGrid.contentHeight),
                    "small viewport " + smallGrid.height + " vs content "
                    + smallGrid.contentHeight);
            verify(smallGrid.height < small.gridMaxHeight);

            // The slim scroll affordance appears only when there is more than
            // one viewport of rows.
            verify(findChild(roomy, "appsDrawerScrollTrack").visible);
            verify(!findChild(small, "appsDrawerScrollTrack").visible);
        }

        // The drawer's top section follows the measured Spotlight Apps row:
        // a Back button, a large rounded search field, and a More button.
        function test_top_section_is_the_measured_search_row() {
            var drawer = make({ apps: sampleApps });
            var search = findChild(drawer, "appsDrawerSearch");
            verify(search !== null);
            compare(search.flat, false);
            compare(search.placeholderText, "Applications");
            compare(search.height, drawer.searchHeight);
            verify(findChild(drawer, "appsDrawerBackButton") !== null);
            verify(findChild(drawer, "appsDrawerMoreButton") !== null);
            // The old hairline rules are gone; the field is a real rounded
            // control on the glass card.
            compare(findChild(drawer, "appsDrawerTopSeparator"), null);
            compare(findChild(drawer, "appsDrawerPillsSeparator"), null);
            // The field carries the measured larger type and the glass sheen.
            compare(search.fontSize,
                    Theme.controls.appsDrawer.searchFontSize);
            compare(search.fieldHighlight,
                    Theme.controls.appsDrawer.searchHighlightOpacity);
            // The card's specular top edge is part of the glass treatment.
            verify(findChild(drawer, "appsDrawerTopSpecular") !== null);
        }

        // The category row is a set of translucent capsules: the selected
        // chip is the light plate (white in light, the elevated dark surface
        // in dark) with primary ink, never an inverted black/white pill.
        function test_category_chips_use_the_reference_selected_plate() {
            var previousReducedMotion = Theme.reducedMotion;
            var previousDark = Theme.dark;
            Theme.reducedMotion = true;
            Theme.dark = false;
            var drawer = make({ apps: sampleApps });
            var row = findChild(drawer, "appsDrawerCategoryControl");
            verify(row !== null);
            var allChip = null;
            for (var c = 0; c < row.children.length; ++c) {
                var candidate = row.children[c];
                if (candidate.modelData !== undefined
                        && candidate.modelData.key === "all") {
                    allChip = candidate;
                }
            }
            verify(allChip !== null);
            var allFill = findChild(allChip, "appsDrawerChipFill");
            var allLabel = findChild(allChip, "appsDrawerChipLabel");
            compareTranslucentColor(allFill.color,
                                    Qt.rgba(Theme.color.surface.r,
                                            Theme.color.surface.g,
                                            Theme.color.surface.b,
                                            Theme.controls.appsDrawer.chipSelectedOpacity));
            compare(allLabel.color, Theme.color.textPrimary);
            compare(allLabel.font.pixelSize,
                    Theme.controls.appsDrawer.chipFontSize);

            drawer.category = "utilities";
            waitForRendering(stage);
            // The previously selected All chip is now an unselected glass
            // capsule: faint ink fill and secondary label.
            compareTranslucentColor(allFill.color,
                                    Qt.rgba(Theme.color.textPrimary.r,
                                            Theme.color.textPrimary.g,
                                            Theme.color.textPrimary.b,
                                            Theme.controls.appsDrawer.chipFillOpacity));
            compare(allLabel.color, Theme.color.textSecondary);

            Theme.reducedMotion = previousReducedMotion;
            Theme.dark = previousDark;
        }

        // Opening the drawer puts active focus in the search field, so the
        // keys the shell forwards are text input rather than dropped by the
        // root's navigation handler.
        function test_search_field_takes_focus_when_the_drawer_opens() {
            var drawer = make({ apps: sampleApps, active: false });
            drawer.active = true;
            wait(0);
            var search = findChild(drawer, "appsDrawerSearch");
            verify(search !== null);
            verify(search.input.activeFocus, "search input did not take focus");
        }

        // Printable keys inserted by the shell's chrome key bridge reach the
        // search field and narrow the grid (the native "can't type" failure).
        function test_typing_filters_through_the_search_field() {
            var drawer = make({ apps: sampleApps, active: false });
            drawer.active = true;
            wait(0);
            keyClick(Qt.Key_C);
            keyClick(Qt.Key_A);
            keyClick(Qt.Key_L);
            keyClick(Qt.Key_C);
            waitForRendering(stage);
            compare(drawer.query, "calc");
            compare(tiles(drawer).length, 1);
            compare(labels(drawer)[0], "Calculator");
        }

        // Filtering rebuilds the tile model; that must not hand focus to a
        // delegate. The reported native failure was that the first character
        // worked, then Backspace/Delete and the next characters went nowhere
        // (or were appended as control characters by the root fallback).
        function test_filtering_keeps_focus_in_the_search_field() {
            var drawer = make({ apps: sampleApps, active: false });
            drawer.active = true;
            wait(0);
            var search = findChild(drawer, "appsDrawerSearch");
            verify(search.input.activeFocus, "search field lost focus on open");

            keyClick(Qt.Key_C);
            waitForRendering(stage);
            compare(drawer.query, "c");
            verify(search.input.activeFocus,
                   "filtering handed active focus to a tile delegate");

            keyClick(Qt.Key_Backspace);
            waitForRendering(stage);
            compare(drawer.query, "");
            compare(search.text, "");
            verify(search.input.activeFocus,
                   "Backspace left the search field unfocused");

            // The list is rebuilt again on every character and on clear.
            keyClick(Qt.Key_A);
            keyClick(Qt.Key_Backspace);
            waitForRendering(stage);
            compare(drawer.query, "");
            verify(search.input.activeFocus,
                   "clearing the filter left the search field unfocused");
        }

        // Opening the launcher is a fresh session: a query/category/selection
        // from the previous open must not survive and hide the grid.
        function test_reopening_resets_the_query_category_and_selection() {
            var drawer = make({ apps: sampleApps, active: true });
            drawer.setQueryText("calc");
            drawer.category = "utilities";
            drawer.currentIndex = 1;
            waitForRendering(stage);
            compare(drawer.visibleApps.length, 1);

            drawer.active = false;
            wait(0);
            drawer.active = true;
            wait(0);
            var search = findChild(drawer, "appsDrawerSearch");
            compare(drawer.query, "");
            compare(drawer.category, "all");
            compare(drawer.currentIndex, -1);
            compare(search.text, "");
            compare(tiles(drawer).length, sampleApps.length);
            verify(search.input.activeFocus, "reopened drawer lost search focus");
        }

        // Launchpad behaviour: Escape dismisses even while a search is active.
        // The next open starts clean, so Escape never has to be pressed twice.
        function test_escape_dismisses_with_a_non_empty_query() {
            var drawer = make({ apps: sampleApps, active: false });
            drawer.active = true;
            wait(0);
            keyClick(Qt.Key_C);
            waitForRendering(stage);
            compare(drawer.query, "c");
            keyClick(Qt.Key_Escape);
            compare(dismissedSpy.count, 1);
        }

        // With no arrow selection, Return on a filtered list opens the top
        // hit, matching the launcher's Spotlight-style expectation.
        function test_return_launches_the_top_hit_when_filtered() {
            var drawer = make({ apps: sampleApps, active: false });
            drawer.active = true;
            wait(0);
            drawer.setQueryText("calc");
            waitForRendering(stage);
            compare(drawer.currentIndex, -1);
            var search = findChild(drawer, "appsDrawerSearch");
            search.input.forceActiveFocus();
            keyClick(Qt.Key_Return);
            waitForRendering(stage);
            compare(launchedSpy.count, 1);
            compare(launchedSpy.signalArguments[0][0],
                    "org.example.Calculator.desktop");
        }

        // Keyboard selection follows the real grid stride and the viewport
        // scrolls to keep the highlighted tile visible.
        function test_keyboard_navigation_scrolls_the_selection_into_view() {
            var many = [];
            for (var i = 0; i < 50; ++i)
                many.push({ desktopId: "r" + i + ".desktop", name: "Row " + i,
                            iconPath: "", categories: [] });
            var drawer = make({ apps: many, active: false, height: 720 });
            drawer.active = true;
            wait(0);
            var grid = findChild(drawer, "appsDrawerGrid");
            compare(drawer.currentIndex, -1);
            compare(grid.contentY, 0);

            keyClick(Qt.Key_Down);
            waitForRendering(stage);
            compare(drawer.currentIndex, 0);

            for (var k = 0; k < 10; ++k)
                keyClick(Qt.Key_Down);
            waitForRendering(stage);
            compare(drawer.currentIndex,
                    Math.min(many.length - 1, drawer.columns * 10));
            verify(grid.contentY > 0, "grid did not scroll to the selection");

            var selected = tiles(drawer)[drawer.currentIndex];
            var top = selected.y;
            var bottom = top + selected.height;
            verify(top >= grid.contentY - 1,
                   "selected tile above viewport: " + top + " < " + grid.contentY);
            verify(bottom <= grid.contentY + grid.height + 1,
                   "selected tile below viewport: " + bottom + " > "
                   + (grid.contentY + grid.height));
        }

        // The card can be measured before the controller gives the offscreen
        // window an output size; `panelRect` must never report a negative
        // extent (Smithay's Size constructor panics on one and takes the
        // session down).
        function test_panel_rect_is_never_negative() {
            var drawer = make({ apps: sampleApps, width: 0, height: 0 });
            var rect = drawer.panelRect();
            verify(rect.width >= 0, "panel width " + rect.width);
            verify(rect.height >= 0, "panel height " + rect.height);
            compare(rect.width, 0);
            compare(rect.height, 0);
        }

        // A filtering action starts a new selection rather than carrying the
        // old index onto a different app.
        function test_filtering_clears_the_keyboard_selection() {
            var drawer = make({ apps: sampleApps });
            drawer.currentIndex = 2;
            drawer.setQueryText("o");
            waitForRendering(stage);
            compare(drawer.currentIndex, -1);
        }

        function compareColor(actual, expected, label) {
            fuzzyCompare(actual.r, expected.r, 0.001, label + " red");
            fuzzyCompare(actual.g, expected.g, 0.001, label + " green");
            fuzzyCompare(actual.b, expected.b, 0.001, label + " blue");
            fuzzyCompare(actual.a, expected.a, 0.001, label + " alpha");
        }

        // QML color properties round their channels to 8 bits; a token alpha
        // such as 0.06 lands on 15/255. Use this looser comparison only where
        // the expected colour is a low-alpha translucent fill.
        function compareTranslucentColor(actual, expected, label) {
            fuzzyCompare(actual.r, expected.r, 0.01, label + " red");
            fuzzyCompare(actual.g, expected.g, 0.01, label + " green");
            fuzzyCompare(actual.b, expected.b, 0.01, label + " blue");
            fuzzyCompare(actual.a, expected.a, 0.01, label + " alpha");
        }

        function test_drawer_role_material_covers_the_applications_drawer() {
            // T-21.2c: the Applications drawer is the transient Drawer role.
            // It keeps its scrim and tile grid; the card fill switches to the
            // milky plate when the sampled pass is absent.
            var states = [
                { tier: 0, blur: true, fallback: 0, reason: "none", label: "full" },
                { tier: 1, blur: true, fallback: 0, reason: "none", label: "reduced" },
                { tier: 2, blur: false, fallback: 0, reason: "minimal", label: "minimal" },
                { tier: 2, blur: false, fallback: 2, reason: "software", label: "software" }
            ];
            for (var scheme = 0; scheme < 2; ++scheme) {
                Theme.dark = scheme === 1;
                for (var i = 0; i < states.length; ++i) {
                    var state = states[i];
                    var drawer = make({
                        apps: sampleApps,
                        materialTier: state.tier,
                        materialBlurActive: state.blur,
                        materialFallback: state.fallback
                    });
                    compare(drawer.materialTier, state.tier, state.label);
                    compare(drawer.materialBlurActive, state.blur, state.label);
                    compare(drawer.materialFallback, state.fallback, state.label);
                    compare(drawer.materialFallbackReason, state.reason,
                            state.label);
                    var panel = findChild(drawer, "appsDrawerPanel");
                    verify(panel !== null);
                    compareColor(panel.color, Material.drawerFill(state.blur, state.tier, state.fallback),
                                 state.label);
                    verify(panel.color.a >= 0.85 || state.blur,
                           state.label + " fallback must read as a plate");
                    if (state.blur && state.tier === 0) {
                        // The card is a light glass plate in light mode and a
                        // slightly stronger tint in dark mode, from the two
                        // per-scheme opacity tokens.
                        var expected = scheme === 1
                            ? Theme.controls.appsDrawer.panelOpacityDark
                            : Theme.controls.appsDrawer.panelOpacity;
                        fuzzyCompare(panel.color.a, expected, 0.02,
                                     state.label + " card opacity");
                    }
                }
            }
            Theme.dark = false;
        }
    }
}