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
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
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

        function test_highlight_applies_live_and_converges() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.highlightSelect.currentIndex, 0); // Default

            pane.highlightSelect.activateIndex(1); // Violet
            compare(Settings.values["appearance.highlight"], "#b3a1ff");
            compare(Theme.highlightOverride, "#b3a1ff");
            verify(Qt.colorEqual(Theme.color.selection, "#b3a1ff"));

            Settings.set("appearance.highlight", "#7fd6a4");
            tryVerify(function() { return pane.highlightSelect.currentIndex === 3; });
            verify(Qt.colorEqual(Theme.color.selection, "#7fd6a4"));

            Settings.set("appearance.highlight", "");
            tryVerify(function() { return pane.highlightSelect.currentIndex === 0; });
            compare(Theme.highlightOverride, "");
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