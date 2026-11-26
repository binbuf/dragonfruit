// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Appearance-pane tests (T-09.2). Headless with `DF_SETTINGS_FIXTURE`, so the
// `Settings` singleton serves the schema defaults and keeps writes in memory:
// no bus, no daemon, deterministic. Each control is asserted to apply live —
// the settings key and the app-local `Theme` both change on the same
// event-loop turn, with no Apply button and no restart.
Item {
    id: stage
    width: 900
    height: 700

    TestCase {
        id: testCase
        name: "SettingsAppearance"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global, so reset the keys this pane
        // touches before every case.
        function init() {
            Settings.set("appearance.colorScheme", "auto");
            Settings.set("appearance.accent", "");
            Settings.set("appearance.transparency", 0.5255);
            Settings.set("appearance.highlight", "");
            Settings.set("appearance.sidebarIconSize", "medium");
            Settings.set("appearance.wallpaperTint", true);
            Settings.set("appearance.scrollBars", "automatic");
            Settings.set("appearance.scrollBarClick", "jump-to-next-page");
            Settings.set("appearance.folderColor", "");
            Settings.set("appearance.iconStyle", "default");
            Settings.set("appearance.iconTheme", "");
            Settings.set("appearance.cursorTheme", "");
            Settings.set("appearance.cursorSize", 24);
            Settings.setIconThemeFixture("present");
            Settings.setCursorThemeFixture("present");
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("appearance");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the appearance body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function schemeTile(pane, index) {
            var tile = pane.schemeTileRepeater.itemAt(index);
            verify(tile, "the scheme tile at index " + index + " must exist");
            return tile;
        }

        function iconStyleTile(pane, index) {
            var tile = pane.iconStyleTileRepeater.itemAt(index);
            verify(tile, "the icon-style tile at index " + index + " must exist");
            return tile;
        }

        // T290e: a stacked radio group inside a SettingsRow must keep every
        // radio circle and label inside the group, and the group inside the
        // row's control slot.
        function verifyRadioGroupFits(group, row) {
            verify(group.width > 0, "the radio group must have a width");
            verify(group.width <= row.width * 0.6 + 0.5,
                   "the group must stay inside the 60% control slot: group="
                   + group.width + " row=" + row.width);
            verify(row.control.x >= 0
                   && row.control.x + row.control.width <= row.width + 0.5,
                   "the control slot must stay inside the row");
            for (var i = 0; i < group.count; ++i) {
                var option = group.repeater.itemAt(i);
                verify(option, "radio option " + i + " must exist");
                verify(option.width > 0);
                var radio = option.children[0];
                var labelColumn = option.children[1];
                verify(radio && labelColumn);
                verify(radio.x >= 0
                       && radio.x + radio.width <= option.width + 0.5,
                       "radio " + i + " must stay inside the option row");
                verify(labelColumn.x >= radio.x + radio.width - 0.5,
                       "the label column must start after the radio");
                verify(labelColumn.x + labelColumn.width
                       <= option.width + 0.5,
                       "the label column must stay inside the option row");
            }
        }

        function collectGroups(item, out) {
            if (item === null || item === undefined)
                return out;
            if (item.sectionId !== undefined && item.titleText !== undefined)
                out.push(item);
            var children = item.children;
            for (var i = 0; i < children.length; ++i)
                collectGroups(children[i], out);
            return out;
        }

        function test_pane_opens_with_the_wired_controls() {
            var shell = make();
            compare(shell.currentPaneId, "appearance");
            var pane = paneOf(shell);
            compare(pane.schemeTileRepeater.count, 3);
            compare(schemeTile(pane, 2).selected, true); // auto
            compare(schemeTile(pane, 0).selected, false);
            compare(schemeTile(pane, 1).selected, false);
            compare(pane.accentRepeater.count, 6);
            compare(pane.iconStyleTileRepeater.count, 4);
            compare(pane.highlightSelect.currentLabel, "Default");
            compare(pane.folderColorSelect.currentLabel, "Automatic");
            compare(Settings.values["appearance.colorScheme"], "auto");
        }

        function test_pane_reports_the_source_revision_canary() {
            var shell = make();
            var pane = paneOf(shell);
            // T290a: the pane that actually loaded must come from the source
            // tree. A stale build copy or a stale compiled resource fails here
            // instead of silently testing the previous pane.
            compare(pane.sourceRevision, "t290f");
        }

        function test_theme_card_holds_the_four_controls_in_order() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.themeGroup.sectionId, "Theme");
            compare(pane.themeGroup.title, "Theme");

            // One card, in the mockup order: accent, Text highlight, icon
            // style, Folder color. T290e moved the sidebar icon size row to
            // the Windows card.
            var rows = pane.themeGroup.rows.children;
            compare(rows.length, 4);
            verify(rows[0] === pane.accentRow);
            verify(rows[1] === pane.highlightRow);
            verify(rows[2] === pane.iconStyleRow);
            verify(rows[3] === pane.folderColorRow);
            compare(pane.accentRow.settingsKey, "appearance.accent");
            compare(pane.highlightRow.settingsKey, "appearance.highlight");
            compare(pane.iconStyleRow.settingsKey, "appearance.iconStyle");
            compare(pane.folderColorRow.settingsKey, "appearance.folderColor");

            // The old two cards are gone, not merely renamed.
            var groups = collectGroups(pane, []);
            var hasTheme = false;
            for (var i = 0; i < groups.length; ++i) {
                verify(groups[i].sectionId !== "Color",
                       "the Color card must be merged into Theme");
                verify(groups[i].sectionId !== "Icon & widget style",
                       "the icon/style card must be merged into Theme");
                if (groups[i] === pane.themeGroup)
                    hasTheme = true;
            }
            verify(hasTheme, "the Theme group must be part of the pane");
        }

        // T290e: the Windows card owns the sidebar icon size popup and the
        // wallpaper tint toggle, in that order.
        function test_windows_card_holds_sidebar_size_and_wallpaper_tint() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.windowsGroup.sectionId, "Windows");
            compare(pane.windowsGroup.title, "Windows");

            var rows = pane.windowsGroup.rows.children;
            compare(rows.length, 2);
            verify(rows[0] === pane.sidebarIconSizeRow);
            verify(rows[1] === pane.wallpaperTintRow);
            compare(pane.sidebarIconSizeRow.settingsKey,
                    "appearance.sidebarIconSize");
            compare(pane.wallpaperTintRow.settingsKey,
                    "appearance.wallpaperTint");
            compare(pane.wallpaperTintRow.label,
                    "Tint window background with wallpaper color");

            // The controls are still the live ones, not re-created copies.
            verify(pane.sidebarIconSizeRow.control.children[0]
                   === pane.sidebarIconSizeSelect);
            verify(pane.wallpaperTintRow.control.children[0]
                   === pane.wallpaperTintToggle);
            compare(pane.wallpaperTintRow.settingsId, "wallpaperTint");
            compare(pane.wallpaperTintRow.owner, "settingsd + shell");
        }

        // T290f: the footer opens the Advanced Appearance sheet and the sheet
        // keeps the six Linux-only rows; the pane body no longer renders a
        // "Desktop integration" card.
        function test_footer_opens_and_closes_the_advanced_sheet() {
            var shell = make();
            var pane = paneOf(shell);

            verify(pane.advancedFooter);
            compare(pane.advancedButton.text, "Advanced Appearance…");
            compare(pane.accessibilityButton.text,
                    "More in Accessibility Settings…");
            compare(pane.advancedDialog.open, false);
            verify(pane.linuxGroup === undefined,
                   "the old Desktop integration card must be gone");

            pane.advancedButton.clicked();
            tryVerify(function() { return pane.advancedDialog.open; });
            compare(pane.advancedDialog.title, "Advanced Appearance");
            compare(pane.advancedDialog.Accessible.role, Accessible.Dialog);

            pane.advancedDialog.closeAction.clicked();
            tryVerify(function() { return !pane.advancedDialog.open; });
        }

        function test_advanced_sheet_holds_the_six_linux_rows() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.advancedGroup.sectionId, "Advanced Appearance");
            compare(pane.advancedGroup.title, "Advanced Appearance");

            var rows = pane.advancedGroup.rows.children;
            compare(rows.length, 6);
            verify(rows[0] === pane.iconThemeRow);
            verify(rows[1] === pane.cursorThemeRow);
            verify(rows[2] === pane.cursorSizeRow);
            verify(rows[3] === pane.windowControlsRow);
            verify(rows[4] === pane.materialRow);
            verify(rows[5] === pane.accentFromWallpaperRow);

            var expected = [
                ["iconTheme", "Icon theme", "appearance.iconTheme",
                 "settingsd + icon-theme provider"],
                ["cursorTheme", "Cursor theme", "appearance.cursorTheme",
                 "settingsd + compositor"],
                ["cursorSize", "Cursor size", "appearance.cursorSize",
                 "settingsd + compositor"],
                ["windowControls", "Window control layout",
                 "appearance.windowControls", "compositor/window decoration"],
                ["material", "Material effects", "appearance.material",
                 "shell/design-system Glass"],
                ["accentFromWallpaper", "Accent from wallpaper",
                 "appearance.accentFromWallpaper", "shell/design-system Theme"]
            ];
            for (var i = 0; i < expected.length; ++i) {
                var row = rows[i];
                compare(row.settingsId, expected[i][0]);
                compare(row.label, expected[i][1]);
                compare(row.settingsKey, expected[i][2]);
                compare(row.owner, expected[i][3]);
                verify(row.description.length > 0,
                       expected[i][0] + " must keep its description");
                compare(SettingsMap.rowFor("appearance", expected[i][0]).label,
                        expected[i][1]);
                compare(SettingsMap.rowFor("appearance",
                                           expected[i][0]).owner,
                        expected[i][3]);
            }

            // The three rows with no consumer stay visible and disabled with
            // their reason; they are never silently dropped.
            compare(pane.windowControlsRow.control.children[0].enabled, false);
            compare(pane.materialRow.control.children[0].enabled, false);
            compare(pane.accentFromWallpaperRow.control.children[0].enabled,
                    false);
            verify(pane.windowControlsRow.description.indexOf("not available") >= 0);
            verify(pane.materialRow.description.indexOf("automatically") >= 0);
            verify(pane.accentFromWallpaperRow.description.indexOf("wallpaper") >= 0);
        }

        function test_advanced_provider_rows_disable_without_a_provider() {
            var shell = make();
            var pane = paneOf(shell);
            // The advanced rows live in a closed modal, which is
            // input-disabled until it opens. Read their own enabled state
            // with the sheet open (ADR 0341).
            pane.openAdvanced();
            waitForRendering(shell);
            verify(pane.iconThemeRow.enabled);
            verify(pane.cursorThemeRow.enabled);

            Settings.setIconThemeFixture("none");
            Settings.setCursorThemeFixture("none");
            tryVerify(function() {
                return !pane.iconThemeRow.enabled
                    && !pane.cursorThemeRow.enabled;
            });
            verify(pane.iconThemeRow.description.indexOf("No icon theme") >= 0);
            verify(pane.cursorThemeRow.description.indexOf("No cursor theme") >= 0);
            compare(pane.advancedGroup.rows.children.length, 6);
        }

        function test_advanced_row_deep_links_open_the_sheet_and_land() {
            var shell = make();
            var pane = paneOf(shell);
            var ids = ["iconTheme", "cursorTheme", "cursorSize",
                       "windowControls", "material", "accentFromWallpaper"];

            for (var i = 0; i < ids.length; ++i) {
                if (pane.advancedDialog.open)
                    pane.closeAdvanced();
                compare(pane.advancedDialog.open, false);
                verify(shell.deepLink("settings://pane/appearance/row/"
                                      + ids[i]),
                       "deep link to " + ids[i] + " must be accepted");
                tryVerify(function() {
                    return pane.advancedDialog.open
                        && shell.highlightTarget !== null
                        && shell.highlightTarget.settingsId === ids[i];
                });
                compare(shell.currentPaneId, "appearance");
                compare(shell.highlightTarget.highlighted, true);
            }
        }

        // T290f: the whole Appearance map/search surface in one audit. Every
        // rendered row (the pane body and the Advanced sheet) has a map row
        // with the same section and label, and every map row is landable.
        function test_every_rendered_appearance_row_has_a_map_row() {
            var shell = make();
            var pane = paneOf(shell);

            var rendered = [];
            function collect(item) {
                if (item === null || item === undefined)
                    return;
                if (item.settingsId !== undefined
                        && String(item.settingsId).length > 0
                        && item.settingsKey !== undefined
                        && item.label !== undefined)
                    rendered.push(item);
                var kids = item.children;
                for (var i = 0; i < kids.length; ++i)
                    collect(kids[i]);
            }
            function enclosingGroup(item) {
                var node = item ? item.parent : null;
                while (node !== null && node !== undefined) {
                    if (node.sectionId !== undefined
                            && node.titleText !== undefined)
                        return node;
                    node = node.parent;
                }
                return null;
            }

            collect(pane);
            // The dialog adopts the window overlay layer, so its group is not
            // a descendant of the pane body; include it explicitly.
            collect(pane.advancedGroup);

            compare(rendered.length, 16,
                    "every rendered Appearance row must be collected");
            var seen = {};
            for (var i = 0; i < rendered.length; ++i) {
                var row = rendered[i];
                var mapped = SettingsMap.rowFor("appearance",
                                                String(row.settingsId));
                verify(mapped !== null,
                       "rendered row " + row.settingsId + " must be in the map");
                verify(seen[String(row.settingsId)] === undefined,
                       "row " + row.settingsId + " must be rendered once");
                seen[String(row.settingsId)] = true;
                compare(row.label, mapped.label,
                        row.settingsId + " rendered label");
                compare(row.settingsKey, mapped.key,
                        row.settingsId + " rendered key");
                var group = enclosingGroup(row);
                verify(group !== null,
                       row.settingsId + " must sit in a SettingsGroup");
                compare(group.sectionId, mapped.section,
                        row.settingsId + " map section");
                verify(SettingsMap.isRowLandable("appearance",
                                                 String(row.settingsId)),
                       row.settingsId + " must be landable");
            }
        }

        // T290f: the footer's second action lands on Accessibility -> Display.
        function test_accessibility_footer_lands_on_the_display_section() {
            var shell = make();
            var pane = paneOf(shell);

            pane.accessibilityButton.clicked();
            tryVerify(function() {
                return shell.currentPaneId === "accessibility"
                    && shell.paneBody.status === Loader.Ready
                    && shell.highlightTarget !== null
                    && shell.highlightTarget.sectionId === "display";
            });
            compare(shell.highlightTarget.sectionId, "display");
            compare(shell.highlightTarget.highlighted, true);
            compare(shell.pendingSection, "");
        }

        function test_scheme_row_handle_reaches_the_tile_row() {
            var shell = make();
            var pane = paneOf(shell);

            // T290a: tests read a row's control through a pane-body accessor
            // and the row's `control` slot, never through an id declared inside
            // an inline component.
            var handle = pane.schemeRowHandle;
            verify(handle);
            var control = handle.control.children[0];
            verify(control);
            verify(control === pane.schemeTiles);
            compare(pane.schemeTileRepeater.count, 3);
            compare(schemeTile(pane, 2).selected, true); // auto

            var dark = schemeTile(pane, 1);
            mouseClick(dark, dark.width / 2, dark.height / 2);
            compare(Settings.values["appearance.colorScheme"], "dark");
            compare(Theme.dark, true);
            tryVerify(function() { return dark.selected; });
        }

        function test_scheme_tiles_apply_live_on_click() {
            var shell = make();
            var pane = paneOf(shell);
            var light = schemeTile(pane, 0);
            var dark = schemeTile(pane, 1);
            var auto = schemeTile(pane, 2);

            mouseClick(dark, dark.width / 2, dark.height / 2);
            compare(Settings.values["appearance.colorScheme"], "dark");
            compare(Theme.dark, true);
            tryVerify(function() { return dark.selected; });
            compare(light.selected, false);

            mouseClick(light, light.width / 2, light.height / 2);
            compare(Settings.values["appearance.colorScheme"], "light");
            compare(Theme.dark, false);
            tryVerify(function() { return light.selected; });
            compare(dark.selected, false);

            mouseClick(auto, auto.width / 2, auto.height / 2);
            compare(Settings.values["appearance.colorScheme"], "auto");
            tryVerify(function() { return auto.selected; });
            compare(light.selected, false);
        }

        function test_scheme_tiles_follow_an_external_change() {
            var shell = make();
            var pane = paneOf(shell);
            var light = schemeTile(pane, 0);
            var dark = schemeTile(pane, 1);
            var auto = schemeTile(pane, 2);

            Settings.set("appearance.colorScheme", "dark");
            tryVerify(function() { return dark.selected; });
            compare(light.selected, false);
            compare(auto.selected, false);
            compare(dark.Accessible.checked, true);

            Settings.set("appearance.colorScheme", "light");
            tryVerify(function() { return light.selected; });
            compare(dark.selected, false);
            compare(dark.Accessible.checked, false);
            compare(light.Accessible.checked, true);

            Settings.set("appearance.colorScheme", "auto");
            tryVerify(function() { return auto.selected; });
            compare(light.selected, false);
            compare(auto.Accessible.checked, true);
        }

        function test_scheme_tiles_select_from_the_keyboard() {
            var shell = make();
            var pane = paneOf(shell);
            var light = schemeTile(pane, 0);
            var dark = schemeTile(pane, 1);
            var auto = schemeTile(pane, 2);

            light.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["appearance.colorScheme"], "light");
            tryVerify(function() { return light.selected; });

            dark.forceActiveFocus();
            keyClick(Qt.Key_Return);
            compare(Settings.values["appearance.colorScheme"], "dark");
            tryVerify(function() { return dark.selected; });

            // Left/Right are the radio-group movement keys.
            keyClick(Qt.Key_Right);
            compare(Settings.values["appearance.colorScheme"], "auto");
            tryVerify(function() { return auto.selected; });
            keyClick(Qt.Key_Left);
            compare(Settings.values["appearance.colorScheme"], "dark");
            tryVerify(function() { return dark.selected; });
        }

        function test_scheme_tiles_are_one_radio_group() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.schemeTiles.Accessible.role, Accessible.Grouping);
            compare(pane.schemeTiles.Accessible.name, "Appearance");
            for (var i = 0; i < pane.schemeTileRepeater.count; ++i) {
                var tile = schemeTile(pane, i);
                compare(tile.Accessible.role, Accessible.RadioButton);
                compare(tile.Accessible.checkable, true);
                compare(tile.Accessible.checked, tile.selected);
            }
        }

        function test_accent_swatch_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            var violet = pane.accentRepeater.itemAt(1);
            verify(violet);
            violet.choose();
            compare(Settings.values["appearance.accent"], "#7c5cff");
            compare(Theme.accentOverride, "#7c5cff");
            verify(Qt.colorEqual(Theme.color.accent, "#7c5cff"));
            verify(violet.selected);

            pane.accentRepeater.itemAt(0).choose(); // Default (token accent)
            compare(Settings.values["appearance.accent"], "");
            compare(Theme.accentOverride, "");
            verify(!violet.selected);
        }

        function test_custom_hex_picker_applies_and_validates() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.isHexColor("#1234ab"));
            verify(!pane.isHexColor("#1234"));
            verify(!pane.isHexColor("blue"));

            pane.accentPopup.show();
            waitForRendering(stage);
            pane.accentInput.text = "#123456";
            pane.accentInput.applyCustom();
            compare(Settings.values["appearance.accent"], "#123456");
            compare(Theme.accentOverride, "#123456");
            verify(Qt.colorEqual(Theme.color.accent, "#123456"));

            // Invalid input is rejected: the previous value stays.
            pane.accentPopup.show();
            pane.accentInput.text = "#nope";
            pane.accentInput.applyCustom();
            compare(Settings.values["appearance.accent"], "#123456");
        }

        // Reduced motion stays the Accessibility key, but the app-local Theme
        // owner mirrors it. T290e removed the Appearance Motion link; the key
        // and its binding are untouched.
        function test_reduced_motion_binding_is_live() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.motionRow === undefined);
            verify(pane.motionButton === undefined);
            Settings.set("accessibility.reduceMotion", true);
            compare(Theme.reducedMotion, true);
            Settings.set("accessibility.reduceMotion", false);
            compare(Theme.reducedMotion, false);
        }

        function test_transparency_control_opens_at_the_schema_default() {
            var shell = make();
            var pane = paneOf(shell);
            fuzzyCompare(pane.transparency, 0.5255, 0.0001);
            fuzzyCompare(pane.transparencySlider.value, 0.5255, 0.0001);
            compare(pane.glassGroup.sectionId, "Glass");
            compare(pane.glassGroup.title, "Glass");
            compare(pane.transparencyRow.label, "Glass tint");
            verify(pane.transparencyRow.description.indexOf("Attached menus only") >= 0,
                   "the control text must name the affected surface");
            verify(pane.transparencyRow.description.indexOf("Dock") >= 0,
                   "the control text must say which surfaces keep their material");
            verify(pane.transparencyRow.description.indexOf("Reduce Transparency") >= 0,
                   "the control text must say the accessibility override wins");
        }

        function test_transparency_drag_is_coalesced_and_committed() {
            var shell = make();
            var pane = paneOf(shell);
            var initial = Settings.values["appearance.transparency"];
            pane.transparencySlider.setValue(0.6);
            pane.transparencySlider.setValue(0.7);
            pane.transparencySlider.setValue(0.8);
            // The preview is immediate, but the durable owner has not seen the
            // intermediate drag samples.
            fuzzyCompare(pane.previewTransparency, 0.8, 0.0001);
            fuzzyCompare(Settings.values["appearance.transparency"], initial, 0.0001);
            pane.transparencySlider.commit();
            fuzzyCompare(Settings.values["appearance.transparency"], 0.8, 0.0001);
            fuzzyCompare(pane.transparency, 0.8, 0.0001);
        }

        function test_transparency_keyboard_commits_the_final_value() {
            var shell = make();
            var pane = paneOf(shell);
            var before = Settings.values["appearance.transparency"];
            pane.transparencySlider.forceActiveFocus();
            keyClick(Qt.Key_Right);
            tryVerify(function() {
                return Settings.values["appearance.transparency"] > before;
            });
            fuzzyCompare(Settings.values["appearance.transparency"],
                         pane.transparencySlider.value, 0.0001);
        }

        function test_transparency_follows_an_external_change() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("appearance.transparency", 0.2);
            tryVerify(function() {
                return Math.abs(pane.transparencySlider.value - 0.2) < 0.0001;
            });
            fuzzyCompare(pane.previewTransparency, 0.2, 0.0001);
        }

        function test_preview_uses_the_shared_preset_resolver() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("appearance.colorScheme", "light");
            pane.previewRollout = "replacement";

            pane.transparencySlider.setValue(0.0);
            var low = pane.previewResolved;
            compare(low.sliderApplied, true);
            pane.transparencySlider.setValue(1.0);
            var high = pane.previewResolved;
            verify(high.alpha < low.alpha,
                   "higher transparency must resolve to a clearer plate");

            var expected = Glass.resolve("menuAttached", {
                scheme: "light",
                rollout: "replacement",
                slider: 1.0,
                reduceTransparency: false,
                increaseContrast: false,
                tier: Glass.tier,
                software: Glass.software,
                sampledBlur: Glass.sampledBlur
            });
            fuzzyCompare(high.alpha, expected.alpha, 0.000001);
            compare(high.renderMode, expected.renderMode);
            pane.transparencySlider.commit();
        }

        function test_preview_keeps_the_accessibility_overrides() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("accessibility.reduceTransparency", true);
            Settings.set("accessibility.increaseContrast", true);
            pane.previewRollout = "replacement";
            var resolved = pane.previewResolved;
            compare(resolved.renderMode, "solid");
            compare(resolved.sliderApplied, false);
            verify(resolved.edge > 0.0, "Increase Contrast must boost the edge");
        }

        // Both rows place their control in the `controlData` slot, so it sits
        // at the right and cannot overlap the label (the T-09.3 follow-up (a)
        // regression).
        function test_row_controls_are_right_aligned() {
            var shell = make();
            var pane = paneOf(shell);
            var rows = [pane.schemeRow, pane.accentRow, pane.highlightRow,
                        pane.iconStyleRow, pane.folderColorRow];
            for (var r = 0; r < rows.length; ++r) {
                verify(rows[r].control.x + rows[r].control.width
                           > rows[r].width * 0.8,
                       "the control must sit at the right, right edge="
                       + (rows[r].control.x + rows[r].control.width)
                       + " row.width=" + rows[r].width);
            }
            verify(pane.schemeTiles.width > 0);
            compare(pane.schemeTileRepeater.count, 3);
            for (var i = 0; i < pane.schemeTileRepeater.count; ++i) {
                var tile = schemeTile(pane, i);
                verify(tile.x + tile.width <= pane.schemeTiles.width + 0.5,
                       "tile " + i + " must stay inside the tile row");
            }
            compare(pane.iconStyleTileRepeater.count, 4);
            for (var t = 0; t < pane.iconStyleTileRepeater.count; ++t) {
                var styleTile = iconStyleTile(pane, t);
                verify(styleTile.x + styleTile.width
                           <= pane.iconStyleTiles.width + 0.5,
                       "icon-style tile " + t + " must stay inside the tile row");
            }
            verify(pane.accentRepeater.count === 6);
        }

        // T-09.2 follow-up: at the default window size the detail pane is only
        // ~400 px wide, so the rows must fill it and every control must stay
        // inside its row instead of overflowing the card (the text-clipping bug
        // the narrower reference-matched window exposed).
        function test_controls_fit_the_default_window() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 640, height: 640 });
            waitForRendering(stage);
            shell.selectPane("appearance");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = paneOf(shell);
            var rows = [pane.schemeRow, pane.accentRow, pane.highlightRow,
                        pane.iconStyleRow, pane.folderColorRow];
            for (var i = 0; i < rows.length; ++i) {
                var row = rows[i];
                var control = row.control;
                verify(control.x + control.width <= row.width,
                       "control must fit inside its row: right="
                       + (control.x + control.width) + " row=" + row.width);
                verify(row.x + row.width <= shell.width,
                       "row must fit inside the window: right="
                       + (row.x + row.width) + " window=" + shell.width);
            }

            // The tile rows are taller than a stock row and their thumbnails
            // must stay inside the control slot at 640×640.
            verify(pane.schemeRow.height > Theme.controls.settingsRow.height * 2,
                   "the scheme row must grow for the tile thumbnails");
            compare(pane.schemeTileRepeater.count, 3);
            for (var t = 0; t < pane.schemeTileRepeater.count; ++t) {
                var tile = schemeTile(pane, t);
                verify(tile.x >= 0
                       && tile.x + tile.width <= pane.schemeTiles.width + 0.5,
                       "tile " + t + " must stay inside the tile row");
                verify(tile.height > 0);
            }
            verify(pane.iconStyleRow.height > Theme.controls.settingsRow.height * 2,
                   "the icon-style row must grow for the tile thumbnails");
            compare(pane.iconStyleTileRepeater.count, 4);
            for (var s = 0; s < pane.iconStyleTileRepeater.count; ++s) {
                var styleTile = iconStyleTile(pane, s);
                verify(styleTile.x >= 0
                       && styleTile.x + styleTile.width
                          <= pane.iconStyleTiles.width + 0.5,
                       "icon-style tile " + s + " must stay inside the tile row");
                verify(styleTile.height > 0);
            }

            // T290e: the stacked scroll-bar radios keep their circles and
            // labels inside the card at 640×640.
            verifyRadioGroupFits(pane.scrollBarsGroup, pane.scrollBarsRow);
            verifyRadioGroupFits(pane.scrollBarClickGroup, pane.scrollBarClickRow);
        }

        // -- T227: the remaining Appearance rows --------------------------------

        function test_highlight_popup_applies_live_and_converges() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.highlightSelectPopup.optionCount, 6);
            compare(pane.highlightSelect.currentLabel, "Default");

            // The popup is the single control: it shows the current swatch and
            // writes only when a palette entry is chosen.
            pane.highlightSelect.open();
            waitForRendering(stage);
            verify(pane.highlightSelect.opened);
            var violet = pane.highlightSelect.optionAt(1);
            verify(violet);
            violet.activate();
            compare(Settings.values["appearance.highlight"], "#b3a1ff");
            compare(Theme.highlightOverride, "#b3a1ff");
            verify(Qt.colorEqual(Theme.color.selection, "#b3a1ff"));
            tryVerify(function() {
                return pane.highlightSelect.currentLabel === "Violet";
            });
            verify(Qt.colorEqual(pane.highlightSelect.swatchColor, "#b3a1ff"));
            compare(pane.highlightSelect.opened, false);

            // An external write converges the label and swatch.
            Settings.set("appearance.highlight", "#7fd6a4");
            tryVerify(function() {
                return pane.highlightSelect.currentLabel === "Green";
            });
            verify(Qt.colorEqual(Theme.color.selection, "#7fd6a4"));

            // A custom hex set elsewhere is shown, never cleared.
            Settings.set("appearance.highlight", "#123456");
            tryVerify(function() {
                return pane.highlightSelect.currentLabel === "#123456";
            });
            compare(Settings.values["appearance.highlight"], "#123456");
            verify(Qt.colorEqual(pane.highlightSelect.swatchColor, "#123456"));

            Settings.set("appearance.highlight", "");
            tryVerify(function() {
                return pane.highlightSelect.currentLabel === "Default";
            });
            compare(Theme.highlightOverride, "");
        }

        function test_folder_color_popup_applies_and_converges() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.folderColorSelectPopup.optionCount, 6);
            compare(pane.folderColorSelect.currentLabel, "Automatic");
            compare(Theme.hasFolderColor, false);

            pane.folderColorSelect.open();
            waitForRendering(stage);
            pane.folderColorSelect.chooseIndex(1); // Blue
            compare(Settings.values["appearance.folderColor"], "#4a7dff");
            compare(Theme.folderColor, "#4a7dff");
            compare(Theme.hasFolderColor, true);
            verify(Qt.colorEqual(Theme.folderColorColor, "#4a7dff"));
            tryVerify(function() {
                return pane.folderColorSelect.currentLabel === "Blue";
            });

            // A custom hex set elsewhere is shown, never cleared.
            Settings.set("appearance.folderColor", "#123456");
            tryVerify(function() {
                return pane.folderColorSelect.currentLabel === "#123456";
            });
            compare(Settings.values["appearance.folderColor"], "#123456");
            compare(Theme.folderColor, "#123456");

            pane.folderColorSelect.open();
            pane.folderColorSelect.chooseIndex(0); // Automatic
            compare(Settings.values["appearance.folderColor"], "");
            compare(Theme.hasFolderColor, false);
            tryVerify(function() {
                return pane.folderColorSelect.currentLabel === "Automatic";
            });
        }

        function test_icon_style_tiles_write_and_reject() {
            var shell = make();
            var pane = paneOf(shell);

            // The fixture provider has three real themes; Default maps to the
            // first, Dark to the matching variant, Clear/Tinted to nothing.
            compare(Settings.iconThemes().length, 4);
            verify(pane.iconStyleRow.enabled);
            compare(pane.iconStyleTileRepeater.count, 4);

            var defaultTile = iconStyleTile(pane, 0);
            var darkTile = iconStyleTile(pane, 1);
            var clearTile = iconStyleTile(pane, 2);
            var tintedTile = iconStyleTile(pane, 3);
            verify(defaultTile.available);
            verify(darkTile.available);
            verify(!clearTile.available);
            verify(!tintedTile.available);
            verify(defaultTile.selected);
            compare(defaultTile.Accessible.role, Accessible.RadioButton);
            verify(clearTile.Accessible.description.indexOf("Clear") >= 0);

            darkTile.clicked();
            compare(Settings.values["appearance.iconStyle"], "dark");
            compare(Settings.effectiveIconTheme(), "Plain Dark");
            tryVerify(function() { return darkTile.selected; });
            compare(defaultTile.selected, false);

            // A style the provider cannot map is rejected with its reason and
            // is not written.
            clearTile.clicked();
            compare(Settings.values["appearance.iconStyle"], "dark");
            verify(pane.iconStyleRejectedReason.indexOf("Clear") >= 0);
            verify(pane.iconStyleRow.description.indexOf("Clear") >= 0);

            // Choosing an available style clears the stale rejection reason.
            darkTile.clicked();
            compare(Settings.values["appearance.iconStyle"], "dark");
            compare(pane.iconStyleRejectedReason, "");
            verify(pane.iconStyleRow.description.indexOf("No Clear icon theme") < 0);

            // An external write converges the selected ring.
            Settings.set("appearance.iconStyle", "default");
            tryVerify(function() { return defaultTile.selected; });
            compare(darkTile.selected, false);
            compare(defaultTile.Accessible.checked, true);

            // An explicit theme still wins over the style mapping.
            pane.iconThemeSelect.activateIndex(3); // High Contrast
            compare(Settings.values["appearance.iconTheme"], "High Contrast");
            compare(Settings.effectiveIconTheme(), "High Contrast");

            Settings.set("appearance.iconTheme", "");
            compare(Settings.effectiveIconTheme(), "Plain");
        }

        function test_icon_style_tiles_move_with_the_arrow_keys() {
            var shell = make();
            var pane = paneOf(shell);
            var defaultTile = iconStyleTile(pane, 0);

            defaultTile.forceActiveFocus();
            keyClick(Qt.Key_Right);
            compare(Settings.values["appearance.iconStyle"], "dark");
            tryVerify(function() { return iconStyleTile(pane, 1).selected; });

            keyClick(Qt.Key_Left);
            compare(Settings.values["appearance.iconStyle"], "default");
            tryVerify(function() { return defaultTile.selected; });
        }


        function test_icon_and_cursor_rows_disable_without_a_provider() {
            var shell = make();
            var pane = paneOf(shell);
            // The provider rows are in the closed Advanced Appearance sheet;
            // open it so their own enabled state is not masked by the modal's
            // input gate (ADR 0341).
            pane.openAdvanced();
            waitForRendering(shell);

            Settings.setIconThemeFixture("none");
            Settings.setCursorThemeFixture("none");
            compare(Settings.iconThemes().length, 1);
            compare(pane.iconProviderAvailable, false);
            tryVerify(function() { return !pane.iconStyleRow.enabled; });
            verify(!pane.iconThemeRow.enabled);
            verify(!pane.cursorThemeRow.enabled);
            verify(pane.iconStyleRow.description.indexOf("No icon theme") >= 0);
            verify(pane.cursorThemeRow.description.indexOf("No cursor theme") >= 0);
            for (var i = 0; i < pane.iconStyleTileRepeater.count; ++i) {
                verify(!iconStyleTile(pane, i).available,
                       "every style is unavailable without a provider");
            }

            Settings.setIconThemeFixture("present");
            Settings.setCursorThemeFixture("present");
            tryVerify(function() { return pane.iconStyleRow.enabled; });
            verify(pane.iconThemeRow.enabled);
            verify(pane.cursorThemeRow.enabled);
        }

        function test_cursor_theme_and_size_persist() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.cursorThemes().length, 3);

            pane.cursorThemeSelect.activateIndex(1);
            compare(Settings.values["appearance.cursorTheme"], "Plain Cursors");

            pane.cursorSizeSlider.setValue(32);
            pane.cursorSizeSlider.commit();
            compare(Settings.values["appearance.cursorSize"], 32);
            compare(pane.currentCursorSize, 32);

            Settings.set("appearance.cursorSize", 20);
            tryVerify(function() { return pane.cursorSizeSlider.value === 20; });
        }

        // T290e: the two scroll-bar policies are stacked radio groups over the
        // same values. Each card keeps its row metadata and inline error.
        function test_scroll_bar_groups_are_stacked_radio_groups() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.scrollBarsSection.title, "Show scroll bars");
            compare(pane.scrollBarsSection.sectionId, "Show scroll bars");
            compare(pane.scrollBarClickSection.title, "Click in the scroll bar to");
            compare(pane.scrollBarClickSection.sectionId,
                    "Click in the scroll bar to");

            // Stacked, not inline: one option per line.
            compare(pane.scrollBarsGroup.inline, false);
            compare(pane.scrollBarClickGroup.inline, false);
            compare(pane.scrollBarsGroup.count, 3);
            compare(pane.scrollBarClickGroup.count, 2);
            compare(pane.scrollBarsGroup.optionAt(0).label,
                    "Automatically based on mouse or trackpad");
            compare(pane.scrollBarsGroup.optionAt(1).label, "When scrolling");
            compare(pane.scrollBarsGroup.optionAt(2).label, "Always");
            compare(pane.scrollBarClickGroup.optionAt(0).label,
                    "Jump to the next page");
            compare(pane.scrollBarClickGroup.optionAt(1).label,
                    "Jump to the spot that's clicked");

            // One AT-SPI radio group each, with radio semantics per option.
            compare(pane.scrollBarsGroup.Accessible.role, Accessible.Grouping);
            compare(pane.scrollBarsGroup.Accessible.name, "Show scroll bars");
            compare(pane.scrollBarClickGroup.Accessible.role,
                    Accessible.Grouping);
            compare(pane.scrollBarClickGroup.Accessible.name,
                    "Click in the scroll bar to");
            for (var i = 0; i < pane.scrollBarsGroup.count; ++i) {
                var option = pane.scrollBarsGroup.repeater.itemAt(i);
                verify(option);
                compare(option.Accessible.role, Accessible.RadioButton);
                compare(option.Accessible.checkable, true);
                compare(option.Accessible.checked,
                        pane.scrollBarsGroup.currentIndex === i);
            }

            // The rows keep their map wiring and control slot.
            compare(pane.scrollBarsRow.control.children[0],
                    pane.scrollBarsGroup);
            compare(pane.scrollBarClickRow.control.children[0],
                    pane.scrollBarClickGroup);
            compare(pane.scrollBarsRow.settingsId, "scrollBars");
            compare(pane.scrollBarsRow.settingsKey, "appearance.scrollBars");
            compare(pane.scrollBarClickRow.settingsId, "scrollBarClick");
            compare(pane.scrollBarClickRow.settingsKey,
                    "appearance.scrollBarClick");
            compare(pane.scrollBarClickRow.owner, "settingsd + shell");

            // A stacked group is taller than the 33 pt row token, and the row
            // grows from the control instead of a fixed height.
            verify(pane.scrollBarsRow.height > Theme.controls.settingsRow.height);
            verify(pane.scrollBarClickRow.height > Theme.controls.settingsRow.height);
            verify(!pane.scrollBarsRow.fixedRowHeight);
            verify(!pane.scrollBarClickRow.fixedRowHeight);
            verifyRadioGroupFits(pane.scrollBarsGroup, pane.scrollBarsRow);
            verifyRadioGroupFits(pane.scrollBarClickGroup, pane.scrollBarClickRow);
        }

        function test_scroll_bar_click_policy_applies_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Theme.scrollBarClick, "jump-to-next-page");
            compare(shell.detailScroll.scrollBarClickBehavior, "jump-to-next-page");
            compare(pane.scrollBarClickGroup.currentIndex, 0);

            pane.scrollBarClickGroup.select(1); // jump to spot
            compare(Settings.values["appearance.scrollBarClick"], "jump-to-spot");
            compare(Theme.scrollBarClick, "jump-to-spot");
            compare(shell.detailScroll.scrollBarClickBehavior, "jump-to-spot");
            compare(pane.scrollBarClickGroup.currentIndex, 1);

            Settings.set("appearance.scrollBarClick", "jump-to-next-page");
            tryVerify(function() { return pane.scrollBarClickGroup.currentIndex === 0; });
            compare(shell.detailScroll.scrollBarClickBehavior, "jump-to-next-page");
        }

        function test_scroll_bar_modes_are_visually_distinct() {
            var shell = make();
            var pane = paneOf(shell);
            var scroll = shell.detailScroll;
            // The pane is taller than the viewport at 740×820, so the bar is
            // overflowing in every case.
            verify(scroll.maxY > 0);

            Settings.set("appearance.scrollBars", "always");
            tryVerify(function() { return scroll.showBars === true; });

            Settings.set("appearance.scrollBars", "when-scrolling");
            tryVerify(function() { return scroll.showBars === false; });

            Settings.set("appearance.scrollBars", "automatic");
            scroll.flickable.contentY = 0;
            scroll.forceActiveFocus();
            tryVerify(function() { return scroll.showBars === true; });
        }

        function test_scroll_bar_groups_move_with_the_keyboard() {
            var shell = make();
            var pane = paneOf(shell);

            var first = pane.scrollBarsGroup.repeater.itemAt(0);
            verify(first);
            first.forceActiveFocus();
            keyClick(Qt.Key_Down);
            compare(Settings.values["appearance.scrollBars"], "when-scrolling");
            compare(Theme.scrollBars, "when-scrolling");
            compare(shell.detailScroll.scrollBars, "when-scrolling");
            tryVerify(function() { return pane.scrollBarsGroup.currentIndex === 1; });
            compare(pane.scrollBarsGroup.repeater.itemAt(1).Accessible.checked,
                    true);

            keyClick(Qt.Key_Up);
            compare(Settings.values["appearance.scrollBars"], "automatic");
            tryVerify(function() { return pane.scrollBarsGroup.currentIndex === 0; });

            var clickFirst = pane.scrollBarClickGroup.repeater.itemAt(0);
            verify(clickFirst);
            clickFirst.forceActiveFocus();
            keyClick(Qt.Key_Down);
            compare(Settings.values["appearance.scrollBarClick"], "jump-to-spot");
            compare(Theme.scrollBarClick, "jump-to-spot");
            tryVerify(function() {
                return pane.scrollBarClickGroup.currentIndex === 1;
            });
        }

        function test_transparency_is_disabled_and_discarded_under_the_override() {
            var shell = make();
            var pane = paneOf(shell);
            var before = Settings.values["appearance.transparency"];

            Settings.set("accessibility.reduceTransparency", true);
            tryVerify(function() { return !pane.transparencyRow.enabled; });
            verify(!pane.transparencySlider.enabled);
            verify(pane.transparencyRow.description.indexOf("Reduce Transparency") >= 0);
            verify(pane.previewResolved.sliderApplied === false);

            // A drag, a commit, and a queued write all leave the stored value
            // alone: the slider cannot accept a write that cannot apply.
            pane.transparencySlider.setValue(0.9);
            pane.transparencySlider.commit();
            pane.queueTransparency(0.85);
            wait(120);
            fuzzyCompare(Settings.values["appearance.transparency"], before, 0.0001);
            fuzzyCompare(pane.previewTransparency, before, 0.0001);

            // A queued write in flight when the override turns on is dropped.
            Settings.set("accessibility.reduceTransparency", false);
            tryVerify(function() { return pane.transparencyRow.enabled; });
            pane.queueTransparency(0.75);
            Settings.set("accessibility.reduceTransparency", true);
            wait(120);
            fuzzyCompare(Settings.values["appearance.transparency"], before, 0.0001);

            Settings.set("accessibility.reduceTransparency", false);
        }

        // T290c: the Glass card is the preview. The scheme readout, the tint,
        // and the plate all come from the one resolver; the accent, highlight,
        // and folder chips that used to sit in the removed Preview group are
        // covered by the Theme control tests above and move fully to T290d.
        function test_glass_preview_renders_the_scheme_and_tint() {
            var shell = make();
            var pane = paneOf(shell);
            pane.previewRollout = "replacement";

            verify(pane.previewPlate);
            compare(pane.glassGroup.sectionId, "Glass");

            Settings.set("appearance.colorScheme", "dark");
            tryVerify(function() { return pane.previewScheme === "Dark"; });
            fuzzyCompare(pane.previewPlate.color.a,
                         pane.previewResolved.alpha, 0.001);

            // Only the light attached-menu preset follows the slider, so the
            // tint half of this case uses the light scheme.
            Settings.set("appearance.colorScheme", "light");
            tryVerify(function() { return pane.previewScheme === "Light"; });

            // The plate is the resolver output at both ends of the slider, not
            // a decorative blend that happens to look similar.
            pane.transparencySlider.setValue(0.0);
            var low = pane.previewResolved.alpha;
            tryVerify(function() {
                return Math.abs(pane.previewPlate.color.a - low) < 0.001;
            });
            pane.transparencySlider.setValue(1.0);
            var high = pane.previewResolved.alpha;
            verify(high < low,
                   "higher transparency must resolve to a clearer plate");
            tryVerify(function() {
                return Math.abs(pane.previewPlate.color.a - high) < 0.001;
            });

            var expected = Glass.resolve("menuAttached", {
                scheme: "light",
                rollout: "replacement",
                slider: pane.previewTransparency,
                reduceTransparency: false,
                increaseContrast: false,
                tier: Glass.tier,
                software: Glass.software,
                sampledBlur: Glass.sampledBlur
            });
            fuzzyCompare(pane.previewResolved.alpha, expected.alpha, 0.000001);
            compare(pane.previewResolved.renderMode, expected.renderMode);
            compare(pane.previewMode, expected.renderMode);
            compare(pane.previewPercent,
                    Math.round(pane.previewTransparency * 100) + "%");
            pane.transparencySlider.commit();
        }

        function test_long_labels_and_the_measured_window_keep_controls_inside() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("appearance");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = paneOf(shell);
            pane.labelSuffix = " — " + qsTr("a deliberately long localized label that "
                                           + "must elide instead of moving the control");

            var rows = [pane.schemeRow, pane.accentRow, pane.highlightRow,
                        pane.iconStyleRow, pane.folderColorRow,
                        pane.sidebarIconSizeRow, pane.scrollBarsRow,
                        pane.scrollBarClickRow, pane.transparencyRow,
                        pane.iconThemeRow, pane.cursorThemeRow, pane.cursorSizeRow];
            for (var i = 0; i < rows.length; ++i) {
                var row = rows[i];
                var control = row.control;
                verify(control.x + control.width <= row.width + 0.5,
                       "control must fit inside its row: right="
                       + (control.x + control.width) + " row=" + row.width);
                verify(row.x + row.width <= shell.width + 0.5,
                       "row must fit inside the window: right="
                       + (row.x + row.width) + " window=" + shell.width);
                verify(row.labelText.width <= row.width,
                       "long label must stay inside the row");
            }

            // T290e: the radio option labels may elide, but the circle and the
            // label column stay inside each option row at the measured window.
            verifyRadioGroupFits(pane.scrollBarsGroup, pane.scrollBarsRow);
            verifyRadioGroupFits(pane.scrollBarClickGroup, pane.scrollBarClickRow);
        }

        function test_a_new_row_write_failure_shows_inline() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.failNextWrite("appearance.scrollBarClick",
                                   "The settings service rejected the change.");
            pane.scrollBarClickGroup.select(1);
            tryVerify(function() { return pane.scrollBarClickRow.errorText.length > 0; });
            compare(pane.scrollBarClickRow.errorText,
                    "The settings service rejected the change.");
            verify(pane.scrollBarClickRow.errorLabel.visible);
            tryVerify(function() {
                return Settings.values["appearance.scrollBarClick"]
                        === "jump-to-next-page";
            });
            // The group snaps back to the stored value, not the rejected one.
            tryVerify(function() {
                return pane.scrollBarClickGroup.currentIndex === 0;
            });
            compare(Theme.scrollBarClick, "jump-to-next-page");
        }

        function test_sidebar_icon_size_applies_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(shell.sidebar.categoryIconSize, 20);

            pane.sidebarIconSizeSelect.activateIndex(2); // large
            compare(Settings.values["appearance.sidebarIconSize"], "large");
            compare(shell.sidebar.categoryIconSize, 24);
            compare(shell.sidebar.iconSize, 19);

            pane.sidebarIconSizeSelect.activateIndex(0); // small
            compare(shell.sidebar.categoryIconSize, 16);
        }

        function test_wallpaper_tint_toggle_changes_the_glass_resolver() {
            var shell = make();
            var pane = paneOf(shell);
            pane.previewRollout = "replacement";
            compare(pane.wallpaperTintRow.label,
                    "Tint window background with wallpaper color");
            compare(pane.previewResolved.wallpaperTint, true);

            pane.wallpaperTintToggle.toggled(false);
            compare(Settings.values["appearance.wallpaperTint"], false);
            compare(Glass.wallpaperTint, false);
            compare(pane.previewResolved.renderMode, "solid");
            compare(pane.previewResolved.fallbackReason, "wallpaper-tint-off");

            pane.wallpaperTintToggle.toggled(true);
            compare(Settings.values["appearance.wallpaperTint"], true);
            compare(Glass.wallpaperTint, true);
        }

        function test_scroll_bar_policy_applies_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(shell.detailScroll.scrollBars, "automatic");
            compare(pane.scrollBarsGroup.currentIndex, 0);
            pane.scrollBarsGroup.select(1); // when scrolling
            compare(Settings.values["appearance.scrollBars"], "when-scrolling");
            compare(Theme.scrollBars, "when-scrolling");
            compare(shell.detailScroll.scrollBars, "when-scrolling");
            compare(pane.scrollBarsGroup.currentIndex, 1);

            // The always value still restyles the shell live.
            pane.scrollBarsGroup.select(2);
            compare(Settings.values["appearance.scrollBars"], "always");
            compare(Theme.scrollBars, "always");
            tryVerify(function() { return shell.detailScroll.showBars === true; });

            Settings.set("appearance.scrollBars", "automatic");
            tryVerify(function() { return pane.scrollBarsGroup.currentIndex === 0; });
        }
    }
}