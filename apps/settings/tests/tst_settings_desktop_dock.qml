// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Desktop & Dock-pane tests (T-09.4; T292a restyle; T292b merged Workspace
// Overview). Headless with `DF_SETTINGS_FIXTURE`, so the `Settings` singleton
// serves the schema defaults and keeps writes in memory: no bus, no daemon,
// deterministic. Every control is asserted to apply live to its `dock.*` key on
// the same event-loop turn, and an external settingsd change flows back into
// the control (the T-09.1b converge rule).
//
// T292a covers the mockup's section order and labels, the advanced sheets
// (their rows keep their keys), the footer actions, and the absence of the
// menu-bar row. T292b covers the merged Workspace Overview section (five
// toggles), its Advanced Workspace Overview sheet (workspace count, gestures,
// hot corners), and the removed Mission Control pane's deep link landing on the
// section.
Item {
    id: stage
    width: 900
    height: 660

    // T337: a free-running timer proves the GUI thread keeps processing events
    // while a slow app-index reply or default-apps subprocess is pending.
    property int responsivenessTicks: 0
    Timer {
        id: responsivenessTimer
        interval: 40
        repeat: true
        onTriggered: stage.responsivenessTicks += 1
    }

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
            Settings.setDefaultBrowserFixture("ready");
            Settings.set("overview.hotCornerTopLeft", "mission-control");
            Settings.set("overview.hotCornerTopRight", "notification-center");
            Settings.set("overview.hotCornerBottomLeft", "desktop-reveal");
            Settings.set("overview.hotCornerBottomRight", "lock-screen");
            Settings.set("overview.hotCornerTopLeftModifier", "none");
            Settings.set("overview.hotCornerTopRightModifier", "none");
            Settings.set("overview.hotCornerBottomLeftModifier", "none");
            Settings.set("overview.hotCornerBottomRightModifier", "none");
            Settings.set("overview.switchToAppSpace", true);
            Settings.set("gestures.missionControl", true);
            Settings.set("gestures.spaceSwitch", true);
            Settings.set("workspaces.count", 3);
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

        // The rendered, id-bearing rows of one group, in order. The merged
        // Workspace Overview group starts with an absence note, which carries
        // no `settingsId`.
        function rowLabels(group) {
            var labels = [];
            var rows = group.rows.children;
            for (var i = 0; i < rows.length; ++i) {
                if (rows[i].settingsId === undefined)
                    continue;
                labels.push(rows[i].label);
            }
            return labels;
        }

        // T292b: the four corner rows of the Advanced Workspace Overview
        // sheet, in order. They sit inside the sheet's width-constrained
        // column, so a direct-children walk of the group skips them.
        function cornerRows(pane) {
            var ids = ["topLeft", "topRight", "bottomLeft", "bottomRight"];
            var rows = [];
            for (var i = 0; i < ids.length; ++i)
                rows.push(pane.rowTargetFor(ids[i]));
            return rows;
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
        // T292b: the last one is the merged Workspace Overview.
        function test_sections_follow_the_mockup_order_and_labels() {
            var shell = make();
            var pane = paneOf(shell);
            compare(renderedGroupTitles(pane).join(","),
                    ["Dock", "Desktop & Workspaces", "Widgets", "Windows",
                     "Workspace Overview"].join(","));

            // Dock rows, in the mockup order and with the mockup labels.
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
            compare(rowLabels(pane.dockGroup).join("|"), dockLabels.join("|"));

            // Desktop & Workspaces, with the Stage -> Workspace Focus rename.
            var desktopLabels = [
                "Show items", "Click wallpaper to show desktop",
                "Workspace Focus", "Show recent apps in Workspace Focus",
                "Show windows from an application"
            ];
            compare(rowLabels(pane.desktopGroup).join("|"),
                    desktopLabels.join("|"));
            compare(pane.desktopItemsOptions[1].label, "In Workspace Focus");

            // Widgets: the two disabled widget rows and the live browser row.
            compare(rowLabels(pane.widgetsGroup).join("|"), ["Show Widgets",
                    "Dim widgets on desktop", "Default web browser"].join("|"));
            compare(pane.defaultBrowserRow.settingsId, "defaultBrowser");

            // Windows, with the Super modifier and both edge-tile labels.
            var windowLabels = [
                "Prefer tabs when opening documents",
                "Ask to keep changes when closing documents",
                "Close windows when quitting an application",
                "Drag windows to left or right edge of screen to tile",
                "Drag windows to menu bar to fill screen",
                "Hold Super key while dragging windows to tile",
                "Tiled windows have margins"
            ];
            compare(rowLabels(pane.windowsGroup).join("|"),
                    windowLabels.join("|"));

            // T292b: the merged Workspace Overview section, five toggles.
            compare(pane.missionControlGroup.title, "Workspace Overview");
            compare(pane.missionControlGroup.sectionId, "Workspace Overview");
            var overviewLabels = [
                "Automatically rearrange Workspaces", "Switch to an app's Workspace",
                "Group windows by application",
                "Displays have separate Workspaces",
                "Drag windows to the top to enter Workspace Overview"
            ];
            compare(rowLabels(pane.missionControlGroup).join("|"),
                    overviewLabels.join("|"));
            compare(collectRowIds(pane.missionControlGroup, []).join(","),
                    ["rearrange", "switchToAppSpace",
                     "groupByApp", "perDisplay", "dragToOpen"].join(","),
                    "the five toggles are the section; nothing is duplicated");
            // The controls are not duplicated inline: the workspace count, the
            // gestures, and the corner list live in the advanced sheet.
            compare(collectRowIds(pane.missionControlGroup, []).indexOf("workspaceCount"), -1);
            compare(collectRowIds(pane.missionControlGroup, []).indexOf("topLeft"), -1);
            compare(pane.overviewRow, undefined,
                    "the temporary Workspace Overview entry is gone");

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
            compare(pane.sourceRevision, "t292b");
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
        // adopts the overlay layer, so the sheet rows are collected through the
        // pane's row handles (T292a; T292b adds the Workspace Overview sheet).
        function collectRowIds(item, out) {
            if (item.settingsId !== undefined
                    && String(item.settingsId).length > 0)
                out.push(item.settingsId);
            if (item.windowRulesRowHandle !== undefined) {
                var sheet = [item.chooserOnHoverRowHandle,
                             item.minimizeReactionRowHandle,
                             item.windowRulesRowHandle,
                             item.workspaceCountRowHandle,
                             item.swipeUpRowHandle,
                             item.spaceSwitchRowHandle,
                             // The four corner rows are plain SettingsRows;
                             // rowTargetFor names them through the pane.
                             item.rowTargetFor("topLeft"),
                             item.rowTargetFor("topRight"),
                             item.rowTargetFor("bottomLeft"),
                             item.rowTargetFor("bottomRight")];
                for (var k = 0; k < sheet.length; ++k) {
                    if (sheet[k] !== null && sheet[k] !== undefined
                            && out.indexOf(sheet[k].settingsId) < 0)
                        out.push(sheet[k].settingsId);
                }
            }
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i)
                collectRowIds(kids[i], out);
            return out;
        }

        // T276/T292a/T292b: the rendered rows are the executable map's Desktop
        // & Dock inventory, not a second list. The Shortcuts…/Hot Corners…
        // footer actions keep their map ids (the shortcut editor is still
        // absent; Hot Corners… opens the advanced sheet).
        function test_rows_match_the_executable_map() {
            var shell = make();
            var pane = paneOf(shell);
            var rendered = collectRowIds(pane, []);
            var footerIds = { "shortcuts": pane.shortcutsButton,
                              "hotCorners": pane.hotCornersButton };
            var mapRows = SettingsMap.rowsFor("desktop-dock");
            verify(mapRows.length === 42,
                   "expected forty-two map rows, got " + mapRows.length);
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

        // The map's rowSurface answers "dialog" for exactly the sheet rows so a
        // deep link can open the sheet and land on the row.
        function test_advanced_rows_are_dialog_rows() {
            var dialogIds = ["chooserOnHover", "minimizeReaction", "windowRules",
                             "workspaceCount", "swipeUp", "spaceSwitch",
                             "topLeft", "topRight", "bottomLeft", "bottomRight"];
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

        // -- T292b: the merged Workspace Overview --------------------------

        // The five mockup toggles: two live keys, one read-only compositor
        // fact, and two honest disabled rows.
        function test_workspace_overview_toggles() {
            var shell = make();
            var pane = paneOf(shell);
            var group = pane.missionControlGroup;

            compare(pane.rearrangeRow.settingsKey, "overview.autoRearrange");
            compare(pane.groupByAppRow.settingsKey, "overview.groupByApp");
            compare(pane.dragToOpenRow.settingsKey, "overview.dragToOpen");
            verifyDisabledReason(pane.rearrangeRow, "Automatically rearrange Workspaces");
            verifyDisabledReason(pane.groupByAppRow, "Group windows by application");
            verifyDisabledReason(pane.dragToOpenRow,
                                 "Drag windows to the top to enter Workspace Overview");
            // Per-display workspaces is read-only: one workspace list per output.
            compare(SettingsMap.rowFor("desktop-dock", "perDisplay").absence,
                    "read-only");
            verify(pane.perDisplayRow.enabled, "the per-display row states a fact");
            verify(pane.perDisplayRow.description.length > 0);

            // Switch to an app's Space is live and converges both ways.
            compare(pane.switchToAppSpaceToggle.checked, true);
            pane.switchToAppSpaceToggle.toggle();
            compare(Settings.values["overview.switchToAppSpace"], false);
            Settings.set("overview.switchToAppSpace", true);
            compare(pane.switchToAppSpaceToggle.checked, true);
            compare(pane.switchToAppSpaceRow.settingsKey,
                    "overview.switchToAppSpace");

            // The merged section carries no inline control that duplicates the
            // advanced sheet.
            verify(group.rows.children.length > 0);
        }

        function test_advanced_workspace_overview_sheet_keeps_the_real_controls() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.advancedOverviewDialog.open, false);
            compare(pane.advancedOverviewDialog.title, "Advanced Workspace Overview");

            pane.hotCornersButton.clicked();
            tryVerify(function() { return pane.advancedOverviewDialog.open; });
            var group = pane.advancedOverviewGroup;
            compare(group.title, "Advanced Workspace Overview");
            // The top three rows are the group's own; the four corner rows sit
            // in the width-constrained column below the Hot Corners note.
            compare(rowLabels(group).join("|"),
                    ["Workspace count", "Swipe up to open",
                     "Swipe between Workspaces"].join("|"));
            var corners = cornerRows(pane);
            compare(corners.length, 4);
            compare(corners.map(function(row) { return row.label; }).join("|"),
                    ["Top Left", "Top Right", "Bottom Left",
                     "Bottom Right"].join("|"));
            compare(pane.hotCornersDescription.text.length > 0, true);

            // Workspace count.
            compare(pane.workspaceCountSelect.currentIndex, 2); // three spaces
            pane.workspaceCountSelect.activateIndex(4);         // five spaces
            compare(Settings.values["workspaces.count"], 5);
            Settings.set("workspaces.count", 2);
            tryVerify(function() {
                return pane.workspaceCountSelect.currentIndex === 1;
            });

            // Gestures.
            compare(pane.gestureMissionToggle.checked, true);
            pane.gestureMissionToggle.toggle();
            compare(Settings.values["gestures.missionControl"], false);
            Settings.set("gestures.missionControl", true);
            compare(pane.gestureMissionToggle.checked, true);
            pane.gestureSpaceToggle.toggle();
            compare(Settings.values["gestures.spaceSwitch"], false);
            Settings.set("gestures.spaceSwitch", true);
            compare(pane.gestureSpaceToggle.checked, true);

            // The four corners and their modifiers.
            compare(pane.topLeftSelect.currentIndex, 1);        // Overview
            compare(pane.bottomRightSelect.currentIndex, 4);    // Lock Screen
            pane.topLeftSelect.activateIndex(0);                // none
            compare(Settings.values["overview.hotCornerTopLeft"], "none");
            pane.bottomLeftSelect.activateIndex(1);             // Overview
            compare(Settings.values["overview.hotCornerBottomLeft"], "mission-control");
            pane.topRightSelect.activateIndex(2);               // Notifications
            compare(Settings.values["overview.hotCornerTopRight"], "notification-center");
            pane.topRightModifierSelect.activateIndex(1);       // Command
            compare(Settings.values["overview.hotCornerTopRightModifier"], "command");
            Settings.set("overview.hotCornerBottomRight", "desktop-reveal");
            tryVerify(function() {
                return pane.bottomRightSelect.currentIndex === 3;
            });

            // The full action vocabulary the compositor's wire uses.
            var expected = [
                "none", "mission-control", "notification-center",
                "desktop-reveal", "lock-screen", "launchpad",
                "control-center", "show-dock", "menu-bar", "focus-dock"
            ];
            compare(pane.actionOptions.length, expected.length);
            for (var i = 0; i < expected.length; ++i)
                compare(pane.actionOptions[i].value, expected[i]);

            // Deep links land on the sheet rows too. The reveal is deferred
            // (one `Qt.callLater` after the dialog opens), so wait for the
            // landing rather than for the sheet.
            verify(shell.deepLink("settings://pane/desktop-dock/row/workspaceCount"));
            tryVerify(function() {
                return shell.paneBody.item.advancedOverviewDialog.open
                        && shell.highlightTarget
                           === shell.paneBody.item.workspaceCountRowHandle;
            });
            compare(shell.currentRowId, "workspaceCount");

            pane.closeAdvancedOverview();
            tryVerify(function() { return !pane.advancedOverviewDialog.open; });
            compare(pane.advancedOverviewDialog.open, false);
            compare(pane.advancedDockDialog.open, false);
            compare(pane.advancedWindowsDialog.open, false);
        }

        // The former Mission Control pane is gone: its alias and deep link land
        // on the merged section, and the pane id is not a top-level pane.
        function test_mission_control_deep_link_lands_on_the_merged_section() {
            var shell = make();
            var pane = paneOf(shell);
            compare(SettingsPanes.paneById("mission-control"), null,
                    "the Overview pane is not a top-level pane");
            compare(SettingsPanes.resolve("mission-control").paneId,
                    "desktop-dock");
            compare(SettingsPanes.resolve("mission-control").anchor,
                    "Workspace Overview");
            compare(SettingsPanes.resolve("overview").anchor,
                    "Workspace Overview");
            compare(SettingsMap.rowsFor("mission-control").length, 0,
                    "the map keeps no removed-pane inventory");

            verify(shell.deepLink("settings://pane/mission-control"));
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            compare(shell.highlightTarget, shell.paneBody.item.missionControlGroup);
            compare(shell.highlightTarget.sectionId, "Workspace Overview");
            verify(shell.highlightTarget.highlighted);

            // The footer's Shortcuts… and Hot Corners… open existing surfaces
            // and no inline duplicate lives in the pane.
            var sections = [];
            pane.sectionRequested.connect(function(paneId, section) {
                sections.push(paneId + "/" + section);
            });
            pane.shortcutsButton.clicked();
            compare(sections.join(","), "keyboard/shortcuts");

            // The shell wires Shortcuts… to the Keyboard pane's section.
            shell.openSection("keyboard", "shortcuts");
            tryVerify(function() { return shell.currentPaneId === "keyboard"; });
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                        && shell.highlightTarget !== null;
            }, 5000);
            compare(shell.currentPaneId, "keyboard");
        }

        function test_footer_actions_open_the_existing_surfaces() {
            var shell = make();
            var pane = paneOf(shell);

            var navigated = [];
            pane.navigateRequested.connect(function(paneId) { navigated.push(paneId); });
            pane.hotCornersButton.clicked();
            compare(navigated.length, 0,
                    "Hot Corners… stays on this pane");
            tryVerify(function() { return pane.advancedOverviewDialog.open; });
            pane.closeAdvancedOverview();
            tryVerify(function() { return !pane.advancedOverviewDialog.open; });

            // No inline shortcut editor or hot-corner control is duplicated.
            compare(collectRowIds(pane.missionControlGroup, []).indexOf("topLeft"), -1);
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

        // T337: a slow app-index `Enumerate` must not freeze the Settings
        // window. The fixture stands in for a slow reply; the row keeps its
        // last known choices and shows the loading state until it lands.
        function test_slow_app_index_does_not_block_the_window() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.browserChoices.length >= 2);

            Settings.setDefaultBrowserFixture("slow-index");
            stage.responsivenessTicks = 0;
            responsivenessTimer.start();
            var started = Date.now();
            Settings.refreshDefaultBrowsers();
            var elapsed = Date.now() - started;
            verify(elapsed < 150,
                   "refreshDefaultBrowsers returned in " + elapsed + " ms");
            verify(Settings.defaultBrowserLoading,
                   "the row is loading while the reply is pending");
            compare(pane.defaultBrowserRow.enabled, false);
            verify(pane.browserChoices.length >= 2,
                   "the last known choices stay visible while loading");
            verify(pane.defaultBrowserRow.description.indexOf("Looking") >= 0,
                   "the row shows a loading state");

            wait(200);
            verify(Settings.defaultBrowserLoading,
                   "the slow reply is still pending after 200 ms");
            verify(stage.responsivenessTicks >= 2,
                   "the GUI thread processed " + stage.responsivenessTicks
                   + " timer ticks");
            responsivenessTimer.stop();

            tryVerify(function() { return !Settings.defaultBrowserLoading; }, 3000);
            compare(pane.browserChoices.length, 2);
            compare(pane.defaultBrowserRow.enabled, true);
            compare(Settings.defaultBrowserError, "");
            compare(pane.defaultBrowserRow.description.indexOf("Looking"), -1);
        }

        // T337: the host default-apps delegate runs through QProcess signals.
        // A slow delegate keeps the row responsive and the last known value.
        function test_slow_default_apps_subprocess_does_not_block() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setDefaultBrowserFixture("slow-subprocess");
            stage.responsivenessTicks = 0;
            responsivenessTimer.start();
            var started = Date.now();
            Settings.refreshDefaultBrowsers();
            var elapsed = Date.now() - started;
            verify(elapsed < 150,
                   "refreshDefaultBrowsers returned in " + elapsed + " ms");
            verify(Settings.defaultBrowserLoading);
            verify(pane.browserChoices.length >= 2,
                   "app-index choices arrive before the delegate");

            wait(200);
            verify(Settings.defaultBrowserLoading,
                   "the delegate is still running after 200 ms");
            verify(stage.responsivenessTicks >= 2,
                   "the GUI thread processed " + stage.responsivenessTicks
                   + " timer ticks");
            responsivenessTimer.stop();

            tryVerify(function() { return !Settings.defaultBrowserLoading; }, 4000);
            compare(Settings.defaultBrowser, "org.example.HostBrowser.desktop");
            compare(Settings.defaultBrowserError, "");
        }

        // T337: a hung delegate is killed at the timeout; it never blocks and
        // never spins forever.
        function test_hung_default_apps_subprocess_times_out() {
            var shell = make();
            Settings.setDefaultBrowserFixture("timeout-subprocess");
            var started = Date.now();
            Settings.refreshDefaultBrowsers();
            var elapsed = Date.now() - started;
            verify(elapsed < 150,
                   "refreshDefaultBrowsers returned in " + elapsed + " ms");
            verify(Settings.defaultBrowserLoading);
            tryVerify(function() { return !Settings.defaultBrowserLoading; }, 3000);
            var total = Date.now() - started;
            verify(total >= 250, "the timeout must actually expire (took "
                   + total + " ms)");
            verify(total < 2500, "the timeout must end the wait (took "
                   + total + " ms)");
            compare(Settings.defaultBrowserChoices.length, 2);
        }

        // T337: an absent app-index is an absence, not a spinner forever.
        function test_absent_app_index_shows_an_absence() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setDefaultBrowserFixture("absent");
            var started = Date.now();
            Settings.refreshDefaultBrowsers();
            verify(Date.now() - started < 100);
            tryVerify(function() { return !Settings.defaultBrowserLoading; }, 2000);
            compare(Settings.defaultBrowserChoices.length, 0);
            compare(pane.defaultBrowserRow.enabled, false);
            verify(pane.defaultBrowserRow.description.indexOf(
                       "No installed web browser") >= 0,
                   "the row states the absence");
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
                       "tileMargins", "chooserOnHover", "minimizeReaction",
                       "windowRules", "rearrange", "switchToAppSpace",
                       "groupByApp", "perDisplay", "dragToOpen", "hotCorners",
                       "shortcuts", "workspaceCount", "swipeUp", "spaceSwitch",
                       "topLeft", "topRight", "bottomLeft", "bottomRight"];
            var rows = SettingsMap.rowsFor("desktop-dock");
            compare(rows.length, ids.length);
            for (var r = 0; r < rows.length; ++r) {
                compare(rows[r].id, ids[r], "row " + r + " order");
                compare(rows[r].section,
                        ids[r] === "workspaceCount" || ids[r] === "swipeUp"
                        || ids[r] === "spaceSwitch" || ids[r] === "topLeft"
                        || ids[r] === "topRight" || ids[r] === "bottomLeft"
                        || ids[r] === "bottomRight"
                        ? "Advanced Workspace Overview" : rows[r].section,
                        ids[r] + " section");
                verify(SettingsSearch.rowSynonymsFor("desktop-dock",
                                                     rows[r].id).length > 0,
                       rows[r].id + " must carry search synonyms");
            }
            var results = SettingsSearch.search("workspace focus");
            var landed = results.filter(function(entry) {
                return entry.paneId === "desktop-dock" && entry.targetRow === "stage";
            });
            verify(landed.length > 0, "'workspace focus' must find the renamed row");

            // T292b: the moved names still land in the merged home.
            var moved = ["mission control", "workspace overview", "hot corners",
                         "swipe up", "workspace count", "switch to app space"];
            for (var q = 0; q < moved.length; ++q) {
                var found = SettingsSearch.search(moved[q]).filter(function(entry) {
                    return entry.paneId === "desktop-dock";
                });
                verify(found.length > 0, "'" + moved[q]
                       + "' must reach Desktop & Dock");
            }
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
                    if (row.settingsId === undefined)
                        continue; // the Workspace Overview absence note card
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

            // The advanced Workspace Overview sheet's rows stay inside their
            // 480 pt card, with both selects visible on each corner row.
            pane.openAdvancedOverview();
            tryVerify(function() { return pane.advancedOverviewDialog.open; });
            waitForRendering(stage);
            var sheetRows = pane.advancedOverviewGroup.rows.children;
            for (var s = 0; s < sheetRows.length; ++s) {
                var sheetRow = sheetRows[s];
                if (sheetRow.control === undefined)
                    continue;
                verify(sheetRow.control.x + sheetRow.control.width
                       <= sheetRow.width + 1,
                       "advanced row " + s + " control must stay inside the row");
            }
            compare(pane.workspaceCountSelect.width > 0, true);
            var corners = cornerRows(pane);
            compare(corners.length, 4);
            for (var c = 0; c < corners.length; ++c) {
                var row = corners[c];
                // The two popups are reached through the pane's handles; the
                // row's control slot (60% of the row) must hold both plus the
                // spacing between them.
                var first = [pane.topLeftSelect, pane.topRightSelect,
                             pane.bottomLeftSelect, pane.bottomRightSelect][c];
                var second = [pane.topLeftModifierSelect,
                              pane.topRightModifierSelect,
                              pane.bottomLeftModifierSelect,
                              pane.bottomRightModifierSelect][c];
                verify(row.control.width >= first.width + second.width,
                       "corner row " + c + " must fit both popups (" +
                       row.control.width + " vs " + first.width + "+" +
                       second.width + ")");
                var left = first.mapToItem(row, 0, 0).x;
                var right = second.mapToItem(row, 0, 0).x + second.width;
                verify(left >= 0 && right <= row.width + 1,
                       "corner row " + c + " popups must stay inside the row");
            }
            pane.closeAdvancedOverview();
        }
    }
}
