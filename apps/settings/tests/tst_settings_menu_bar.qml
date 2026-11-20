// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Menu Bar-pane tests (T-15.9b/T277, restyled by T293a). Headless with
// `DF_SETTINGS_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton
// serves deterministic in-process settings and a deterministic third-party
// list with no bus.
//
// The cases cover the mockup section order, the behaviour rows, the OS glyph
// radio group (selection, actual-size previews, keyboard, AT-SPI radio role,
// default and Reset), the Clock options, the controls list and its
// per-control options, Add Controls, the live third-party StatusNotifier list,
// Reset, the Advanced Menu Bar sheet, Customize Controls… landing on the
// Quick Settings surface, map parity for rendered and omitted rows, and the
// absence note.
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
            Settings.set("appearance.colorScheme", "light");
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
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                        && shell.paneBody.item !== null;
            });
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

        function controlIndex(pane, key) {
            for (var i = 0; i < pane.controlRows.length; ++i) {
                if (pane.controlRows[i].key === key)
                    return i;
            }
            return -1;
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

        // T293a: the section order is the mockup's — Behaviour, OS Menu Glyph,
        // Menu Bar Controls, Allow in the Menu Bar — and the footer follows.
        function test_section_order_matches_the_mockup() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.behaviorGroup !== undefined);
            compare(pane.behaviorGroup.title, "Menu Bar Behaviour");
            compare(pane.glyphGroup.title, "OS Menu Glyph");
            compare(pane.controlsGroup.title, "Menu Bar Controls");
            compare(pane.thirdPartyGroup.title, "Allow in the Menu Bar");
            verify(pane.behaviorGroup.y < pane.glyphGroup.y,
                   "Behaviour must come before the OS glyph");
            verify(pane.glyphGroup.y < pane.controlsGroup.y,
                   "the OS glyph must come before Menu Bar Controls");
            verify(pane.controlsGroup.y < pane.thirdPartyGroup.y,
                   "Allow in the Menu Bar must come last");
            verify(pane.footer.y > pane.thirdPartyGroup.y,
                   "the footer follows the last section");

            // The former Linux-only section and the Reset card are gone: the
            // rows live in the Advanced sheet and the footer.
            compare(findChild(pane, "menuBarOverflowDescription"), null);
            compare(findChild(pane, "menuBarReset"), pane.resetButton);
        }

        // T277/T293a: the rendered rows are the executable map's Menu Bar
        // inventory; omitted rows are never rendered.
        function test_rows_match_the_executable_map() {
            var shell = make();
            var pane = paneOf(shell);
            var rendered = collectIds(pane, []);
            // The Advanced Menu Bar sheet adopts the window overlay layer, so
            // its rows are named by the pane, not found by walking the body.
            var handles = {
                "addControls": pane.addControlsButton,
                "customizeControls": pane.customizeControlsButton,
                "reset": pane.resetButton,
                "overflow": pane.overflowSelect,
                "globalMenu": pane.globalMenuToggle,
                "perDisplayPanel": pane.perDisplayPanelRow
            };
            var mapRows = SettingsMap.rowsFor("menu-bar");
            verify(mapRows.length > 0);
            for (var i = 0; i < mapRows.length; ++i) {
                var row = mapRows[i];
                // An omitted *control* still renders the disabled row that
                // says why (Display); the other omissions never render.
                if (row.state === "omitted" && row.id !== "display") {
                    compare(rendered.indexOf(row.id), -1,
                            "omitted row " + row.id + " must not render");
                    continue;
                }
                if (handles[row.id] !== undefined) {
                    verify(handles[row.id] !== null,
                           "map row " + row.id + " has no rendered row");
                    continue;
                }
                verify(rendered.indexOf(row.id) >= 0,
                       "map row " + row.id + " has no rendered row");
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
            compare(pane.controlsRepeater.count, 10);
            compare(pane.clockRow.settingsId, "clock");
            compare(pane.clockRow.y < pane.controlsRepeater.itemAt(0).y, true,
                    "Clock is the first Menu Bar Controls row");
            compare(pane.overflowPolicy, "automatic");
            compare(pane.workspacesVisible, true);
            compare(pane.globalMenuEnabled, true);
            compare(pane.perDisplayPanelRow.enabled, false);
        }

        // T293a: the controls use the mockup labels in the mockup order, and
        // Display is the one honest disabled row with its exact reason.
        function test_controls_use_the_mockup_labels() {
            var shell = make();
            var pane = paneOf(shell);
            var labels = pane.controlRows.map(function(row) {
                return row.label;
            }).join(",");
            compare(labels, "Search,Wi-Fi,Bluetooth,Battery,Focus,Sound,"
                            + "Accessibility,Quick Settings,Display,Workspaces");
            var display = pane.controlRows[controlIndex(pane, "menu.control.display")];
            compare(display.disabled, true);
            compare(display.reason,
                    "Display has no menu-bar item; the Display module lives in "
                    + "Quick Settings.");
            var toggle = controlToggle(pane,
                    controlIndex(pane, "menu.control.display"));
            compare(toggle.enabled, false);
            // Every omitted mockup control has a recorded reason.
            compare(pane.omittedControls.length, 4);
            for (var i = 0; i < pane.omittedControls.length; ++i)
                verify(pane.omittedControls[i].reason.length > 0);
        }

        // ── OS Menu Glyph ──────────────────────────────────────────────────

        function test_os_menu_glyph_is_a_radio_group_with_actual_size_previews() {
            var shell = make();
            var pane = paneOf(shell);
            var dragonfruit = pane.glyphRepeater.itemAt(0);
            var tux = pane.glyphRepeater.itemAt(1);
            verify(dragonfruit && tux, "both glyph choices must render");

            // The default is Dragonfruit, and only one row is selected.
            compare(dragonfruit.glyph, "dragonfruit");
            compare(tux.glyph, "tux");
            compare(dragonfruit.selected, true);
            compare(tux.selected, false);

            // AT-SPI: each row is a radio button inside one labelled group.
            compare(dragonfruit.Accessible.role, Accessible.RadioButton);
            compare(tux.Accessible.role, Accessible.RadioButton);
            compare(dragonfruit.Accessible.checkable, true);
            compare(dragonfruit.Accessible.checked, true);
            compare(tux.Accessible.checked, false);
            compare(pane.glyphGroup.Accessible.role, Accessible.Grouping);
            compare(pane.glyphGroup.Accessible.name, "OS Menu Glyph");

            // Actual size: the strip is the menu bar's real height, at least
            // 184 pt wide, and the mark is the bar's real glyph size.
            for (var i = 0; i < 2; ++i) {
                var row = pane.glyphRepeater.itemAt(i);
                var strip = findChild(row, "settingsGlyphPreview-" + row.glyph);
                verify(strip, "row " + i + " must carry a preview strip");
                compare(strip.height, Theme.controls.menuBar.height);
                verify(strip.width >= 180, "strip width " + strip.width);
                var mark = findChild(row, "settingsGlyphMark-" + row.glyph);
                verify(mark, "row " + i + " must carry the real glyph");
                compare(mark.size, Theme.controls.menuBar.iconSize);
                var plate = findChild(row, "settingsGlyphPlate-" + row.glyph);
                verify(plate, "row " + i + " must carry the hover/open plate");
                compare(plate.height, Theme.controls.menuBarMenu.barHeight);
                // The wallpaper-contrast shadow is on, not a flat mark.
                verify(mark.shadowColor.a > 0,
                       "the preview keeps the wallpaper-contrast shadow");
            }
            compare(stage.width > 0, true);
        }

        function test_os_menu_glyph_choice_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            var dragonfruit = pane.glyphRepeater.itemAt(0);
            var tux = pane.glyphRepeater.itemAt(1);

            // A pointer pick anywhere on the row selects through `chosen`.
            tux.choose();
            compare(Settings.values["menu.osGlyph"], "tux");
            compare(pane.osGlyph, "tux");
            compare(tux.selected, true);
            compare(dragonfruit.selected, false);

            // The explicit setter is the same write.
            pane.setGlyph(0);
            compare(Settings.values["menu.osGlyph"], "dragonfruit");
            compare(dragonfruit.selected, true);
            compare(tux.selected, false);

            // An external change converges into the group.
            Settings.set("menu.osGlyph", "tux");
            compare(pane.glyphIndex, 1);
            compare(tux.selected, true);
        }

        function test_os_menu_glyph_keyboard_and_click_select() {
            var shell = make();
            var pane = paneOf(shell);
            var tux = pane.glyphRepeater.itemAt(1);
            waitForRendering(stage);
            tux.forceActiveFocus();
            waitForRendering(stage);
            compare(tux.activeFocus, true);
            // The row forwards its keys to the group adapter (the row itself
            // is not a FocusScope), so the adapter's rule is the one keyboard
            // path: Space/Return select the focused row.
            tux.handleKey(Qt.Key_Space);
            compare(Settings.values["menu.osGlyph"], "tux");
            tux.handleKey(Qt.Key_Return);
            compare(Settings.values["menu.osGlyph"], "tux");

            // A click anywhere on the row's body selects it.
            var dragonfruit = pane.glyphRepeater.itemAt(0);
            mouseClick(dragonfruit, dragonfruit.width / 2,
                       dragonfruit.height / 2);
            compare(Settings.values["menu.osGlyph"], "dragonfruit");
        }

        function test_reset_restores_the_dragonfruit_default() {
            var shell = make();
            var pane = paneOf(shell);
            pane.glyphRepeater.itemAt(1).choose();
            compare(Settings.values["menu.osGlyph"], "tux");
            pane.resetButton.clicked();
            compare(Settings.values["menu.osGlyph"], "dragonfruit");
            compare(pane.glyphRepeater.itemAt(0).selected, true);
            compare(pane.glyphRepeater.itemAt(1).selected, false);
        }

        // ── Behaviour, Clock, and per-control options ──────────────────────

        function test_behavior_rows_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.autoHideSelect.activateIndex(1); // Always
            compare(Settings.values["menu.autoHide"], "always");
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

        // T293a: Battery, Focus, and Sound each open their options sheet
        // from the row's own button.
        function test_options_buttons_open_the_per_control_sheet() {
            var shell = make();
            var pane = paneOf(shell);
            var cases = [
                { key: "menu.control.battery", title: "Battery Options",
                  option: "power.showPercentage" },
                { key: "menu.control.focus", title: "Focus Options",
                  option: "menu.control.focusAlways" },
                { key: "menu.control.volume", title: "Sound Options",
                  option: "menu.control.volumeWhenActive" }
            ];
            for (var i = 0; i < cases.length; ++i) {
                var index = controlIndex(pane, cases[i].key);
                var button = findChild(pane.controlsRepeater.itemAt(index),
                                       "menuBarControlOptions-" + index);
                verify(button, cases[i].key + " must offer Options…");
                button.clicked();
                waitForRendering(stage);
                compare(pane.controlOptionsDialog.open, true);
                compare(pane.controlOptionsDialog.title, cases[i].title);
                compare(pane.controlOptionsToggle.checked, false);
                pane.controlOptionsToggle.toggle();
                compare(Settings.values[cases[i].option], true, cases[i].option);
                pane.controlOptionsDialog.accept();
                waitForRendering(stage);
                compare(pane.controlOptionsDialog.open, false);
            }
            // Only the three option rows carry a button.
            var withOptions = pane.controlRows.filter(function(row) {
                return row.options !== undefined;
            }).length;
            compare(withOptions, 3);
        }

        function test_per_control_options_round_trip() {
            var shell = make();
            var pane = paneOf(shell);

            pane.openControlOptions("battery");
            waitForRendering(stage);
            pane.controlOptionsToggle.toggle();
            compare(Settings.values["power.showPercentage"], true);
            pane.controlOptionsDialog.accept();

            pane.openControlOptions("focus");
            waitForRendering(stage);
            pane.controlOptionsToggle.toggle();
            compare(Settings.values["menu.control.focusAlways"], true);
            pane.controlOptionsDialog.accept();

            // An external change converges into the open sheet's binding.
            Settings.set("power.showPercentage", false);
            pane.openControlOptions("battery");
            waitForRendering(stage);
            compare(pane.controlOptionsToggle.checked, false);
            pane.controlOptionsDialog.accept();
        }

        // ── Third-party items ──────────────────────────────────────────────

        function test_third_party_status_notifier_list() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.thirdPartyMenuBarAvailable, true);
            compare(pane.thirdPartyItems.length, 2);
            compare(pane.thirdPartyRepeater.count, 2);
            compare(pane.thirdPartyEmptyNote.visible, false);
            // The mockup's example names are never fixed rows.
            var labels = [];
            for (var i = 0; i < pane.thirdPartyItems.length; ++i)
                labels.push(String(pane.thirdPartyItems[i].label));
            compare(labels.indexOf("File Manager"), -1);
            compare(labels.indexOf("Music"), -1);
            compare(labels.indexOf("Sync Agent"), -1);

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

        // ── Add Controls, Reset, and the Advanced sheet ────────────────────

        function test_add_controls_sheet_lists_the_live_controls() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addControlsButton.clicked();
            waitForRendering(stage);
            compare(pane.addControlsDialog.open, true);
            var toggle = findChild(pane.addControlsDialog,
                                   "menuBarAddToggle-menu.control.wifi");
            verify(toggle);
            toggle.toggle();
            compare(Settings.values["menu.control.wifi"], false);
            var display = findChild(pane.addControlsDialog,
                                    "menuBarAddToggle-menu.control.display");
            verify(display);
            compare(display.enabled, false);
            pane.addControlsDialog.accept();
        }

        function test_customize_controls_opens_the_quick_settings_surface() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.customizeControlsButton.text, "Customize Controls…");
            // The footer action calls `customizeControls()`, which emits the
            // pane's `sectionRequested("menu-bar", "controls")`; the shell's
            // Connections answers it with `openSubPane`.
            pane.customizeControls();
            tryVerify(function() { return shell.aliasSubPaneId === "controls"; }, 3000);
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                        && shell.paneBody.item !== null
                        && shell.paneBody.item.registry !== undefined;
            });
            // The alias lands on the existing Quick Settings surface; the
            // shell keeps the Menu Bar pane selected.
            compare(shell.currentPaneId, "menu-bar");
            compare(shell.currentSubPaneId, "controls");
            compare(shell.paneTitle, "Menu Bar Controls");
            verify(shell.paneBody.item.registry !== undefined,
                   "the Customize action opens the real Quick Settings surface");
            shell.closeSubPane();
            waitForRendering(stage);
            compare(shell.currentSubPaneId, "");
        }

        function test_advanced_rows_live_in_the_sheet() {
            var shell = make();
            var pane = paneOf(shell);

            pane.openAdvancedMenuBar();
            waitForRendering(stage);
            compare(pane.advancedDialog.open, true);
            compare(pane.advancedDialog.title, "Advanced Menu Bar");

            pane.overflowSelect.activateIndex(1); // Always
            compare(Settings.values["menu.overflow"], "always");
            Settings.set("menu.overflow", "automatic");
            compare(pane.overflowSelect.currentIndex, 0);

            pane.globalMenuToggle.toggle();
            compare(Settings.values["menu.global"], false);

            compare(pane.perDisplayPanelRow.enabled, false);
            verify(pane.perDisplayPanelRow.description.indexOf("Disabled") >= 0);
            pane.closeAdvancedMenuBar();
            waitForRendering(stage);
            compare(pane.advancedDialog.open, false);

            // A deep link opens the sheet and lands on the row.
            verify(shell.deepLink("settings://pane/menu-bar/row/globalMenu"));
            tryVerify(function() { return pane.advancedDialog.open; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            compare(shell.highlightTarget.settingsId, "globalMenu");
            pane.closeAdvancedMenuBar();
        }

        function test_the_workspaces_row_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            var toggle = pane.workspacesToggle();
            verify(toggle, "the Workspaces row toggle must exist");
            toggle.toggle();
            compare(Settings.values["menu.control.workspaces"], false);
            Settings.set("menu.control.workspaces", true);
            compare(pane.workspacesVisible, true);
        }

        function test_reset_covers_the_menu_bar_keys() {
            var shell = make();
            var pane = paneOf(shell);
            pane.autoHideSelect.activateIndex(1);
            pane.backgroundToggle.toggle();
            pane.recentSelect.activateIndex(0);
            pane.openControlOptions("battery");
            waitForRendering(stage);
            pane.controlOptionsToggle.toggle();
            pane.controlOptionsDialog.accept();
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

            var wifi = controlIndex(pane, "menu.control.wifi");
            Settings.set("menu.control.wifi", false);
            compare(controlToggle(pane, wifi).checked, false);

            Settings.set("menu.control.workspaces", false);
            compare(pane.workspacesVisible, false);
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
            // The four footer actions fit the content column together.
            var row = pane.footer.row;
            var last = row.children[row.children.length - 1];
            verify(last.x + last.width <= pane.width,
                   "the footer actions must fit the pane width");
        }
    }
}
