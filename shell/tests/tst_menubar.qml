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
        Component { id: batteryMenuComponent; BatteryMenu { } }
        Component { id: bluetoothMenuComponent; BluetoothMenu { } }

        SignalSpy { id: wifiJoinSpy; signalName: "joinRequested" }
        SignalSpy { id: wifiRefreshSpy; signalName: "refreshRequested" }
        SignalSpy { id: batteryRefreshSpy; signalName: "refreshRequested" }
        SignalSpy { id: volumeSetSpy; signalName: "volumeSetRequested" }
        SignalSpy { id: muteSpy; signalName: "muteToggleRequested" }
        SignalSpy { id: statusSpy; signalName: "statusItemActivated" }
        SignalSpy { id: ccSpy; signalName: "controlCenterRequested" }
        SignalSpy { id: mcSpy; signalName: "missionControlRequested" }
        SignalSpy { id: clockSpy; signalName: "clockActivated" }
        SignalSpy { id: openedSpy; signalName: "appMenuOpened" }
        SignalSpy { id: closedSpy; signalName: "appMenuClosed" }
        SignalSpy { id: triggeredSpy; signalName: "appMenuTriggered" }
        SignalSpy { id: traySpy; signalName: "trayMenuTriggered" }
        SignalSpy { id: bluetoothPowerSpy; signalName: "bluetoothPowerToggleRequested" }
        SignalSpy { id: bluetoothDeviceSpy; signalName: "bluetoothDeviceToggled" }
        SignalSpy { id: bluetoothSettingsSpy; signalName: "bluetoothSettingsRequested" }

        // These tests exercise the legacy sampled/fallback rail unless a
        // case explicitly selects the replacement look.
        function init() {
            Glass.rollout = "legacy";
        }

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

        // The offscreen grab renders a transparent glyph background as an
        // opaque colour (white on this backend). True when a pixel differs
        // from that background, so dimmed marks (wifi-off/-disabled/
        // -connecting) count even though they do not match the tint exactly.
        function isBackground(image, x, y) {
            var bg = image.pixel(0, 0);
            var c = image.pixel(x, y);
            return Math.abs(c.r - bg.r) < 0.02 && Math.abs(c.g - bg.g) < 0.02
                    && Math.abs(c.b - bg.b) < 0.02;
        }

        function imageHasMark(image) {
            for (var y = 0; y < image.height; ++y) {
                for (var x = 0; x < image.width; ++x) {
                    if (!isBackground(image, x, y))
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

        function batteryModel() {
            return {
                kind: "battery", state: "available", present: true,
                glyph: "battery", label: "82% charging", percent: 82, level: 0.82,
                charging: true, plugged: true, onBattery: false,
                timeToEmpty: null, timeToFull: 5400
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

        function test_menu_bar_uses_a_milky_fallback_without_sampled_blur() {
            // P1: the DRM rail reports blurActive=false. The bar must switch
            // from the translucent chrome fill to the near-opaque milky
            // fallback while keeping the same chrome geometry.
            Theme.dark = false;
            var bar = make(menuBarComponent, {
                width: 600,
                materialBlurActive: false
            });
            var expected = Qt.rgba(Theme.color.chromeFallbackFill.r,
                                   Theme.color.chromeFallbackFill.g,
                                   Theme.color.chromeFallbackFill.b,
                                   Theme.material.chromeFallbackOpacity);
            fuzzyCompare(bar.color.a, expected.a, 0.001);
            fuzzyCompare(bar.color.r, expected.r, 0.001);
            fuzzyCompare(bar.color.g, expected.g, 0.001);
            fuzzyCompare(bar.color.b, expected.b, 0.001);
            verify(bar.color.a >= 0.8, "the fallback must read as a material");

            var glass = make(menuBarComponent, {
                width: 600,
                materialBlurActive: true
            });
            fuzzyCompare(glass.color.a, Theme.material.chromeOpacity, 0.001);
            verify(glass.color.a < bar.color.a,
                   "the sampled-glass complement stays more translucent");
        }

        function test_forced_material_tiers_apply_the_role_state() {
            // T-21.2c: the shell applies the Chrome role's effective state.
            // Full and Reduced keep the sampled glass; the persistent
            // floor-missed fallback (tier reduced, blur off, reason 4) and the
            // software/headless fallback both draw the milky complement.
            var tiers = [
                { tier: 0, blur: true, fallback: 0, label: "full" },
                { tier: 1, blur: true, fallback: 0, label: "reduced" },
                { tier: 1, blur: false, fallback: 4, label: "floor-missed" },
                { tier: 2, blur: false, fallback: 2, label: "software" }
            ];
            for (var scheme = 0; scheme < 2; ++scheme) {
                Theme.dark = scheme === 1;
                for (var i = 0; i < tiers.length; ++i) {
                    var state = tiers[i];
                    var bar = make(menuBarComponent, {
                        width: 600,
                        materialTier: state.tier,
                        materialBlurActive: state.blur,
                        materialFallback: state.fallback
                    });
                    compare(bar.materialTier, state.tier, state.label);
                    compare(bar.materialBlurActive, state.blur, state.label);
                    compare(bar.materialFallback, state.fallback, state.label);
                    if (state.blur) {
                        fuzzyCompare(bar.color.a, Theme.material.chromeOpacity,
                                     0.001, state.label);
                    } else {
                        var expected = Qt.rgba(
                            Theme.color.chromeFallbackFill.r,
                            Theme.color.chromeFallbackFill.g,
                            Theme.color.chromeFallbackFill.b,
                            Theme.material.chromeFallbackOpacity);
                        fuzzyCompare(bar.color.a, expected.a, 0.001, state.label);
                        fuzzyCompare(bar.color.r, expected.r, 0.001, state.label);
                        fuzzyCompare(bar.color.g, expected.g, 0.001, state.label);
                        fuzzyCompare(bar.color.b, expected.b, 0.001, state.label);
                        verify(bar.color.a >= 0.8,
                               state.label + " must read as a material");
                    }
                }
            }
            Theme.dark = false;
        }

        function test_load_sequence_keeps_the_applied_material() {
            // T-21.2c load sequence on one menu-bar scene: Full -> Reduced ->
            // floor-missed -> Reduced -> Full. The shell receives change-only
            // role events; the already-created surface must follow each one
            // without being rebuilt.
            var steps = [
                { tier: 0, blur: true, fallback: 0, label: "full" },
                { tier: 1, blur: true, fallback: 0, label: "reduced" },
                { tier: 1, blur: false, fallback: 4, label: "floor-missed" },
                { tier: 1, blur: true, fallback: 0, label: "recovered" },
                { tier: 0, blur: true, fallback: 0, label: "full-again" }
            ];
            for (var scheme = 0; scheme < 2; ++scheme) {
                Theme.dark = scheme === 1;
                var bar = make(menuBarComponent, { width: 600 });
                for (var i = 0; i < steps.length; ++i) {
                    var step = steps[i];
                    bar.materialTier = step.tier;
                    bar.materialBlurActive = step.blur;
                    bar.materialFallback = step.fallback;
                    compare(bar.materialTier, step.tier, step.label);
                    compare(bar.materialBlurActive, step.blur, step.label);
                    compare(bar.materialFallback, step.fallback, step.label);
                    if (step.blur) {
                        fuzzyCompare(bar.color.a, Theme.material.chromeOpacity,
                                     0.001, step.label);
                    } else {
                        var expected = Qt.rgba(
                            Theme.color.chromeFallbackFill.r,
                            Theme.color.chromeFallbackFill.g,
                            Theme.color.chromeFallbackFill.b,
                            Theme.material.chromeFallbackOpacity);
                        fuzzyCompare(bar.color.a, expected.a, 0.001, step.label);
                        fuzzyCompare(bar.color.r, expected.r, 0.001, step.label);
                        fuzzyCompare(bar.color.g, expected.g, 0.001, step.label);
                        fuzzyCompare(bar.color.b, expected.b, 0.001, step.label);
                        verify(bar.color.a >= 0.8,
                               step.label + " must read as a material");
                    }
                }
            }
            Theme.dark = false;
        }

        // -- T200: the compositor no-blur veil -------------------------------

        function test_replacement_veil_moves_the_wash_to_the_compositor() {
            // In the replacement rail the QML surface paints no fill: the
            // compositor's sharp veil is the only backdrop. The shell still
            // declares the full 95 pt visual extent, while the reference strip
            // and Dragonfruit's layout height stay separate.
            Glass.rollout = "replacement";
            Theme.dark = false;
            var bar = make(menuBarComponent, {
                width: 600,
                materialBlurActive: true
            });
            compare(bar.veilResolution.renderMode, "no-blur-veil");
            compare(bar.veilActive, true);
            compare(bar.color.a, 0, "the QML wash must be gone");
            compare(bar.compositorPanelActive, true);
            compare(bar.compositorPanelHeight, 95);
            compare(bar.compositorPanelRadius, 0);
            compare(bar.veilExtent, 95);
            compare(bar.referenceStripHeight, 39);
            fuzzyCompare(bar.veilResolution.tint, 1.0, 0.001,
                         "the Light veil is a white lift");
            // The measured luminance adaptation must reach the shell so the
            // Light veil can disappear over a white wallpaper.
            fuzzyCompare(bar.veilResolution.veil.adaptScale[3], 0.48, 0.001,
                         "the Light veil keeps the measured adaptation");
            fuzzyCompare(bar.veilResolution.veil.adaptScale[4], 0.0, 0.001,
                         "the Light veil is invisible over white");

            // Dark uses the documented black wash, still with no QML fill.
            Theme.dark = true;
            bar.materialBlurActive = true;
            compare(bar.veilActive, true);
            compare(bar.color.a, 0);
            fuzzyCompare(bar.veilResolution.tint, 0.0, 0.001,
                         "the Dark veil is a black wash");
            bar.destroy();
            Glass.rollout = "legacy";
            Theme.dark = false;
        }

        function test_menu_bar_foreground_follows_the_wallpaper() {
            // The reference flips the text/glyphs to black when the rendered
            // bar is light. In Light the white veil keeps it light over black;
            // in Dark the black wash stays dark over mid-grey and disappears
            // over white.
            Glass.rollout = "replacement";
            Theme.dark = false;
            var bar = make(menuBarComponent, {
                width: 600,
                materialBlurActive: true
            });
            bar.backgroundLuma = 0.0;
            compare(bar.foreground.toString(),
                    Theme.controls.menuBar.foregroundOnLight.toString(),
                    "Light over black stays dark text");
            bar.backgroundLuma = 1.0;
            compare(bar.foreground.toString(),
                    Theme.controls.menuBar.foregroundOnLight.toString(),
                    "Light over white stays dark text");

            Theme.dark = true;
            bar.backgroundLuma = 0.5;
            compare(bar.foreground.toString(),
                    Theme.controls.menuBar.foregroundOnDark.toString(),
                    "Dark over mid-grey uses light text");
            bar.backgroundLuma = 1.0;
            compare(bar.foreground.toString(),
                    Theme.controls.menuBar.foregroundOnLight.toString(),
                    "Dark over white flips to dark text");
            bar.destroy();
            Glass.rollout = "legacy";
            Theme.dark = false;
        }

        function test_veil_fallback_and_accessibility_stay_scheme_appropriate() {
            Glass.rollout = "replacement";
            // No sampled scene texture: the compositor cannot draw the veil,
            // so QML keeps the milky plate and declares no material panel.
            for (var scheme = 0; scheme < 2; ++scheme) {
                Theme.dark = scheme === 1;
                var bar = make(menuBarComponent, {
                    width: 600,
                    materialBlurActive: false
                });
                compare(bar.veilActive, false);
                compare(bar.compositorPanelActive, false);
                verify(bar.color.a >= 0.8, "the fallback must read as a plate");
                fuzzyCompare(bar.color.r, Theme.color.chromeFallbackFill.r, 0.001);
                fuzzyCompare(bar.color.g, Theme.color.chromeFallbackFill.g, 0.001);
                fuzzyCompare(bar.color.b, Theme.color.chromeFallbackFill.b, 0.001);
                bar.destroy();
            }

            // Reduce Transparency is the explicit opaque scheme plate, not a
            // sampled strip. The compositor resolves the veil to solid and the
            // shell commits an empty panel set.
            Theme.dark = false;
            var reduced = make(menuBarComponent, {
                width: 600,
                materialBlurActive: true,
                reduceTransparency: true
            });
            compare(reduced.veilResolution.renderMode, "solid");
            compare(reduced.veilActive, false);
            compare(reduced.compositorPanelActive, false);
            verify(reduced.color.a >= 0.8);
            reduced.destroy();

            // The legacy rail keeps the old sampled QML fill and the 28 pt
            // panel; it must not silently switch to the veil extent.
            Glass.rollout = "legacy";
            var legacy = make(menuBarComponent, {
                width: 600,
                materialBlurActive: true
            });
            compare(legacy.veilResolution.renderMode, "sampled-glass");
            compare(legacy.veilActive, false);
            compare(legacy.compositorPanelActive, true);
            compare(legacy.compositorPanelHeight, Theme.controls.menuBar.height);
            compare(legacy.compositorPanelRadius, Theme.controls.menuBar.radius);
            fuzzyCompare(legacy.color.a, Theme.material.chromeOpacity, 0.001);
            legacy.destroy();
            Theme.dark = false;
        }

        // -- T201: menu-bar visibility settings ------------------------------

        function test_show_background_off_keeps_items_but_drops_fill_and_panel() {
            // The old rail's background toggle must clear both the QML fill and
            // the compositor's material panel; only the items remain.
            Glass.rollout = "replacement";
            Theme.dark = false;
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 600,
                showBackground: false,
                materialBlurActive: true,
                appName: "Safari",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems,
                statusItems: defaultStatus()
            });
            compare(bar.color.a, 0, "no QML fill with the background off");
            compare(bar.veilActive, false);
            compare(bar.compositorPanelActive, false);
            verify(bar.appMenuAt(0) !== null, "the system menu stays");
            verify(bar.appMenuAt(1) !== null, "the application menu stays");
            verify(bar.appMenuAt(2) !== null, "the app's exported menu stays");
            verify(bar.statusItemAt(0) !== null, "the status row stays");

            // Turning it back on restores the same live scene's veil.
            bar.showBackground = true;
            waitForRendering(stage);
            compare(bar.veilActive, true);
            compare(bar.compositorPanelActive, true);

            // The legacy rail must also clear its sampled strip, not leave a
            // stale 28 pt panel behind.
            Glass.rollout = "legacy";
            var legacy = make(menuBarComponent, {
                width: 600,
                showBackground: false,
                materialBlurActive: true
            });
            compare(legacy.color.a, 0);
            compare(legacy.compositorPanelActive, false);
            legacy.showBackground = true;
            waitForRendering(stage);
            compare(legacy.compositorPanelActive, true);
            fuzzyCompare(legacy.color.a, Theme.material.chromeOpacity, 0.001);
            legacy.destroy();
            bar.destroy();
            Glass.rollout = "replacement";
            Theme.dark = false;
        }

        function test_auto_hide_modes_apply_the_visibility_rule() {
            // The compositor owns the actual per-output reveal; this is the
            // same pure rule the bar exposes for its scene/tests.
            var never = make(menuBarComponent, { autoHideMode: "never" });
            compare(never.visibleIn(false, false, false), true);
            compare(never.visibleIn(true, false, false), true);
            compare(never.visibleIn(true, false, true), true);

            var always = make(menuBarComponent, { autoHideMode: "always" });
            compare(always.visibleIn(false, false, false), false,
                    "always starts hidden on the desktop");
            compare(always.visibleIn(true, false, false), false,
                    "always starts hidden in fullscreen");
            compare(always.visibleIn(false, true, false), true,
                    "the edge reveal shows it");
            compare(always.visibleIn(false, false, true), true,
                    "an active menu/focus keeps it available");
            compare(always.visibleIn(true, true, false), true);

            var full = make(menuBarComponent, { autoHideMode: "full-screen" });
            compare(full.visibleIn(false, false, false), true,
                    "full-screen mode is normal on the desktop");
            compare(full.visibleIn(true, false, false), false,
                    "full-screen mode hides over fullscreen");
            compare(full.visibleIn(true, true, false), true,
                    "the fullscreen edge reveal shows it");
            compare(full.visibleIn(true, false, true), true,
                    "an active fullscreen menu keeps it available");

            // Live toggles re-evaluate the same rule.
            full.autoHideMode = "always";
            compare(full.autoHideAlways, true);
            compare(full.visibleIn(false, false, false), false);
            full.autoHideMode = "never";
            compare(full.visibleIn(true, false, false), true);
            compare(full.autoHideInFullscreen, false);

            // The schema default is full-screen.
            var defaultBar = make(menuBarComponent, {});
            compare(defaultBar.autoHideMode, "full-screen");
            compare(defaultBar.visibleIn(false, false, false), true);
            compare(defaultBar.visibleIn(true, false, false), false);

            defaultBar.destroy();
            full.destroy();
            always.destroy();
            never.destroy();
        }

        function compareColor(actual, expected, label) {
            fuzzyCompare(actual.r, expected.r, 0.001, label + " red");
            fuzzyCompare(actual.g, expected.g, 0.001, label + " green");
            fuzzyCompare(actual.b, expected.b, 0.001, label + " blue");
            fuzzyCompare(actual.a, expected.a, 0.001, label + " alpha");
        }

        // -- T202: menu families ------------------------------------------
        function bluetoothModel(overrides) {
            var model = {
                state: "available", present: true, powered: true,
                discovering: false, readOnly: false,
                label: "Bluetooth on",
                knownDevices: [
                    { address: "AA:BB:CC:DD:EE:FF", name: "Headset",
                      connected: true },
                    { address: "11:22:33:44:55:66", name: "Speaker",
                      connected: false }
                ]
            };
            for (var key in (overrides || {}))
                model[key] = overrides[key];
            return model;
        }

        function test_menu_family_presets_and_regions() {
            var savedRollout = Glass.rollout;
            try {
            Glass.rollout = "replacement";
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 800,
                appName: "Test",
                appMenuModel: [ { title: "File", items: [
                    { label: "Open With", type: "submenu",
                      submenu: [ { label: "Text" }, { label: "Image" } ] }
                ] } ],
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems,
                statusItems: defaultStatus(),
                wifiMenu: { state: "available", label: "Wi-Fi", networks: [] },
                trayMenu: [ { id: 1, type: "item", label: "Show", enabled: true } ]
            });
            bar.openMenu(2);
            waitForRendering(stage);
            compare(bar.dropdownPreset, "menuAttached");
            verify(bar.dropdownRegions.length >= 1);
            compare(bar.dropdownRegions[0].preset, "menuAttached");
            compare(bar.dropdownRegions[0].tl, 0);
            compare(bar.dropdownRegions[0].tr, 0);
            fuzzyCompare(bar.dropdownRegions[0].br,
                         Theme.controls.contextMenu.attachedRadius, 0.001);
            var popup = bar.appMenuAt(2).popup;
            compare(popup.materialPreset, "menuAttached");
            compare(popup.materialBackdrop, "trusted");
            compare(popup.motionPreset, "menu");
            verify(Math.abs(popup.surfaceFill.a
                            - Glass.resolve("menuAttached", {
                                  scheme: Theme.dark ? "dark" : "light",
                                  rollout: "replacement",
                                  slider: Glass.slider,
                                  backend: "gpu-sampled", tier: "full"
                              }).alpha) < 1e-4);

            bar.appMenuAt(2).openSubmenu(0);
            waitForRendering(stage);
            verify(bar.dropdownRegions.length >= 2);
            compare(bar.dropdownRegions[1].preset, "menuDetached");

            bar.closeMenus();
            bar.openStatusMenu("wifi");
            waitForRendering(stage);
            compare(bar.dropdownPreset, "statusMenu");
            compare(bar.dropdownRegions[0].preset, "statusMenu");
            compare(findChild(bar, "wifiMenu").popup.materialPreset,
                    "statusMenu");
            bar.closeStatusMenu();

            bar.openTrayMenu("app");
            waitForRendering(stage);
            compare(bar.dropdownPreset, "menuDetached");
            compare(bar.dropdownRegions[0].preset, "menuDetached");
            bar.closeTrayMenu();
            } finally {
                Glass.rollout = savedRollout;
            }
        }

        function test_menu_close_motion_and_reduce_motion() {
            var savedReduced = Theme.reducedMotion;
            Theme.reducedMotion = false;
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 800,
                appName: "Test",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            bar.openMenu(2);
            waitForRendering(stage);
            var popup = bar.appMenuAt(2).popup;
            compare(Theme.motion.menuClose.fullDuration, 183);
            fuzzyCompare(popup.closeScale, 0.95, 0.001);
            compare(popup.motionPreset, "menu");
            bar.closeMenus();
            waitForRendering(stage);
            compare(bar.openMenuIndex, -1, "logical close is immediate");
            compare(popup.open, false, "interaction closes before the visual exit");
            // Reopen before the exit timer: the new menu is the live one.
            bar.openMenu(3);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 3);
            compare(bar.appMenuAt(3).popup.open, true);
            bar.closeMenus();

            Theme.reducedMotion = true;
            var reduced = make(menuBarComponent, {
                width: 800,
                appName: "Test",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            reduced.openMenu(2);
            waitForRendering(stage);
            fuzzyCompare(reduced.appMenuAt(2).popup.closeScale, 1.0, 0.001);
            compare(Theme.motion.menuClose.duration, 0);
            reduced.closeMenus();
            Theme.reducedMotion = savedReduced;
        }

        function test_bluetooth_status_menu_routes_the_action_policy() {
            bluetoothPowerSpy.clear();
            bluetoothDeviceSpy.clear();
            bluetoothSettingsSpy.clear();
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 900,
                appName: "Test",
                appMenuModel: [],
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems,
                statusItems: [
                    { id: "bluetooth", icon: "bluetooth",
                      accessibleName: "Bluetooth", available: true,
                      enabled: true }
                ],
                bluetoothMenu: bluetoothModel({})
            });
            bluetoothPowerSpy.target = bar;
            bluetoothDeviceSpy.target = bar;
            bluetoothSettingsSpy.target = bar;
            bar.openStatusMenu("bluetooth");
            waitForRendering(stage);
            compare(bar.openStatusItem, "bluetooth");
            var menu = findChild(bar, "bluetoothMenu");
            verify(menu !== null);
            compare(menu.popup.open, true);
            compare(menu.popup.materialPreset, "statusMenu");
            compare(menu.materialBackdrop, "trusted");
            menu.togglePower();
            compare(bluetoothPowerSpy.count, 1);
            compare(bluetoothPowerSpy.signalArguments[0][0], false);
            menu.toggleDevice("11:22:33:44:55:66");
            compare(bluetoothDeviceSpy.count, 1);
            compare(bluetoothDeviceSpy.signalArguments[0][0],
                    "11:22:33:44:55:66");
            compare(bluetoothDeviceSpy.signalArguments[0][1], true);
            menu.settingsRequested();
            compare(bluetoothSettingsSpy.count, 1);
            bar.closeStatusMenu();
        }

        function test_bluetooth_absent_read_only_and_menu_bar_gate() {
            var absent = make(bluetoothMenuComponent, {
                model: bluetoothModel({ state: "unavailable" })
            });
            compare(absent.available, false);
            compare(absent.writable, false);
            compare(absent.togglePower(), undefined);
            var readOnly = make(bluetoothMenuComponent, {
                model: bluetoothModel({ readOnly: true })
            });
            compare(readOnly.available, true);
            compare(readOnly.writable, false);
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                width: 900,
                appName: "Test",
                appMenuModel: [],
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems,
                statusItems: [
                    { id: "bluetooth", icon: "bluetooth",
                      accessibleName: "Bluetooth", available: true }
                ],
                bluetoothMenu: bluetoothModel({ state: "unavailable" })
            });
            bar.openStatusMenu("bluetooth");
            compare(bar.openStatusItem, "", "absent adapter has no menu");
            var hidden = make(menuBarComponent, {
                width: 900,
                statusItems: [
                    { id: "bluetooth", icon: "bluetooth",
                      accessibleName: "Bluetooth", available: false }
                ]
            });
            compare(hidden.statusItemAt(0).visible, false);
        }

        function test_popup_role_material_covers_every_menu_popover() {
            // T-21.2c: the bar scene hosts the top-level dropdowns, their
            // submenus, the status menus, and the tray context menu. All of
            // them apply the one transient Popup state the shell pushes.
            var states = [
                { tier: 0, blur: true, fallback: 0, reason: "none", label: "full" },
                { tier: 1, blur: true, fallback: 0, reason: "none", label: "reduced" },
                { tier: 2, blur: false, fallback: 0, reason: "minimal", label: "minimal" },
                { tier: 2, blur: false, fallback: 2, reason: "software", label: "software" }
            ];
            var fixed = fixedMenus();
            for (var scheme = 0; scheme < 2; ++scheme) {
                Theme.dark = scheme === 1;
                for (var i = 0; i < states.length; ++i) {
                    var state = states[i];
                    var bar = make(menuBarComponent, {
                        width: 800,
                        appName: "Test",
                        appMenuModel: sampleModel(),
                        systemMenuItems: fixed.systemMenuItems,
                        applicationMenuItems: fixed.applicationMenuItems,
                        statusItems: defaultStatus(),
                        trayMenu: [ { id: 1, type: "item", label: "Show", enabled: true } ],
                        popupMaterialTier: state.tier,
                        popupMaterialBlurActive: state.blur,
                        popupMaterialFallback: state.fallback
                    });
                    compare(bar.popupMaterialTier, state.tier, state.label);
                    compare(bar.popupMaterialBlurActive, state.blur, state.label);
                    compare(bar.popupMaterialFallback, state.fallback, state.label);
                    compare(bar.popupMaterialFallbackReason, state.reason,
                            state.label);

                    var expected = Material.popupFill(state.blur);
                    var menus = [
                        { item: bar.appMenuAt(0), surface: "popupSurface", label: "dropdown" },
                        { item: bar.appMenuAt(0), surface: "submenuSurface", label: "submenu" },
                        { item: findChild(bar, "wifiMenu"), surface: "popupSurface", label: "wifi" },
                        { item: findChild(bar, "volumeMenu"), surface: "popupSurface", label: "volume" },
                        { item: findChild(bar, "batteryMenu"), surface: "popupSurface", label: "battery" },
                        { item: findChild(bar, "trayContextMenu"), surface: "contextMenuSurface", label: "tray" }
                    ];
                    for (var j = 0; j < menus.length; ++j) {
                        var menu = menus[j].item;
                        verify(menu !== null, menus[j].label + " exists");
                        compare(menu.materialTier, state.tier,
                                menus[j].label + " " + state.label);
                        compare(menu.materialBlurActive, state.blur,
                                menus[j].label + " " + state.label);
                        compare(menu.materialFallback, state.fallback,
                                menus[j].label + " " + state.label);
                        compare(menu.materialFallbackReason, state.reason,
                                menus[j].label + " " + state.label);
                        var surface = findChild(menu, menus[j].surface);
                        verify(surface !== null, menus[j].surface + " exists");
                        compareColor(surface.color, expected,
                                     menus[j].label + " " + state.label);
                        verify(surface.color.a >= 0.9 || state.blur,
                               menus[j].label + " fallback must read as a plate");
                    }
                }
            }
            Theme.dark = false;
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

        function test_global_menu_toggle_hides_app_menus() {
            var fixed = fixedMenus();
            var bar = make(menuBarComponent, {
                appName: "Safari",
                appMenuModel: sampleModel(),
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems,
                globalMenuEnabled: false
            });
            // T-14.2b: off suppresses the exported menus only; the fixed system
            // and application menus stay, so the empty desktop still reads.
            compare(bar.appMenuAt(0).showLogo, true);
            compare(bar.appMenuAt(1).title, "Safari");
            compare(bar.appMenuAt(2), null);

            // Turning the toggle back on restores the app's own menus.
            bar.globalMenuEnabled = true;
            compare(bar.appMenuAt(2).title, "File");
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

        // T-07.6a: mask one daemon at a time. That slot hides and its popover
        // refuses to open; every other slot stays live and opens. This mirrors
        // the controller's output (visible from the model, menu state from the
        // same view) without a host on the bus.
        function test_absent_daemon_matrix_hides_and_refuses_each_item() {
            var ids = ["wifi", "volume", "battery"];
            var presentModels = { wifi: wifiModel(), volume: audioModel(false, 0.6),
                                  battery: batteryModel() };
            var maskedModels = {
                wifi: { kind: "wifi", state: "unavailable" },
                volume: { kind: "audio", state: "unavailable" },
                battery: { kind: "battery", state: "unavailable" }
            };

            for (var i = 0; i < ids.length; ++i) {
                var items = [];
                for (var j = 0; j < ids.length; ++j) {
                    items.push({ id: ids[j], icon: ids[j],
                                 accessibleName: ids[j], available: j !== i });
                }
                var bar = make(menuBarComponent, {
                    width: 800,
                    statusItems: items,
                    wifiMenu: i === 0 ? maskedModels.wifi : presentModels.wifi,
                    volumeMenu: i === 1 ? maskedModels.volume : presentModels.volume,
                    batteryMenu: i === 2 ? maskedModels.battery : presentModels.battery
                });

                // The masked slot is hidden and has nothing to open.
                compare(bar.statusItemFor(ids[i]).visible, false);
                bar.openStatusMenu(ids[i]);
                waitForRendering(stage);
                compare(bar.openStatusItem, "");

                // Every other slot is still live and opens its popover.
                for (var k = 0; k < ids.length; ++k) {
                    if (k === i)
                        continue;
                    compare(bar.statusItemFor(ids[k]).visible, true);
                    bar.openStatusMenu(ids[k]);
                    waitForRendering(stage);
                    compare(bar.openStatusItem, ids[k]);
                    bar.closeStatusMenu();
                    waitForRendering(stage);
                }
            }
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
            // Wi-Fi, volume, and battery open a popover (T-07.5a/b); every
            // other item still raises statusItemActivated.
            var bar = make(menuBarComponent, {
                statusItems: [ { id: "accessibility", icon: "accessibility",
                                 accessibleName: "Accessibility", available: true } ]
            });
            statusSpy.target = bar;
            statusSpy.clear();
            var item = bar.statusItemAt(0);
            mouseClick(item, item.width / 2, item.height / 2);
            compare(statusSpy.count, 1);
            compare(statusSpy.signalArguments[0][0], "accessibility");
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
            compare(controlCenter.Accessible.name, "Quick Settings");
            compare(missionControl.Accessible.name, "Overview");

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

        // -- Battery status menu (T-07.5b) ---------------------------------

        function test_battery_menu_exposes_read_only_state() {
            var menu = make(batteryMenuComponent, { model: batteryModel() });
            compare(menu.available, true);
            compare(menu.present, true);
            compare(menu.charging, true);
            compare(menu.percent, 82);
            compare(menu.level, 0.82);
            compare(menu.label, "82% charging");
            // Charging reports time to full, not time left.
            compare(menu.remaining, "1:30");
        }

        function test_battery_menu_open_refreshes() {
            var menu = make(batteryMenuComponent, { model: batteryModel() });
            batteryRefreshSpy.target = menu;
            batteryRefreshSpy.clear();
            menu.open = true;
            waitForRendering(stage);
            compare(batteryRefreshSpy.count, 1);
        }

        function test_status_item_click_opens_the_battery_popover() {
            var bar = make(menuBarComponent, {
                width: 800,
                statusItems: [ { id: "battery", icon: "battery",
                                 accessibleName: "Battery", available: true } ],
                batteryMenu: batteryModel()
            });
            var battery = bar.statusItemFor("battery");
            verify(battery !== null);
            mouseClick(battery, battery.width / 2, battery.height / 2);
            waitForRendering(stage);
            compare(bar.openStatusItem, "battery");
            var menu = findChild(bar, "batteryMenu");
            verify(menu !== null);
            compare(menu.popup.open, true);
            verify(bar.dropdownWidth > 0);
            bar.closeStatusMenu();
            waitForRendering(stage);
            compare(bar.openStatusItem, "");
        }

        function test_a_no_battery_machine_opens_nothing() {
            var model = batteryModel();
            model.present = false;
            var bar = make(menuBarComponent, {
                width: 800,
                statusItems: [ { id: "battery", icon: "battery",
                                 accessibleName: "Battery", available: true } ],
                batteryMenu: model
            });
            bar.openStatusMenu("battery");
            waitForRendering(stage);
            compare(bar.openStatusItem, "");
        }

        // -- Keyboard accessibility (T-07.5b) -------------------------------

        function test_keyboard_arrows_select_status_items() {
            var bar = make(menuBarComponent, {
                width: 800,
                statusItems: defaultStatus()
            });
            bar.forceActiveFocus();
            waitForRendering(stage);
            compare(bar.keyboardStatusIndex, -1);

            keyClick(Qt.Key_Right);
            waitForRendering(stage);
            // The bluetooth slot is hidden, so the first selection is Wi-Fi.
            compare(bar.keyboardStatusIndex, 0);
            compare(bar.statusItemAt(0).keyboardFocus, true);

            keyClick(Qt.Key_Right);
            waitForRendering(stage);
            // Bluetooth is skipped.
            compare(bar.keyboardStatusIndex, 2);
        }

        function test_keyboard_return_opens_and_escape_closes_the_battery_menu() {
            var bar = make(menuBarComponent, {
                width: 800,
                statusItems: [ { id: "battery", icon: "battery",
                                 accessibleName: "Battery", available: true } ],
                batteryMenu: batteryModel()
            });
            bar.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Right);
            waitForRendering(stage);
            compare(bar.keyboardStatusIndex, 0);

            keyClick(Qt.Key_Return);
            waitForRendering(stage);
            compare(bar.openStatusItem, "battery");

            keyClick(Qt.Key_Escape);
            waitForRendering(stage);
            compare(bar.openStatusItem, "");
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

        // T226: the Date & Time pane's 24-hour preference forces the clock
        // format while the default keeps following the locale.
        function test_clock_honours_the_24_hour_preference() {
            var fixed = new Date(2026, 8, 19, 14, 5, 0);
            var clock = make(clockComponent, { now: fixed, use24Hour: true });
            compare(clock.timeText, "14:05");
            clock.showSeconds = true;
            compare(clock.timeText, "14:05:00");
            clock.use24Hour = false;
            compare(clock.timeText,
                    Qt.formatTime(fixed, Qt.locale().timeFormat(Locale.LongFormat)));
        }

        function test_the_clock_is_the_notification_center_entry_point() {
            var bar = make(menuBarComponent, {
                appMenu: [],
                statusItems: [],
                showDate: true
            });
            clockSpy.target = bar;
            clockSpy.clear();
            var clock = findChild(bar, "clock");
            verify(clock);
            mouseClick(clock, clock.width / 2, clock.height / 2);
            compare(clockSpy.count, 1);
        }

        // -- Status glyph rendering (FR-5) ---------------------------------

        // Every state must render a non-empty Tabler mark (ADR 0185). This is
        // the state-coverage case: a typo in StatusGlyph's state -> glyph
        // mapping or an unvendored name would leave a state blank.
        function test_status_glyphs_render_pixels() {
            var names = ["wifi", "wifi-secure", "wifi-off", "wifi-disabled",
                         "wifi-connecting", "wifi-error", "bluetooth",
                         "volume", "volume-muted", "battery", "battery-charging",
                         "focus", "accessibility", "control-center",
                         "mission-control"];
            for (var i = 0; i < names.length; ++i) {
                var glyph = make(glyphComponent, { name: names[i], size: 32 });
                verify(glyph.glyphName.length > 0,
                       "state " + names[i] + " must map to a Tabler glyph");
                verify(imageHasMark(grabImage(glyph)),
                       "glyph " + names[i] + " should render visible pixels");
                glyph.destroy();
            }
        }

        // The battery level stays continuous: a high level inks the cell
        // interior, a near-empty level leaves it clear. The sample point is
        // 60% across the mark, inside the outline's cell but right of where a
        // low fill stops.
        function test_battery_level_fill_is_continuous() {
            // 60% across the mark, mid-height: inside the Tabler cell, right
            // of where a 5% fill stops and left of a 95% fill's edge. The two
            // glyphs are made and grabbed one at a time (overlapping siblings
            // would share a window grab).
            var high = make(glyphComponent, { name: "battery", size: 64, level: 0.95 });
            var x = Math.round(high.width * 0.60);
            var y = Math.round(high.height * 0.50);
            var hi = grabImage(high);
            verify(!isBackground(hi, x, y),
                   "a high battery level must fill the cell interior");
            high.destroy();

            var low = make(glyphComponent, { name: "battery", size: 64, level: 0.05 });
            var lo = grabImage(low);
            verify(isBackground(lo, x, y),
                   "a low battery level must leave the cell interior clear");
            low.destroy();
        }

        // -- Focus/DND reflection (T-11.2b) --------------------------------

        function test_focus_status_item_reflects_policy() {
            // The controller feeds this map from `focusStatusItem`; here we
            // assert the bar renders the three policy states.
            var bar = make(menuBarComponent, {
                width: 800,
                statusItems: [ { id: "focus", icon: "focus", accessibleName: "Focus",
                                 available: true, enabled: true, selected: false } ]
            });
            var focus = bar.statusItemFor("focus");
            compare(focus.visible, true);
            compare(focus.selected, false);

            // DND is selected and carries the suppressed batch count.
            bar.statusItems = [ { id: "focus", icon: "focus", label: "4",
                                  accessibleName: "Do Not Disturb (4 notifications silenced)",
                                  available: true, enabled: true, selected: true } ];
            waitForRendering(stage);
            focus = bar.statusItemFor("focus");
            compare(focus.visible, true);
            compare(focus.selected, true);
            compare(focus.label, "4");
            verify(focus.accessibleName.indexOf("Do Not Disturb") === 0);

            // Off hides the item and gives it no width.
            bar.statusItems = [ { id: "focus", icon: "focus", accessibleName: "Focus",
                                  available: false } ];
            waitForRendering(stage);
            focus = bar.statusItemFor("focus");
            compare(focus.visible, false);
            compare(focus.width, 0);
        }

        // -- System menu brand mark (T-09) ----------------------------------

        function test_logo_renders_pixels_and_tints_data() {
            return [
                { tag: "menu-size", size: Theme.controls.menuBar.iconSize },
                { tag: "large", size: 120 }
            ];
        }

        function test_logo_renders_pixels_and_tints(data) {
            var logo = make(logoComponent, { size: data.size, color: Theme.color.textPrimary });
            // Swept Trio uses a square view box and a solid, tintable body.
            compare(logo.implicitWidth, data.size);
            compare(logo.implicitHeight, data.size);
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

        // -- Fixed application menu live state (T-14.2a) -------------------

        // The menu-broker fixes the application menu's Hide/Hide Others/Show
        // All enabled state from the live window projection. Here the model
        // the controller publishes is injected directly: the bar renders a
        // disabled verb dimmed and refuses to dispatch it.
        function test_fixed_application_menu_reflects_live_hide_state() {
            var fixed = fixedMenus();
            fixed.applicationMenuItems = [
                { label: "About Test", action: "about" },
                { type: "separator" },
                { label: "Hide Test", shortcut: "Super+H", action: "hide", enabled: false },
                { label: "Hide Others", shortcut: "Super+Alt+H",
                  action: "hide-others", enabled: true },
                { label: "Show All", action: "show-all", enabled: false },
                { type: "separator" },
                { label: "Quit Test", action: "quit" }
            ];
            var bar = make(menuBarComponent, {
                appName: "Test",
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            triggeredSpy.target = bar;
            triggeredSpy.clear();

            var app = bar.appMenuAt(1);
            compare(app.model.length, 7);
            compare(app.model[2].label, "Hide Test");
            compare(app.model[2].enabled, false);
            compare(app.model[3].enabled, true);
            compare(app.model[4].enabled, false);

            // A disabled row is inert; an enabled one dispatches.
            app.activate(2);
            compare(triggeredSpy.count, 0);
            app.activate(3);
            compare(triggeredSpy.count, 1);
            compare(triggeredSpy.signalArguments[0][2].action, "hide-others");
        }

        // -- StatusNotifier tray items (T-14.3) -----------------------------

        // A third-party tray item renders in the same status row as the
        // first-party items, and clicking it opens its DBusMenu; a chosen row
        // raises the trigger with the item's registered name and DBusMenu id.
        function test_tray_item_renders_and_opens_its_menu() {
            var bar = make(menuBarComponent, {
                statusItems: [
                    { id: "tray:org.example.Mock", icon: "application",
                      iconSource: "/tmp/df-tray-mock.png",
                      accessibleName: "Mock Tray", available: true }
                ]
            });
            var slot = bar.statusItemFor("tray:org.example.Mock");
            verify(slot !== null);
            compare(slot.visible, true);
            compare(slot.accessibleName, "Mock Tray");

            // The shell sets `trayMenu` from app-index, then opens it.
            bar.trayMenu = [
                { id: 1, type: "item", label: "Show Window", enabled: true },
                { id: 9, type: "separator", label: "" },
                { id: 5, type: "submenu", label: "Tools", enabled: true,
                  submenu: [ { id: 6, type: "item", label: "Preferences" } ] }
            ];
            traySpy.target = bar;
            traySpy.clear();
            bar.openTrayMenu("org.example.Mock");
            verify(bar.trayMenuOpen);
            var menu = findChild(bar, "trayContextMenu");
            verify(menu !== null);
            verify(menu.open);
            compare(menu.model.length, 3);

            menu.activate(0);
            compare(traySpy.count, 1);
            compare(traySpy.signalArguments[0][0], "org.example.Mock");
            compare(traySpy.signalArguments[0][1], 1);

            // Escape/close collapses the tray menu.
            bar.closeTrayMenu();
            verify(!bar.trayMenuOpen);
            verify(!menu.open);
        }
    }
}
