// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Wallpaper-pane tests (T-09.3). Headless with `DF_SETTINGS_FIXTURE`, so the
// `Settings` singleton serves the schema defaults and keeps writes in memory:
// no bus, no daemon, deterministic. Each control is asserted to apply live —
// picking a built-in wallpaper, toggling "Show on all Spaces", and changing
// the fit all change the settings key on the same event-loop turn, and an
// external settingsd change flows back into the preview and selection.
Item {
    id: stage
    width: 900
    height: 660

    TestCase {
        id: testCase
        name: "SettingsWallpaper"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global, so reset the keys this pane
        // touches before every case.
        function init() {
            Settings.set("wallpaper.source", "");
            Settings.set("wallpaper.fit", "fill");
            Settings.set("wallpaper.showOnAllSpaces", true);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("wallpaper");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the wallpaper body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_wired_controls() {
            var shell = make();
            compare(shell.currentPaneId, "wallpaper");
            var pane = paneOf(shell);
            compare(pane.presets.length, 6);
            compare(pane.collections.length, 2);
            compare(pane.collections[0], "Dragonfruit");
            compare(pane.currentSource, "");
            compare(pane.currentName, "Default");
            compare(pane.fitControl.currentIndex, 0); // fill
            compare(pane.showOnAllToggle.checked, true);
            verify(pane.photoButton);
        }

        function test_choosing_a_preset_applies_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.tileItems.length, 6);

            var tile = pane.tileItems[1];
            var source = pane.presets[1].source;
            tile.choose();
            compare(Settings.values["wallpaper.source"], source);
            compare(pane.currentSource, source);
            compare(pane.currentName, pane.presets[1].name);
            compare(pane.currentUrl, pane.presets[1].url);
            verify(tile.selected);

            // Another pick moves the selection.
            pane.tileItems[3].choose();
            compare(pane.currentSource, pane.presets[3].source);
            verify(!tile.selected);
            verify(pane.tileItems[3].selected);
        }

        function test_show_on_all_spaces_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.showOnAllToggle.toggle();
            compare(Settings.values["wallpaper.showOnAllSpaces"], false);
            compare(pane.showOnAllSpaces, false);

            pane.showOnAllToggle.toggle();
            compare(Settings.values["wallpaper.showOnAllSpaces"], true);
        }

        function test_fit_selection_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.fitControl.activateIndex(1); // fit
            compare(Settings.values["wallpaper.fit"], "fit");
            compare(pane.currentFit, "fit");
            compare(pane.fitControl.currentIndex, 1);

            pane.fitControl.activateIndex(3); // center
            compare(Settings.values["wallpaper.fit"], "center");
            compare(pane.currentFit, "center");
        }

        function test_external_change_converges() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.set("wallpaper.source", "/tmp/dragonfruit-custom.png");
            compare(pane.currentSource, "/tmp/dragonfruit-custom.png");
            compare(pane.currentName, "dragonfruit-custom.png");
            compare(pane.currentUrl, "file:///tmp/dragonfruit-custom.png");

            // A preset picked after the external change wins.
            pane.tileItems[0].choose();
            compare(pane.currentSource, pane.presets[0].source);
        }

        // The SettingsRow lays the label on the left and the control on the
        // right; assert it so a future width regression (which would let the
        // control ride over the label) fails here.
        function test_row_controls_are_right_aligned() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.showOnAllRow.control.x > pane.width * 0.6,
                   "the all-Spaces control must sit at the right of the row, x="
                   + pane.showOnAllRow.control.x + " pane.width=" + pane.width);
            verify(pane.showOnAllToggle.width > 0,
                   "the toggle must have a non-zero width");
        }

        function test_photo_choice_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.applyPhoto("/home/user/Pictures/holiday.jpg");
            compare(Settings.values["wallpaper.source"], "/home/user/Pictures/holiday.jpg");
            compare(pane.currentName, "holiday.jpg");
            verify(pane.photoNotice.text.length > 0);
        }
    }
}