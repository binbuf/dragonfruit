// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Settings absent-provider matrix (T-09.6b).
//
// The headless half of the matrix: with no settingsd on the bus and no
// xdg-desktop-portal, every shipped Wave-1 pane still renders, every control
// stays functional, and writes are served from the schema defaults in memory.
// No pane is advertised that has no body (the no-half-panes rule), and the
// wallpaper "Add Photo…" row is the one control that reflects a missing
// provider (the portal) by disabling with an explanatory description.
//
// Run under a private `dbus-run-session` with **no** `DF_SETTINGS_FIXTURE`, so
// the `Settings` singleton is the live `DbusSettingsClient` and both providers
// are honestly absent (see CMakeLists.txt).
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // Every shipped Wave-1 pane, with the exact control surface the
        // matrix asserts stays live while its provider is absent.
        readonly property var shippedPaneIds:
            ["appearance", "desktop-dock", "displays", "wallpaper"]

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item, "pane body for " + id + " must load");
            return shell.paneBody.item;
        }

        // -- Both providers are absent -----------------------------------------

        function test_no_daemon_and_no_portal_are_the_absent_state() {
            compare(Settings.available, false,
                    "no settingsd on the bus is the absent-provider state under test");
            compare(Settings.wallpaperChooserAvailable, false,
                    "no portal on the bus is the absent-provider state under test");
        }

        // -- The no-half-panes catalog rule ------------------------------------

        function test_every_shipped_pane_has_a_body_and_no_other_does() {
            var shell = make();
            compare(SettingsPanes.shippedPanes.length, 4);
            for (var i = 0; i < SettingsPanes.catalog.length; ++i) {
                var pane = SettingsPanes.catalog[i];
                var body = shell.paneComponent(pane.id);
                if (pane.shipped)
                    verify(body !== null, "shipped pane " + pane.id + " must have a body");
                else
                    verify(body === null, "unshipped pane " + pane.id
                           + " must not advertise a body");
            }
            // The sidebar/search list is exactly the shipped subset.
            var listed = shell.visiblePanes.map(function(pane) { return pane.id; });
            compare(listed.sort().join(","), shippedPaneIds.slice().sort().join(","));
        }

        // -- Each pane degrades cleanly ----------------------------------------

        function test_appearance_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "appearance");
            verify(pane.schemeControl.enabled);
            compare(pane.accentRepeater.count, 6);
            verify(pane.accentRepeater.itemAt(0).enabled);

            // A write lands in the in-memory store even with no daemon.
            pane.schemeControl.activateIndex(1); // dark
            compare(Settings.values["appearance.colorScheme"], "dark");
            compare(Theme.dark, true);
            pane.schemeControl.activateIndex(0); // light
            compare(Settings.values["appearance.colorScheme"], "light");
        }

        function test_wallpaper_pane_disables_only_the_absent_portal_row() {
            var shell = make();
            var pane = showPane(shell, "wallpaper");

            // The portal-backed control is the only disabled one.
            compare(pane.photoButton.enabled, false,
                    "Add Photo… must be disabled when the FileChooser portal is absent");
            compare(pane.photoRow.description, "No file chooser is available.");
            verify(pane.showOnAllToggle.enabled);
            verify(pane.fitControl.enabled);
            compare(pane.tileItems.length, 6);

            // The settingsd-backed controls still write in memory.
            pane.tileItems[0].choose();
            compare(Settings.values["wallpaper.source"], pane.presets[0].source);
            pane.showOnAllToggle.toggle();
            compare(Settings.values["wallpaper.showOnAllSpaces"], false);
            pane.fitControl.activateIndex(2); // stretch
            compare(Settings.values["wallpaper.fit"], "stretch");
        }

        function test_desktop_dock_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "desktop-dock");
            compare(pane.dockGroup.rows.children.length, 10);
            var rows = pane.dockGroup.rows.children;
            for (var i = 0; i < rows.length; ++i)
                verify(rows[i].control.enabled, "Dock row " + i + " control must stay enabled");

            pane.autohideToggle.toggle();
            compare(Settings.values["dock.autohide"], true);
            pane.positionSelect.activateIndex(1); // left
            compare(Settings.values["dock.position"], "left");
            pane.sizeSlider.setValue(0.8);
            pane.sizeSlider.commit();
            verify(Math.abs(Settings.values["dock.size"] - 0.8) < 0.001);
        }

        function test_displays_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "displays");
            compare(pane.tileItems.length, 5);
            for (var i = 0; i < pane.tileItems.length; ++i)
                verify(pane.tileItems[i].enabled, "resolution tile " + i + " must stay enabled");
            verify(pane.rotationSelect.enabled);

            pane.tileItems[0].choose(); // Larger Text
            verify(Math.abs(Settings.values["display.scale"] - 1.5) < 0.001);
            pane.rotationSelect.activateIndex(2); // 180°
            compare(Settings.values["display.rotation"], "180");
        }

        // A control changed while the daemon is absent still converges into the
        // pane on the next event-loop turn (the in-memory store is the source
        // of truth for the session).
        function test_absent_writes_converge_into_the_controls() {
            var shell = make();
            var pane = showPane(shell, "desktop-dock");
            Settings.set("dock.showIndicators", false);
            compare(pane.showIndicatorsToggle.checked, false);
            Settings.set("dock.showIndicators", true);
            compare(pane.showIndicatorsToggle.checked, true);
        }
    }
}