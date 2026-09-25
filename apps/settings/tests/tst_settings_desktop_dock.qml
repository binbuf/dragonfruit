// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Desktop & Dock-pane tests (T-09.4). Headless with `DF_SETTINGS_FIXTURE`, so
// the `Settings` singleton serves the schema defaults and keeps writes in
// memory: no bus, no daemon, deterministic. Every control is asserted to apply
// live to its `dock.*` key on the same event-loop turn, and an external
// settingsd change flows back into the control (the T-09.1b converge rule).
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
            Settings.set("dock.minimizedAnimation", "scale");
            Settings.set("dock.titlebarDoubleClick", "zoom");
            Settings.set("dock.minimizeIntoTileIcon", false);
            Settings.set("dock.autohide", false);
            Settings.set("dock.animateOpening", true);
            Settings.set("dock.showIndicators", true);
            Settings.set("dock.showRecentApps", false);
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

        function test_toggles_apply_live() {
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

        function test_external_change_converges_for_toggles() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.set("dock.autohide", true);
            compare(pane.autohideToggle.checked, true);
            Settings.set("dock.showIndicators", false);
            compare(pane.showIndicatorsToggle.checked, false);
            Settings.set("dock.minimizeIntoTileIcon", true);
            compare(pane.minimizeIntoTileToggle.checked, true);
        }

        // The SettingsRow lays the label on the left and the control on the
        // right; assert it for every row so a future width regression (which
        // would let a control ride over its label) fails here.
        function test_row_controls_are_right_aligned() {
            var shell = make();
            var pane = paneOf(shell);
            var rows = pane.dockGroup.rows.children;
            verify(rows.length === 10, "expected ten Dock rows, got " + rows.length);
            for (var i = 0; i < rows.length; ++i) {
                var row = rows[i];
                verify(row.control.x > row.width * 0.55,
                       "row " + i + " control must sit at the right, x="
                       + row.control.x + " row.width=" + row.width);
                verify(row.control.width > 0, "row " + i + " control must have width");
            }
        }
    }
}