// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings
import Dragonfruit.ControlCenter

// Menu Bar Controls sub-pane tests (T227/T277, moved in place by T293b). The
// sub-pane consumes the shell's one `ControlCenterRegistry` and writes the
// same `quickSettings.*` / `menu.control.*` keys the live Quick Settings panel
// reads; `tst_controlcenter.qml` asserts the panel side.
//
// Most cases mount the body directly. The last group exercises the real shell:
// Menu Bar -> Menu Bar Controls opens, the `control-center`/`quick-settings`
// aliases land there, row deep links open the sub-pane, Back returns to the
// Menu Bar list, and no top-level Quick Settings pane exists.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsQuickSettings"
        when: windowShown

        Component { id: quickSettingsComponent; MenuBarControlsPane { } }

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

        // T285 removed the top-level Quick Settings pane; T293b moved its
        // surface in place under Menu Bar. The direct cases below exercise the
        // body; `test_*_through_the_shell` cases open it through the shell.
        function make(width, height) {
            var pane = createTemporaryObject(quickSettingsComponent, stage, {
                width: width !== undefined ? width : stage.width,
                height: height !== undefined ? height : stage.height
            });
            waitForRendering(stage);
            verify(pane);
            return pane;
        }

        function paneOf(pane) {
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

        // T277/T293b: the rendered rows are the executable map's Menu Bar
        // Controls inventory (formerly the `control-center` rows), not a
        // second list. The map carries no `control-center` row list any more.
        function test_rows_match_the_executable_map() {
            var shell = make();
            var pane = paneOf(shell);
            var rendered = collectIds(pane, []);
            var mapRows = SettingsMap.rowsFor("menu-bar").filter(function(entry) {
                return SettingsMap.subPaneForRow("menu-bar", entry.id)
                        === "controls";
            });
            verify(mapRows.length > 0);
            compare(SettingsMap.rowsFor("control-center").length, 0);
            for (var i = 0; i < mapRows.length; ++i) {
                verify(rendered.indexOf(mapRows[i].id) >= 0,
                       "map row " + mapRows[i].id + " has no rendered row");
            }
        }

        function test_pane_opens_with_the_categorized_registry() {
            var pane = make();
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
            var pane = make(740, 820);
            verify(pane.width <= 740, "pane width " + pane.width);
            compare(pane.width, 740);
            compare(pane.height, 820);
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

        // ── Through the shell: the in-place sub-pane (T293b) ──────────────

        Component { id: shellComponent; SettingsShell { } }

        // A fresh shell, then one full event-loop turn so no previous case's
        // in-flight highlight timer or `Qt.callLater` reveal can leak into the
        // next deep link.
        function makeShell(width, height) {
            var shell = createTemporaryObject(shellComponent, stage, {
                width: width !== undefined ? width : 740,
                height: height !== undefined ? height : 820
            });
            waitForRendering(stage);
            verify(shell);
            tryVerify(function() {
                return shell.loadedPaneId === shell.currentPaneId
                        && shell.paneBody.item !== null;
            });
            wait(50);
            waitForRendering(stage);
            return shell;
        }

        function showMenuBar(shell) {
            shell.selectPane("menu-bar");
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                        && shell.paneBody.item !== null;
            });
            waitForRendering(stage);
            var list = shell.paneBody.item;
            verify(list.listLayout !== undefined,
                   "the Menu Bar pane body must be mounted");
            return list;
        }

        function bodyOf(shell) {
            return shell.paneBody.item;
        }

        // Wait for the in-place sub-pane body and return it.
        function waitForControls(shell) {
            tryVerify(function() {
                var body = bodyOf(shell);
                return body !== null && body.subPaneLoader !== undefined
                        && body.subPaneLoader.status === Loader.Ready
                        && body.subPaneLoader.item !== null;
            });
            waitForRendering(stage);
            return bodyOf(shell).subPaneLoader.item;
        }

        function test_customize_controls_opens_the_in_place_sub_pane() {
            var shell = makeShell();
            var list = showMenuBar(shell);
            compare(shell.currentSubPaneId, "");
            compare(list.subPaneId, "");

            list.customizeControlsButton.clicked();
            var body = waitForControls(shell);
            compare(body.subPaneId, "controls");
            compare(shell.currentPaneId, "menu-bar");
            compare(shell.currentSubPaneId, "controls");
            compare(shell.paneTitle, "Menu Bar Controls");
            compare(list.subPaneId, "controls");
            compare(list.subPaneTitle, "Menu Bar Controls");
            // The real body: the former Quick Settings surface, on the same
            // registry. It is not a second module list.
            compare(body.registry, ControlCenterRegistry);
            compare(body.catalogueIds.length, 37);
            verify(body.pagesGroup !== undefined);
            verify(body.modulesGroup !== undefined);
            verify(body.menuBarGroup !== undefined);
            verify(body.previewGroup !== undefined);
            verify(body.resetGroup !== undefined);
            // The pane list is hidden while the sub-pane is open.
            compare(list.listLayout.visible, false);
            compare(list.subPaneLayout.visible, true);
        }

        // The aliases deep links land on the same in-place sub-pane.
        function test_aliases_land_on_the_sub_pane() {
            var shell = makeShell();
            verify(shell.deepLink("settings://pane/control-center"));
            tryVerify(function() {
                return shell.currentSubPaneId === "controls";
            });
            compare(shell.currentPaneId, "menu-bar");
            compare(shell.paneTitle, "Menu Bar Controls");
            var body = waitForControls(shell);
            compare(body.registry, ControlCenterRegistry);

            shell.selectPane("general");
            tryVerify(function() { return shell.loadedPaneId === "general"; });
            waitForRendering(stage);
            verify(shell.deepLink("settings://pane/quick-settings"));
            tryVerify(function() {
                return shell.currentSubPaneId === "controls";
            });
            compare(shell.currentPaneId, "menu-bar");
            compare(shell.currentSubPaneId, "controls");
            compare(shell.paneTitle, "Menu Bar Controls");
        }

        // Pages, spans, modules, and the preview all write the same
        // `quickSettings.*` keys from inside the sub-pane.
        function test_the_sub_pane_edits_pages_spans_modules_and_preview() {
            var shell = makeShell();
            showMenuBar(shell);
            shell.openSubPane("controls");
            var body = waitForControls(shell);
            verify(body);
            compare(shell.currentPaneId, "menu-bar");
            compare(shell.currentSubPaneId, "controls");
            body.addPage();
            compare(body.pageCount, 2);
            body.renamePage(1, "Work");
            compare(Settings.values["quickSettings.pageNames"][1], "Work");
            body.selectPage(0);

            body.cycleSpan("wifi");
            verify(Settings.values["quickSettings.spans"].join(",")
                   .indexOf("wifi=") >= 0);

            body.setModuleEnabled("storage", true);
            verify(Settings.values["quickSettings.modules"]
                   .indexOf("storage") >= 0);
            body.setModuleEnabled("storage", false);
            compare(Settings.values["quickSettings.modules"]
                    .indexOf("storage"), -1);

            // The live preview is the registry layout of the modules on the
            // active page that are neither menu-bar-only, unknown, nor the
            // footer action (the registry drops all three).
            var expected = body.activeList.filter(function(id) {
                return !body.isMenuBarOnly(id)
                        && ControlCenterRegistry.module(id) !== null
                        && body.spanOf(id) !== "footer";
            });
            verify(body.previewLayout.length > 0);
            compare(body.previewLayout.length, expected.length);
        }

        function test_back_returns_to_the_menu_bar_list() {
            var shell = makeShell();
            var list = showMenuBar(shell);
            shell.openSubPane("controls");
            waitForControls(shell);
            compare(shell.currentSubPaneId, "controls");

            list.subPaneBackButton.clicked();
            waitForRendering(stage);
            compare(shell.currentSubPaneId, "");
            compare(shell.paneTitle, "Menu Bar");
            compare(list.listLayout.visible, true);
            compare(list.subPaneLayout.visible, false);
        }

        // A search/deep-link result for a moved row opens the sub-pane and
        // lands on the rendered row.
        function test_a_row_deep_link_opens_the_sub_pane() {
            var shell = makeShell();
            showMenuBar(shell);
            verify(shell.deepLink("settings://pane/menu-bar/row/pages"));
            var body = waitForControls(shell);
            compare(shell.currentSubPaneId, "controls");
            // The row landed (`revealPending` consumed it); the accent wash
            // is hold-timed, so `currentRowId` is the stable assertion.
            compare(shell.currentRowId, "pages");
            verify(body.pagesGroup !== undefined);
            // The remaining rows are landable too.
            compare(SettingsMap.rowSurface("menu-bar", "preview"), "subpane");
            compare(SettingsMap.rowSurface("menu-bar", "reset"), "subpane");
            compare(SettingsMap.rowSurface("menu-bar", "overflow"), "dialog");
            compare(SettingsMap.rowSurface("menu-bar", "search"), "pane");
            compare(SettingsMap.isRowLandable("menu-bar", "screenMirroring"),
                    false);
        }

        // The former Quick Settings pane is gone: no catalog entry, no pane
        // case, and the `quickSettings.*` / `menu.control.*` keys the shell
        // status items read are untouched by the move.
        function test_no_top_level_quick_settings_and_the_panel_keys_are_unchanged() {
            var shell = makeShell();
            compare(SettingsPanes.paneById("control-center"), null);
            compare(SettingsPanes.indexOf("control-center"), -1);
            var ids = shell.sidebarSections.reduce(function(out, section) {
                return out.concat((section.items || []).map(function(item) {
                    return item.id;
                }));
            }, []);
            compare(ids.indexOf("control-center"), -1);
            compare(ids.indexOf("menu-bar") >= 0, true);
            // The Control Center panel keys are the same ones the sub-pane
            // writes; the move adds and removes none.
            compare(Settings.values["menu.control.controlCenter"], true);
            compare(Settings.values["menu.control.search"], true);
            compare(ControlCenterRegistry.menuBarIds.length > 0, true);
            verify(shell.deepLink("settings://pane/control-center"));
            tryVerify(function() { return shell.currentSubPaneId === "controls"; });
            compare(Settings.values["menu.control.controlCenter"], true);
        }
    }
}
