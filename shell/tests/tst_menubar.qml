// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.MenuBar
import Dragonfruit.ControlCenter

// Menu-bar tests (T-09 FR-1…FR-7). Runs headless on the offscreen platform
// and the software scene graph, so the interaction walkthrough is scripted
// and CI-able. `stage` is the visual root so `grabImage` returns pixels.
Item {
    id: stage
    width: 1000
    height: 560

    TestCase {
        id: testCase
        name: "MenuBar"
        when: windowShown

        Component { id: menuBarComponent; MenuBar { } }
        Component { id: displayPolicyComponent; MenuBarDisplayPolicy { } }
        Component {
            id: backdropComponent
            Rectangle { }
        }
        Component { id: statusItemComponent; StatusItem { } }
        Component { id: registryComponent; StatusItemRegistry { } }
        Component { id: dateTimePanelComponent; DateTimePanel { } }
        Component { id: clockComponent; MenuBarClock { } }
        Component { id: glyphComponent; StatusGlyph { } }
        Component { id: logoComponent; DragonfruitLogo { } }
        Component { id: wifiMenuComponent; WifiMenu { } }
        Component { id: volumeMenuComponent; VolumeMenu { } }
        Component { id: batteryMenuComponent; BatteryMenu { } }
        Component { id: bluetoothMenuComponent; BluetoothMenu { } }
        Component { id: controlCenterComponent; ControlCenter { } }

        SignalSpy { id: wifiJoinSpy; signalName: "joinRequested" }
        SignalSpy { id: wifiRefreshSpy; signalName: "refreshRequested" }
        SignalSpy { id: batteryRefreshSpy; signalName: "refreshRequested" }
        SignalSpy { id: volumeSetSpy; signalName: "volumeSetRequested" }
        SignalSpy { id: muteSpy; signalName: "muteToggleRequested" }
        SignalSpy { id: statusSpy; signalName: "statusItemActivated" }
        SignalSpy { id: ccSpy; signalName: "controlCenterRequested" }
        SignalSpy { id: mcSpy; signalName: "missionControlRequested" }
        SignalSpy { id: openedSpy; signalName: "appMenuOpened" }
        SignalSpy { id: closedSpy; signalName: "appMenuClosed" }
        SignalSpy { id: triggeredSpy; signalName: "appMenuTriggered" }
        SignalSpy { id: traySpy; signalName: "trayMenuTriggered" }
        SignalSpy { id: bluetoothPowerSpy; signalName: "bluetoothPowerToggleRequested" }
        SignalSpy { id: bluetoothDeviceSpy; signalName: "bluetoothDeviceToggled" }
        SignalSpy { id: bluetoothSettingsSpy; signalName: "bluetoothSettingsRequested" }
        SignalSpy { id: bluetoothMenuSettingsSpy; signalName: "settingsRequested" }
        SignalSpy { id: bluetoothDiscoverySpy; signalName: "discoveringToggleRequested" }
        SignalSpy { id: bluetoothPairSpy; signalName: "pairRequested" }
        SignalSpy { id: wifiSettingsSpy; signalName: "settingsRequested" }
        SignalSpy { id: volumeSettingsSpy; signalName: "settingsRequested" }
        SignalSpy { id: sinkSpy; signalName: "sinkSelected" }
        SignalSpy { id: batterySettingsSpy; signalName: "settingsRequested" }
        SignalSpy { id: powerProfileSpy; signalName: "powerProfileSelected" }

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
                width: 600, backgroundLuma: 1.0,
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
            bar.backgroundLuma = 0.5;
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
            // Wallpaper brightness selects the pair in both app appearances;
            // dark and mid-grey backgrounds prefer white glyphs.
            Glass.rollout = "replacement";
            Theme.dark = false;
            var bar = make(menuBarComponent, {
                width: 600,
                materialBlurActive: true
            });
            bar.backgroundLuma = 0.0;
            compare(bar.foreground.toString(),
                    Theme.controls.menuBar.foregroundOnDark.toString(),
                    "Light appearance still uses white text over black");
            // The lettering follows the veil-composited backdrop, not the raw
            // wallpaper: the dark wash keeps white text through the mid band,
            // and the white lift flips to black at the measured handover.
            bar.backgroundLuma = 0.6;
            compare(bar.foreground.toString(),
                    Theme.controls.menuBar.foregroundOnDark.toString(),
                    "the dark veil keeps white text over mid-grey");
            bar.backgroundLuma = 0.78;
            compare(bar.foreground.toString(),
                    Theme.controls.menuBar.foregroundOnLight.toString(),
                    "the white lift flips to black text at the handover");
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

        // -- T233: measured overflow, notch, reveal motion, multi-display ---

        function t233Status() {
            return [
                { id: "wifi", icon: "wifi", priority: 10, accessibleName: "Wi-Fi" },
                { id: "bluetooth", icon: "bluetooth", priority: 20,
                  accessibleName: "Bluetooth" },
                { id: "battery", icon: "battery", priority: 30,
                  accessibleName: "Battery" },
                { id: "volume", icon: "volume", priority: 40,
                  accessibleName: "Volume" },
                { id: "search", icon: "search", priority: 50,
                  accessibleName: "Search" },
                { id: "notifications", icon: "notifications", priority: 60,
                  accessibleName: "Notifications" },
                { id: "clock", kind: "clock", menu: "clock", priority: 70,
                  essential: true, accessibleName: "Date & Time",
                  tooltip: "Date & Time" },
                { id: "control-center", icon: "control-center", priority: 80,
                  essential: true, accessibleName: "Quick Settings" },
                { id: "mission-control", icon: "mission-control", priority: 90,
                  essential: true, accessibleName: "Overview" },
                { id: "tray:one", icon: "application", priority: 1000,
                  thirdParty: true, accessibleName: "Tray One" },
                { id: "tray:two", icon: "application", priority: 1001,
                  thirdParty: true, accessibleName: "Tray Two" }
            ];
        }

        function test_measured_overflow_follows_width_notch_and_priority() {
            var wide = make(menuBarComponent, {
                width: 1400, statusItems: t233Status()
            });
            compare(wide.statusHasOverflow, false,
                    "a wide bar keeps every item in the row");
            compare(wide.effectiveStatusCapacity,
                    wide.statusRowEntries.length);

            var narrow = make(menuBarComponent, {
                width: 420, statusItems: t233Status()
            });
            verify(narrow.statusHasOverflow,
                   "a narrow bar moves extras into overflow");
            verify(narrow.measuredStatusCapacity < wide.measuredStatusCapacity,
                   "the capacity follows the measured width");
            compare(narrow.statusRowEntries[narrow.statusRowEntries.length - 1].id,
                    "overflow");
            for (var i = 0; i < narrow.statusRowEntries.length - 1; ++i) {
                compare(narrow.statusRowEntries[i].thirdParty, false,
                        "no third-party item stays while a system item is pushed out");
            }
            var overflowIds = [];
            for (var j = 0; j < narrow.statusOverflowEntries.length; ++j)
                overflowIds.push(narrow.statusOverflowEntries[j].id);
            verify(overflowIds.indexOf("tray:one") >= 0);
            verify(overflowIds.indexOf("tray:two") >= 0);

            // A notch is a safe-area inset, not a hit target: it reduces the
            // budget and therefore can only move more items to overflow.
            var notched = make(menuBarComponent, {
                width: 420, notchWidth: 120, statusItems: t233Status()
            });
            verify(notched.measuredStatusCapacity <= narrow.measuredStatusCapacity,
                   "the notch reduces the measured status budget");
            verify(notched.statusRowEntries.length > 0);
        }

        function test_a_tiny_bar_keeps_every_item_reachable_through_overflow() {
            var bar = make(menuBarComponent, {
                width: 40, statusItems: t233Status()
            });
            compare(bar.measuredStatusCapacity, 0);
            compare(bar.statusRowEntries.length, 1,
                    "the overflow affordance is the only slot that fits");
            compare(bar.statusRowEntries[0].id, "overflow");
            verify(bar.statusOverflowEntries.length >= 9,
                   "every item that did not fit is listed, not clipped");

            bar.openOverflowMenu();
            waitForRendering(stage);
            compare(bar.overflowMenuOpen, true);
            verify(bar.overflowMenu.length >= 9);
            bar.closeOverflowMenu();
        }

        function test_active_display_owns_the_application_menu() {
            var active = make(menuBarComponent, {
                width: 800, activeDisplay: true,
                appName: "Files",
                applicationMenuItems: [ { label: "About Files" } ]
            });
            verify(active.appMenuAt(0), "the system menu is always present");
            verify(active.appMenuAt(1), "the active display owns the app menu");
            compare(active.appMenuAt(1).title, "Files");

            var secondary = make(menuBarComponent, {
                width: 800, activeDisplay: false, showStatusItems: false,
                appName: "Files",
                applicationMenuItems: [ { label: "About Files" } ]
            });
            verify(secondary.appMenuAt(0), "the system menu stays on every display");
            compare(secondary.appMenuAt(1), null,
                    "a secondary display does not duplicate the app menu");
            compare(secondary.statusRowVisible, false,
                    "status items follow the per-display policy");
        }

        function test_display_policy_resolves_one_bar_per_display_and_hotplug() {
            var policy = make(displayPolicyComponent, {
                displays: [
                    { name: "eDP-1", active: true, notchWidth: 0 },
                    { name: "HDMI-A-1", active: false, notchWidth: 90 }
                ]
            });
            compare(policy.barCount, 2, "one bar per online display");
            compare(policy.activeDisplay, "eDP-1");
            compare(policy.bars[0].ownsAppMenu, true);
            compare(policy.bars[1].ownsAppMenu, false);
            compare(policy.bars[1].showStatusItems, true,
                    "the default policy shows status items on every display");
            compare(policy.bars[1].notchWidth, 90);

            policy.statusPolicy = "active";
            compare(policy.bars[0].showStatusItems, true);
            compare(policy.bars[1].showStatusItems, false,
                    "the active-only policy hides the secondary status row");

            // Hotplug: removing the active display re-homes the app menu to
            // the remaining online display; an offline entry has no bar.
            policy.displays = [
                { name: "eDP-1", active: true, online: false },
                { name: "HDMI-A-1", active: false, online: true }
            ];
            compare(policy.barCount, 1);
            compare(policy.activeDisplay, "HDMI-A-1");
            compare(policy.ownsAppMenu("HDMI-A-1"), true);
            compare(policy.barFor("eDP-1").present, false);
        }

        function test_reveal_progress_mirrors_the_measured_slide() {
            var bar = make(menuBarComponent, { width: 800 });
            compare(bar.revealProgress, 1);
            compare(bar.revealOffset, 0);
            bar.revealProgress = 0;
            compare(bar.revealOffset, Theme.controls.menuBar.height);
            compare(bar.revealMotionActive, false);
            bar.revealProgress = 0.5;
            fuzzyCompare(bar.revealOffset, Theme.controls.menuBar.height / 2, 0.001);
            compare(bar.revealMotionActive, true);
            bar.revealProgress = 1;
            compare(bar.revealOffset, 0);
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
            compare(bar.dropdownSurfaceRegions[0].x,
                    bar.dropdownX - Math.floor(bar.dropdownX));
            compare(bar.dropdownSurfaceRegions[0].y,
                    bar.dropdownY - Math.floor(bar.dropdownY));
            compare(bar.dropdownRegions[0].tl, 0);
            compare(bar.dropdownRegions[0].tr, 0);
            fuzzyCompare(bar.dropdownRegions[0].br,
                         Theme.controls.contextMenu.attachedRadius, 0.001);
            var popup = bar.appMenuAt(2).popup;
            compare(popup.materialPreset, "menuAttached");
            compare(popup.materialBackdrop, "trusted");
            compare(popup.motionPreset, "menu");
            fuzzyCompare(popup.surfaceFill.a, Glass.resolve("menuAttached", {scheme: Theme.dark ? "dark" : "light"}).alpha, 1e-4);

            bar.appMenuAt(2).openSubmenu(0);
            waitForRendering(stage);
            verify(bar.dropdownRegions.length >= 2);
            compare(bar.dropdownRegions[1].preset, "menuDetached");
            for (var i = 0; i < bar.dropdownRegions.length; ++i) {
                var local = bar.dropdownSurfaceRegions[i];
                compare(local.x + Math.floor(bar.dropdownX), bar.dropdownRegions[i].x);
                compare(local.y + Math.floor(bar.dropdownY), bar.dropdownRegions[i].y);
                verify(local.x >= 0 && local.x + local.w <= Math.ceil(bar.dropdownX + bar.dropdownWidth) - Math.floor(bar.dropdownX));
                verify(local.y >= 0 && local.y + local.h <= Math.ceil(bar.dropdownY + bar.dropdownHeight) - Math.floor(bar.dropdownY));
            }

            bar.closeMenus();
            bar.openStatusMenu("wifi");
            waitForRendering(stage);
            compare(bar.dropdownPreset, "statusMenu");
            compare(bar.dropdownRegions[0].preset, "statusMenu");
            compare(bar.dropdownSurfaceRegions[0].x,
                    bar.dropdownX - Math.floor(bar.dropdownX));
            compare(bar.dropdownSurfaceRegions[0].y,
                    bar.dropdownY - Math.floor(bar.dropdownY));
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

        // -- The status-item registry (T231) --------------------------------

        function test_registry_orders_system_items_and_puts_third_party_last() {
            var registry = make(registryComponent, {
                items: [
                    { id: "volume", icon: "volume", priority: 40 },
                    { id: "tray:one", icon: "application", priority: 1000,
                      thirdParty: true, accessibleName: "Tray One" },
                    { id: "wifi", icon: "wifi", priority: 10 },
                    { id: "battery", icon: "battery", priority: 30 },
                    { id: "bluetooth", icon: "bluetooth", priority: 20 }
                ]
            });
            compare(registry.rowItems.length, 5);
            compare(registry.rowItems[0].id, "wifi");
            compare(registry.rowItems[1].id, "bluetooth");
            compare(registry.rowItems[2].id, "battery");
            compare(registry.rowItems[3].id, "volume");
            compare(registry.rowItems[4].id, "tray:one",
                    "a third-party item never sorts above a system item");
            compare(registry.orderedItems[4].thirdParty, true);
            compare(registry.hasOverflow, false);
        }

        function test_registry_normalizes_the_status_item_shape() {
            var registry = make(registryComponent, {
                items: [
                    { id: "wifi", icon: "wifi", priority: 10, visible: false,
                      tooltip: "Wi-Fi", state: "unavailable" },
                    { id: "battery", icon: "battery", priority: 30,
                      available: false, state: "error" },
                    { id: "search", icon: "search", priority: 50,
                      accessibleName: "Search" }
                ]
            });
            // A hidden/absent slot keeps its index so later items do not
            // shift; the delegate draws it with zero width.
            compare(registry.rowItems.length, 3);
            compare(registry.rowItems[0].available, false);
            compare(registry.rowItems[0].state, "unavailable");
            compare(registry.rowItems[0].tooltip, "Wi-Fi");
            compare(registry.rowItems[1].available, false);
            compare(registry.rowItems[1].state, "error");
            compare(registry.rowItems[2].available, true);
            compare(registry.rowItems[2].accessibleName, "Search");
            compare(registry.rowItems[2].tooltip, "Search");
        }

        function test_registry_overflow_pushes_third_party_before_system() {
            var registry = make(registryComponent, {
                maxVisible: 5,
                items: [
                    { id: "wifi", icon: "wifi", priority: 10 },
                    { id: "bluetooth", icon: "bluetooth", priority: 20 },
                    { id: "battery", icon: "battery", priority: 30 },
                    { id: "volume", icon: "volume", priority: 40 },
                    { id: "clock", icon: "", priority: 70, kind: "clock",
                      essential: true },
                    { id: "tray:one", icon: "application", priority: 1000,
                      thirdParty: true, accessibleName: "Tray One" },
                    { id: "tray:two", icon: "application", priority: 1001,
                      thirdParty: true, accessibleName: "Tray Two" }
                ]
            });
            compare(registry.rowItems.length, 5);
            compare(registry.overflowItems.length, 2);
            compare(registry.overflowItems[0].id, "tray:one");
            compare(registry.overflowItems[1].id, "tray:two");
            for (var i = 0; i < registry.rowItems.length; ++i)
                compare(registry.rowItems[i].thirdParty, false,
                        "no third-party item stays while overflow is active");
            compare(registry.statusRowItems[registry.statusRowItems.length - 1].id,
                    "overflow");
        }

        function test_registry_keeps_essential_system_items_in_the_row() {
            var registry = make(registryComponent, {
                maxVisible: 3,
                items: [
                    { id: "wifi", icon: "wifi", priority: 10 },
                    { id: "bluetooth", icon: "bluetooth", priority: 20 },
                    { id: "battery", icon: "battery", priority: 30 },
                    { id: "volume", icon: "volume", priority: 40 },
                    { id: "clock", icon: "", priority: 70, kind: "clock",
                      essential: true },
                    { id: "tray:one", icon: "application", priority: 1000,
                      thirdParty: true, accessibleName: "Tray One" }
                ]
            });
            compare(registry.rowItems.length, 3);
            compare(registry.rowItems[0].id, "wifi");
            compare(registry.rowItems[1].id, "bluetooth");
            compare(registry.rowItems[2].id, "clock",
                    "the essential clock survives before optional system items");
            compare(registry.overflowItems.length, 3);
            compare(registry.overflowItems[0].id, "battery");
            compare(registry.overflowItems[1].id, "volume");
            compare(registry.overflowItems[2].id, "tray:one");
        }

        function test_registry_hidden_items_do_not_consume_the_budget() {
            var registry = make(registryComponent, {
                maxVisible: 2,
                items: [
                    { id: "wifi", icon: "wifi", priority: 10 },
                    { id: "bluetooth", icon: "bluetooth", priority: 20,
                      visible: false },
                    { id: "battery", icon: "battery", priority: 30 },
                    { id: "volume", icon: "volume", priority: 40 },
                    { id: "tray:one", icon: "application", priority: 1000,
                      thirdParty: true, accessibleName: "Tray One" }
                ]
            });
            compare(registry.rowItems.length, 3,
                    "the hidden slot stays in the row with zero width");
            compare(registry.rowItems[1].id, "bluetooth");
            compare(registry.rowItems[1].available, false);
            compare(registry.overflowItems.length, 2);
            compare(registry.overflowItems[0].id, "volume");
            compare(registry.overflowItems[1].id, "tray:one");
        }

        function test_overflow_menu_lists_hidden_items_and_activates_them() {
            var bar = make(menuBarComponent, {
                width: 700,
                statusOverflowCapacity: 4,
                statusItems: [
                    { id: "wifi", icon: "wifi", priority: 10,
                      accessibleName: "Wi-Fi" },
                    { id: "tray:one", icon: "application", priority: 1000,
                      thirdParty: true, accessibleName: "Tray One" },
                    { id: "tray:two", icon: "application", priority: 1001,
                      thirdParty: true, accessibleName: "Tray Two" }
                ]
            });
            var overflow = bar.statusItemFor("overflow");
            verify(overflow);
            compare(overflow.visible, true);
            compare(overflow.Accessible.name, "More menu bar items");

            statusSpy.target = bar;
            statusSpy.clear();
            bar.openOverflowMenu();
            waitForRendering(stage);
            compare(bar.overflowMenuOpen, true);
            compare(bar.overflowMenu.length, 2);
            compare(bar.overflowMenu[0].label, "Tray One");
            compare(bar.overflowMenu[1].label, "Tray Two");

            bar.activateStatusItem("tray:two");
            compare(statusSpy.count, 1);
            compare(statusSpy.signalArguments[0][0], "tray:two");
            bar.closeOverflowMenu();
        }

        function test_status_tooltip_is_the_accessible_description() {
            var bar = make(menuBarComponent, {
                statusItems: [
                    { id: "wifi", icon: "wifi", priority: 10,
                      accessibleName: "Wi-Fi", tooltip: "Wi-Fi, home" }
                ]
            });
            var wifi = bar.statusItemFor("wifi");
            compare(wifi.tooltip, "Wi-Fi, home");
            compare(wifi.Accessible.name, "Wi-Fi");
            compare(wifi.Accessible.description, "Wi-Fi, home");
            compare(wifi.Accessible.focusable, true);
        }

        function test_keyboard_reaches_search_notifications_clock_and_overflow() {
            var bar = make(menuBarComponent, {
                width: 700,
                statusOverflowCapacity: 6,
                statusItems: [
                    { id: "wifi", icon: "wifi", priority: 10 },
                    { id: "search", icon: "search", priority: 50,
                      accessibleName: "Search" },
                    { id: "notifications", icon: "notifications", priority: 60,
                      accessibleName: "Notifications" },
                    { id: "tray:one", icon: "application", priority: 1000,
                      thirdParty: true, accessibleName: "Tray One" }
                ]
            });
            statusSpy.target = bar;
            statusSpy.clear();

            function walkTo(id) {
                for (var step = 0; step < 20; ++step) {
                    var current = bar.statusItemAt(bar.keyboardStatusIndex);
                    if (current && current.itemId === id)
                        return current;
                    bar.moveStatusFocus(1);
                }
                return null;
            }

            // The keyboard walk visits every registry entry, including the
            // clock and the overflow affordance. Search and Notifications
            // raise the shell's one activation signal.
            bar.keyboardStatusIndex = -1;
            var search = walkTo("search");
            verify(search, "search is keyboard reachable");
            bar.activateKeyboardStatus();
            compare(statusSpy.count, 1);
            compare(statusSpy.signalArguments[0][0], "search");

            var notifications = walkTo("notifications");
            verify(notifications, "notifications is keyboard reachable");
            bar.activateKeyboardStatus();
            compare(statusSpy.count, 2);
            compare(statusSpy.signalArguments[1][0], "notifications");

            var clock = walkTo("clock");
            verify(clock, "clock is keyboard reachable");
            compare(clock.Accessible.name.length > 0, true);
            bar.activateKeyboardStatus();
            compare(bar.openStatusItem, "clock");
            bar.closeStatusMenu();

            bar.keyboardStatusIndex = -1;
            var overflow = walkTo("overflow");
            verify(overflow, "overflow is keyboard reachable");
            bar.activateKeyboardStatus();
            compare(bar.overflowMenuOpen, true);
            bar.closeOverflowMenu();
        }

        // -- Date & Time / calendar panel (T231) ----------------------------

        function test_date_time_panel_formats_and_calendar_grid() {
            var panel = make(dateTimePanelComponent, {
                showSeconds: true, use24Hour: true
            });
            panel.open = true;
            waitForRendering(stage);
            var fixed = new Date(2026, 8, 19, 14, 5, 0);
            panel.now = fixed;
            panel.selectedDate = fixed;
            panel.visibleMonth = new Date(2026, 8, 1);
            waitForRendering(stage);
            compare(panel.timeText, "14:05:00");
            compare(panel.dateText, Qt.formatDate(fixed, Locale.LongFormat));
            compare(panel.monthTitleTextValue, Qt.formatDate(new Date(2026, 8, 1),
                                                              "MMMM yyyy"));
            compare(panel.calendarCells.length, 42);
            var selected = -1;
            for (var i = 0; i < panel.calendarCells.length; ++i) {
                if (panel.calendarCells[i].selected)
                    selected = i;
            }
            verify(selected >= 0, "the selected day is in the grid");
            compare(panel.calendarCells[selected].day, 19);
            compare(panel.dayRepeater.itemAt(selected).Accessible.name,
                    panel.calendarCells[selected].accessibleName);
            verify(panel.dayRepeater.itemAt(selected).Accessible.name.indexOf("19") >= 0);
            compare(panel.previousButton.Accessible.name, "Previous month");
            compare(panel.nextButton.Accessible.name, "Next month");
            compare(panel.todayButton.Accessible.name, "Show today");
            panel.open = false;
        }

        function test_date_time_panel_keyboard_traversal() {
            var panel = make(dateTimePanelComponent, {
                showSeconds: false, use24Hour: true
            });
            panel.open = true;
            waitForRendering(stage);
            var fixed = new Date(2026, 8, 19, 14, 5, 0);
            panel.now = fixed;
            panel.selectedDate = fixed;
            panel.visibleMonth = new Date(2026, 8, 1);
            waitForRendering(stage);

            panel.popup.forceActiveFocus();
            keyClick(Qt.Key_Right);
            compare(panel.selectedDate.getDate(), 20);
            keyClick(Qt.Key_Down);
            compare(panel.selectedDate.getDate(), 27);
            keyClick(Qt.Key_Left);
            compare(panel.selectedDate.getDate(), 26);
            keyClick(Qt.Key_Up);
            compare(panel.selectedDate.getDate(), 19);
            keyClick(Qt.Key_PageDown);
            compare(panel.visibleMonth.getMonth(), 9);
            keyClick(Qt.Key_PageUp);
            compare(panel.visibleMonth.getMonth(), 8);
            keyClick(Qt.Key_Home);
            compare(panel.visibleMonth.getMonth(), new Date().getMonth());
            keyClick(Qt.Key_Escape);
            compare(panel.open, false);
        }

        // T231 evidence: the four required live-check states. The runner
        // (`scripts/capture-t231-extras.sh`) sets DF_MENUBAR_CAPTURE_DIR; a
        // normal ctest run skips this case.
        function test_t231_capture_states() {
            if (typeof TestEnvironment === "undefined"
                    || TestEnvironment.captureDir.length === 0)
                skip("DF_MENUBAR_CAPTURE_DIR not set");
            var dir = TestEnvironment.captureDir;
            var status = [
                { id: "wifi", icon: "wifi", priority: 10, accessibleName: "Wi-Fi" },
                { id: "bluetooth", icon: "bluetooth", priority: 20,
                  accessibleName: "Bluetooth" },
                { id: "battery", icon: "battery", priority: 30,
                  accessibleName: "Battery" },
                { id: "volume", icon: "volume", priority: 40,
                  accessibleName: "Volume" },
                { id: "search", icon: "search", priority: 50,
                  accessibleName: "Search" },
                { id: "notifications", icon: "notifications", priority: 60,
                  accessibleName: "Notifications" },
                { id: "clock", kind: "clock", menu: "clock", priority: 70,
                  essential: true, accessibleName: "Date & Time",
                  tooltip: "Date & Time" },
                { id: "control-center", icon: "control-center", priority: 80,
                  accessibleName: "Quick Settings" }
            ];

            // 1. Seconds + 24-hour clock.
            var clockBar = make(menuBarComponent, {
                width: 1000, showDate: true, showSeconds: true,
                use24Hour: true, statusItems: status
            });
            waitForRendering(stage);
            grabImage(clockBar).save(dir + "/t231-extras-clock.png");
            clockBar.destroy();

            // 2. Open calendar.
            var calendarBar = make(menuBarComponent, {
                width: 1000, showDate: true, statusItems: status
            });
            waitForRendering(stage);
            calendarBar.openStatusMenu("clock");
            waitForRendering(stage);
            wait(350);
            waitForRendering(stage);
            grabImage(stage).save(dir + "/t231-extras-calendar.png");
            calendarBar.closeStatusMenu();
            calendarBar.destroy();

            // 3. A removed item (Wi-Fi hidden by the user).
            var removed = status.slice();
            removed[0] = { id: "wifi", icon: "wifi", priority: 10,
                           visible: false, accessibleName: "Wi-Fi" };
            var removedBar = make(menuBarComponent, {
                width: 1000, statusItems: removed
            });
            waitForRendering(stage);
            grabImage(removedBar).save(dir + "/t231-extras-removed.png");
            removedBar.destroy();

            // 4. Third-party items overflow into the affordance.
            var overflowStatus = status.slice();
            overflowStatus.push({ id: "tray:one", icon: "application",
                                  priority: 1000, thirdParty: true,
                                  accessibleName: "Tray One" });
            overflowStatus.push({ id: "tray:two", icon: "application",
                                  priority: 1001, thirdParty: true,
                                  accessibleName: "Tray Two" });
            var overflowBar = make(menuBarComponent, {
                width: 1000, statusItems: overflowStatus,
                statusOverflowCapacity: 7
            });
            waitForRendering(stage);
            overflowBar.openOverflowMenu();
            waitForRendering(stage);
            grabImage(stage).save(dir + "/t231-extras-overflow.png");
            overflowBar.closeOverflowMenu();
            overflowBar.destroy();

            // 5. A third-party item's own DBusMenu opens from its slot.
            var trayStatus = status.slice();
            trayStatus.push({ id: "tray:one", icon: "application",
                              priority: 1000, thirdParty: true,
                              accessibleName: "Tray One" });
            var trayBar = make(menuBarComponent, {
                width: 1000, statusItems: trayStatus,
                trayMenu: [ { label: "Open Tray One", id: 1 },
                            { label: "Quit Tray One", id: 2 } ]
            });
            waitForRendering(stage);
            trayBar.openTrayMenu("one");
            waitForRendering(stage);
            wait(350);
            waitForRendering(stage);
            grabImage(stage).save(dir + "/t231-extras-tray.png");
            trayBar.closeTrayMenu();
            trayBar.destroy();
        }

        // T233 evidence: auto-hide reveal (mid-slide and full), the concealed
        // resting state, and a measured overflow menu on a narrow display.
        // The runner (`scripts/capture-t233-menubar.sh`) sets the directory.
        function test_t233_capture_states() {
            if (typeof TestEnvironment === "undefined"
                    || TestEnvironment.captureDir.length === 0)
                skip("DF_MENUBAR_CAPTURE_DIR not set");
            var dir = TestEnvironment.captureDir;
            var backdrop = createTemporaryObject(backdropComponent, stage, {
                width: stage.width, height: stage.height,
                color: "#0d1016", z: -10
            });
            var status = t233Status();

            // 1. Auto-hide reveal mid-slide: the bar is partly on screen.
            var reveal = make(menuBarComponent, {
                width: 1000, autoHideMode: "always", revealProgress: 0.35,
                appName: "Files",
                applicationMenuItems: [ { label: "About Files" } ],
                statusItems: status
            });
            reveal.y = 0;
            waitForRendering(stage);
            grabImage(stage).save(dir + "/t233-menubar-reveal.png");
            reveal.revealProgress = 1;
            waitForRendering(stage);
            grabImage(stage).save(dir + "/t233-menubar-revealed.png");
            reveal.destroy();

            // 2. Concealed: nothing is reserved and the bar is off-screen.
            var concealed = make(menuBarComponent, {
                width: 1000, autoHideMode: "always", revealProgress: 0,
                statusItems: status
            });
            concealed.y = 0;
            waitForRendering(stage);
            grabImage(stage).save(dir + "/t233-menubar-concealed.png");
            concealed.destroy();

            // 3. Overflow on a narrow display, with the overflow menu open.
            var overflow = make(menuBarComponent, {
                width: 420, revealProgress: 1, appName: "Files",
                applicationMenuItems: [ { label: "About Files" } ],
                statusItems: status
            });
            overflow.y = 0;
            waitForRendering(stage);
            overflow.openOverflowMenu();
            waitForRendering(stage);
            wait(350);
            waitForRendering(stage);
            grabImage(stage).save(dir + "/t233-menubar-overflow.png");
            overflow.closeOverflowMenu();
            overflow.destroy();
            backdrop.destroy();
        }

        function test_t232_capture_states() {
            if (typeof TestEnvironment === "undefined"
                    || TestEnvironment.captureDir.length === 0)
                skip("DF_MENUBAR_CAPTURE_DIR not set");
            var dir = TestEnvironment.captureDir;

            var status = [
                { id: "wifi", icon: "wifi", priority: 10, accessibleName: "Wi-Fi" },
                { id: "bluetooth", icon: "bluetooth", priority: 20,
                  accessibleName: "Bluetooth" },
                { id: "battery", icon: "battery", priority: 30,
                  accessibleName: "Battery" },
                { id: "volume", icon: "volume", priority: 40,
                  accessibleName: "Volume" },
                { id: "search", icon: "search", priority: 50,
                  accessibleName: "Search" },
                { id: "control-center", icon: "control-center", priority: 80,
                  accessibleName: "Quick Settings" }
            ];

            var wifi = wifiModel();
            wifi.activeSsid = "home";
            wifi.activeAddress = "192.168.1.7";
            wifi.radioEnabled = true;
            wifi.knownNetworks = [
                { ssid: "home", uuid: "u1", autoconnect: true, connected: true,
                  security: "WPA2", secured: true, lastUsed: 1700000000 },
                { ssid: "office", uuid: "u2", autoconnect: false, connected: false,
                  security: "WPA2", secured: true, lastUsed: 0 }
            ];
            var bluetooth = bluetoothModel({
                discovering: true,
                nearbyDevices: [
                    { address: "22:33:44:55:66:77", name: "Mouse",
                      paired: false, connected: false, signal: 60 }
                ]
            });
            var battery = batteryModel();
            battery.profilesAvailable = true;
            battery.activeProfile = "balanced";
            battery.profileLabel = "Balanced";
            battery.profiles = [
                { id: "power-saver", label: "Power Saver", glyph: "power-saver",
                  active: false },
                { id: "balanced", label: "Balanced", glyph: "power-balanced",
                  active: true },
                { id: "performance", label: "Performance", glyph: "power-performance",
                  active: false }
            ];
            var audio = audioModel(false, 0.6);
            audio.sinks.push({ id: 9, name: "headphones", description: "Headphones",
                               volume: 0.6, percent: 60, muted: false,
                               default: false });

            var schemes = ["light", "dark"];
            for (var s = 0; s < schemes.length; ++s) {
                var scheme = schemes[s];
                Theme.dark = scheme === "dark";

                var wifiBar = make(menuBarComponent, {
                    width: 1000, statusItems: status, wifiMenu: wifi
                });
                wifiBar.openStatusMenu("wifi");
                waitForRendering(stage);
                wait(250);
                grabImage(stage).save(dir + "/t232-extras-wifi-" + scheme + ".png");
                wifiBar.closeStatusMenu();
                wifiBar.destroy();

                var bluetoothBar = make(menuBarComponent, {
                    width: 1000, statusItems: status, bluetoothMenu: bluetooth
                });
                bluetoothBar.openStatusMenu("bluetooth");
                waitForRendering(stage);
                wait(250);
                grabImage(stage).save(dir + "/t232-extras-bluetooth-" + scheme + ".png");
                bluetoothBar.closeStatusMenu();
                bluetoothBar.destroy();

                var batteryBar = make(menuBarComponent, {
                    width: 1000, statusItems: status, batteryMenu: battery
                });
                batteryBar.openStatusMenu("battery");
                waitForRendering(stage);
                wait(250);
                grabImage(stage).save(dir + "/t232-extras-battery-" + scheme + ".png");
                batteryBar.closeStatusMenu();
                batteryBar.destroy();

                var volumeBar = make(menuBarComponent, {
                    width: 1000, statusItems: status, volumeMenu: audio
                });
                volumeBar.openStatusMenu("volume");
                waitForRendering(stage);
                wait(250);
                grabImage(stage).save(dir + "/t232-extras-volume-" + scheme + ".png");
                volumeBar.closeStatusMenu();
                volumeBar.destroy();

                var searchBar = make(menuBarComponent, {
                    width: 1000,
                    statusItems: [ { id: "search", icon: "search", priority: 50,
                                     accessibleName: "Search" } ]
                });
                waitForRendering(stage);
                grabImage(stage).save(dir + "/t232-extras-search-" + scheme + ".png");
                searchBar.destroy();

                var quickBar = make(menuBarComponent, {
                    width: 1000,
                    statusItems: [ { id: "control-center", icon: "control-center",
                                     priority: 80, accessibleName: "Quick Settings" } ]
                });
                waitForRendering(stage);
                grabImage(stage).save(dir + "/t232-extras-quick-settings-bar-"
                                      + scheme + ".png");
                quickBar.destroy();

                var panel = createTemporaryObject(controlCenterComponent, stage, {
                    wifi: wifi, bluetooth: bluetooth, battery: battery, audio: audio
                });
                panel.width = 360;
                panel.height = 1164;
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t232-extras-quick-settings-"
                                      + scheme + ".png");
                panel.destroy();
            }
            Theme.dark = false;
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
            compare(bar.appMenuAt(0).popup.opacity, 0,
                    "retargeting must not composite the old dropdown's fade over the new one");

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

        // -- T232: extras parity -------------------------------------------

        function test_wifi_menu_known_current_details_and_settings() {
            var model = wifiModel();
            model.activeSsid = "home";
            model.activeAddress = "192.168.1.7";
            model.knownNetworks = [
                { ssid: "home", uuid: "u1", autoconnect: true, connected: true,
                  security: "WPA2", secured: true, lastUsed: 1700000000 },
                { ssid: "office", uuid: "u2", autoconnect: false, connected: false,
                  security: "WPA2", secured: true, lastUsed: 0 }
            ];
            model.networks.push({ ssid: "office", strength: 55, security: "WPA2",
                                  secured: true, active: false, band: "5 GHz" });
            var menu = make(wifiMenuComponent, { model: model });
            compare(menu.activeSsid, "home");
            compare(menu.knownRows.length, 2);
            compare(menu.otherRows.length, 1, "cafe is the only unsaved network");
            compare(menu.rows[0].ssid, "home");
            compare(menu.rows[0].section, "known");
            compare(menu.rows[0].active, true);
            compare(menu.rows[1].ssid, "office");
            compare(menu.rows[2].ssid, "cafe");
            compare(menu.securityLabel("home"), "WPA2");
            compare(menu.signalLabel("office"), "55% signal");
            menu.toggleDetails("home");
            compare(menu.detailsSsid, "home");
            menu.toggleDetails("home");
            compare(menu.detailsSsid, "");
            wifiSettingsSpy.target = menu;
            wifiSettingsSpy.clear();
            menu.settingsRequested();
            compare(wifiSettingsSpy.count, 1);
        }

        function test_wifi_menu_scanning_and_error_states() {
            var scanning = wifiModel();
            scanning.scanning = true;
            var menu = make(wifiMenuComponent, { model: scanning });
            compare(menu.scanning, true);
            compare(menu.available, true);
            compare(menu.radioEnabled, true);
            compare(menu.scanningNoteVisible, true);
            compare(menu.scanningNote.text, "Scanning…");
            menu.open = true;
            waitForRendering(stage);
            menu.open = false;
            waitForRendering(stage);

            var connecting = wifiModel();
            connecting.wifiState = "connecting";
            var connectingMenu = make(wifiMenuComponent, { model: connecting });
            compare(connectingMenu.scanning, true);

            var error = wifiModel();
            error.state = "error";
            error.error = "NetworkManager read failed";
            var errorMenu = make(wifiMenuComponent, { model: error });
            compare(errorMenu.available, false);
            compare(errorMenu.errorMessage, "NetworkManager read failed");
            compare(errorMenu.stateNote.text, "NetworkManager read failed");
        }

        function test_bluetooth_menu_discovery_nearby_and_pairing() {
            var model = bluetoothModel({
                discovering: true,
                nearbyDevices: [
                    { address: "22:33:44:55:66:77", name: "Mouse",
                      paired: false, connected: false, signal: 60 }
                ]
            });
            var menu = make(bluetoothMenuComponent, { model: model });
            compare(menu.discovering, true);
            compare(menu.knownRows.length, 2);
            compare(menu.nearbyRows.length, 1);
            compare(menu.rows.length, 3);
            compare(findChild(menu.popup, "bluetoothScanningNote").visible, false,
                    "nearby rows replace the scanning note");

            bluetoothDiscoverySpy.target = menu;
            bluetoothDiscoverySpy.clear();
            menu.toggleDiscovery();
            compare(bluetoothDiscoverySpy.count, 1);
            compare(bluetoothDiscoverySpy.signalArguments[0][0], false);

            bluetoothPairSpy.target = menu;
            bluetoothPairSpy.clear();
            compare(menu.writable, true);
            compare(menu.rows[2].paired, false);
            compare(menu.rows[2].address, "22:33:44:55:66:77");
            menu.activateRow(2);
            compare(bluetoothPairSpy.count, 1);
            compare(bluetoothPairSpy.signalArguments[0][0], "22:33:44:55:66:77");

            bluetoothMenuSettingsSpy.target = menu;
            bluetoothMenuSettingsSpy.clear();
            menu.settingsRequested();
            compare(bluetoothMenuSettingsSpy.count, 1);

            var absent = bluetoothModel({ state: "unavailable" });
            var absentMenu = make(bluetoothMenuComponent, { model: absent });
            compare(absentMenu.writable, false);
        }

        function test_battery_menu_low_full_power_mode_and_settings() {
            var lowModel = batteryModel();
            lowModel.percent = 10;
            lowModel.level = 0.10;
            lowModel.label = "10%";
            lowModel.charging = false;
            lowModel.plugged = false;
            lowModel.onBattery = true;
            lowModel.chargeState = "discharging";
            lowModel.timeToFull = null;
            lowModel.timeToEmpty = 600;
            var low = make(batteryMenuComponent, { model: lowModel });
            compare(low.low, true);
            compare(low.stateLabel, "Low Battery");
            compare(low.remaining, "10 min");

            var fullModel = batteryModel();
            fullModel.percent = 100;
            fullModel.level = 1.0;
            fullModel.label = "100%";
            fullModel.charging = false;
            fullModel.plugged = true;
            fullModel.onBattery = false;
            fullModel.chargeState = "fully-charged";
            var full = make(batteryMenuComponent, { model: fullModel });
            compare(full.full, true);
            compare(full.stateLabel, "Fully Charged");

            var powerModel = batteryModel();
            powerModel.profilesAvailable = true;
            powerModel.activeProfile = "balanced";
            powerModel.profileLabel = "Balanced";
            powerModel.profiles = [
                { id: "power-saver", label: "Power Saver", glyph: "power-saver",
                  active: false },
                { id: "balanced", label: "Balanced", glyph: "power-balanced",
                  active: true },
                { id: "performance", label: "Performance", glyph: "power-performance",
                  active: false }
            ];
            var power = make(batteryMenuComponent, { model: powerModel });
            compare(power.profiles.length, 3);
            powerProfileSpy.target = power;
            powerProfileSpy.clear();
            power.selectProfile("power-saver");
            compare(powerProfileSpy.count, 1);
            compare(powerProfileSpy.signalArguments[0][0], "power-saver");
            power.selectProfile("balanced");
            compare(powerProfileSpy.count, 1, "the active profile is not rewritten");
            power.moveProfileHighlight(1);
            compare(power.profileHighlightIndex, 0);
            power.activateProfileHighlight();
            compare(powerProfileSpy.count, 2);

            batterySettingsSpy.target = power;
            batterySettingsSpy.clear();
            power.settingsRequested();
            compare(batterySettingsSpy.count, 1);
        }

        function test_volume_menu_output_routing_and_settings() {
            var model = audioModel(false, 0.6);
            model.sinks.push({ id: 9, name: "headphones", description: "Headphones",
                               volume: 0.6, percent: 60, muted: false,
                               default: false });
            var menu = make(volumeMenuComponent, { model: model });
            compare(menu.sinks.length, 2);
            compare(menu.defaultSink.description, "Built-in Speakers");
            sinkSpy.target = menu;
            sinkSpy.clear();
            menu.selectSink(9);
            compare(sinkSpy.count, 1);
            compare(sinkSpy.signalArguments[0][0], 9);
            menu.selectSink(7);
            compare(sinkSpy.count, 1, "the default sink is not rewritten");
            menu.sinkHighlightIndex = 1;
            menu.activateSinkHighlight();
            compare(sinkSpy.count, 2);

            volumeSettingsSpy.target = menu;
            volumeSettingsSpy.clear();
            menu.settingsRequested();
            compare(volumeSettingsSpy.count, 1);

            var absent = make(volumeMenuComponent, {
                model: { kind: "audio", state: "unavailable" }
            });
            compare(absent.available, false);
        }

        function test_search_item_invokes_and_hides_from_the_bar() {
            var bar = make(menuBarComponent, {
                width: 800,
                statusItems: [
                    { id: "search", icon: "search", priority: 50,
                      accessibleName: "Search" }
                ]
            });
            statusSpy.target = bar;
            statusSpy.clear();
            bar.activateStatusItem("search");
            compare(statusSpy.count, 1);
            compare(statusSpy.signalArguments[0][0], "search");

            bar.statusItems = [
                { id: "search", icon: "search", priority: 50, visible: false,
                  accessibleName: "Search" }
            ];
            waitForRendering(stage);
            compare(bar.statusItemFor("search").visible, false);
            compare(bar.statusItemFor("search").width, 0);
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

        function test_the_clock_opens_the_date_and_time_panel() {
            // T231: the clock is a registry item and its click opens the
            // calendar panel; Notifications is the Notification Center entry.
            var bar = make(menuBarComponent, {
                statusItems: [],
                showDate: true
            });
            var clock = bar.statusItemFor("clock");
            verify(clock);
            compare(clock.kind, "clock");
            mouseClick(clock, clock.width / 2, clock.height / 2);
            compare(bar.openStatusItem, "clock");
            var panel = findChild(bar, "dateTimePanel");
            verify(panel);
            verify(panel.popup.open);
            compare(panel.popup.Accessible.name, "Date & Time");
            bar.closeStatusMenu();
            waitForRendering(stage);
            compare(bar.openStatusItem, "");
        }

        // -- Status glyph rendering (FR-5) ---------------------------------

        // Every state must render a non-empty Tabler mark (ADR 0185). This is
        // the state-coverage case: a typo in StatusGlyph's state -> glyph
        // mapping or an unvendored name would leave a state blank.
        function test_status_glyphs_render_pixels() {
            var names = ["wifi", "wifi-secure", "wifi-off", "wifi-disabled",
                         "wifi-connecting", "wifi-error", "bluetooth",
                         "volume", "volume-muted", "battery", "battery-charging",
                         "focus", "accessibility", "search", "notifications",
                         "overflow", "application", "control-center",
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

        // -- Keyboard navigation and dynamic rows (T230) --------------------

        function test_keyboard_navigation_skips_disabled_and_honours_checked() {
            var fixed = fixedMenus();
            fixed.applicationMenuItems = [
                { label: "About Test", action: "about" },
                { type: "separator" },
                { label: "Hide Test", action: "hide", enabled: false,
                  disabledReason: "The app has no visible window." },
                { label: "Hide Others", action: "hide-others", enabled: true },
                { label: "Show All", action: "show-all", enabled: true, checked: true,
                  checkable: true }
            ];
            var bar = make(menuBarComponent, {
                appName: "Test",
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            triggeredSpy.target = bar;
            triggeredSpy.clear();
            var app = bar.appMenuAt(1);
            bar.openMenu(1);
            waitForRendering(stage);
            // The first enabled row is highlighted; Down skips the disabled
            // Hide row and lands on Hide Others.
            compare(app.highlightedIndex, 0);
            keyClick(Qt.Key_Down);
            compare(app.highlightedIndex, 3);
            compare(app.entries[2].enabled, false);
            compare(app.entries[2].disabledReason,
                    "The app has no visible window.");
            compare(app.entries[4].checked, true);
            compare(app.entries[4].checkable, true);
            keyClick(Qt.Key_Return);
            compare(triggeredSpy.count, 1);
            compare(triggeredSpy.signalArguments[0][2].action, "hide-others");
        }

        function test_type_ahead_selects_a_row_by_prefix() {
            var fixed = fixedMenus();
            fixed.applicationMenuItems = [
                { label: "About Test", action: "about" },
                { label: "Settings", action: "settings" },
                { label: "Show All", action: "show-all" }
            ];
            var bar = make(menuBarComponent, {
                appName: "Test",
                systemMenuItems: fixed.systemMenuItems,
                applicationMenuItems: fixed.applicationMenuItems
            });
            var app = bar.appMenuAt(1);
            bar.openMenu(1);
            waitForRendering(stage);
            app.typeAhead("sh");
            compare(app.highlightedIndex, 2);
            app.typeAhead("se");
            compare(app.highlightedIndex, 1);
        }

        function test_left_right_switch_top_level_menus() {
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
            keyClick(Qt.Key_Right);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 1);
            keyClick(Qt.Key_Right);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 2);
            keyClick(Qt.Key_Left);
            waitForRendering(stage);
            compare(bar.openMenuIndex, 1);
            keyClick(Qt.Key_Escape);
            compare(bar.openMenuIndex, -1);
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

        // -- T230: menu-bar business logic --------------------------------

        // A disabled row carries its reason into the normalized entry and the
        // re-emitted model; a checked row keeps its check state.
        function test_disabled_reasons_and_checked_rows() {
            var bar = make(menuBarComponent, {
                appName: "Test",
                applicationMenuItems: [
                    { label: "Hide Test", action: "hide", enabled: false,
                      disabledReason: "The app has no visible window." },
                    { label: "Show All", action: "show-all", enabled: true }
                ],
                appMenuModel: [ { title: "View", items: [
                    { label: "Sidebar", action: "view.sidebar",
                      checked: true, checkable: true },
                    { label: "Toolbar", action: "view.toolbar",
                      checked: false, checkable: true }
                ] } ]
            });
            var app = bar.appMenuAt(1);
            compare(app.entries[0].disabledReason,
                    "The app has no visible window.");
            compare(app.entries[0].enabled, false);
            compare(app.menuModel[0].disabledReason,
                    "The app has no visible window.");

            var view = bar.appMenuAt(2);
            compare(view.entries[0].checked, true);
            compare(view.entries[0].checkable, true);
            compare(view.entries[1].checked, false);
            compare(view.menuModel[0].checked, true);
            compare(view.menuModel[0].checkable, true);

            // A disabled row is inert; the checked row still activates.
            triggeredSpy.target = bar;
            triggeredSpy.clear();
            app.activate(0);
            compare(triggeredSpy.count, 0);
            view.activate(0);
            compare(triggeredSpy.count, 1);
            compare(triggeredSpy.signalArguments[0][2].action, "view.sidebar");
        }

        // Type-ahead accumulates printable keys and moves the highlight to the
        // next enabled row whose label starts with the buffer.
        function test_type_ahead_moves_the_highlight() {
            var bar = make(menuBarComponent, {
                appName: "Test",
                applicationMenuItems: [ { label: "Quit", action: "quit" } ],
                appMenuModel: [ { title: "File", items: [
                    { label: "New", action: "file.new" },
                    { label: "Open", action: "file.open" },
                    { label: "Save", action: "file.save" },
                    { label: "Save As", action: "file.save-as" }
                ] } ]
            });
            var file = bar.appMenuAt(2);
            file.openMenu();
            verify(file.open);

            file.typeAhead("s");
            compare(file.highlightedIndex, 2, "S selects Save");
            file.typeAhead("a");
            compare(file.highlightedIndex, 3, "SA narrows to Save As");
            file.clearTypeAhead();
            compare(file.typeAheadBuffer, "");

            // A disabled row is skipped by type-ahead.
            file.closeMenu();
            bar.appMenuModel = [ { title: "Edit", items: [
                { label: "Copy", action: "edit.copy", enabled: false },
                { label: "Cut", action: "edit.cut" }
            ] } ];
            var edit = bar.appMenuAt(2);
            edit.openMenu();
            edit.typeAhead("c");
            compare(edit.highlightedIndex, 1, "disabled Copy is skipped");
            edit.closeMenu();
        }

        // Left/Right at the top level moves between sibling menus, wrapping
        // and skipping an empty menu.
        function test_left_right_switches_top_level_menus() {
            var bar = make(menuBarComponent, {
                appName: "Test",
                systemMenuItems: [ { label: "About This System", action: "about-system" } ],
                applicationMenuItems: [ { label: "Quit", action: "quit" } ],
                appMenuModel: [
                    { title: "File", items: [ { label: "New", action: "file.new" } ] },
                    { title: "Edit", items: [ { label: "Cut", action: "edit.cut" } ] }
                ]
            });
            bar.openMenu(1);
            var app = bar.appMenuAt(1);
            app.menuSwitchRequested(1);
            compare(bar.openMenuIndex, 2, "right moves to File");
            bar.appMenuAt(2).menuSwitchRequested(1);
            compare(bar.openMenuIndex, 3, "right moves to Edit");
            bar.appMenuAt(3).menuSwitchRequested(1);
            compare(bar.openMenuIndex, 0, "right wraps to the system menu");
            bar.closeMenus();
        }

        // A submenu near the right edge flips to the left; one with room
        // opens to the right.
        function test_submenu_direction_follows_the_window_edge() {
            var bar = make(menuBarComponent, {
                appName: "Test",
                applicationMenuItems: [ { label: "Quit", action: "quit" } ],
                appMenuModel: [ { title: "File", items: [
                    { label: "Open With", type: "submenu", submenu: [
                        { label: "Text Editor", action: "open.text" }
                    ] }
                ] } ]
            });
            var file = bar.appMenuAt(2);
            file.openMenu();
            file.openSubmenu(0);
            verify(file.submenuEntries.length > 0);
            file.popup.x = 0;
            verify(!file.submenuFlips, "room on the right");
            file.popup.x = stage.width - 20;
            verify(file.submenuFlips, "near the right edge flips left");
            file.closeMenu();
        }
    }
}
