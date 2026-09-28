// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Wallpaper-pane tests (T-09.3, extended by T-18.2). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_WALLPAPER_FIXTURE` (see CMakeLists.txt), so
// the `Settings` singleton serves the schema defaults and a deterministic
// in-process provider catalogue: no bus, no daemon. The cases cover the three
// source rows (Featured/Built-in/Custom), the fetching skeleton, the
// ready/offline/error states, attribution (including HTML sanitization),
// selection into `wallpaper.source`, and the eager Preload on pane open.
Item {
    id: stage
    width: 900
    height: 720

    TestCase {
        id: testCase
        name: "SettingsWallpaper"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global, so reset the keys this pane
        // touches before every case, and reseed the provider to a known state.
        function init() {
            Settings.set("wallpaper.source", "");
            Settings.set("wallpaper.fit", "fill");
            Settings.set("wallpaper.showOnAllSpaces", true);
            Settings.set("accessibility.reduceMotion", false);
            Settings.setWallpaperFixture("ready");
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

        // -- Three source rows -------------------------------------------------

        function test_pane_opens_with_the_three_source_rows() {
            var shell = make();
            compare(shell.currentPaneId, "wallpaper");
            var pane = paneOf(shell);
            verify(pane.featuredGroup, "the Featured row must exist");
            verify(pane.builtinGroup, "the Built-in row must exist");
            verify(pane.customGroup, "the Custom row must exist");
            compare(pane.providerStatus, "ready");
            compare(pane.fitControl.currentIndex, 0); // fill
            compare(pane.showOnAllToggle.checked, true);
            verify(pane.photoButton);
        }

        // -- Featured: fetching skeleton --------------------------------------

        function test_fetching_shows_skeletons() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.setWallpaperFixture("fetching");
            waitForRendering(stage);

            compare(pane.providerStatus, "fetching");
            compare(pane.providerItems.length, 0);
            verify(pane.featuredFetching, "an empty fetching catalogue is loading");
            compare(pane.skeletonItems.length, 6);
            compare(pane.skeletonItems[0].Accessible.name, "Downloading wallpapers");
            compare(pane.skeletonItems[0].Accessible.role, Accessible.Graphic);
            verify(!pane.featuredMessageItem.visible,
                   "the empty note must not show while fetching");
        }

        function test_reduced_motion_stops_the_shimmer() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.set("accessibility.reduceMotion", true);
            Settings.setWallpaperFixture("fetching");
            waitForRendering(stage);

            verify(pane.skeletonItems.length > 0);
            verify(!pane.skeletonItems[0].shimmering,
                   "reduced motion must hold the skeleton static");

            Settings.set("accessibility.reduceMotion", false);
            waitForRendering(stage);
            verify(pane.skeletonItems[0].shimmering,
                   "with motion allowed the skeleton shimmers again");
        }

        // -- Featured: ready ---------------------------------------------------

        function test_ready_shows_provider_tiles_with_accessible_names() {
            var shell = make();
            var pane = paneOf(shell);
            waitForRendering(stage);

            compare(pane.featuredTiles.length, 2);
            compare(pane.skeletonItems.length, 0);
            verify(!pane.featuredMessageItem.visible);

            var label = pane.featuredTiles[0].Accessible.name;
            verify(label.indexOf("Nature") >= 0,
                   "the accessible name carries the category, was: " + label);
            verify(label.indexOf("Alpine Lake") >= 0,
                   "the accessible name carries the title, was: " + label);
            verify(label.indexOf("CC BY-SA 4.0") >= 0,
                   "the accessible name carries the license, was: " + label);
            compare(pane.featuredTiles[0].Accessible.role, Accessible.RadioButton);
            compare(pane.featuredTiles[0].Accessible.checkable, true);
        }

        // -- Featured: offline / error / idle ---------------------------------

        function test_offline_and_error_show_the_available_soon_note() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.setWallpaperFixture("offline");
            waitForRendering(stage);
            compare(pane.providerStatus, "offline");
            verify(pane.featuredEmpty);
            verify(pane.featuredMessageItem.visible,
                   "an offline provider shows the available-soon note");
            compare(pane.featuredMessageItem.text, pane.featuredMessage);
            verify(pane.featuredMessageItem.text.length > 0);
            compare(pane.featuredTiles.length, 0);
            verify(!pane.featuredFetching);

            Settings.setWallpaperFixture("error");
            waitForRendering(stage);
            compare(pane.providerStatus, "error");
            verify(pane.featuredMessageItem.visible);
        }

        // -- Selection and attribution ----------------------------------------

        function test_selecting_a_featured_tile_writes_the_source() {
            var shell = make();
            var pane = paneOf(shell);
            waitForRendering(stage);

            var tile = pane.featuredTiles[1];
            var source = pane.featuredItems[1].source;
            tile.choose();
            compare(Settings.values["wallpaper.source"], source);
            compare(pane.currentSource, source);
            compare(pane.currentName, "Ocean Wave");
            verify(tile.selected);
            verify(pane.hasAttribution, "a fetched picture shows its attribution");
            compare(pane.attributionLicense.text, "CC BY 4.0");
        }

        function test_attribution_never_renders_provider_html() {
            var shell = make();
            var pane = paneOf(shell);
            waitForRendering(stage);

            pane.featuredTiles[0].choose();
            compare(pane.currentItem.artist, "Alice Example");
            verify(pane.attributionArtist.text.indexOf("Alice Example") >= 0,
                   "the sanitized artist is shown");
            verify(pane.attributionArtist.text.indexOf("<") < 0,
                   "no markup reaches the artist label");
            compare(pane.attributionLicense.text, "CC BY-SA 4.0");
            compare(pane.currentLicenseUrl, pane.featuredItems[0].licenseUrl);
        }

        // -- Built-in: the shipped default -------------------------------------

        function test_builtin_row_includes_the_shipped_default_tile() {
            var shell = make();
            var pane = paneOf(shell);
            waitForRendering(stage);

            compare(pane.builtinTiles.length, 1 + pane.presets.length);
            var defaultTile = pane.builtinTiles[0];
            compare(defaultTile.modelData.name, "Default");
            compare(defaultTile.modelData.source, pane.builtinDefault);
            verify(pane.builtinDefault.indexOf("Default.jpg") >= 0,
                   "the shipped default is Default.jpg");

            defaultTile.choose();
            compare(Settings.values["wallpaper.source"], pane.builtinDefault);
            verify(defaultTile.selected);
            compare(pane.currentName, "Default");
            verify(!pane.hasAttribution,
                   "the shipped default carries no third-party attribution");
        }

        function test_choosing_a_preset_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            var tile = pane.builtinTiles[1];
            var source = pane.presets[0].source;
            tile.choose();
            compare(Settings.values["wallpaper.source"], source);
            compare(pane.currentName, pane.presets[0].name);
            compare(pane.currentUrl, pane.presets[0].url);
            verify(tile.selected);

            pane.builtinTiles[3].choose();
            verify(!tile.selected);
            verify(pane.builtinTiles[3].selected);
        }

        // -- The eager Preload on pane open ------------------------------------

        function test_opening_the_pane_requests_a_preload() {
            var before = Settings.providerPreloadCount;
            var shell = make();
            verify(Settings.providerPreloadCount > before,
                   "opening the Wallpaper pane must request a Preload");
            paneOf(shell);
        }

        // -- Existing behavior: fit and all-Spaces -----------------------------

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

            pane.builtinTiles[1].choose();
            compare(pane.currentSource, pane.presets[0].source);
        }

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
            verify(!pane.hasAttribution, "a user photo carries no attribution");
        }
    }
}