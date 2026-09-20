// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.MenuBar

// Menu-bar tests (T-09 FR-1…FR-7). Runs headless on the offscreen platform
// and the software scene graph, so the interaction walkthrough is scripted
// and CI-able. `stage` is the visual root so `grabImage` returns pixels.
Item {
    id: stage
    width: 1000
    height: 320

    TestCase {
        id: testCase
        name: "MenuBar"
        when: windowShown

        Component { id: menuBarComponent; MenuBar { } }
        Component { id: statusItemComponent; StatusItem { } }
        Component { id: clockComponent; MenuBarClock { } }
        Component { id: glyphComponent; StatusGlyph { } }

        SignalSpy { id: statusSpy; signalName: "statusItemActivated" }
        SignalSpy { id: ccSpy; signalName: "controlCenterRequested" }
        SignalSpy { id: mcSpy; signalName: "missionControlRequested" }
        SignalSpy { id: openedSpy; signalName: "appMenuOpened" }
        SignalSpy { id: closedSpy; signalName: "appMenuClosed" }
        SignalSpy { id: itemActivatedSpy; signalName: "activated" }

        function make(component, props) {
            var obj = createTemporaryObject(component, stage, props || {});
            waitForRendering(stage);
            return obj;
        }

        function channel(value) {
            return Math.round(value * 255);
        }

        function imageContainsColor(image, color) {
            var r = channel(color.r);
            var g = channel(color.g);
            var b = channel(color.b);
            for (var y = 0; y < image.height; ++y) {
                for (var x = 0; x < image.width; ++x) {
                    if (image.red(x, y) === r && image.green(x, y) === g
                            && image.blue(x, y) === b)
                        return true;
                }
            }
            return false;
        }

        function sampleModel() {
            return [
                { title: "File", items: [{ label: "New" }, { label: "Open" }] },
                { title: "Edit", items: [{ label: "Undo" }] }
            ];
        }

        function defaultStatus() {
            return [
                { id: "wifi", icon: "wifi", accessibleName: "Wi-Fi", available: true },
                { id: "bluetooth", icon: "bluetooth", accessibleName: "Bluetooth", available: false },
                { id: "volume", icon: "volume", accessibleName: "Volume", available: true },
                { id: "battery", icon: "battery", accessibleName: "Battery", available: true }
            ];
        }

        // -- Layout / data injection (FR-1, FR-2) ---------------------------

        function test_app_name_fallback_when_no_menu() {
            var bar = make(menuBarComponent, { appName: "Safari", appMenuModel: [] });
            var nameText = findChild(bar, "appName");
            verify(nameText !== null);
            compare(nameText.visible, true);
            compare(nameText.text, "Safari");
            compare(bar.appMenuAt(0), null);
        }

        function test_app_menu_renders_from_model() {
            var bar = make(menuBarComponent, { appName: "Safari", appMenuModel: sampleModel() });
            compare(findChild(bar, "appName").visible, false);
            compare(bar.appMenuAt(0).title, "File");
            compare(bar.appMenuAt(1).title, "Edit");
            compare(bar.appMenuAt(2), null);
        }

        function test_menus_and_status_are_on_opposite_sides() {
            var bar = make(menuBarComponent, {
                width: 800,
                appMenuModel: sampleModel(),
                statusItems: defaultStatus()
            });
            var appRow = findChild(bar, "appMenuRow");
            var statusRow = findChild(bar, "statusRow");
            verify(appRow.x < statusRow.x);
            verify(appRow.x <= Theme.controls.menuBar.paddingH + 1);
            verify(statusRow.x + statusRow.width >= bar.width - Theme.controls.menuBar.paddingH - 1);
        }

        // -- Adapter degradation (FR-4) ------------------------------------

        function test_status_item_hidden_when_daemon_absent() {
            var bar = make(menuBarComponent, { statusItems: defaultStatus() });
            var bluetooth = bar.statusItemAt(1);
            compare(bluetooth.visible, false);
            compare(bluetooth.width, 0);

            var wifi = bar.statusItemAt(0);
            compare(wifi.visible, true);
            verify(wifi.width > 0);
        }

        function test_disabled_status_item_stays_visible_but_dimmed() {
            var bar = make(menuBarComponent, {
                statusItems: [ { id: "wifi", icon: "wifi", available: true, enabled: false } ]
            });
            var wifi = bar.statusItemAt(0);
            compare(wifi.visible, true);
            verify(wifi.opacity < 1.0);
        }

        function test_status_item_activation() {
            var bar = make(menuBarComponent, { statusItems: defaultStatus() });
            statusSpy.target = bar;
            statusSpy.clear();
            var wifi = bar.statusItemAt(0);
            mouseClick(wifi, wifi.width / 2, wifi.height / 2);
            compare(statusSpy.count, 1);
            compare(statusSpy.signalArguments[0][0], "wifi");
        }

        function test_control_center_and_mission_control_entries() {
            var bar = make(menuBarComponent, { width: 600 });
            ccSpy.target = bar;
            ccSpy.clear();
            mcSpy.target = bar;
            mcSpy.clear();

            var row = findChild(bar, "statusRow");
            var controlCenter = null;
            var missionControl = null;
            for (var i = 0; i < row.children.length; ++i) {
                var child = row.children[i];
                if (child.itemId === "control-center")
                    controlCenter = child;
                if (child.itemId === "mission-control")
                    missionControl = child;
            }
            verify(controlCenter !== null);
            verify(missionControl !== null);
            compare(controlCenter.Accessible.name, "Control Center");
            compare(missionControl.Accessible.name, "Mission Control");

            mouseClick(controlCenter, controlCenter.width / 2, controlCenter.height / 2);
            compare(ccSpy.count, 1);
            mouseClick(missionControl, missionControl.width / 2, missionControl.height / 2);
            compare(mcSpy.count, 1);
        }

        // -- App-menu interaction (FR-3) -----------------------------------

        function test_open_and_close_menu() {
            var bar = make(menuBarComponent, { appMenuModel: sampleModel() });
            openedSpy.target = bar;
            openedSpy.clear();
            closedSpy.target = bar;
            closedSpy.clear();

            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);
            compare(bar.appMenuAt(0).open, true);
            compare(openedSpy.count, 1);

            bar.openMenu(1);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 1);
            compare(bar.appMenuAt(0).open, false);
            compare(bar.appMenuAt(1).open, true);

            bar.closeMenus();
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
            compare(bar.appMenuAt(1).open, false);
            compare(closedSpy.count, 1);
        }

        function test_escape_dismisses_open_menu() {
            var bar = make(menuBarComponent, { appMenuModel: sampleModel() });
            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);

            keyClick(Qt.Key_Escape);
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
            compare(bar.appMenuAt(0).open, false);
        }

        function test_click_away_dismisses_open_menu() {
            var bar = make(menuBarComponent, { width: 600, appMenuModel: sampleModel() });
            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);

            // The right padding strip is empty bar background.
            mouseClick(bar, bar.width - 2, bar.height / 2);
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
        }

        function test_drag_through_switches_menu_on_hover() {
            var bar = make(menuBarComponent, { appMenuModel: sampleModel() });
            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);

            var edit = bar.appMenuAt(1);
            mouseMove(edit, edit.width / 2, edit.height / 2);
            tryCompare(bar, "openMenuIndex", 1);
            compare(bar.appMenuAt(0).open, false);
            compare(bar.appMenuAt(1).open, true);
        }

        function test_open_menu_tracks_focus_switch() {
            var bar = make(menuBarComponent, { appMenuModel: sampleModel() });
            bar.openMenu(1);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 1);

            bar.setFocusedApp("Terminal", "terminal", [
                { title: "Shell", items: [{ label: "New" }] },
                { title: "View", items: [{ label: "Clear" }] }
            ]);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 1);
            compare(bar.appMenuAt(1).title, "View");
            compare(bar.appMenuAt(1).open, true);

            bar.setFocusedApp("Desktop", "", []);
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
        }

        function test_focus_loss_dismisses_open_menu() {
            var bar = make(menuBarComponent, { appMenuModel: sampleModel() });
            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);
            bar.shellFocused = false;
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
        }

        // -- Clock (FR-5) ---------------------------------------------------

        function test_clock_uses_locale_time_format() {
            var fixed = new Date(2026, 8, 19, 14, 5, 0);
            var clock = make(clockComponent, { now: fixed });
            compare(clock.timeText,
                    Qt.formatTime(fixed, Qt.locale().timeFormat(Locale.ShortFormat)));
            verify(clock.timeText.length > 0);
            clock.showDate = true;
            waitForRendering(stage);
            compare(clock.dateText, Qt.formatDate(fixed, Locale.ShortFormat));
        }

        // -- Status glyph rendering (FR-5) ---------------------------------

        function test_status_glyphs_render_pixels() {
            var names = ["wifi", "bluetooth", "volume", "volume-muted",
                         "battery", "focus", "accessibility",
                         "control-center", "mission-control"];
            for (var i = 0; i < names.length; ++i) {
                var glyph = make(glyphComponent, { name: names[i], size: 24 });
                var img = grabImage(glyph);
                verify(imageContainsColor(img, Theme.color.textPrimary),
                       "glyph " + names[i] + " should render visible pixels");
                glyph.destroy();
            }
        }
    }
}
