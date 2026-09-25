// SPDX-License-Identifier: MIT
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
        Component { id: logoComponent; DragonfruitLogo { } }
        Component { id: wifiMenuComponent; WifiMenu { } }
        Component { id: volumeMenuComponent; VolumeMenu { } }

        SignalSpy { id: wifiJoinSpy; signalName: "joinRequested" }
        SignalSpy { id: wifiRefreshSpy; signalName: "refreshRequested" }
        SignalSpy { id: volumeSetSpy; signalName: "volumeSetRequested" }
        SignalSpy { id: muteSpy; signalName: "muteToggleRequested" }
        SignalSpy { id: statusSpy; signalName: "statusItemActivated" }
        SignalSpy { id: ccSpy; signalName: "controlCenterRequested" }
        SignalSpy { id: mcSpy; signalName: "missionControlRequested" }
        SignalSpy { id: openedSpy; signalName: "appMenuOpened" }
        SignalSpy { id: closedSpy; signalName: "appMenuClosed" }
        SignalSpy { id: triggeredSpy; signalName: "appMenuTriggered" }

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

        // The bar always renders the system menu (index 0) and the application
        // menu (index 1) before any exported app menus, so an app's File menu
        // starts at index 2.
        function fixedMenus() {
            return {
                systemMenuItems: [
                    { label: "About This System", action: "about-system" },
                    { type: "separator" },
                    { label: "Sleep", action: "sleep" }
                ],
                applicationMenuItems: [
                    { label: "About Test", action: "about" },
                    { label: "Quit Test", action: "quit" }
                ]
            };
        }

        function defaultStatus() {
            return [
                { id: "wifi", icon: "wifi", accessibleName: "Wi-Fi", available: true },
                { id: "bluetooth", icon: "bluetooth", accessibleName: "Bluetooth", available: false },
                { id: "volume", icon: "volume", accessibleName: "Volume", available: true },
                { id: "battery", icon: "battery", accessibleName: "Battery", available: true }
            ];
        }

        // The bridge host's decoded views (services/system-status).
        function wifiModel() {
            return {
                kind: "wifi", state: "available", glyph: "wifi-secure",
                label: "home \u00b7 82%", readOnly: false,
                networks: [
                    { ssid: "home", strength: 82, security: "WPA2", secured: true,
                      active: true, band: "5 GHz" },
                    { ssid: "cafe", strength: 40, security: "Open", secured: false,
                      active: false, band: "2.4 GHz" }
                ]
            };
        }

        function audioModel(muted, volume) {
            return {
                kind: "audio", state: "available",
                glyph: (muted || volume === 0) ? "volume-muted" : "volume",
                volume: volume, percent: Math.round(volume * 100), muted: muted,
                sinks: [ { id: 7, name: "speakers", description: "Built-in Speakers",
                           volume: volume, percent: Math.round(volume * 100),
                           muted: muted, default: true } ]
            };
        }

        // -- Layout / data injection (FR-1, FR-2) ---------------------------

        function test_system_and_application_menu_always_present() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appName: "Safari",
                appMenuModel: [],
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            // System menu is leftmost and renders the brand mark.
            compare(bar.appMenuAt(0).showLogo, true);
            compare(bar.appMenuAt(0).title, "System");
            // The application menu always follows it, bold, titled with the app.
            compare(bar.appMenuAt(1).showLogo, false);
            compare(bar.appMenuAt(1).emphasized, true);
            compare(bar.appMenuAt(1).title, "Safari");
            // No exported app menus yet, so nothing after the fixed two.
            compare(bar.appMenuAt(2), null);
        }

        function test_app_menu_renders_after_fixed_menus() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appName: "Safari",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            // Fixed system + app menus occupy 0 and 1.
            compare(bar.appMenuAt(0).showLogo, true);
            compare(bar.appMenuAt(1).title, "Safari");
            // The app's exported menus follow.
            compare(bar.appMenuAt(2).title, "File");
            compare(bar.appMenuAt(2).emphasized, false);
            compare(bar.appMenuAt(3).title, "Edit");
            compare(bar.appMenuAt(4), null);
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
            // Wi-Fi and volume open a popover (T-07.5a); the always-activation
            // items (battery here) still raise statusItemActivated.
            var battery = bar.statusItemAt(3);
            mouseClick(battery, battery.width / 2, battery.height / 2);
            compare(statusSpy.count, 1);
            compare(statusSpy.signalArguments[0][0], "battery");
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
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appName: "Safari",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
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

        function test_dropdown_geometry_tracks_open_menu() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 600,
                appName: "Safari",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            // Closed: the overlay popup has no rectangle.
            compare(bar.dropdownWidth, 0);
            compare(bar.dropdownHeight, 0);

            bar.openMenu(1);
            waitForRendering(stage);
            var appMenu = bar.appMenuAt(1);
            var popup = appMenu.popup;
            // The geometry contract is the logical dropdown rectangle (the
            // open/close scale animation is cosmetic), so compare against the
            // popup's logical origin, not its scaled mapped origin.
            var topLeft = appMenu.mapToItem(bar, popup.x, popup.y);
            verify(bar.dropdownWidth > 0);
            verify(bar.dropdownHeight > 0);
            compare(bar.dropdownX, topLeft.x);
            compare(bar.dropdownY, topLeft.y);
            // The dropdown hangs below the bar, which is what the shell uses
            // to place the separate overlay surface.
            verify(bar.dropdownY >= bar.height);

            bar.closeMenus();
            waitForRendering(stage);
            compare(bar.dropdownWidth, 0);
        }

        function test_dropdown_geometry_includes_open_submenu() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 600,
                appName: "Safari",
                appMenuModel: [
                    { title: "File", items: [
                        { label: "Open With", type: "submenu",
                          submenu: [{ label: "Text Editor" }, { label: "Preview" }] }
                    ] }
                ],
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            // Fixed system + app menus occupy 0 and 1; the File menu is 2.
            bar.openMenu(2);
            waitForRendering(stage);
            var closedWidth = bar.dropdownWidth;
            verify(closedWidth > 0);

            bar.appMenuAt(2).openSubmenu(0);
            waitForRendering(stage);
            verify(bar.dropdownWidth > closedWidth,
                   "the overlay dropdown must grow to contain the submenu");

            bar.closeMenus();
            waitForRendering(stage);
            compare(bar.dropdownWidth, 0);
        }

        function test_escape_dismisses_open_menu() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);

            keyClick(Qt.Key_Escape);
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
            compare(bar.appMenuAt(0).open, false);
        }

        function test_click_away_dismisses_open_menu() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 600,
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);

            // The right padding strip is empty bar background.
            mouseClick(bar, bar.width - 2, bar.height / 2);
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
        }

        function test_click_menu_title_opens_and_stays_open() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 600,
                appName: "Safari",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            openedSpy.target = bar;
            openedSpy.clear();
            closedSpy.target = bar;
            closedSpy.clear();

            // Clicking the title toggles it open; the bar's click-away handler
            // must not immediately close it.
            var appMenu = bar.appMenuAt(1);
            mouseClick(appMenu, appMenu.width / 2, appMenu.height / 2);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 1);
            compare(appMenu.open, true);
            compare(openedSpy.count, 1);
            compare(closedSpy.count, 0);

            // A second click on the title toggles it closed.
            mouseClick(appMenu, appMenu.width / 2, appMenu.height / 2);
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
            compare(appMenu.open, false);
        }

        function test_drag_through_switches_menu_on_hover() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appName: "Safari",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);

            var appMenu = bar.appMenuAt(1);
            mouseMove(appMenu, appMenu.width / 2, appMenu.height / 2);
            tryCompare(bar, "openMenuIndex", 1);
            compare(bar.appMenuAt(0).open, false);
            compare(bar.appMenuAt(1).open, true);
        }

        function test_open_menu_tracks_focus_switch() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appName: "Safari",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            // Edit is the second exported menu, at index 3.
            bar.openMenu(3);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 3);

            bar.setFocusedApp("Terminal", "terminal", [
                { title: "Shell", items: [{ label: "New" }] },
                { title: "View", items: [{ label: "Clear" }] }
            ]);
            waitForRendering(stage);
            // The open menu follows the same position in the new app's menus.
            compare(bar.openMenuIndex, 3);
            compare(bar.appMenuAt(3).title, "View");
            compare(bar.appMenuAt(3).open, true);
            // The application menu title followed the focus change too.
            compare(bar.appMenuAt(1).title, "Terminal");

            // Back to the desktop: the fixed app menu stays (Files), and the
            // open exported menu (now gone) dismisses.
            bar.setFocusedApp("Files", "", []);
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
            compare(bar.appMenuAt(1).title, "Files");
        }

        function test_focus_loss_dismisses_open_menu() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            bar.openMenu(0);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 0);
            bar.shellFocused = false;
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1);
        }

        // -- Fixed menu action dispatch (FR-2) ------------------------------

        function test_system_menu_item_carries_action() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appName: "Safari",
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            triggeredSpy.target = bar;
            triggeredSpy.clear();

            var system = bar.appMenuAt(0);
            var sleepRow = system.model[2];
            compare(sleepRow.action, "sleep");
            system.activate(2);
            waitForRendering(stage);
            compare(triggeredSpy.count, 1);
            compare(triggeredSpy.signalArguments[0][0], 0);
            compare(triggeredSpy.signalArguments[0][2].action, "sleep");
        }

        // -- Wi-Fi and volume status menus (T-07.5a) ------------------------

        function test_wifi_menu_model_exposes_join_rows() {
            var menu = make(wifiMenuComponent, { model: wifiModel() });
            compare(menu.rows.length, 2);
            compare(menu.rows[0].ssid, "home");
            compare(menu.rows[0].action, "join");
            compare(menu.rows[0].enabled, true);
            compare(menu.rows[0].secured, true);
            compare(menu.rows[0].strength, 82);
            compare(menu.rows[1].ssid, "cafe");
            compare(menu.rows[1].secured, false);
            compare(menu.headerLabel, "home \u00b7 82%");
        }

        function test_wifi_menu_open_network_joins_immediately() {
            var menu = make(wifiMenuComponent, { model: wifiModel() });
            wifiJoinSpy.target = menu;
            wifiJoinSpy.clear();
            menu.activateRow(1); // cafe is open
            compare(wifiJoinSpy.count, 1);
            compare(wifiJoinSpy.signalArguments[0][0], "cafe");
            compare(wifiJoinSpy.signalArguments[0][1], "");
        }

        function test_wifi_menu_secured_network_collects_a_secret() {
            var menu = make(wifiMenuComponent, { model: wifiModel() });
            wifiJoinSpy.target = menu;
            wifiJoinSpy.clear();
            menu.activateRow(0); // home is secured: prompt first
            compare(wifiJoinSpy.count, 0);
            compare(menu.selectedSsid, "home");
            compare(menu.passwordPromptVisible, true);
            menu.secret = "hunter2";
            menu.joinSelected();
            compare(wifiJoinSpy.count, 1);
            compare(wifiJoinSpy.signalArguments[0][0], "home");
            compare(wifiJoinSpy.signalArguments[0][1], "hunter2");
        }

        function test_wifi_menu_read_only_disables_join() {
            var model = wifiModel();
            model.readOnly = true;
            var menu = make(wifiMenuComponent, { model: model });
            compare(menu.rows[0].enabled, false);
            wifiJoinSpy.target = menu;
            wifiJoinSpy.clear();
            menu.activateRow(0);
            compare(wifiJoinSpy.count, 0);
        }

        function test_wifi_menu_popover_refreshes_on_open() {
            var menu = make(wifiMenuComponent, { model: wifiModel() });
            wifiRefreshSpy.target = menu;
            wifiRefreshSpy.clear();
            menu.open = true;
            waitForRendering(stage);
            compare(wifiRefreshSpy.count, 1);
        }

        function test_volume_menu_exposes_the_slider() {
            var menu = make(volumeMenuComponent, { model: audioModel(false, 0.6) });
            compare(menu.value, 0.6);
            compare(menu.percent, 60);
            compare(menu.muted, false);
            compare(menu.sinkLabel, "Built-in Speakers");
        }

        function test_volume_menu_set_fraction_applies() {
            var menu = make(volumeMenuComponent, { model: audioModel(false, 0.6) });
            volumeSetSpy.target = menu;
            volumeSetSpy.clear();
            menu.setFraction(0.8);
            compare(volumeSetSpy.count, 1);
            verify(Math.abs(volumeSetSpy.signalArguments[0][0] - 0.8) < 0.0001);
            // The slider clamps to the unit range.
            menu.setFraction(1.5);
            compare(menu.value, 1.0);
        }

        function test_volume_menu_mute_toggles() {
            var menu = make(volumeMenuComponent, { model: audioModel(false, 0.6) });
            muteSpy.target = menu;
            muteSpy.clear();
            menu.toggleMute();
            compare(muteSpy.count, 1);
        }

        function test_volume_menu_reflects_muted_state() {
            var menu = make(volumeMenuComponent, { model: audioModel(true, 0.6) });
            compare(menu.muted, true);
            compare(menu.defaultSink.description, "Built-in Speakers");
        }

        function test_status_item_click_opens_the_wifi_popover() {
            var bar = make(menuBarComponent, {
                width: 800,
                statusItems: defaultStatus(),
                wifiMenu: wifiModel()
            });
            var wifi = bar.statusItemFor("wifi");
            mouseClick(wifi, wifi.width / 2, wifi.height / 2);
            waitForRendering(stage);
            compare(bar.openStatusItem, "wifi");
            var menu = findChild(bar, "wifiMenu");
            verify(menu !== null);
            compare(menu.popup.open, true);
            verify(bar.dropdownWidth > 0);
            bar.closeStatusMenu();
            waitForRendering(stage);
            compare(bar.openStatusItem, "");
            compare(menu.popup.open, false);
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
                // Canvas paints are asynchronous; retry the grab so the test is
                // not racy against the queued `requestPaint` (a pre-existing
                // flake that surfaced under the extra menu renders).
                tryVerify(function() {
                    return imageContainsColor(grabImage(glyph),
                                              Theme.color.textPrimary);
                }, 2000, "glyph " + names[i] + " should render visible pixels");
                glyph.destroy();
            }
        }

        // -- System menu brand mark (T-09) ----------------------------------

        function test_logo_renders_pixels_and_tints() {
            // Drawn large enough that the thin line-art has solid interior
            // pixels to sample (at menu-bar size it is entirely antialiased).
            var logo = make(logoComponent, { size: 120, color: Theme.color.textPrimary });
            // The art is taller than wide (270 x 322); the aspect is preserved.
            verify(logo.implicitHeight > logo.implicitWidth);
            var img = grabImage(logo);
            verify(imageContainsColor(img, Theme.color.textPrimary),
                   "the dragonfruit mark should render visible pixels");

            // The mark is tintable, not a baked-in bitmap: switching the color
            // changes the painted pixels.
            logo.color = Theme.color.accent;
            waitForRendering(stage);
            img = grabImage(logo);
            verify(imageContainsColor(img, Theme.color.accent),
                   "the dragonfruit mark should follow the design-system color");
            logo.destroy();
        }
    }
}
