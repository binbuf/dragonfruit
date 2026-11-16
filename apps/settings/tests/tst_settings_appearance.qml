// SPDX-License-Identifier: MIT
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

        function test_pane_opens_with_the_wired_controls() {
            var shell = make();
            compare(shell.currentPaneId, "appearance");
            var pane = paneOf(shell);
            compare(pane.schemeControl.currentIndex, 2); // auto
            compare(pane.accentRepeater.count, 6);
            compare(Settings.values["appearance.colorScheme"], "auto");
        }

        function test_scheme_selection_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.schemeControl.activateIndex(1); // dark
            compare(Settings.values["appearance.colorScheme"], "dark");
            compare(Theme.dark, true);
            compare(pane.schemeControl.currentIndex, 1);

            pane.schemeControl.activateIndex(0); // light
            compare(Settings.values["appearance.colorScheme"], "light");
            compare(Theme.dark, false);

            pane.schemeControl.activateIndex(2); // auto
            compare(Settings.values["appearance.colorScheme"], "auto");
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
        // owner mirrors it (the Appearance pane links to it in the hand-off).
        function test_reduced_motion_binding_is_live() {
            var shell = make();
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
            verify(pane.transparencyRow.description.indexOf("Light attached menus") >= 0,
                   "the control text must name the affected surface");
            verify(pane.transparencyRow.description.indexOf("Dock") >= 0,
                   "the control text must say which surfaces keep their material");
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
            verify(pane.schemeRow.control.x + pane.schemeRow.control.width
                       > pane.schemeRow.width * 0.8,
                   "the scheme control must sit at the right, right edge="
                   + (pane.schemeRow.control.x + pane.schemeRow.control.width)
                   + " row.width=" + pane.schemeRow.width);
            verify(pane.accentRow.control.x + pane.accentRow.control.width
                       > pane.accentRow.width * 0.8,
                   "the accent control must sit at the right, right edge="
                   + (pane.accentRow.control.x + pane.accentRow.control.width)
                   + " row.width=" + pane.accentRow.width);
            verify(pane.schemeControl.width > 0);
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
            var rows = [pane.schemeRow, pane.accentRow];
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
        }

        // -- T227: the remaining Appearance rows --------------------------------

        function test_highlight_swatches_apply_live_and_converge() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.highlightRepeater.count, 6);
            verify(pane.highlightRepeater.itemAt(0).selected); // Default

            pane.highlightRepeater.itemAt(1).choose(); // Violet
            compare(Settings.values["appearance.highlight"], "#b3a1ff");
            compare(Theme.highlightOverride, "#b3a1ff");
            verify(Qt.colorEqual(Theme.color.selection, "#b3a1ff"));
            verify(pane.highlightRepeater.itemAt(1).selected);

            Settings.set("appearance.highlight", "#7fd6a4");
            tryVerify(function() { return pane.highlightRepeater.itemAt(3).selected; });
            verify(Qt.colorEqual(Theme.color.selection, "#7fd6a4"));

            Settings.set("appearance.highlight", "");
            tryVerify(function() { return pane.highlightRepeater.itemAt(0).selected; });
            compare(Theme.highlightOverride, "");
        }

        function test_custom_highlight_picker_applies_and_validates() {
            var shell = make();
            var pane = paneOf(shell);
            pane.highlightPopup.show();
            waitForRendering(stage);
            pane.highlightInput.text = "#123456";
            pane.highlightInput.applyCustom();
            compare(Settings.values["appearance.highlight"], "#123456");
            compare(Theme.highlightOverride, "#123456");
            verify(Qt.colorEqual(Theme.color.selection, "#123456"));

            pane.highlightPopup.show();
            pane.highlightInput.text = "#nope";
            pane.highlightInput.applyCustom();
            compare(Settings.values["appearance.highlight"], "#123456");
        }

        function test_folder_color_applies_and_converges() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.folderColorRepeater.count, 6);
            compare(Theme.hasFolderColor, false);

            pane.folderColorRepeater.itemAt(1).choose(); // Blue
            compare(Settings.values["appearance.folderColor"], "#4a7dff");
            compare(Theme.folderColor, "#4a7dff");
            compare(Theme.hasFolderColor, true);
            verify(Qt.colorEqual(Theme.folderColorColor, "#4a7dff"));
            verify(pane.folderColorRepeater.itemAt(1).selected);

            pane.folderColorPopup.show();
            waitForRendering(stage);
            pane.folderColorInput.text = "#2bb673";
            pane.folderColorInput.applyCustom();
            compare(Settings.values["appearance.folderColor"], "#2bb673");
            compare(Theme.folderColor, "#2bb673");

            pane.folderColorRepeater.itemAt(0).choose(); // Automatic
            compare(Settings.values["appearance.folderColor"], "");
            compare(Theme.hasFolderColor, false);
        }

        function test_icon_style_and_theme_use_the_host_provider() {
            var shell = make();
            var pane = paneOf(shell);

            // The fixture provider has three real themes; Default maps to the
            // first, Dark to the matching variant, Clear/Tinted to nothing.
            compare(Settings.iconThemes().length, 4);
            verify(pane.iconStyleRow.enabled);
            compare(pane.iconStyleControl.entries.length, 4);
            verify(pane.iconStyleControl.entries[0].enabled);
            verify(pane.iconStyleControl.entries[1].enabled);
            verify(!pane.iconStyleControl.entries[2].enabled);
            verify(!pane.iconStyleControl.entries[3].enabled);

            pane.iconStyleControl.activateIndex(1); // Dark
            compare(Settings.values["appearance.iconStyle"], "dark");
            compare(Settings.effectiveIconTheme(), "Plain Dark");

            // A disabled style is rejected with its reason, not written.
            pane.iconStyleControl.activateIndex(2); // Clear
            compare(Settings.values["appearance.iconStyle"], "dark");
            verify(pane.iconStyleRejectedReason.indexOf("Clear") >= 0);
            verify(pane.iconStyleRow.description.indexOf("Clear") >= 0);

            // An explicit theme wins over the style mapping.
            pane.iconThemeSelect.activateIndex(3); // High Contrast
            compare(Settings.values["appearance.iconTheme"], "High Contrast");
            compare(Settings.effectiveIconTheme(), "High Contrast");

            Settings.set("appearance.iconTheme", "");
            compare(Settings.effectiveIconTheme(), "Plain Dark");
        }

        function test_icon_and_cursor_rows_disable_without_a_provider() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.setIconThemeFixture("none");
            Settings.setCursorThemeFixture("none");
            compare(Settings.iconThemes().length, 1);
            compare(pane.iconProviderAvailable, false);
            tryVerify(function() { return !pane.iconStyleRow.enabled; });
            verify(!pane.iconThemeRow.enabled);
            verify(!pane.cursorThemeRow.enabled);
            verify(pane.iconStyleRow.description.indexOf("No icon theme") >= 0);
            verify(pane.cursorThemeRow.description.indexOf("No cursor theme") >= 0);

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

        function test_scroll_bar_click_policy_applies_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Theme.scrollBarClick, "jump-to-next-page");
            compare(shell.detailScroll.scrollBarClickBehavior, "jump-to-next-page");

            pane.scrollBarClickSelect.activateIndex(1); // jump to spot
            compare(Settings.values["appearance.scrollBarClick"], "jump-to-spot");
            compare(Theme.scrollBarClick, "jump-to-spot");
            compare(shell.detailScroll.scrollBarClickBehavior, "jump-to-spot");

            Settings.set("appearance.scrollBarClick", "jump-to-next-page");
            tryVerify(function() { return pane.scrollBarClickSelect.currentIndex === 0; });
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

        function test_preview_renders_scheme_accent_highlight_and_folder() {
            var shell = make();
            var pane = paneOf(shell);
            pane.previewRollout = "replacement";

            Settings.set("appearance.colorScheme", "dark");
            tryVerify(function() { return pane.previewScheme === "Dark"; });

            Settings.set("appearance.accent", "#123456");
            tryVerify(function() {
                return Qt.colorEqual(pane.previewAccentChip.color, "#123456");
            });

            Settings.set("appearance.highlight", "#7fd6a4");
            tryVerify(function() {
                return Qt.colorEqual(pane.previewHighlightChip.color, "#7fd6a4");
            });

            Settings.set("appearance.folderColor", "#e0a43a");
            tryVerify(function() { return pane.previewFolderIcon.tintedFolder; });
            verify(Qt.colorEqual(Theme.folderColorColor, "#e0a43a"));

            // The material plate is the same resolver the live surfaces use.
            var expected = Glass.resolve("menuAttached", {
                scheme: "dark",
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
        }

        function test_a_new_row_write_failure_shows_inline() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.failNextWrite("appearance.scrollBarClick",
                                   "The settings service rejected the change.");
            pane.scrollBarClickSelect.activateIndex(1);
            tryVerify(function() { return pane.scrollBarClickRow.errorText.length > 0; });
            compare(pane.scrollBarClickRow.errorText,
                    "The settings service rejected the change.");
            verify(pane.scrollBarClickRow.errorLabel.visible);
            tryVerify(function() {
                return Settings.values["appearance.scrollBarClick"]
                        === "jump-to-next-page";
            });
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
            pane.scrollBarsSelect.activateIndex(1); // when scrolling
            compare(Settings.values["appearance.scrollBars"], "when-scrolling");
            compare(Theme.scrollBars, "when-scrolling");
            compare(shell.detailScroll.scrollBars, "when-scrolling");
        }

        function test_motion_link_opens_accessibility() {
            var shell = make();
            var pane = paneOf(shell);
            pane.motionButton.clicked();
            compare(shell.currentPaneId, "accessibility");
        }
    }
}