// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Menu Bar-pane tests (T-15.9b/T277). Headless with `DF_SETTINGS_FIXTURE`
// (see CMakeLists.txt), so the `Settings` singleton serves deterministic
// in-process settings and a deterministic third-party list with no bus. The
// cases cover the behavior rows, the Clock options, per-control options, the
// third-party StatusNotifier list, the Linux-only rows, Reset, an external
// change converging into the controls, and the absence note.
Item {
    id: stage
    width: 900
    height: 1600

    TestCase {
        id: testCase
        name: "SettingsMenuBar"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.set("menu.autoHide", "full-screen");
            Settings.set("menu.showBackground", true);
            Settings.set("menu.recentItems", 10);
            Settings.set("menu.osGlyph", "dragonfruit");
            Settings.set("menu.clock.showDate", true);
            Settings.set("menu.clock.showSeconds", false);
            Settings.set("menu.control.wifi", true);
            Settings.set("menu.control.bluetooth", true);
            Settings.set("menu.control.battery", true);
            Settings.set("menu.control.volume", true);
            Settings.set("menu.control.search", true);
            Settings.set("menu.control.notifications", true);
            Settings.set("menu.control.focus", true);
            Settings.set("menu.control.clock", true);
            Settings.set("menu.control.accessibility", true);
            Settings.set("menu.control.controlCenter", true);
            Settings.set("menu.control.workspaces", true);
            Settings.set("menu.control.focusAlways", false);
            Settings.set("menu.control.volumeWhenActive", false);
            Settings.set("power.showPercentage", false);
            Settings.set("menu.overflow", "automatic");
            Settings.setList("menu.tray.hidden", []);
            Settings.set("menu.global", true);
            Settings.set("quickSettings.perDisplay", false);
        }

        function make(width, height) {
            var shell = createTemporaryObject(shellComponent, stage, {
                width: width !== undefined ? width : stage.width,
                height: height !== undefined ? height : stage.height
            });
            waitForRendering(stage);
            shell.selectPane("menu-bar");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Menu Bar pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function controlToggle(pane, index) {
            var row = pane.controlsRepeater.itemAt(index);
            verify(row, "control row " + index + " must exist");
            return findChild(row, "menuBarControlToggle");
        }

        function liveControlKeys(pane) {
            var keys = [];
            for (var i = 0; i < pane.controlRows.length; ++i) {
                if (pane.controlRows[i].disabled !== true)
                    keys.push(pane.controlRows[i].key);
            }
            return keys;
        }

        function collectIds(item, out) {
            if (!item)
                return out;
            if (item.settingsId !== undefined
                    && String(item.settingsId).length > 0)
                out.push(String(item.settingsId));
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i)
                collectIds(kids[i], out);
            return out;
        }

        // T277: the rendered rows are the executable map's Menu Bar inventory.
        function test_rows_match_the_executable_map() {
            var shell = make();
            var pane = paneOf(shell);
            var rendered = collectIds(pane, []);
            var mapRows = SettingsMap.rowsFor("menu-bar");
            verify(mapRows.length > 0);
            for (var i = 0; i < mapRows.length; ++i) {
                verify(rendered.indexOf(mapRows[i].id) >= 0,
                       "map row " + mapRows[i].id + " has no rendered row");
            }
        }

        function test_pane_opens_with_the_schema_defaults() {
            var shell = make();
            compare(shell.currentPaneId, "menu-bar");
            var pane = paneOf(shell);
            compare(pane.autoHide, "full-screen");
            compare(pane.autoHideSelect.currentIndex, 2); // In Full Screen Only
            compare(pane.backgroundToggle.checked, true);
            compare(pane.recentItems, 10);
            compare(pane.recentSelect.currentIndex, 2); // 10
            compare(pane.clockShowDate, true);
            compare(pane.clockShowSeconds, false);
            compare(pane.osGlyph, "dragonfruit");
            compare(pane.glyphRepeater.count, 2);
            compare(pane.controlsRepeater.count, pane.controlRows.length);
            compare(pane.controlsRepeater.count, 12);
            compare(findChild(pane, "menuBarOverflowDescription") !== null, true,
                    "the pane explains the overflow policy");
            compare(pane.clockRow.settingsId, "clock");
            compare(pane.overflowPolicy, "automatic");
            compare(pane.workspacesVisible, true);
            compare(pane.globalMenuEnabled, true);
            compare(pane.perDisplayPanelRow.enabled, false);
        }

        function test_os_menu_glyph_choice_round_trips() {
            var shell = make();
            var pane = paneOf(shell);

            var dragonfruit = pane.glyphRepeater.itemAt(0);
            var tux = pane.glyphRepeater.itemAt(1);
            verify(dragonfruit && tux, "both glyph choices must render");
            compare(dragonfruit.selected, true);
            compare(tux.selected, false);
            compare(dragonfruit.Accessible.role, Accessible.RadioButton);
            compare(tux.Accessible.role, Accessible.RadioButton);

            tux.choose();
            compare(Settings.values["menu.osGlyph"], "tux");
            compare(pane.osGlyph, "tux");
            compare(tux.selected, true);
            compare(dragonfruit.selected, false);

            Settings.set("menu.osGlyph", "dragonfruit");
            compare(dragonfruit.selected, true);
            compare(tux.selected, false);
        }

        function test_behavior_rows_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.autoHideSelect.activateIndex(0); // Never
            compare(Settings.values["menu.autoHide"], "never");
            pane.autoHideSelect.activateIndex(1); // Always
            compare(Settings.values["menu.autoHide"], "always");

            pane.backgroundToggle.toggle();
            compare(Settings.values["menu.showBackground"], false);

            pane.recentSelect.activateIndex(0); // None
            compare(Settings.values["menu.recentItems"], 0);
            pane.recentSelect.activateIndex(6); // 50
            compare(Settings.values["menu.recentItems"], 50);
        }

        function test_control_visibility_toggles_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            var keys = liveControlKeys(pane);
            verify(keys.length > 0);
            for (var i = 0; i < pane.controlRows.length; ++i) {
                var row = pane.controlRows[i];
                var toggle = controlToggle(pane, i);
                verify(toggle, "toggle " + i + " must exist");
                if (row.disabled === true) {
                    compare(toggle.enabled, false,
                            row.label + " must not offer a fake toggle");
                    continue;
                }
                compare(toggle.checked, true);
                toggle.toggle();
                compare(Settings.values[row.key], false, row.key);
            }
        }

        function test_clock_row_and_options_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.clockVisibilityToggle.checked, true);
            pane.clockVisibilityToggle.toggle();
            compare(Settings.values["menu.control.clock"], false);

            pane.clockDialog.show();
            waitForRendering(stage);
            compare(pane.clockDialog.open, true);
            pane.clockDateToggle.toggle();
            compare(Settings.values["menu.clock.showDate"], false);
            pane.clockSecondsToggle.toggle();
            compare(Settings.values["menu.clock.showSeconds"], true);
            pane.clockDialog.accept();
            compare(pane.clockDialog.open, false);
        }

        function test_per_control_options_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            // Battery: the existing `power.showPercentage` consumer.
            pane.controlOptionsDialog.optionKind = "battery";
            pane.controlOptionsDialog.show();
            waitForRendering(stage);
            compare(pane.controlOptionsDialog.title, "Battery Options");
            pane.controlOptionsToggle.toggle();
            compare(Settings.values["power.showPercentage"], true);
            pane.controlOptionsDialog.accept();

            // Focus: show even when no Focus mode is on.
            pane.controlOptionsDialog.optionKind = "focus";
            pane.controlOptionsDialog.show();
            waitForRendering(stage);
            compare(pane.controlOptionsDialog.title, "Focus Options");
            pane.controlOptionsToggle.toggle();
            compare(Settings.values["menu.control.focusAlways"], true);
            pane.controlOptionsDialog.accept();

            // Sound: hide while muted.
            pane.controlOptionsDialog.optionKind = "sound";
            pane.controlOptionsDialog.show();
            waitForRendering(stage);
            compare(pane.controlOptionsDialog.title, "Sound Options");
            pane.controlOptionsToggle.toggle();
            compare(Settings.values["menu.control.volumeWhenActive"], true);
            pane.controlOptionsDialog.accept();

            // An external change converges into the open sheet's binding.
            Settings.set("power.showPercentage", false);
            pane.controlOptionsDialog.optionKind = "battery";
            pane.controlOptionsDialog.show();
            waitForRendering(stage);
            compare(pane.controlOptionsToggle.checked, false);
            pane.controlOptionsDialog.accept();
        }

        function test_third_party_status_notifier_list() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.thirdPartyMenuBarAvailable, true);
            compare(pane.thirdPartyItems.length, 2);
            compare(pane.thirdPartyRepeater.count, 2);

            var first = pane.thirdPartyRepeater.itemAt(0);
            var toggle = findChild(first, "menuBarThirdPartyToggle-0");
            verify(toggle);
            compare(toggle.checked, true);
            var name = String(pane.thirdPartyItems[0].name);
            toggle.toggle();
            compare(Settings.values["menu.tray.hidden"].indexOf(name) >= 0, true);
            compare(pane.trayAllowed(name), false);

            // An external change converges.
            Settings.setList("menu.tray.hidden", []);
            compare(pane.trayAllowed(name), true);
        }

        function test_linux_only_rows_have_real_owners() {
            var shell = make();
            var pane = paneOf(shell);

            pane.overflowSelect.activateIndex(1); // Always
            compare(Settings.values["menu.overflow"], "always");
            Settings.set("menu.overflow", "automatic");
            compare(pane.overflowSelect.currentIndex, 0);

            pane.workspacesToggle.toggle();
            compare(Settings.values["menu.control.workspaces"], false);

            pane.globalMenuToggle.toggle();
            compare(Settings.values["menu.global"], false);

            compare(pane.perDisplayPanelRow.enabled, false);
            verify(pane.perDisplayPanelRow.description.indexOf("one Quick") >= 0);
        }

        function test_reset_covers_the_menu_bar_keys() {
            var shell = make();
            var pane = paneOf(shell);
            pane.autoHideSelect.activateIndex(1);
            pane.backgroundToggle.toggle();
            pane.recentSelect.activateIndex(0);
            pane.overflowSelect.activateIndex(1);
            pane.workspacesToggle.toggle();
            pane.globalMenuToggle.toggle();
            Settings.set("menu.control.wifi", false);
            Settings.set("menu.control.focusAlways", true);
            Settings.set("power.showPercentage", true);
            Settings.setList("menu.tray.hidden", ["org.example.ExampleChat"]);

            pane.resetButton.clicked();
            compare(Settings.values["menu.autoHide"], "full-screen");
            compare(Settings.values["menu.showBackground"], true);
            compare(Settings.values["menu.recentItems"], 10);
            compare(Settings.values["menu.overflow"], "automatic");
            compare(Settings.values["menu.control.workspaces"], true);
            compare(Settings.values["menu.global"], true);
            compare(Settings.values["menu.control.wifi"], true);
            compare(Settings.values["menu.control.focusAlways"], false);
            compare(Settings.values["power.showPercentage"], false);
            compare(Settings.values["menu.tray.hidden"].length, 0);
            for (var i = 0; i < pane.controlRows.length; ++i) {
                if (pane.controlRows[i].disabled === true)
                    continue;
                compare(Settings.values[pane.controlRows[i].key], true,
                        pane.controlRows[i].key);
            }
        }

        function test_external_change_converges_into_the_pane() {
            var shell = make();
            var pane = paneOf(shell);

            Settings.set("menu.autoHide", "always");
            compare(pane.autoHide, "always");
            compare(pane.autoHideSelect.currentIndex, 1);

            Settings.set("menu.recentItems", 30);
            compare(pane.recentSelect.currentIndex, 5);

            Settings.set("menu.clock.showSeconds", true);
            compare(pane.clockSecondsToggle.checked, true);

            Settings.set("menu.control.wifi", false);
            compare(controlToggle(pane, 0).checked, false);

            Settings.set("menu.control.workspaces", false);
            compare(pane.workspacesVisible, false);
        }

        function test_add_controls_sheet_lists_the_live_controls() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addControlsDialog.show();
            waitForRendering(stage);
            compare(pane.addControlsDialog.open, true);
            var toggle = findChild(pane.addControlsDialog,
                                   "menuBarAddToggle-menu.control.wifi");
            verify(toggle);
            toggle.toggle();
            compare(Settings.values["menu.control.wifi"], false);
            pane.addControlsDialog.accept();
        }

        function test_absence_note_hides_when_the_daemon_is_present() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.available, true);
            compare(pane.absenceNote.visible, false);
        }

        function test_pane_fits_the_740_by_820_window() {
            var shell = make(740, 820);
            var pane = paneOf(shell);
            verify(pane.width <= 740, "pane width " + pane.width);
            compare(shell.width, 740);
            compare(shell.height, 820);
        }
    }
}
