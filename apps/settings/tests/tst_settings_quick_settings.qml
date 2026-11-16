// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings
import Dragonfruit.ControlCenter

// Quick Settings pane tests (T227/T277). The pane consumes the shell's one
// `ControlCenterRegistry` and writes the same `quickSettings.*` /
// `menu.control.*` keys the live Control Center panel reads;
// `tst_controlcenter.qml` asserts the panel side.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsQuickSettings"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        readonly property var defaultModules: [
            "wifi", "bluetooth", "now-playing", "airdrop", "stage-manager",
            "screen-mirroring", "dark", "capture-screen", "focus", "brightness",
            "volume", "airplay-audio", "edit-controls"
        ]

        function init() {
            Settings.setList("quickSettings.modules", defaultModules.slice());
            Settings.setList("quickSettings.menuBarOnly", []);
            Settings.setList("quickSettings.page2", []);
            Settings.setList("quickSettings.page3", []);
            Settings.setList("quickSettings.pageNames", []);
            Settings.set("quickSettings.pageCount", 1);
            Settings.set("quickSettings.activePage", 0);
            Settings.setList("quickSettings.spans", []);
            Settings.set("quickSettings.perDisplay", false);
            Settings.set("menu.control.wifi", true);
            Settings.set("menu.control.bluetooth", true);
            Settings.set("menu.control.focus", true);
            Settings.set("menu.control.volume", true);
            Settings.set("menu.control.battery", true);
            Settings.set("menu.control.accessibility", true);
        }

        function make(width, height) {
            var shell = createTemporaryObject(shellComponent, stage, {
                width: width !== undefined ? width : stage.width,
                height: height !== undefined ? height : stage.height
            });
            waitForRendering(stage);
            shell.selectPane("control-center");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready);
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function moduleCount(pane) {
            var count = 0;
            for (var i = 0; i < pane.moduleGroups.length; ++i)
                count += pane.moduleGroups[i].ids.length;
            return count;
        }

        function collectIds(item, out) {
            if (!item)
                return out;
            if (item.settingsId !== undefined
                    && String(item.settingsId).length > 0)
                out.push(String(item.settingsId));
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i)
                collectIds(kids[i], out);
            return out;
        }

        // T277: the rendered rows are the executable map's Quick Settings
        // inventory, not a second list.
        function test_rows_match_the_executable_map() {
            var shell = make();
            var pane = paneOf(shell);
            var rendered = collectIds(pane, []);
            var mapRows = SettingsMap.rowsFor("control-center");
            verify(mapRows.length > 0);
            for (var i = 0; i < mapRows.length; ++i) {
                verify(rendered.indexOf(mapRows[i].id) >= 0,
                       "map row " + mapRows[i].id + " has no rendered row");
            }
        }

        function test_pane_opens_with_the_categorized_registry() {
            var shell = make();
            compare(shell.currentPaneId, "control-center");
            var pane = paneOf(shell);
            // The catalogue is the registry's 37 modules, grouped by the same
            // categories Edit Controls uses. The default panel is the
            // 13-module subset.
            compare(pane.catalogueIds.length, 37);
            compare(moduleCount(pane), 37);
            verify(pane.moduleGroups.length > 1);
            compare(pane.categories.length, 22);
            compare(pane.menuBarRepeater.count,
                    ControlCenterRegistry.menuBarIds.length);
            compare(Settings.values["quickSettings.modules"].length, 13);
            compare(pane.moduleEnabled("wifi"), true);
            compare(pane.categorySelect.model.length, 22);
        }

        // T277: Edit Controls is the fixed panel footer, not a module toggle.
        // The row explains the footer and has no dead switch.
        function test_edit_controls_row_is_a_footer_not_a_toggle() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.moduleDescription("edit-controls").indexOf("footer") >= 0);
            var toggle = findChild(pane, "quickSettingsToggle-edit-controls");
            verify(toggle !== null);
            compare(toggle.visible, false);
        }

        function test_category_filter_keeps_the_registry_rows() {
            var shell = make();
            var pane = paneOf(shell);
            pane.selectedCategory = "connectivity";
            waitForRendering(stage);
            verify(pane.moduleGroups.length >= 1);
            compare(pane.moduleGroups[0].title, "Connectivity");
            verify(pane.moduleGroups[0].ids.indexOf("wifi") >= 0);
            verify(pane.moduleGroups[0].ids.indexOf("bluetooth") >= 0);
            verify(pane.moduleGroups[0].ids.indexOf("dark") < 0);

            pane.selectedCategory = "all-controls";
            waitForRendering(stage);
            compare(moduleCount(pane), 37);
        }

        function test_module_toggle_adds_and_removes_live() {
            var shell = make();
            var pane = paneOf(shell);

            compare(Settings.setList("quickSettings.modules", ["wifi"]), true);
            compare(Settings.values["quickSettings.modules"].length, 1);
            compare(Settings.values["quickSettings.modules"][0], "wifi");
            Settings.setList("quickSettings.modules", defaultModules.slice());
            pane.setModuleEnabled("wifi", false);
            compare(pane.moduleEnabled("wifi"), false);
            compare(Settings.values["quickSettings.modules"].indexOf("wifi"), -1);

            pane.setModuleEnabled("wifi", true);
            compare(pane.moduleEnabled("wifi"), true);
            verify(Settings.values["quickSettings.modules"].indexOf("wifi") >= 0);
        }

        function test_an_ownerless_module_is_never_added() {
            var shell = make();
            var pane = paneOf(shell);
            var before = Settings.values["quickSettings.modules"].slice();
            compare(pane.isAddable("airdrop"), false);
            verify(pane.moduleReason("airdrop").length > 0);
            pane.setModuleEnabled("airdrop", true);
            compare(Settings.values["quickSettings.modules"].join(","),
                    before.join(","));
            // Removing an older persisted entry stays possible.
            Settings.setList("quickSettings.modules", ["airdrop", "wifi"]);
            pane.setModuleEnabled("airdrop", false);
            compare(Settings.values["quickSettings.modules"].join(","), "wifi");
        }

        function test_reorder_writes_the_ordered_list() {
            var shell = make();
            var pane = paneOf(shell);
            var before = Settings.values["quickSettings.modules"].slice();

            pane.moveModule(before[0], 1);
            var after = Settings.values["quickSettings.modules"];
            compare(after[0], before[1]);
            compare(after[1], before[0]);

            pane.moveModule(after[1], -1);
            compare(Settings.values["quickSettings.modules"][0], before[0]);

            // A direct target index (the drag path) also works.
            pane.moveModuleTo(before[0], 2);
            compare(Settings.values["quickSettings.modules"][2], before[0]);
        }

        function test_menu_bar_only_removes_from_the_panel_and_keeps_the_mark() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("menu.control.bluetooth", false);

            pane.setMenuBarOnly("bluetooth", true);
            compare(pane.isMenuBarOnly("bluetooth"), true);
            compare(pane.moduleEnabled("bluetooth"), false);
            compare(Settings.values["menu.control.bluetooth"], true);

            pane.setMenuBarOnly("bluetooth", false);
            compare(pane.isMenuBarOnly("bluetooth"), false);
            compare(pane.moduleEnabled("bluetooth"), true);
        }

        function test_pages_add_rename_select_and_remove() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.pageCount, 1);
            compare(pane.pageName(0), "Page 1");

            pane.addPage();
            compare(pane.pageCount, 2);
            compare(pane.activePage, 1);
            compare(pane.pageName(1), "Page 2");
            compare(Settings.values["quickSettings.pageCount"], 2);
            compare(Settings.values["quickSettings.activePage"], 1);

            pane.renamePage(1, "Work");
            compare(pane.pageName(1), "Work");
            compare(Settings.values["quickSettings.pageNames"][1], "Work");

            pane.selectPage(0);
            compare(pane.activePage, 0);
            compare(Settings.values["quickSettings.activePage"], 0);

            pane.removePage(1);
            compare(pane.pageCount, 1);
            compare(Settings.values["quickSettings.pageCount"], 1);
            compare(Settings.values["quickSettings.pageNames"].length, 0);
        }

        function test_span_cycle_writes_the_measured_span() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.spanOf("wifi"), "2x1");
            pane.cycleSpan("wifi");
            compare(pane.spanOf("wifi"), "2x2");
            verify(Settings.values["quickSettings.spans"].indexOf("wifi=2x2") >= 0);
            var layout = pane.previewLayout;
            var tile = null;
            for (var i = 0; i < layout.length; ++i) {
                if (layout[i].id === "wifi")
                    tile = layout[i];
            }
            verify(tile !== null);
            compare(tile.w, 140);
            compare(tile.h, 140);
        }

        function test_add_controls_lists_addable_and_unavailable_modules() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.addableIds.indexOf("storage") >= 0);
            verify(pane.addableIds.indexOf("airdrop") < 0);
            verify(pane.unavailableIds.indexOf("airdrop") >= 0);
            verify(pane.unavailableIds.indexOf("now-playing") >= 0);

            pane.addControlsDialog.show();
            waitForRendering(stage);
            compare(pane.addControlsDialog.open, true);
            verify(findChild(pane.addControlsDialog, "quickSettingsAdd-storage") !== null);
            var unavailable = findChild(pane.addControlsDialog,
                                        "quickSettingsAdd-airdrop");
            compare(unavailable, null,
                    "an unavailable module must not have an Add control");
            pane.addControlsDialog.accept();
        }

        function test_add_controls_adds_to_the_active_page() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addPage();
            compare(pane.activePage, 1);
            pane.setModuleEnabled("storage", true);
            compare(Settings.values["quickSettings.page2"].indexOf("storage") >= 0, true);
            compare(Settings.values["quickSettings.modules"].indexOf("storage"), -1);
        }

        function test_empty_list_is_a_real_state() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setList("quickSettings.modules", []);
            tryVerify(function() { return pane.modules.length === 0; });
            compare(pane.previewLayout.length, 0);
            compare(pane.previewHasFooter, false);
            // The explicit empty list is not silently replaced by defaults.
            compare(Settings.values["quickSettings.modules"].length, 0);
            // Reset is the documented way back.
            pane.resetAll();
            compare(Settings.values["quickSettings.modules"].join(","),
                    defaultModules.join(","));
        }

        function test_reset_covers_pages_spans_per_display_and_menu_marks() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addPage();
            pane.renamePage(1, "Work");
            pane.cycleSpan("wifi");
            pane.setMenuBarOnly("focus", true);
            Settings.set("quickSettings.perDisplay", true);
            Settings.set("menu.control.wifi", false);

            pane.resetButton.clicked();
            compare(Settings.values["quickSettings.modules"].join(","),
                    defaultModules.join(","));
            compare(Settings.values["quickSettings.menuBarOnly"].length, 0);
            compare(Settings.values["quickSettings.pageCount"], 1);
            compare(Settings.values["quickSettings.activePage"], 0);
            compare(Settings.values["quickSettings.pageNames"].length, 0);
            compare(Settings.values["quickSettings.spans"].length, 0);
            compare(Settings.values["quickSettings.perDisplay"], false);
            for (var i = 0; i < ControlCenterRegistry.menuBarIds.length; ++i) {
                var key = ControlCenterRegistry.menuBarKey(
                    ControlCenterRegistry.menuBarIds[i]);
                compare(Settings.values[key], true, key + " must reset to true");
            }
        }

        function test_per_display_row_is_disabled_with_the_reason() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.perDisplayRow.enabled, false);
            verify(pane.perDisplayRow.description.indexOf("one Quick Settings") >= 0);
            verify(pane.perDisplayRow.description.indexOf("OPEN") < 0);
            compare(Settings.values["quickSettings.perDisplay"], false);
        }

        function test_live_preview_tracks_the_active_page() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.previewLayout.length > 0, true);
            pane.addPage();
            compare(pane.previewLayout.length, 0);
            pane.setModuleEnabled("dark", true);
            compare(pane.previewLayout.length, 1);
            compare(pane.previewLayout[0].id, "dark");
        }

        function test_external_module_change_converges() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setList("quickSettings.modules", ["dark", "wifi"]);
            tryVerify(function() { return pane.modules.length === 2; });
            compare(pane.modules[0], "dark");
            compare(pane.modules[1], "wifi");
        }

        function test_pane_fits_the_740_by_820_window() {
            var shell = make(740, 820);
            var pane = paneOf(shell);
            verify(pane.width <= 740, "pane width " + pane.width);
            compare(shell.width, 740);
            compare(shell.height, 820);
            verify(pane.moduleGroups.length > 0);
        }

        // T277 live-check sequence: add, remove, reorder, create a page,
        // resize, pin to the menu bar, and reset in one flow. The headless
        // stills in `make t277-quick-settings-menu-bar-capture` show the same
        // states; a host Wayland session is not available in this harness.
        function test_live_check_sequence() {
            var shell = make(740, 820);
            var pane = paneOf(shell);

            pane.setModuleEnabled("storage", true);
            verify(pane.moduleEnabled("storage"));
            pane.setModuleEnabled("storage", false);
            compare(pane.moduleEnabled("storage"), false);

            var before = Settings.values["quickSettings.modules"].slice();
            pane.moveModule(before[0], 1);
            compare(Settings.values["quickSettings.modules"][0], before[1]);

            pane.addPage();
            compare(pane.pageCount, 2);
            pane.renamePage(1, "Live");
            compare(pane.pageName(1), "Live");

            pane.cycleSpan("focus");
            verify(Settings.values["quickSettings.spans"].join(",")
                   .indexOf("focus=") >= 0);

            pane.setMenuBarOnly("accessibility", true);
            compare(pane.isMenuBarOnly("accessibility"), true);
            compare(Settings.values["menu.control.accessibility"], true);

            pane.resetButton.clicked();
            compare(Settings.values["quickSettings.modules"].join(","),
                    defaultModules.join(","));
            compare(pane.pageCount, 1);
            compare(Settings.values["quickSettings.menuBarOnly"].length, 0);
        }
    }
}
