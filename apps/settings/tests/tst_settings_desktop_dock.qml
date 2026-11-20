// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Desktop & Dock-pane tests (T-09.4; T292a restyle). Headless with
// `DF_SETTINGS_FIXTURE`, so the `Settings` singleton serves the schema
// defaults and keeps writes in memory: no bus, no daemon, deterministic.
// Every control is asserted to apply live to its `dock.*` key on the same
// event-loop turn, and an external settingsd change flows back into the
// control (the T-09.1b converge rule).
//
// T292a covers the mockup's section order and labels, the two advanced sheets
// (their rows keep their keys), the footer actions, the single live Workspace
// Overview entry, and the absence of the menu-bar row.
Item {
    id: stage
    width: 900
    height: 660

    TestCase {
        id: testCase
        name: "SettingsDesktopDock"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global, so reset every key this pane
        // touches to its schema default before each case.
        function init() {
            Settings.set("dock.size", 0.5);
            Settings.set("dock.magnification", 0.5);
            Settings.set("dock.position", "bottom");
            Settings.set("desktop.showItems", "on-desktop");
            Settings.set("dock.minimizedAnimation", "scale");
            Settings.set("dock.titlebarDoubleClick", "zoom");
            Settings.set("dock.minimizeIntoTileIcon", false);
            Settings.set("dock.autohide", false);
            Settings.set("dock.animateOpening", true);
            Settings.set("dock.showIndicators", true);
            Settings.set("dock.showRecentApps", false);
            Settings.set("dock.chooserOnHover", false);
            Settings.set("dock.minimizeReaction", false);
            Settings.set("menu.global", true);
            Settings.set("defaults.browser", "");
            Settings.set("overview.hotCornerTopLeftModifier", "none");
            Settings.set("overview.hotCornerTopRightModifier", "none");
            Settings.set("overview.hotCornerBottomLeftModifier", "none");
            Settings.set("overview.hotCornerBottomRightModifier", "none");
            Settings.set("overview.switchToAppSpace", true);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("desktop-dock");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the desktop-dock body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function approx(a, b) {
            return Math.abs(a - b) < 0.001;
        }

        // The rendered group titles above the footer, in order.
        function renderedGroupTitles(pane) {
            var titles = [];
            var groups = [pane.dockGroup, pane.desktopGroup, pane.widgetsGroup,
                          pane.windowsGroup, pane.missionControlGroup];
            for (var i = 0; i < groups.length; ++i)
                titles.push(groups[i].title);
            return titles;
        }

        function test_pane_opens_with_the_wired_controls() {
            var shell = make();
            compare(shell.currentPaneId, "desktop-dock");
            var pane = paneOf(shell);

            compare(pane.dockGroup.title, "Dock");
            compare(pane.dockGroup.rows.children.length, 10);
            verify(approx(pane.sizeSlider.value, 0.5));
            verify(approx(pane.magnificationSlider.value, 0.5));
            compare(pane.sizeSlider.minLabel, "Small");
            compare(pane.sizeSlider.maxLabel, "Large");
            compare(pane.magnificationSlider.midLabel, "Small");
            compare(pane.positionSelect.currentIndex, 0);          // bottom
            compare(pane.minimizedAnimationSelect.currentIndex, 1); // scale
            compare(pane.titlebarSelect.currentIndex, 0);          // zoom
            compare(pane.minimizeIntoTileToggle.checked, false);
            compare(pane.autohideToggle.checked, false);
            compare(pane.animateOpeningToggle.checked, true);
            compare(pane.showIndicatorsToggle.checked, true);
            compare(pane.showRecentAppsToggle.checked, false);
        }

        // T292a: the mockup's five sections in order, under the toolbar.
        function test_sections_follow_the_mockup_order_and_labels() {
            var shell = make();
            var pane = paneOf(shell);
            compare(renderedGroupTitles(pane).join(","),
                    ["Dock", "Desktop & Workspaces", "Widgets", "Windows",
                     "Window Overview"].join(","));

            // Dock rows, in the mockup order and with the mockup labels.
            var dockRows = pane.dockGroup.rows.children;
            var dockLabels = [
                "Dock Size", "Dock Magnification Size",
                "Dock position on screen", "Minimized window animation",
                "Window title bar double-click action",
                "Minimize windows into application icon",
                "Automatically hide and show the Dock",
                "Animate opening applications",
                "Show indicators for open applications",
                "Show suggested and recent apps in Dock"
            ];
            compare(dockRows.length, dockLabels.length);
            for (var d = 0; d < dockRows.length; ++d)
                compare(dockRows[d].label, dockLabels[d],
                        "Dock row " + d + " label");

            // Desktop & Workspaces, with the Stage -> Workspace Focus rename.
            var desktopRows = pane.desktopGroup.rows.children;
            var desktopLabels = [
                "Show items", "Click wallpaper to show desktop",
                "Workspace Focus", "Show recent apps in Workspace Focus",
                "Show windows from an application"
            ];
            compare(desktopRows.length, desktopLabels.length);
            for (var s = 0; s < desktopRows.length; ++s)
                compare(desktopRows[s].label, desktopLabels[s],
                        "Desktop row " + s + " label");
            compare(pane.desktopItemsOptions[1].label, "In Workspace Focus");

            // Widgets: the two disabled widget rows and the live browser row.
            var widgetRows = pane.widgetsGroup.rows.children;
            compare(widgetRows.length, 3);
            compare(widgetRows[0].label, "Show Widgets");
            compare(widgetRows[1].label, "Dim widgets on desktop");
            compare(widgetRows[2].label, "Default web browser");
            compare(widgetRows[2].settingsId, "defaultBrowser");

            // Windows, with the Super modifier and both edge-tile labels.
            var windowRows = pane.windowsGroup.rows.children;
            var windowLabels = [
                "Prefer tabs when opening documents",
                "Ask to keep changes when closing documents",
                "Close windows when quitting an application",
                "Drag windows to left or right edge of screen to tile",
                "Drag windows to menu bar to fill screen",
                "Hold Super key while dragging windows to tile",
                "Tiled windows have margins"
            ];
            compare(windowRows.length, windowLabels.length);
            for (var w = 0; w < windowRows.length; ++w)
                compare(windowRows[w].label, windowLabels[w],
                        "Windows row " + w + " label");

            // The Single live Overview entry.
            compare(pane.missionControlGroup.rows.children.length, 1);
            compare(pane.overviewRow.label, "Workspace Overview");
            compare(pane.hotCornersRow, undefined,
                    "hot corners are a footer action, not an inline row");
            compare(pane.shortcutsRow, undefined,
                    "shortcuts are a footer action, not an inline row");

            // The footer carries the four actions in the mockup order.
            compare(pane.footer.row.children.length, 4);
            compare(pane.advancedDockButton.text, "Advanced…");
            compare(pane.advancedWindowsButton.text, "Advanced…");
            compare(pane.advancedDockButton.accessibleName,
                    "Open advanced Dock settings");
            compare(pane.advancedWindowsButton.accessibleName,
                    "Open advanced window settings");
            compare(pane.shortcutsButton.text, "Shortcuts…");
            compare(pane.hotCornersButton.text, "Hot Corners…");
            compare(pane.sourceRevision, "t292a");
        }

        function test_size_slider_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.sizeSlider.setValue(0.8);
            pane.sizeSlider.commit();
            verify(approx(Settings.values["dock.size"], 0.8));
            verify(approx(pane.sizeValue, 0.8));

            // An external change converges back into the slider.
            Settings.set("dock.size", 0.2);
            verify(approx(pane.sizeSlider.value, 0.2));
        }

        function test_magnification_slider_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.magnificationSlider.setValue(0.9);
            pane.magnificationSlider.commit();
            verify(approx(Settings.values["dock.magnification"], 0.9));
            verify(approx(pane.magnificationValue, 0.9));

            // Off (0) disables magnification; the token step still works.
            pane.magnificationSlider.nudge(-1.0);
            verify(approx(Settings.values["dock.magnification"], 0.0));
        }

        function test_position_select_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.positionSelect.activateIndex(1); // left
            compare(Settings.values["dock.position"], "left");
            compare(pane.positionSelect.currentIndex, 1);

            pane.positionSelect.activateIndex(2); // right
            compare(Settings.values["dock.position"], "right");
            compare(pane.positionSelect.currentIndex, 2);

            // External change converges.
            Settings.set("dock.position", "bottom");
            compare(pane.positionSelect.currentIndex, 0);
        }

        function test_minimized_animation_select_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.minimizedAnimationSelect.activateIndex(0); // genie
            compare(Settings.values["dock.minimizedAnimation"], "genie");
            compare(pane.minimizedAnimationSelect.currentLabel, "Genie Effect");

            pane.minimizedAnimationSelect.activateIndex(2); // none
            compare(Settings.values["dock.minimizedAnimation"], "none");
        }

        function test_titlebar_double_click_select_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.titlebarSelect.activateIndex(1); // minimize
            compare(Settings.values["dock.titlebarDoubleClick"], "minimize");
            compare(pane.titlebarSelect.currentIndex, 1);

            pane.titlebarSelect.activateIndex(2); // none
            compare(Settings.values["dock.titlebarDoubleClick"], "none");
        }

        function test_main_toggles_apply_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.minimizeIntoTileToggle.toggle();
            compare(Settings.values["dock.minimizeIntoTileIcon"], true);
            pane.autohideToggle.toggle();
            compare(Settings.values["dock.autohide"], true);
            pane.animateOpeningToggle.toggle();
            compare(Settings.values["dock.animateOpening"], false);
            pane.showIndicatorsToggle.toggle();
            compare(Settings.values["dock.showIndicators"], false);
            pane.showRecentAppsToggle.toggle();
            compare(Settings.values["dock.showRecentApps"], true);

            // Every change landed and round-trips through the binding.
            compare(pane.minimizeIntoTileToggle.checked, true);
            compare(pane.autohideToggle.checked, true);
            compare(pane.animateOpeningToggle.checked, false);
            compare(pane.showIndicatorsToggle.checked, false);
            compare(pane.showRecentAppsToggle.checked, true);
        }

        // T292a: the menu-bar row moved to the Menu Bar pane (T293a). It must
        // not be rendered, searched, or landable here any more.
        function test_the_menu_bar_row_left_this_pane() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.menuGroup, undefined);
            compare(pane.globalMenuToggle, undefined);
            verify(collectRowIds(pane, []).indexOf("menuBarToggle") < 0);
            compare(SettingsMap.rowFor("desktop-dock", "menuBarToggle"), null);
            for (var i = 0; i < SettingsSearch.entries.length; ++i) {
                verify(!(SettingsSearch.entries[i].paneId === "desktop-dock"
                         && SettingsSearch.entries[i].label ===
                            "Show application menus in the menu bar"),
                       "the menu-bar row must not be indexed under Desktop & Dock");
            }
            // The key itself is untouched and still owned elsewhere.
            compare(Settings.values["menu.global"], true);
        }

        function test_external_change_converges_for_toggles() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.set("dock.autohide", true);
            compare(pane.autohideToggle.checked, true);
            Settings.set("dock.showIndicators", false);
            compare(pane.showIndicatorsToggle.checked, false);
            Settings.set("dock.minimizeIntoTileIcon", true);
            compare(pane.minimizeIntoTileToggle.checked, true);
            Settings.set("dock.chooserOnHover", true);
            compare(pane.chooserOnHoverToggle.checked, true);
            Settings.set("dock.minimizeReaction", true);
            compare(pane.minimizeReactionToggle.checked, true);
        }

        // The SettingsRow lays the label on the left and the control on the
        // right; assert it for every row so a future width regression (which
        // would let a control ride over its label) fails here.
        function test_row_controls_are_right_aligned() {
            var shell = make();
            var pane = paneOf(shell);
            var rows = pane.dockGroup.rows.children;
            compare(rows.length, 10);
            for (var i = 0; i < rows.length; ++i) {
                var row = rows[i];
                verify(row.control.x > row.width * 0.55,
                       "row " + i + " control must sit at the right, x="
                       + row.control.x + " row.width=" + row.width);
                verify(row.control.width > 0, "row " + i + " control must have width");
            }
        }

        // -- T227: Desktop & Workspace Focus -------------------------------

        function test_desktop_items_apply_live_and_converge() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.desktopItemsControl.currentIndex, 0);

            pane.desktopItemsControl.activateIndex(1); // In Workspace Focus
            compare(Settings.values["desktop.showItems"], "in-stage");
            pane.desktopItemsControl.activateIndex(0); // On Desktop
            compare(Settings.values["desktop.showItems"], "on-desktop");

            Settings.set("desktop.showItems", "in-stage");
            tryVerify(function() {
                return pane.desktopItemsControl.currentIndex === 1;
            });
        }

        // Rows inside a Dialog are not pane-body descendants once the Dialog
        // adopts the overlay layer, so the three sheet rows are collected
        // through the pane's row handles.
        function collectRowIds(item, out) {
            if (item.settingsId !== undefined
                    && String(item.settingsId).length > 0)
                out.push(item.settingsId);
            if (item.chooserOnHoverRowHandle !== undefined) {
                var sheet = [item.chooserOnHoverRowHandle,
                             item.minimizeReactionRowHandle,
                             item.windowRulesRowHandle];
                for (var k = 0; k < sheet.length; ++k) {
                    if (sheet[k] !== null && out.indexOf(sheet[k].settingsId) < 0)
                        out.push(sheet[k].settingsId);
                }
            }
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i)
                collectRowIds(kids[i], out);
            return out;
        }

        // T276/T292a: the rendered rows are the executable map's Desktop & Dock
        // inventory, not a second list. The Shortcuts…/Hot Corners… ids stay in
        // the map (their Editor surfaces are still absent) but are rendered as
        // footer actions, so they are checked as live footer buttons.
        function test_rows_match_the_executable_map() {
            var shell = make();
            var pane = paneOf(shell);
            var rendered = collectRowIds(pane, []);
            var footerIds = { "shortcuts": pane.shortcutsButton,
                              "hotCorners": pane.hotCornersButton };
            var mapRows = SettingsMap.rowsFor("desktop-dock");
            verify(mapRows.length === 31,
                   "expected thirty-one map rows, got " + mapRows.length);
            for (var i = 0; i < mapRows.length; ++i) {
                var id = mapRows[i].id;
                if (footerIds[id] !== undefined) {
                    verify(footerIds[id].visible, "footer action " + id
                           + " must be visible");
                    continue;
                }
                verify(rendered.indexOf(id) >= 0,
                       "map row " + id + " has no rendered row");
            }
        }

        // T276: every row either applies live or is disabled with an honest
        // reason. Task ids and "OPEN" never appear in user-facing strings.
        function verifyDisabledReason(row, name) {
            compare(row.enabled, false, name + " must be disabled");
            verify(row.description.length > 0, name + " must explain why");
            verify(row.description.indexOf("OPEN") < 0,
                   name + " must not say OPEN");
            verify(row.description.indexOf("T2") < 0,
                   name + " must not leak a task id");
        }

        function test_desktop_rows_are_disabled_with_reasons() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.desktopGroup.title, "Desktop & Workspaces");
            compare(pane.desktopItemsControl.currentIndex, 0);
            verifyDisabledReason(pane.clickWallpaperRow, "Click wallpaper");
            verifyDisabledReason(pane.stageRow, "Workspace Focus");
            verifyDisabledReason(pane.stageRecentRow, "Recent apps in Workspace Focus");
            verifyDisabledReason(pane.windowsFromAppRow, "Windows from an app");
            compare(pane.stageRow.label, "Workspace Focus");
        }

        function test_widgets_group_is_disabled_with_reasons() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.widgetsGroup.title, "Widgets");
            verifyDisabledReason(pane.showWidgetsRow, "Show Widgets");
            verifyDisabledReason(pane.dimWidgetsRow, "Dim widgets");
        }

        function test_windows_group_matches_the_tiling_inventory() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.windowsGroup.title, "Windows");
            verifyDisabledReason(pane.preferTabsRow, "Prefer tabs");
            verifyDisabledReason(pane.askToKeepChangesRow, "Ask to keep changes");
            verifyDisabledReason(pane.closeOnQuitRow, "Close windows on quit");
            verifyDisabledReason(pane.dragToTileRow, "Drag to tile");
            verifyDisabledReason(pane.dragToFillRow, "Drag to fill");
            verifyDisabledReason(pane.holdModifierRow, "Hold modifier to tile");
            verifyDisabledReason(pane.tiledMarginsRow, "Tiled margins");
            compare(pane.missionControlGroup.rows.children.length, 1);
            compare(SettingsMap.rowFor("desktop-dock", "windowRules").absence,
                    "disabled-with-reason");
            verifyDisabledReason(pane.windowRulesRowHandle,
                                 "Application window rules");
            compare(pane.windowRulesRowHandle.settingsId, "windowRules");
        }

        // -- T292a: the advanced sheets ------------------------------------

        function test_advanced_dock_sheet_keeps_the_linux_rows() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.advancedDockDialog.open, false);
            compare(collectRowIds(pane.dockGroup, []).indexOf("chooserOnHover"), -1,
                    "the hover chooser is not in the Dock group");

            pane.advancedDockButton.clicked();
            tryVerify(function() { return pane.advancedDockDialog.open; });
            compare(pane.advancedDockGroup.title, "Advanced Dock");
            compare(pane.advancedDockGroup.rows.children.length, 2);
            compare(pane.advancedDockGroup.rows.children[0].label,
                    "Open a window chooser by hovering");
            compare(pane.advancedDockGroup.rows.children[0].settingsId,
                    "chooserOnHover");
            compare(pane.advancedDockGroup.rows.children[1].label,
                    "Bounce app icons when windows minimize");
            compare(pane.advancedDockGroup.rows.children[1].settingsId,
                    "minimizeReaction");

            pane.chooserOnHoverToggle.toggle();
            compare(Settings.values["dock.chooserOnHover"], true);
            pane.minimizeReactionToggle.toggle();
            compare(Settings.values["dock.minimizeReaction"], true);
            compare(pane.chooserOnHoverRowHandle.owner.trim().length > 0, true);
            compare(pane.chooserOnHoverRowHandle.settingsKey, "dock.chooserOnHover");

            pane.closeAdvancedDock();
            tryVerify(function() { return !pane.advancedDockDialog.open; });
        }

        function test_advanced_windows_sheet_keeps_the_window_rules_row() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.advancedWindowsDialog.open, false);
            pane.advancedWindowsButton.clicked();
            tryVerify(function() { return pane.advancedWindowsDialog.open; });
            compare(pane.advancedWindowsGroup.title, "Advanced Windows");
            compare(pane.windowRulesRowHandle.settingsKey, "windows.rules");
            verifyDisabledReason(pane.windowRulesRowHandle,
                                 "Application window rules");
            pane.closeAdvancedWindows();
            tryVerify(function() { return !pane.advancedWindowsDialog.open; });
        }

        // The map's rowSurface answers "dialog" for exactly the three sheet
        // rows so a deep link can open the sheet and land on the row.
        function test_advanced_rows_are_dialog_rows() {
            var dialogIds = ["chooserOnHover", "minimizeReaction", "windowRules"];
            var rows = SettingsMap.rowsFor("desktop-dock");
            for (var i = 0; i < rows.length; ++i) {
                var surface = SettingsMap.rowSurface("desktop-dock", rows[i].id);
                if (dialogIds.indexOf(rows[i].id) >= 0)
                    compare(surface, "dialog", rows[i].id + " must be a dialog row");
                else
                    compare(surface, "pane", rows[i].id + " must stay on the pane");
                verify(SettingsMap.isRowLandable("desktop-dock", rows[i].id),
                       rows[i].id + " must be landable");
            }
        }

        function test_deep_link_opens_the_advanced_sheet() {
            var shell = make();
            var pane = paneOf(shell);
            compare(shell.deepLink("settings://pane/desktop-dock/row/windowRules"),
                    true);
            tryVerify(function() { return pane && shell.paneBody.item.advancedWindowsDialog.open; });
            var windowsPane = shell.paneBody.item;
            compare(shell.highlightTarget, windowsPane.windowRulesRowHandle);
            compare(shell.highlightTarget.highlighted, true);
            compare(shell.currentRowId, "windowRules");

            verify(shell.deepLink("settings://pane/desktop-dock/row/chooserOnHover"));
            tryVerify(function() { return shell.paneBody.item.advancedDockDialog.open; });
            compare(shell.highlightTarget,
                    shell.paneBody.item.chooserOnHoverRowHandle);
        }

        // -- T292a: the live Workspace Overview entry ----------------------

        function test_workspace_overview_entry_opens_the_existing_surface() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.missionControlGroup.title, "Window Overview");
            compare(pane.overviewRow.settingsId, "overview");
            compare(pane.overviewRow.label, "Workspace Overview");
            verify(pane.overviewRow.disclosure);
            verify(pane.overviewRow.enabled);

            // The alias and the entry share one landing.
            compare(SettingsPanes.resolve("mission-control").paneId,
                    "desktop-dock");
            compare(SettingsPanes.resolve("mission-control").anchor,
                    "Workspace Overview");
            compare(SettingsPanes.resolve("overview").anchor,
                    "Workspace Overview");

            var seen = [];
            pane.navigateRequested.connect(function(paneId) { seen.push(paneId); });
            pane.overviewRow.disclosureClicked();
            compare(seen.length, 1);
            compare(seen[0], "mission-control");

            // The alias deep link lands on the section, not a dead end.
            shell.deepLink("settings://pane/mission-control");
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            compare(shell.highlightTarget.sectionId, "Workspace Overview");
            verify(shell.highlightTarget.highlighted);
        }

        function test_footer_actions_open_the_existing_surfaces() {
            var shell = make();
            var pane = paneOf(shell);

            var navigated = [];
            pane.navigateRequested.connect(function(paneId) { navigated.push(paneId); });
            pane.hotCornersButton.clicked();
            compare(navigated.length, 1);
            compare(navigated[0], "mission-control");

            var sections = [];
            pane.sectionRequested.connect(function(paneId, section) {
                sections.push(paneId + "/" + section);
            });
            pane.shortcutsButton.clicked();
            compare(sections.length, 1);
            compare(sections[0], "keyboard/shortcuts");

            // No inline shortcut editor or hot-corner control is duplicated.
            compare(collectRowIds(pane, []).indexOf("topLeft"), -1);

            // The shell hands the shortcut action to the Keyboard pane.
            shell.openSection("keyboard", "shortcuts");
            tryVerify(function() { return shell.currentPaneId === "keyboard"; });
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                        && shell.highlightTarget !== null;
            }, 5000);
            compare(shell.currentPaneId, "keyboard");
        }

        function test_default_browser_choices_apply_and_persist() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.browserChoices.length >= 2,
                   "the fixture exposes installed browsers");
            compare(pane.defaultBrowserRow.enabled, true);
            compare(pane.defaultBrowserSelect.currentIndex, 0);

            pane.defaultBrowserSelect.activateIndex(1);
            compare(Settings.values["defaults.browser"],
                    "org.chromium.Chromium.desktop");
            compare(Settings.defaultBrowser, "org.chromium.Chromium.desktop");
            compare(pane.defaultBrowserSelect.currentIndex, 1);

            // An external change converges into the row.
            Settings.set("defaults.browser", "org.mozilla.firefox.desktop");
            tryVerify(function() {
                return pane.defaultBrowserSelect.currentIndex === 0;
            });
        }

        // Every rendered row is searchable under Desktop & Dock.
        function test_every_rendered_row_is_searchable() {
            var ids = ["size", "magnification", "position", "minimizeAnimation",
                       "titlebarDoubleClick", "minimizeIntoTileIcon", "autohide",
                       "animateOpening", "showIndicators", "showRecentApps",
                       "desktopItems", "clickWallpaper", "stage", "stageRecent",
                       "stageWindowMode", "showWidgets", "dimWidgets",
                       "defaultBrowser", "preferTabs", "askToKeepChanges",
                       "closeOnQuit", "dragToTile", "dragToFill", "tileModifier",
                       "tileMargins", "overview", "hotCorners", "shortcuts",
                       "chooserOnHover", "minimizeReaction", "windowRules"];
            var rows = SettingsMap.rowsFor("desktop-dock");
            compare(rows.length, ids.length);
            for (var r = 0; r < rows.length; ++r) {
                compare(rows[r].id, ids[r], "row " + r + " order");
                verify(SettingsSearch.rowSynonymsFor("desktop-dock",
                                                     rows[r].id).length > 0,
                       rows[r].id + " must carry search synonyms");
            }
            var results = SettingsSearch.search("workspace focus");
            var landed = results.filter(function(entry) {
                return entry.paneId === "desktop-dock" && entry.targetRow === "stage";
            });
            verify(landed.length > 0, "'workspace focus' must find the renamed row");
        }

        // T276: the measured 740×820 window must not clip or overflow the new
        // groups. Every row's control keeps its right-aligned slot.
        function test_measured_window_740x820_keeps_every_row_reachable() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("desktop-dock");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = paneOf(shell);

            var groups = [pane.dockGroup, pane.desktopGroup, pane.widgetsGroup,
                          pane.windowsGroup, pane.missionControlGroup];
            for (var g = 0; g < groups.length; ++g) {
                verify(groups[g].width > 0, "group " + g + " must have width");
                var rows = groups[g].rows.children;
                for (var i = 0; i < rows.length; ++i) {
                    var row = rows[i];
                    if (row.control === undefined)
                        continue; // a group description, not a SettingsRow
                    verify(row.width > 0, "row " + i + " in group " + g
                           + " must have width");
                    if (row.control.width === 0)
                        continue; // a disclosure-only row has no control
                    verify(row.control.x + row.control.width <= row.width + 1,
                           "row " + i + " in group " + g
                           + " control must stay inside the row");
                }
            }
            verify(pane.footer.width > 0);
            // The four footer actions share one right-aligned row; each stays
            // inside the content column so none is clipped at 740 pt.
            var buttons = [pane.advancedDockButton, pane.advancedWindowsButton,
                           pane.shortcutsButton, pane.hotCornersButton];
            for (var b = 0; b < buttons.length; ++b) {
                var point = buttons[b].mapToItem(pane, 0, 0);

                verify(buttons[b].visible, "footer action " + b + " must show");
                verify(point.x >= 0 && point.x + buttons[b].width
                       <= pane.width + 1,
                       "footer action " + b + " must stay inside the pane");
            }
        }
    }
}
