// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Wallpaper-pane tests (T-09.3, extended by T-18.2 and ADR 0240). Headless
// with `DF_SETTINGS_FIXTURE` and `DF_WALLPAPER_FIXTURE` (see CMakeLists.txt),
// so the `Settings` singleton serves the schema defaults and a deterministic
// in-process provider catalogue: no bus, no daemon. The cases cover the hero,
// category rows, Show All/Less, the fetching skeleton, ready/offline/error
// states, attribution (including HTML sanitization), the apply spinner,
// deferred downloads, and the eager Preload on pane open.
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

        function waitForApply(pane) {
            tryVerify(function() { return pane.pending === false; }, 5000);
            waitForRendering(stage);
        }

        function chooseAndWait(pane, tile) {
            tile.choose();
            waitForApply(pane);
        }

        function sectionSlugs(pane) {
            var out = [];
            for (var i = 0; i < pane.sections.length; ++i) {
                if (pane.sections[i].slug.length > 0)
                    out.push(pane.sections[i].slug);
            }
            return out;
        }

        // -- Hero and collection rows -----------------------------------------

        function test_pane_opens_with_the_hero_and_collections() {
            var shell = make();
            compare(shell.currentPaneId, "wallpaper");
            var pane = paneOf(shell);
            verify(pane.hero, "the hero must exist");
            verify(pane.heroPreview, "the hero preview must exist");
            verify(pane.featuredGroup, "the online collection must exist");
            verify(pane.builtinGroup, "the Built-in section must exist");
            verify(pane.customGroup, "the Your Photos section must exist");
            compare(pane.providerStatus, "ready");
            compare(pane.fitControl.currentIndex, 0); // fill
            compare(pane.showOnAllToggle.checked, true);
            verify(pane.photoButton, "the Add Photo tile must exist");
            verify(pane.sections.length >= 10,
                   "eight provider rows plus Built-in and Your Photos");
            compare(sectionSlugs(pane).join(","),
                    "abstract,nature,water,cityscapes,earth,space,scenery,underwater");
        }

        function test_default_tile_is_selected_when_no_user_choice() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.currentSource, "");
            compare(pane.selectedSource, pane.builtinDefault);
            verify(pane.builtinTiles[0].selected,
                   "the shipped Default is the out-of-box selection");
            compare(pane.currentName, "Default");
            verify(pane.currentPreviewUrl.indexOf("Default.jpg") >= 0);
            verify(!pane.hasAttribution);
        }

        // -- Featured: fetching skeleton --------------------------------------

        function test_fetching_shows_skeletons() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.setWallpaperFixture("fetching");
            waitForRendering(stage);

            compare(pane.providerStatus, "fetching");
            compare(pane.providerItems.length, 0);
            verify(pane.featuredFetching, "the provider is loading");
            verify(pane.skeletonItems.length > 0);
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
            verify(pane.retryButton.visible, "offline offers Try Again");

            Settings.setWallpaperFixture("error");
            waitForRendering(stage);
            compare(pane.providerStatus, "error");
            verify(pane.featuredMessageItem.visible);
            verify(pane.retryButton.visible);
        }

        // -- Selection and attribution ----------------------------------------

        function test_selecting_a_featured_tile_shows_the_apply_state() {
            var shell = make();
            var pane = paneOf(shell);
            waitForRendering(stage);

            var tile = pane.featuredTiles[1];
            var source = pane.featuredItems[1].source;
            tile.choose();
            verify(pane.pending, "the apply state starts immediately");
            verify(pane.heroSpinner.visible, "the hero shows the progress ring");
            verify(!pane.fitControl.visible, "the Fit popup yields to the ring");
            compare(pane.pendingPhase, "applying");

            waitForApply(pane);
            compare(Settings.values["wallpaper.source"], source);
            compare(pane.currentSource, source);
            compare(pane.currentName, "Ocean Wave");
            verify(tile.selected);
            verify(pane.hasAttribution, "a fetched picture shows its attribution");
            compare(pane.attributionLicense.text, "CC BY 4.0");
            verify(!pane.heroSpinner.visible);
            verify(pane.fitControl.visible);
        }

        function test_attribution_never_renders_provider_html() {
            var shell = make();
            var pane = paneOf(shell);
            waitForRendering(stage);

            chooseAndWait(pane, pane.featuredTiles[0]);
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

            chooseAndWait(pane, defaultTile);
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
            chooseAndWait(pane, tile);
            compare(Settings.values["wallpaper.source"], source);
            compare(pane.currentName, pane.presets[0].name);
            compare(pane.currentUrl, pane.presets[0].url);
            verify(tile.selected);

            var other = pane.builtinTiles[3];
            chooseAndWait(pane, other);
            verify(!tile.selected);
            verify(other.selected);
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
            verify(!pane.hasAttribution);

            chooseAndWait(pane, pane.builtinTiles[1]);
            compare(pane.currentSource, pane.presets[0].source);
        }

        function test_row_controls_are_right_aligned() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.showOnAllRow.control.x > pane.showOnAllRow.width * 0.6,
                   "the all-Workspaces control must sit at the right of the row, x="
                   + pane.showOnAllRow.control.x + " row.width="
                   + pane.showOnAllRow.width);
            verify(pane.showOnAllToggle.width > 0,
                   "the toggle must have a non-zero width");
        }

        function test_photo_choice_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            // A real, checked-in image so the apply state machine can load it;
            // it is not a built-in or provider source, so it stays a custom
            // photo.
            var photo = Qt.resolvedUrl("../../../tests/glass/fixtures/checker.png")
                    .toString().replace(/^file:\/\//, "");
            pane.applyPhoto(photo);
            verify(pane.pending, "the apply state starts immediately");
            verify(pane.pendingPhotoNotice.length > 0,
                   "a photo apply records its notice");
            waitForApply(pane);
            compare(Settings.values["wallpaper.source"], photo);
            compare(pane.currentName, "checker.png");
            verify(pane.photoNotice.text.length > 0,
                   "the apply notice survives a successful write");
            verify(!pane.hasAttribution, "a user photo carries no attribution");
            compare(pane.customTiles.length, 2,
                    "Add Photo plus the current custom photo");
        }

        // -- T229a: screen saver / clock appearance links ----------------------

        function test_screen_saver_and_clock_links_open_the_screen_saver() {
            var shell = make();
            var pane = paneOf(shell);
            compare(shell.currentPaneId, "wallpaper");
            verify(pane.screenSaverRow.visible);
            verify(pane.clockAppearanceRow.visible);

            pane.screenSaverButton.clicked();
            tryCompare(shell, "currentPaneId", "screen-saver");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });

            // Going back and using the clock link lands on the same pane.
            shell.selectPane("wallpaper");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            pane = paneOf(shell);
            pane.clockAppearanceButton.clicked();
            tryCompare(shell, "currentPaneId", "screen-saver");
        }

        function test_long_wallpaper_names_elide_without_widening_the_row() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("wallpaper.source",
                         "/tmp/a-very-long-wallpaper-name-that-keeps-going-and-going.png");
            compare(pane.currentName,
                    "a-very-long-wallpaper-name-that-keeps-going-and-going.png");
            verify(pane.currentUrl.length > 0);
            verify(pane.width <= stage.width,
                   "the pane must not grow past its host for a long name");
        }

        // -- ADR 0240: rows, Show All, deferred downloads ----------------------

        function test_show_all_expands_and_collapses() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWallpaperFixture("many");
            waitForRendering(stage);

            var nature = pane.sectionFor("nature");
            verify(nature, "the Nature section exists");
            compare(nature.items.length, 6);
            compare(nature.tileRepeater.count, 4);
            verify(nature.showAllButton.visible);
            verify(nature.showAllButton.text.indexOf("Show All (6)") >= 0);

            nature.toggleExpanded();
            waitForRendering(stage);
            compare(nature.tileRepeater.count, 6);
            verify(nature.showAllButton.text.indexOf("Show Less") >= 0);

            nature.toggleExpanded();
            waitForRendering(stage);
            compare(nature.tileRepeater.count, 4);
        }

        function test_deferred_item_downloads_then_applies() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWallpaperFixture("deferred");
            waitForRendering(stage);

            var space = pane.sectionFor("space");
            verify(space, "the Space section exists");
            compare(space.items.length, 1);
            compare(space.items[0].source, "",
                    "the NASA full image is deferred");
            var tile = space.tileRepeater.itemAt(0);
            tile.choose();
            verify(pane.pending);
            compare(pane.pendingPhase, "downloading");
            verify(pane.heroSpinner.visible);

            waitForApply(pane);
            compare(pane.currentName, "Space Nebula");
            verify(pane.hasAttribution);
            verify(pane.attributionArtist.text.indexOf("NASA") >= 0);
            compare(pane.attributionLicense.text, "Public domain (NASA)");
            verify(Settings.values["wallpaper.source"].length > 0);
        }

        function test_download_failure_keeps_the_old_wallpaper() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWallpaperFixture("deferred-fail");
            waitForRendering(stage);

            var before = Settings.values["wallpaper.source"];
            var space = pane.sectionFor("space");
            space.tileRepeater.itemAt(0).choose();
            waitForApply(pane);
            compare(Settings.values["wallpaper.source"], before,
                    "a failed download must not change the wallpaper");
            verify(pane.applyError.length > 0);
            verify(pane.pending === false);
        }

        function test_empty_provider_offers_try_again() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWallpaperFixture("offline");
            waitForRendering(stage);
            verify(pane.featuredMessageItem.visible);
            verify(pane.retryButton.visible);

            pane.retryButton.clicked();
            tryCompare(pane, "providerStatus", "ready", 3000);
            waitForRendering(stage);
            verify(!pane.featuredMessageItem.visible);
            verify(pane.featuredTiles.length > 0);
        }

        function test_reduced_motion_spinner_is_static() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("accessibility.reduceMotion", true);
            waitForRendering(stage);

            pane.featuredTiles[0].choose();
            verify(pane.pending);
            verify(!pane.heroSpinner.spinning,
                   "reduced motion holds the progress ring static");
            waitForApply(pane);
        }
    }
}
