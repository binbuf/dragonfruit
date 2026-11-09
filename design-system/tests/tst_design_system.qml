// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Gallery

// Per-component and per-token tests for the design system (T-08, FR-1…FR-6).
// Runs headless on the offscreen platform + software scene graph; the visual
// assertions sample real grabbed pixels, so they are deterministic without
// depending on font rasterization.
//
// `stage` is the visual root so it is shown in the QuickTest window and
// `grabImage` returns real pixels.
Item {
    id: stage
    width: 600
    height: 400

TestCase {
    id: testCase
    name: "DesignSystem"
    when: windowShown

    Component { id: toggleComponent; Toggle { } }
    Component { id: lightsComponent; TrafficLights { } }
    Component { id: titleBarComponent; TitleBar { } }
    Component {
        id: titleBarInSlotComponent
        Item {
            width: 640
            height: WindowStyle.height("titlebar")
            TitleBar { }
        }
    }
    Component { id: ssdComponent; SsdTitlebarReference { } }
    Component { id: shadowComponent; Shadow { width: 120; height: 80 } }
    Component { id: popupComponent; Popup { } }
    Component { id: menuComponent; MenuBarMenu { } }
    Component { id: rectComponent; Rectangle { } }
    Component { id: buttonComponent; Button { } }
    Component { id: toolbarComponent; Toolbar { } }
    Component { id: splitViewComponent; SplitView { } }
    Component { id: settingsRowComponent; SettingsRow { } }
    Component { id: settingsGroupComponent; SettingsGroup { } }
    Component { id: segmentedComponent; SegmentedControl { } }
    Component { id: sliderComponent; Slider { width: 200 } }
    Component { id: selectComponent; Select { width: 200 } }
    Component { id: contextMenuComponent; ContextMenu { } }
    Component { id: menuRowComponent; MenuRow { width: 300 } }
    Component { id: shortcutHintComponent; ShortcutHint { } }
    Component { id: searchFieldComponent; SearchField { } }
    Component { id: sourceListComponent; SourceList { } }
    Component { id: sidebarComponent; Sidebar { } }
    Component { id: chromeSheetComponent; ChromeSheet { width: 120; height: 80 } }
    Component { id: dialogComponent; Dialog { } }
    Component { id: sheetComponent; Sheet { } }
    Component { id: popoverComponent; Popover { } }
    Component { id: tooltipComponent; Tooltip { } }
    Component { id: skeletonComponent; Skeleton { width: 160; height: 24 } }
    Component { id: spinnerComponent; Spinner { } }
    Component { id: phosphorIconComponent; PhosphorIcon { name: "gear"; weight: "fill" } }
    Component { id: tablerIconComponent; TablerIcon { name: "folder" } }
    Component { id: legacyIconComponent; Icon { name: "folder" } }
    Component { id: settingsCategoryComponent; SettingsCategoryIcon { } }
    Component {
        id: settingsCategorySlotComponent
        Item {
            width: 80
            height: 80
            // A 20px sidebar slot with the tile's 32px default `size`: the
            // container must stay inside the slot rather than paint at its
            // implicit size over the label and the neighbouring rows.
            SettingsCategoryIcon {
                x: 30
                y: 30
                width: 20
                height: 20
                size: 32
                radius: 0
                gradientStart: "#2266ff"
                gradientEnd: "#2266ff"
            }
        }
    }
    Component {
        id: scrollViewComponent
        ScrollView {
            width: 200
            height: 100
            Item { width: 200; height: 400 }
        }
    }

    SignalSpy { id: closeSpy; signalName: "closeClicked" }
    SignalSpy { id: zoomSpy; signalName: "zoomRequested" }
    SignalSpy { id: menuSpy; signalName: "menuRequested" }
    SignalSpy { id: triggeredSpy; signalName: "triggered" }
    SignalSpy { id: popupClosedSpy; signalName: "closed" }
    SignalSpy { id: buttonSpy; signalName: "clicked" }
    SignalSpy { id: segmentedSpy; signalName: "activated" }
    SignalSpy { id: sliderMovedSpy; signalName: "moved" }
    SignalSpy { id: sliderCommittedSpy; signalName: "committed" }
    SignalSpy { id: selectActivatedSpy; signalName: "activated" }
    SignalSpy { id: sidebarSpy; signalName: "activated" }
    SignalSpy { id: sourceListSpy; signalName: "activated" }
    SignalSpy { id: acceptedSpy; signalName: "accepted" }
    SignalSpy { id: rejectedSpy; signalName: "rejected" }

    function make(component, props) {
        var obj = createTemporaryObject(component, stage, props || {});
        waitForRendering(stage);
        return obj;
    }

    function channel(value) {
        return Math.round(value * 255);
    }

    // -- Tokens (FR-2) ------------------------------------------------------

    function test_tokens_have_three_tiers() {
        compare(Theme.primitive.radius.md, 10);
        compare(Theme.primitive.spacing.sm, 8);
        compare(WindowStyle.height("titlebar"), 32);
        compare(Theme.controls.trafficLights.buttonSize, 16);
        compare(Theme.controls.trafficLights.diameter, 14);
        compare(Theme.motion.menuOpen.fullDuration, 0);
        verify(Theme.color.accent !== undefined);
    }

    // -- T230b: window chrome style system ---------------------------------

    function test_window_style_table_covers_the_measured_heights() {
        var expected = {
            borderless: 0, titlebar: 32, unifiedCompact: 40,
            expanded: 48, unified: 52, preference: 88
        };
        compare(WindowStyle.styleIds.length, 6);
        for (var i = 0; i < WindowStyle.styleIds.length; ++i) {
            var id = WindowStyle.styleIds[i];
            compare(WindowStyle.height(id), expected[id], id);
            compare(WindowStyle.titleRowHeight(id)
                    + WindowStyle.toolbarRowHeight(id),
                    WindowStyle.height(id), id + " row sum");
        }
        compare(WindowStyle.chromeHeight("unified", { labelledToolbar: true }), 66);
        compare(WindowStyle.chromeHeight("expanded", { toolbarRowHeight: 22 }), 70);
        compare(WindowStyle.isBorderless("borderless"), true);
        compare(WindowStyle.isBorderless("titlebar"), false);
    }

    function test_traffic_light_geometry_is_style_derived() {
        // 16 pt is the AppKit AX/hit frame; 14 pt is the drawn circle.
        compare(WindowStyle.trafficLightSize, 16);
        compare(WindowStyle.trafficLightDiameter, 14);
        compare(WindowStyle.trafficLightSpacing, 23);
        compare(WindowStyle.trafficLightGap(), 7);
        compare(WindowStyle.trafficLightInset(32), 8);
        compare(WindowStyle.trafficLightInset(52), 18);
        compare(WindowStyle.trafficLightInset(66), 25);
        compare(WindowStyle.trafficLightInset(28), 6);
        // Preference/expanded keep the lights in the title row, not the
        // middle of the full chrome.
        compare(WindowStyle.trafficLightInsetFor("preference"), 8);
        compare(WindowStyle.trafficLightInsetFor("expanded"), 6);
        compare(WindowStyle.trafficLightInsetFor("unified"), 18);
    }

    function test_titlebar_height_comes_from_the_named_style() {
        var utility = make(titleBarComponent, { width: 320, style: "titlebar" });
        compare(utility.implicitHeight, 32);
        var unified = make(titleBarComponent, { width: 320, style: "unified" });
        compare(unified.implicitHeight, 52);
        var preference = make(titleBarComponent, { width: 320, style: "preference" });
        compare(preference.implicitHeight, 88);
        var borderless = make(titleBarComponent, { width: 320, style: "borderless" });
        compare(borderless.implicitHeight, 0);
        compare(borderless.visible, false);
    }

    function test_titlebar_drag_region_is_chrome_only() {
        var bar = make(titleBarComponent, { width: 320, style: "titlebar" });
        compare(bar.dragArea.height, bar.chromeHeight);
        compare(bar.dragArea.height, 32);
        // A toolbar row is part of the chrome; an accessory bar is not.
        var unified = make(titleBarComponent, { width: 320, style: "unified" });
        compare(unified.dragArea.height, 52);
        compare(Theme.controls.accessoryBar.tab, 28);
        compare(Theme.controls.accessoryBar.path, 30);
        compare(Theme.controls.accessoryBar.status, 28);
        compare(Theme.controls.accessoryBar.format, 20);
        compare(WindowStyle.height("unified"), 52,
                "an accessory bar must not increase the chrome height");
    }

    function test_light_and_dark_semantics() {
        var saved = Theme.dark;
        Theme.dark = false;
        compare(String(Theme.color.surface), "#ffffff");
        Theme.dark = true;
        compare(String(Theme.color.surface), "#1d1723");
        Theme.dark = saved;
    }

    function test_reduced_motion_collapses_durations() {
        var saved = Theme.reducedMotion;
        Theme.reducedMotion = false;
        compare(Theme.motion.popupOpen.duration, 160);
        Theme.reducedMotion = true;
        compare(Theme.motion.popupOpen.duration, 0);
        Theme.reducedMotion = saved;
    }

    // -- T195: glass material presets and resolution -------------------------

    function init() {
        Glass.rollout = Theme.glass.rollout.defaultMode;
        Glass.scheme = Theme.dark ? "dark" : "light";
        Glass.slider = Theme.glass.slider.defaultPosition;
        Glass.reduceTransparency = false;
        Glass.increaseContrast = false;
        Glass.sampledBlur = true;
        Glass.software = false;
        Glass.appChromeSupported = true;
        Glass.tier = "full";
    }

    function test_glass_presets_cover_the_required_surfaces() {
        compare(Glass.presetNames.length, 17);
        for (var i = 0; i < Glass.presetNames.length; ++i) {
            var name = Glass.presetNames[i];
            verify(Theme.glass.presets[name] !== undefined, name + " preset exists");
            var resolved = Glass.resolve(name, { rollout: "replacement" });
            verify(resolved.renderMode !== undefined, name + " resolves");
            compare(resolved.preset, name);
        }
    }

    function test_glass_light_slider_anchors() {
        var anchors = Theme.glass.slider.lightAnchors;
        var expected = [
            [0.0, anchors.low.alpha, anchors.low.tint],
            [anchors.mid.position, anchors.mid.alpha, anchors.mid.tint],
            [1.0, anchors.high.alpha, anchors.high.tint]
        ];
        for (var i = 0; i < expected.length; ++i) {
            var resolved = Glass.resolve("menuAttached", {
                scheme: "light", rollout: "replacement", slider: expected[i][0]
            });
            verify(Math.abs(resolved.alpha - expected[i][1]) < 1e-6,
                   "light slider alpha at " + expected[i][0]);
            verify(Math.abs(resolved.tint - expected[i][2]) < 1e-6,
                   "light slider tint at " + expected[i][0]);
            compare(resolved.sliderApplied, true);
        }
        // The measured midpoint must stay exactly the 0.552/0.993 sample.
        var mid = Glass.resolve("menuAttached", {
            scheme: "light", rollout: "replacement", slider: 0.5255
        });
        verify(Math.abs(mid.alpha - 0.552) < 1e-6);
        verify(Math.abs(mid.tint - 0.993) < 1e-6);
    }

    function test_glass_slider_only_moves_the_light_attached_menu() {
        for (var i = 0; i < Glass.presetNames.length; ++i) {
            var name = Glass.presetNames[i];
            var low = Glass.resolve(name, { scheme: "light", rollout: "replacement", slider: 0 });
            var high = Glass.resolve(name, { scheme: "light", rollout: "replacement", slider: 1 });
            var relative = name === "menuDetached" || name === "statusMenu";
            var expected = name === "menuAttached" || relative;
            compare(low.sliderApplied, expected, name + " slider support");
            if (relative) {
                var defaultFit = Glass.resolve(name, {
                    scheme: "light", rollout: "replacement",
                    slider: Theme.glass.slider.defaultPosition
                });
                verify(low.alpha > defaultFit.alpha && defaultFit.alpha > high.alpha,
                       name + " relative alpha follows the attached curve");
                verify(low.tint < defaultFit.tint && defaultFit.tint < high.tint,
                       name + " relative tint follows the attached curve");
            } else if (!expected) {
                verify(Math.abs(low.alpha - high.alpha) < 1e-6,
                       name + " slider must not change alpha");
                verify(Math.abs(low.tint - high.tint) < 1e-6,
                       name + " slider must not change tint");
            }
        }
        var darkLow = Glass.resolve("menuAttached", {
            scheme: "dark", rollout: "replacement", slider: 0
        });
        var darkHigh = Glass.resolve("menuAttached", {
            scheme: "dark", rollout: "replacement", slider: 1
        });
        compare(darkLow.sliderApplied, false);
        verify(Math.abs(darkLow.alpha - darkHigh.alpha) < 1e-6);
    }

    function test_glass_dark_attached_and_detached_are_distinct() {
        var attached = Glass.resolve("menuAttached", {
            scheme: "dark", rollout: "replacement"
        });
        var detached = Glass.resolve("menuDetached", {
            scheme: "dark", rollout: "replacement"
        });
        verify(Math.abs(attached.alpha - detached.alpha) > 1e-6
               || Math.abs(attached.tint - detached.tint) > 1e-6,
               "dark attached/detached presets must differ");
    }

    function test_glass_reduce_transparency_precedence() {
        var reduced = Glass.resolve("menuAttached", {
            scheme: "light", rollout: "replacement", slider: 0,
            reduceTransparency: true
        });
        compare(reduced.renderMode, "solid");
        compare(reduced.sliderApplied, false);
        verify(reduced.alpha >= 0.9, "Reduce Transparency uses an opaque plate");
        compare(reduced.blur, 0);

        var contrasted = Glass.resolve("menuAttached", {
            scheme: "light", rollout: "replacement", slider: 0,
            reduceTransparency: true, increaseContrast: true
        });
        verify(contrasted.edge > reduced.edge, "Increase Contrast strengthens the edge");
        verify(contrasted.foreground > reduced.foreground,
               "Increase Contrast strengthens the foreground");
    }

    function test_glass_veil_is_explicit_not_a_fallback() {
        var veil = Glass.resolve("menuBarVeil", {
            scheme: "light", rollout: "replacement", backend: "gpu-sampled"
        });
        compare(veil.renderMode, "no-blur-veil");
        compare(veil.fallbackReason, "none");
        compare(veil.blur, 0);
        // The measured background-luminance response travels with the preset:
        // the veil must fall to zero over white, not stay at the black alpha.
        compare(veil.veil.adaptLuma.length, 5);
        fuzzyCompare(veil.veil.adaptLuma[3], 0.78, 0.0001);
        fuzzyCompare(veil.veil.adaptScale[3], 0.48, 0.0001);
        fuzzyCompare(veil.veil.adaptScale[4], 0.0, 0.0001);
        var unsupported = Glass.resolve("menuBarVeil", {
            scheme: "light", rollout: "replacement", backend: "gpu-no-sampled-blur"
        });
        compare(unsupported.renderMode, "no-blur-veil");
        compare(unsupported.fallbackReason, "none");
    }

    function test_glass_unsupported_paths_are_safe() {
        var software = Glass.resolve("menuDetached", {
            scheme: "dark", rollout: "replacement", backend: "software"
        });
        compare(software.renderMode, "software-fallback");
        verify(software.alpha >= 0.9);
        compare(software.blur, 0);

        var unsupported = Glass.resolve("menuDetached", {
            scheme: "dark", rollout: "replacement", backend: "gpu-no-sampled-blur"
        });
        compare(unsupported.renderMode, "solid");
        verify(unsupported.alpha >= 0.9);
        compare(unsupported.blur, 0);

        var unknown = Glass.resolve("not-a-preset", {
            scheme: "light", rollout: "replacement"
        });
        compare(unknown.renderMode, "solid");
        compare(unknown.fallbackReason, "unknown-preset");
    }

    function test_glass_rollout_defaults_to_the_replacement_appearance() {
        compare(Theme.glass.rollout.defaultMode, "replacement");
        Glass.rollout = "replacement";
        var replacement = Glass.resolve("dock", { scheme: "light" });
        compare(replacement.look, "replacement");
        verify(Math.abs(replacement.alpha
                       - Theme.glass.presets.dock.light.calibration.neutralAlpha) < 1e-6);
        Glass.rollout = "legacy";
        var legacy = Glass.resolve("dock", { scheme: "light" });
        compare(legacy.look, "legacy");
        verify(Math.abs(legacy.alpha - Theme.material.dockOpacity) < 1e-6);
    }

    function test_glass_reduce_motion_is_orthogonal() {
        var moving = Glass.resolve("dock", {
            scheme: "light", rollout: "replacement", reduceMotion: false
        });
        var still = Glass.resolve("dock", {
            scheme: "light", rollout: "replacement", reduceMotion: true
        });
        verify(Math.abs(moving.alpha - still.alpha) < 1e-6);
        compare(moving.renderMode, still.renderMode);
    }

    function test_glass_first_party_app_chrome_presets() {
        var savedDark = Theme.dark;
        var savedRollout = Glass.rollout;
        var savedReduce = Glass.reduceTransparency;
        var savedContrast = Glass.increaseContrast;
        var savedSupported = Glass.appChromeSupported;
        try {
            Theme.dark = false;
            Glass.rollout = "replacement";
            Glass.reduceTransparency = false;
            Glass.increaseContrast = false;
            Glass.appChromeSupported = true;
            var targets = {
                settingsSidebar: [0.986, 0.857],
                settingsToolbar: [0.961, 0.965],
                filesSidebar: [0.985, 0.858],
                filesToolbar: [0.994, 0.922]
            };
            for (var name in targets) {
                var resolved = Glass.resolve(name, {
                    scheme: "light", rollout: "replacement"
                });
                fuzzyCompare(resolved.alpha, targets[name][0], 1e-6, name + " alpha");
                fuzzyCompare(resolved.tint, targets[name][1], 1e-6, name + " tint");
                compare(resolved.renderMode, "solid");
                compare(resolved.blur, 0);
            }
            var fill = Material.appChromeFill("settingsSidebar", "#123456");
            fuzzyCompare(fill.a, 0.986, 1e-4);
            fuzzyCompare(fill.r, 0.857, 1e-4);
            fuzzyCompare(fill.g, 0.857, 1e-4);
            fuzzyCompare(fill.b, 0.857, 1e-4);

            Theme.dark = true;
            fill = Material.appChromeFill("filesToolbar", "#123456");
            fuzzyCompare(fill.a, 0.900, 1e-4);
            fuzzyCompare(fill.r, 0.247, 1e-4);

            // Non-composited/unsupported client surfaces keep the opaque
            // design-system plate, never a translucent slab over unfrosted
            // content.
            Glass.appChromeSupported = false;
            var fallback = Qt.rgba(0.2, 0.3, 0.4, 1.0);
            fill = Material.appChromeFill("filesToolbar", fallback);
            fuzzyCompare(fill.a, 1.0, 1e-6);
            fuzzyCompare(fill.r, 0.2, 1e-6);

            // The legacy rollout is the same opaque plate.
            Glass.appChromeSupported = true;
            Glass.rollout = "legacy";
            fill = Material.appChromeFill("settingsSidebar", fallback);
            fuzzyCompare(fill.a, 1.0, 1e-6);

            // Reduce Transparency resolves to the shared opaque plate.
            Theme.dark = false;
            Glass.rollout = "replacement";
            Glass.reduceTransparency = true;
            fill = Material.appChromeFill("settingsSidebar", "#123456");
            fuzzyCompare(fill.a, 0.98, 1e-4);
            fuzzyCompare(fill.r, 0.97, 1e-4);

            // Increase Contrast raises the separator ink after the fill.
            Glass.reduceTransparency = false;
            Glass.increaseContrast = true;
            var border = Material.appChromeBorder("settingsSidebar",
                                                  Qt.rgba(0, 0, 0, 0));
            verify(border.a > 0, "Increase Contrast must raise the app chrome edge");
        } finally {
            Theme.dark = savedDark;
            Glass.rollout = savedRollout;
            Glass.reduceTransparency = savedReduce;
            Glass.increaseContrast = savedContrast;
            Glass.appChromeSupported = savedSupported;
        }
    }

    function test_chrome_sheet_keeps_one_fill_and_per_corner_radii() {
        var sheet = make(chromeSheetComponent, {
            color: "#ff0000", bottomRightRadius: 12
        });
        compare(sheet.bottomRightRadius, 12);
        compare(sheet.topLeftRadius, 0);
        compare(sheet.topRightRadius, 0);
        compare(sheet.bottomLeftRadius, 0);
        compare(String(sheet.color), "#ff0000");
    }

    function test_glass_persistent_floor_and_transient_fallback() {
        var dock = Glass.resolve("dock", {
            scheme: "light", rollout: "replacement", tier: "minimal"
        });
        compare(dock.renderMode, "sampled-glass");
        var popup = Glass.resolve("menuDetached", {
            scheme: "light", rollout: "replacement", tier: "minimal"
        });
        compare(popup.renderMode, "solid");
    }

    // -- Toggle (FR-1) ------------------------------------------------------

    function test_toggle_keyboard_and_accessible_role() {
        var t = make(toggleComponent, { text: "Wi-Fi", checked: false });
        compare(t.Accessible.role, Accessible.Switch);
        compare(t.Accessible.name, "Wi-Fi");
        compare(t.Accessible.checkable, true);
        compare(t.Accessible.checked, false);

        t.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Space);
        compare(t.checked, true);
        compare(t.Accessible.checked, true);
    }

    function test_toggle_visual_track_color() {
        var t = make(toggleComponent, { checked: true });
        var img = grabImage(t);
        var cy = Math.round(Theme.controls.toggle.height / 2);
        compare(img.red(4, cy), channel(Theme.color.accent.r));
        compare(img.green(4, cy), channel(Theme.color.accent.g));
        compare(img.blue(4, cy), channel(Theme.color.accent.b));
    }

    // -- Traffic lights (FR-1, FR-3) ---------------------------------------

    function test_traffic_light_signals_and_roles() {
        var lights = make(lightsComponent, {});
        compare(lights.closeButton.Accessible.role, Accessible.Button);
        compare(lights.closeButton.Accessible.name, "Close");

        closeSpy.target = lights;
        closeSpy.clear();
        var s = Theme.controls.trafficLights.buttonSize;
        mouseClick(lights, s / 2, s / 2);
        compare(closeSpy.count, 1);
    }

    function test_traffic_light_visual_colors() {
        var lights = make(lightsComponent, {});
        var img = grabImage(lights);
        var s = Theme.controls.trafficLights.buttonSize;
        var gap = WindowStyle.trafficLightGap();
        compare(img.red(s / 2, s / 2), channel(Theme.color.close.r));
        compare(img.red(s + gap + s / 2, s / 2), channel(Theme.color.minimize.r));
        compare(img.red(2 * (s + gap) + s / 2, s / 2), channel(Theme.color.zoom.r));
    }

    // -- Popup (FR-1, FR-5) -------------------------------------------------

    // -- T202: menu family presets, geometry and motion ---------------------

    function test_menu_family_uses_attached_detached_and_status_presets() {
        var savedRollout = Glass.rollout;
        try {
        Glass.rollout = "replacement";
        var attached = make(popupComponent, {
            open: true, materialPreset: "menuAttached",
            materialBackdrop: "trusted", motionPreset: "menu"
        });
        compare(attached.materialPreset, "menuAttached");
        compare(attached.cornerTopRadius, 0);
        fuzzyCompare(attached.cornerRadius,
                     Theme.controls.contextMenu.attachedRadius, 0.001);
        fuzzyCompare(attached.surfaceFill.a, Glass.resolve("menuAttached", {scheme: Theme.dark ? "dark" : "light"}).alpha, 1e-4);

        var detached = make(popupComponent, {
            open: true, materialPreset: "menuDetached",
            materialBackdrop: "local", motionPreset: "menu"
        });
        fuzzyCompare(detached.cornerTopRadius, detached.cornerRadius, 1e-6);
        var local = Glass.resolve("menuDetached", {
            scheme: Theme.dark ? "dark" : "light", rollout: "replacement",
            backend: "software", tier: "full"
        });
        fuzzyCompare(detached.surfaceFill.a, local.alpha, 1e-4);
        verify(detached.surfaceFill.a >= 0.9,
               "an app-owned menu must paint the opaque local fallback");

        var status = make(popupComponent, {
            open: true, materialPreset: "statusMenu",
            materialBackdrop: "trusted", motionPreset: "menu"
        });
        compare(status.materialPreset, "statusMenu");
        compare(status.cornerTopRadius, status.cornerRadius);
        } finally {
            Glass.rollout = savedRollout;
        }
    }

    function test_menu_close_motion_is_measured_and_reduced() {
        var savedReduced = Theme.reducedMotion;
        Theme.reducedMotion = false;
        compare(Theme.motion.menuClose.fullDuration, 183);
        var popup = make(popupComponent, {
            open: true, materialPreset: "menuAttached",
            materialBackdrop: "trusted", motionPreset: "menu"
        });
        fuzzyCompare(popup.closeScale, 0.95, 1e-6);
        compare(popup.motionPreset, "menu");
        popup.open = false;
        waitForRendering(stage);
        compare(popup.open, false, "logical close is immediate");
        Theme.reducedMotion = true;
        fuzzyCompare(popup.closeScale, 1.0, 1e-6);
        compare(Theme.motion.menuClose.duration, 0);
        Theme.reducedMotion = savedReduced;
    }

        function test_submenu_pointer_corridor_data() {
            return [{ tag: "context", component: contextMenuComponent },
                    { tag: "menubar", component: menuComponent }];
        }

        function test_submenu_pointer_corridor(data) {
            var menu = make(data.component, {
                x: 10, y: 10,
                model: [{ label: "Options", type: "submenu",
                          submenu: [{ label: "One" }, { label: "Two" }] },
                        { label: "Other" }, { label: "Unavailable", enabled: false },
                        { type: "separator" }]
            });
            menu.open = true;
            waitForRendering(stage);
            var row = findChild(menu, "menuRow0");
            mouseMove(row, row.width / 2, row.height / 2);
            tryCompare(menu, "openSubmenuIndex", 0);
            var sibling = findChild(menu, "menuRow1");
            mouseMove(sibling, sibling.width - 2, sibling.height / 2);
            compare(menu.openSubmenuIndex, 0, "diagonal crossing retains the child");
            compare(menu.highlightedIndex, 0, "the parent stays selected during crossing");
            var sub = findChild(menu, "submenuPanel");
            mouseMove(sub, 12, 12);
            wait(Theme.controls.contextMenu.submenuDelay + 50);
            compare(menu.openSubmenuIndex, 0, "entering child cancels sibling dwell");
            compare(menu.highlightedIndex, 0);
            mouseMove(sibling, sibling.width / 2, sibling.height / 2);
            tryCompare(menu, "openSubmenuIndex", -1);
            compare(menu.highlightedIndex, 1, "a deliberate sibling dwell wins");
            var disabled = findChild(menu, "menuRow2");
            mouseMove(disabled, disabled.width / 2, disabled.height / 2);
            compare(menu.highlightedIndex, -1, "disabled rows retire stale selection");
        }

        function test_submenu_hover_is_cancelled_on_leave_data() {
            return test_submenu_pointer_corridor_data();
        }

        function test_keyboard_navigation_cancels_pending_submenu_data() {
            return test_submenu_pointer_corridor_data();
        }

        function test_keyboard_navigation_cancels_pending_submenu(data) {
            var menu = make(data.component, {
                x: 10, y: 10,
                model: [{ label: "Options", type: "submenu", submenu: [{ label: "One" }] },
                        { label: "Other" }]
            });
            menu.open = true;
            waitForRendering(stage);
            var row = findChild(menu, "menuRow0");
            mouseMove(row, 20, row.height / 2);
            tryCompare(menu, "pendingHoverIndex", 0);
            keyClick(Qt.Key_Down);
            compare(menu.highlightedIndex, 1);
            wait(Theme.controls.contextMenu.submenuDelay + 50);
            compare(menu.openSubmenuIndex, -1, "keyboard input supersedes hover dwell");
            compare(menu.highlightedIndex, 1);
        }

        function test_submenu_hover_is_cancelled_on_leave(data) {
            var menu = make(data.component, {
                x: 10, y: 10,
                model: [{ label: "Options", type: "submenu", submenu: [{ label: "One" }] }]
            });
            menu.open = true;
            waitForRendering(stage);
            var row = findChild(menu, "menuRow0");
            mouseMove(row, 20, row.height / 2);
            mouseMove(stage, stage.width - 1, stage.height - 1);
            wait(Theme.controls.contextMenu.submenuDelay + 50);
            compare(menu.openSubmenuIndex, -1, "a departed row cannot open a child later");
        }

        function test_context_menu_regions_keep_the_submenu_detached() {
        var cm = make(contextMenuComponent, {
            model: [ { label: "Options", type: "submenu",
                       submenu: [ { label: "A" }, { label: "B" } ] } ]
        });
        cm.open = true;
        cm.openSubmenu(0);
        waitForRendering(stage);
        verify(cm.regions.length >= 2, "submenu region must be declared");
        compare(cm.regions[0].preset, "menuDetached");
        compare(cm.regions[1].preset, "menuDetached");
        fuzzyCompare(cm.regions[0].br, cm.cornerRadius, 1e-6);
        fuzzyCompare(cm.regions[1].br, cm.cornerRadius, 1e-6);
    }

    function test_popup_escape_closes() {
        var p = make(popupComponent, { open: true });
        compare(p.open, true);
        compare(p.Accessible.role, Accessible.Pane);
        p.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(p.open, false);
    }

    // -- MenuBarMenu (FR-1, FR-4) ------------------------------------------

    function test_menu_model_is_publishable() {
        var m = make(menuComponent, {
            title: "File",
            model: [
                { label: "New" },
                { type: "separator" },
                { label: "Show Sidebar", checked: true, checkable: true }
            ]
        });
        compare(m.menuModel.length, 3);
        compare(m.menuModel[0].label, "New");
        compare(m.menuModel[1].type, "separator");
        compare(m.menuModel[2].checked, true);
        compare(m.menuModel[2].checkable, true);
        verify(JSON.stringify(m.menuModel).length > 0);
    }

    function test_menu_keyboard_activation() {
        var m = make(menuComponent, {
            title: "File",
            model: [{ label: "First" }, { label: "Second" }]
        });
        triggeredSpy.target = m;
        triggeredSpy.clear();
        m.openMenu();
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        keyClick(Qt.Key_Return);
        compare(m.open, false);
        compare(triggeredSpy.count, 1);
        compare(triggeredSpy.signalArguments[0][0], 1);
    }

    function test_menu_type_ahead_skips_disabled_rows() {
        var m = make(menuComponent, {
            title: "File",
            model: [
                { label: "Open" },
                { label: "Save", enabled: false,
                  disabledReason: "The document is read-only." },
                { label: "Settings" }
            ]
        });
        m.openMenu();
        waitForRendering(stage);
        m.typeAhead("s");
        // Save matches the prefix but is disabled, so Settings is selected.
        compare(m.highlightedIndex, 2);
        compare(m.entries[1].enabled, false);
        compare(m.entries[1].disabledReason, "The document is read-only.");
    }

    function test_disabled_menu_row_exposes_its_reason() {
        var row = make(menuRowComponent, {
            entry: { index: 0, label: "Save", enabled: false,
                     disabledReason: "The document is read-only." }
        });
        compare(row.disabled, true);
        compare(row.Accessible.description, "The document is read-only.");
    }

    // -- TitleBar intent ----------------------------------------------------

    function test_titlebar_emits_intent() {
        var bar = make(titleBarComponent, { width: 320, title: "T" });
        menuSpy.target = bar;
        menuSpy.clear();
        mouseClick(bar, 160, 20, Qt.RightButton);
        compare(menuSpy.count, 1);

        zoomSpy.target = bar;
        zoomSpy.clear();
        mouseDoubleClickSequence(bar, 160, 20);
        compare(zoomSpy.count, 1);
    }

    function test_titlebar_fills_its_chrome_slot() {
        var slot = make(titleBarInSlotComponent, {});
        var bar = slot.children[0];
        compare(bar.width, slot.width,
                "a parented TitleBar must span its chrome slot (the window width)");
        compare(bar.trafficLights.x,
                WindowStyle.trafficLightInset(bar.titleRowHeight));
    }

    // -- FR-3: app TitleBar vs compositor SSD reference ---------------------

    function test_fr3_titlebar_matches_ssd_reference() {
        var bar = make(titleBarComponent, { width: 520, title: "Dragonfruit", active: true });
        var ref = make(ssdComponent, { width: 520, title: "Dragonfruit", active: true });
        var app = grabImage(bar);
        var ssd = grabImage(ref);
        compare(app.width, ssd.width);
        compare(app.height, ssd.height);
        // Prove the images are non-trivial (not two blank surfaces): the close
        // traffic light must be present at its token-derived position.
        var inset = WindowStyle.trafficLightInset(bar.titleRowHeight);
        var cx = inset + WindowStyle.trafficLightSize / 2;
        var cy = Math.round(bar.chromeHeight / 2);
        compare(app.red(cx, cy), channel(Theme.color.close.r));
        compare(ssd.red(cx, cy), channel(Theme.color.close.r));
        verify(app.equals(ssd),
               "app TitleBar and compositor SSD reference must render identically at the same tokens");
    }

    // -- Shadows (T-04.1a, FR-2) -------------------------------------------

    function test_shadow_geometry_comes_from_elevation_tokens() {
        var high = Theme.controls.elevation.high;
        var shadow = make(shadowComponent, { width: 120, height: 80, level: "high" });
        compare(shadow.blur, high.blur);
        compare(shadow.layers, high.layers);
        compare(shadow.offset.y, high.offsetY);

        // Switching the level re-derives the geometry from the token group,
        // exactly as the compositor's shadow pass does.
        shadow.level = "overlay";
        waitForRendering(stage);
        compare(shadow.blur, Theme.controls.elevation.overlay.blur);
        compare(shadow.offset.y, Theme.controls.elevation.overlay.offsetY);

        // The four levels follow the primitive elevation scale.
        compare(Theme.controls.elevation.low.blur, Theme.primitive.elevation.low);
        compare(Theme.controls.elevation.med.blur, Theme.primitive.elevation.med);
        compare(Theme.controls.elevation.high.blur, Theme.primitive.elevation.high);
        compare(Theme.controls.elevation.overlay.blur, Theme.primitive.elevation.overlay);
    }

    // -- Button (FR-1) ------------------------------------------------------

    function test_button_keyboard_and_role() {
        var b = make(buttonComponent, { text: "Save" });
        compare(b.Accessible.role, Accessible.Button);
        compare(b.Accessible.name, "Save");
        buttonSpy.target = b;
        buttonSpy.clear();
        b.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Space);
        compare(buttonSpy.count, 1);
    }

    // -- Toolbar (FR-1) -----------------------------------------------------

    function test_toolbar_role_and_height() {
        var t = make(toolbarComponent, { width: 400, title: "Documents" });
        compare(t.Accessible.role, Accessible.ToolBar);
        compare(t.Accessible.name, "Documents");
        compare(t.implicitHeight, Theme.controls.toolbar.height);
    }

    // -- SplitView (FR-1) ---------------------------------------------------

    function test_splitview_keyboard_resizes() {
        var s = make(splitViewComponent, { width: 500, height: 200 });
        compare(s.divider.Accessible.role, Accessible.Splitter);
        var before = s.firstSize;
        s.divider.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Right);
        verify(s.firstSize > before, "Right arrow must widen the first pane");
        keyClick(Qt.Key_Home);
        compare(s.firstSize, s.minimumPane);
    }

    // -- SettingsRow / SettingsGroup (FR-1) ---------------------------------

    function test_settings_roles() {
        var row = make(settingsRowComponent, { label: "Wi-Fi" });
        compare(row.Accessible.role, Accessible.Grouping);
        compare(row.Accessible.name, "Wi-Fi");

        var group = make(settingsGroupComponent, { width: 300, title: "General" });
        compare(group.Accessible.role, Accessible.Grouping);
        compare(group.Accessible.name, "General");
    }

    // -- SegmentedControl (FR-1) -------------------------------------------

    function test_segmented_keyboard_selection() {
        var seg = make(segmentedComponent, { model: ["One", "Two", "Three"], currentIndex: 0 });
        compare(seg.Accessible.role, Accessible.Grouping);
        segmentedSpy.target = seg;
        segmentedSpy.clear();
        var first = seg.currentSegment;
        verify(first !== null, "the current segment must be instantiated");
        compare(first.Accessible.role, Accessible.RadioButton);
        first.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Right);
        compare(seg.currentIndex, 1);
        keyClick(Qt.Key_Return);
        compare(segmentedSpy.count, 1);
        compare(segmentedSpy.signalArguments[0][0], 1);
    }

    // -- Slider (T-09.4) -----------------------------------------------------

    function test_slider_keyboard_and_role() {
        var s = make(sliderComponent, { value: 0.5, accessibleName: "Dock size" });
        compare(s.Accessible.role, Accessible.Slider);
        compare(s.Accessible.name, "Dock size");
        sliderMovedSpy.target = s;
        sliderMovedSpy.clear();
        sliderCommittedSpy.target = s;
        sliderCommittedSpy.clear();

        s.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Right);
        verify(Math.abs(s.value - (0.5 + Theme.controls.slider.step)) < 0.0001,
               "Right must increase the value by the token step");
        compare(sliderMovedSpy.count, 1);
        compare(sliderCommittedSpy.count, 1);

        keyClick(Qt.Key_Home);
        compare(s.value, 0);
        keyClick(Qt.Key_End);
        compare(s.value, 1);
    }

    function test_slider_captions_and_visual_track() {
        var s = make(sliderComponent, {
            value: 1.0, minLabel: "Small", maxLabel: "Large"
        });
        verify(s.hasCaptions);
        verify(s.implicitHeight > Theme.controls.slider.height,
               "captioned sliders reserve room below the track");
        var img = grabImage(s);
        var cy = Math.round(Theme.controls.slider.height / 2);
        compare(img.red(4, cy), channel(Theme.color.accent.r));
        compare(img.green(4, cy), channel(Theme.color.accent.g));
        compare(img.blue(4, cy), channel(Theme.color.accent.b));
    }

    // -- Select (T-09.4) -----------------------------------------------------

    function test_select_keyboard_and_role() {
        var sel = make(selectComponent, {
            accessibleName: "Dock position",
            model: ["Bottom", "Left", "Right"],
            currentIndex: 0
        });
        compare(sel.Accessible.role, Accessible.ComboBox);
        compare(sel.Accessible.name, "Dock position");
        compare(sel.currentLabel, "Bottom");
        selectActivatedSpy.target = sel;
        selectActivatedSpy.clear();

        sel.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        verify(sel.menu.open, "Down must open the value popup");
        keyClick(Qt.Key_Down);
        keyClick(Qt.Key_Return);
        compare(sel.currentIndex, 1);
        compare(sel.currentLabel, "Left");
        compare(selectActivatedSpy.count, 1);
        compare(selectActivatedSpy.signalArguments[0][0], 1);
        compare(sel.menu.open, false);
    }

    function test_select_activate_emits_value() {
        var sel = make(selectComponent, {
            model: [{ value: "bottom", label: "Bottom" },
                    { value: "left", label: "Left" }],
            currentIndex: 0
        });
        selectActivatedSpy.target = sel;
        selectActivatedSpy.clear();
        sel.activateIndex(1);
        compare(sel.currentValue, "left");
        compare(selectActivatedSpy.count, 1);
    }

    // -- ContextMenu (FR-1, FR-4) ------------------------------------------

    function test_context_menu_keyboard_activation() {
        var cm = make(contextMenuComponent, {
            model: [{ label: "One" }, { label: "Two" }]
        });
        compare(cm.Accessible.role, Accessible.PopupMenu);
        compare(cm.menuModel.length, 2);
        triggeredSpy.target = cm;
        triggeredSpy.clear();
        cm.showAt(10, 10);
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        keyClick(Qt.Key_Return);
        compare(cm.open, false);
        compare(triggeredSpy.count, 1);
        compare(triggeredSpy.signalArguments[0][0], 1);
    }

    function test_context_menu_keep_open_item_does_not_dismiss() {
        var cm = make(contextMenuComponent, {
            model: [{ label: "More", keepOpen: true }, { label: "Done" }]
        });
        triggeredSpy.target = cm;
        triggeredSpy.clear();
        cm.showAt(10, 10);
        waitForRendering(stage);
        cm.activate(0);
        compare(triggeredSpy.count, 1);
        compare(cm.open, true);
        cm.activate(1);
        compare(cm.open, false);
    }

    function test_context_menu_disabled_item_is_not_activated() {
        var cm = make(contextMenuComponent, {
            model: [{ label: "Empty Trash", enabled: false }]
        });
        triggeredSpy.target = cm;
        triggeredSpy.clear();
        cm.activate(0);
        compare(triggeredSpy.count, 0);
    }

    function test_context_menu_submenu_opens_and_activates() {
        var cm = make(contextMenuComponent, {
            model: [
                { label: "Open With", type: "submenu",
                  submenu: [{ label: "Text" }, { label: "Image" }] }
            ]
        });
        triggeredSpy.target = cm;
        triggeredSpy.clear();
        cm.showAt(10, 10);
        waitForRendering(stage);
        compare(cm.openSubmenuIndex, -1);
        keyClick(Qt.Key_Right);
        compare(cm.openSubmenuIndex, 0);
        compare(cm.submenuEntries.length, 2);
        compare(cm.submenuHighlightedIndex, 0);
        keyClick(Qt.Key_Down);
        compare(cm.submenuHighlightedIndex, 1);
        keyClick(Qt.Key_Return);
        compare(triggeredSpy.count, 1);
        compare(triggeredSpy.signalArguments[0][1].label, "Image");
        compare(cm.open, false);
        compare(cm.openSubmenuIndex, -1);
    }

    function test_context_menu_submenu_escape_closes_only_submenu() {
        var cm = make(contextMenuComponent, {
            model: [
                { label: "Options", type: "submenu",
                  submenu: [{ label: "One" }, { label: "Two" }] }
            ]
        });
        cm.showAt(10, 10);
        waitForRendering(stage);
        cm.openSubmenu(0);
        compare(cm.openSubmenuIndex, 0);
        keyClick(Qt.Key_Escape);
        compare(cm.openSubmenuIndex, -1);
        compare(cm.open, true);
        keyClick(Qt.Key_Escape);
        compare(cm.open, false);
    }

    function test_context_menu_content_rect_covers_submenu() {
        var cm = make(contextMenuComponent, {
            model: [
                { label: "Options", type: "submenu",
                  submenu: [{ label: "A" }, { label: "B" }] }
            ]
        });
        cm.showAt(10, 10);
        waitForRendering(stage);
        var closedWidth = cm.contentRect.w;
        cm.openSubmenu(0);
        wait(300);
        waitForRendering(stage);
        verify(cm.contentRect.w > closedWidth,
               "an open submenu must extend the committed content rect");
    }

    // -- Shortcut hints and menu row anatomy --------------------------------

    function test_shortcut_hint_parses_plus_chords_in_canonical_order() {
        var hint = make(shortcutHintComponent, { shortcut: "Super+Shift+Z" });
        compare(hint.keys.length, 3);
        compare(hint.keys[0].id, "shift");
        compare(hint.keys[1].id, "super");
        compare(hint.keys[2].glyph, "Z");
    }

    function test_shortcut_hint_parses_glyph_clusters_and_key_aliases() {
        var hint = make(shortcutHintComponent, { shortcut: "⇧⌘N" });
        compare(hint.keys.length, 3);
        compare(hint.keys[0].id, "shift");
        compare(hint.keys[1].id, "super");
        compare(hint.keys[2].glyph, "N");
        hint.shortcut = "Ctrl+Delete";
        compare(hint.keys.length, 2);
        compare(hint.keys[0].glyph, "⌃");
        compare(hint.keys[1].glyph, "⌫");
    }

    function test_shortcut_hint_keeps_plain_trailing_text() {
        var hint = make(shortcutHintComponent, { shortcut: "Space 2" });
        compare(hint.hasShortcut, false);
        compare(hint.keys.length, 0);
        hint.shortcut = "Home";
        compare(hint.hasShortcut, false);
    }

    function test_shortcut_hint_renders_fn_and_function_keys() {
        var hint = make(shortcutHintComponent, { shortcut: "Fn+Ctrl+F3" });
        compare(hint.keys.length, 3);
        compare(hint.keys[0].id, "fn");
        compare(hint.keys[1].id, "ctrl");
        compare(hint.keys[2].glyph, "F3");
    }

    function test_context_menu_grows_to_fit_its_longest_label() {
        var cm = make(contextMenuComponent, {
            open: true,
            model: [
                { label: "Open", shortcut: "Super+O" },
                { label: "Enter Full Screen", shortcut: "Ctrl+Super+F" }
            ]
        });
        verify(cm.width > Theme.controls.contextMenu.minWidth,
               "a long label must widen the menu instead of eliding at minWidth");
        var row = findChild(cm, "menuRow1");
        verify(row !== null, "the long row must be addressable");
        verify(row.width >= row.implicitWidth,
               "the row must fill the resolved menu width");
    }

    function test_context_menu_measures_before_it_opens() {
        var cm = make(contextMenuComponent, {
            model: [{ label: "Enter Full Screen", shortcut: "Ctrl+Super+F" }]
        });
        var closedWidth = cm.width;
        verify(closedWidth > Theme.controls.contextMenu.minWidth,
               "the closed menu must already include its widest row and hint");
        cm.open = true;
        waitForRendering(stage);
        compare(cm.width, closedWidth);
    }

    function test_context_menu_rows_stretch_to_the_panel_width() {
        var cm = make(contextMenuComponent, {
            open: true,
            model: [{ label: "Open", shortcut: "Super+O" }]
        });
        compare(cm.width, Theme.controls.contextMenu.minWidth);
        var row = findChild(cm, "menuRow0");
        verify(row !== null, "the short row must be addressable");
        verify(row.width > row.implicitWidth,
               "a short row must stretch so its highlight spans the panel");
    }

    function test_menu_row_reserves_the_shortcut_column() {
        var plain = make(menuRowComponent, {
            entry: { index: 0, type: "item", label: "Open", enabled: true }
        });
        var chord = make(menuRowComponent, {
            entry: { index: 0, type: "item", label: "Open",
                     shortcut: "Super+O", enabled: true }
        });
        verify(chord.implicitWidth > plain.implicitWidth,
               "a shortcut hint must widen the row rather than overlap the label");
        var trailing = make(menuRowComponent, {
            entry: { index: 0, type: "item", label: "Open",
                     shortcut: "Space 2", enabled: true }
        });
        verify(trailing.implicitWidth > plain.implicitWidth,
               "plain trailing metadata must reserve the same cell as a chord");
    }

    // -- MenuBarMenu submenus (T-10 section 13) ----------------------------

    function test_menu_submenu_opens_and_activates() {
        var m = make(menuComponent, {
            title: "File",
            model: [
                { label: "Open With", type: "submenu",
                  submenu: [{ label: "Text" }, { label: "Image" }] }
            ]
        });
        triggeredSpy.target = m;
        triggeredSpy.clear();
        m.openMenu();
        waitForRendering(stage);
        compare(m.openSubmenuIndex, -1);
        keyClick(Qt.Key_Right);
        compare(m.openSubmenuIndex, 0);
        compare(m.submenuEntries.length, 2);
        compare(m.submenuHighlightedIndex, 0);
        keyClick(Qt.Key_Down);
        compare(m.submenuHighlightedIndex, 1);
        keyClick(Qt.Key_Return);
        compare(triggeredSpy.count, 1);
        compare(triggeredSpy.signalArguments[0][1].label, "Image");
        compare(m.open, false);
        compare(m.openSubmenuIndex, -1);
    }

    function test_menu_submenu_escape_closes_only_submenu() {
        var m = make(menuComponent, {
            title: "File",
            model: [
                { label: "Options", type: "submenu",
                  submenu: [{ label: "One" }, { label: "Two" }] }
            ]
        });
        m.openMenu();
        waitForRendering(stage);
        m.openSubmenu(0);
        compare(m.openSubmenuIndex, 0);
        keyClick(Qt.Key_Escape);
        compare(m.openSubmenuIndex, -1);
        compare(m.open, true);
        keyClick(Qt.Key_Escape);
        compare(m.open, false);
    }

    function test_menu_content_rect_covers_submenu() {
        var m = make(menuComponent, {
            title: "File",
            model: [
                { label: "Options", type: "submenu",
                  submenu: [{ label: "A" }, { label: "B" }] }
            ]
        });
        m.openMenu();
        waitForRendering(stage);
        var closedWidth = m.contentRect.w;
        m.openSubmenu(0);
        waitForRendering(stage);
        verify(m.contentRect.w > closedWidth,
               "an open submenu must extend the committed content rect");
    }

    // -- SearchField (FR-1) -------------------------------------------------

    function test_search_field_escape_clears() {
        var sf = make(searchFieldComponent, { width: 200, text: "abc" });
        compare(sf.Accessible.role, Accessible.EditableText);
        compare(sf.Accessible.searchEdit, true);
        sf.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(sf.text, "");
    }

    // -- SourceList (FR-1) --------------------------------------------------

    function test_source_list_navigation_and_expansion() {
        var sl = make(sourceListComponent, {
            model: [
                { label: "A", hasChildren: true, expanded: false },
                { label: "B", depth: 1 },
                { label: "C" }
            ]
        });
        compare(sl.Accessible.role, Accessible.Tree);
        sl.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        compare(sl.currentIndex, 0);
        keyClick(Qt.Key_Right);
        verify(sl.isExpanded(0), "Right arrow must expand a collapsed branch");
        keyClick(Qt.Key_Right);
        compare(sl.currentIndex, 1);
    }

    // -- Sidebar (FR-1) -----------------------------------------------------

    function test_sidebar_navigation_and_role() {
        var sb = make(sidebarComponent, {
            sections: [{ title: "Favorites", items: [{ label: "One" }, { label: "Two" }] }]
        });
        compare(sb.Accessible.role, Accessible.List);
        sidebarSpy.target = sb;
        sidebarSpy.clear();
        sb.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        compare(sb.currentIndex, 1);
        keyClick(Qt.Key_Return);
        compare(sidebarSpy.count, 1);
    }

    // -- Dialog / Sheet (FR-1) ----------------------------------------------

    function test_dialog_accept_and_reject() {
        var d = make(dialogComponent, { open: true, title: "Replace?" });
        compare(d.Accessible.role, Accessible.Dialog);
        acceptedSpy.target = d;
        rejectedSpy.target = d;
        acceptedSpy.clear();
        rejectedSpy.clear();
        d.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(rejectedSpy.count, 1);
        compare(d.open, false);

        d.show();
        waitForRendering(stage);
        d.forceActiveFocus();
        keyClick(Qt.Key_Return);
        compare(acceptedSpy.count, 1);
    }

    function test_sheet_reject() {
        var s = make(sheetComponent, { open: true, title: "Go to Folder" });
        compare(s.Accessible.role, Accessible.Dialog);
        rejectedSpy.target = s;
        rejectedSpy.clear();
        s.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(rejectedSpy.count, 1);
        compare(s.open, false);
    }

    // -- Popover (FR-1) -----------------------------------------------------

    function test_popover_escape_closes() {
        var p = make(popoverComponent, { open: true });
        compare(p.Accessible.role, Accessible.Pane);
        p.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(p.open, false);
    }

    // -- Tooltip (T-14.7i, ADR 0093) ----------------------------------------

    function test_tooltip_is_passive_and_never_takes_focus() {
        var anchor = make(rectComponent, { width: 48, height: 48, x: 100, y: 120 });
        var t = make(tooltipComponent, { anchorItem: anchor, text: "Safari", open: true });
        tryCompare(t, "opacity", 1);
        compare(t.activeFocus, false, "a tooltip must never take active focus");
        compare(t.focus, false);
        compare(t.activeFocusOnTab, false);
        compare(t.Accessible.ignored, true, "a tooltip is presentational only");
    }

    function test_tooltip_opens_and_closes_with_motion() {
        var anchor = make(rectComponent, { width: 48, height: 48, x: 100, y: 120 });
        var t = make(tooltipComponent, { anchorItem: anchor, text: "Safari", open: false });
        compare(t.opacity, 0);
        compare(t.visible, false);
        t.open = true;
        tryCompare(t, "opacity", 1);
        compare(t.visible, true);
        t.open = false;
        tryCompare(t, "opacity", 0);
    }

    function test_tooltip_elides_long_text_at_the_max_width() {
        var anchor = make(rectComponent, { width: 48, height: 48, x: 100, y: 120 });
        var t = make(tooltipComponent, {
            anchorItem: anchor,
            text: "A very long hover label that cannot possibly fit inside the narrow token width",
            open: true
        });
        tryCompare(t, "opacity", 1);
        compare(t.width, Theme.controls.tooltip.maxWidth);
        verify(t.textElided, "a long single line must elide, never widen the capsule");
    }

    function test_tooltip_placement_above_and_below() {
        var anchor = make(rectComponent, { width: 48, height: 48, x: 100, y: 120 });
        var t = make(tooltipComponent, {
            anchorItem: anchor, text: "Safari", placement: "above", open: true
        });
        tryCompare(t, "opacity", 1);
        verify(t.y + t.height <= anchor.y, "an above label must clear the anchor");
        t.placement = "below";
        waitForRendering(stage);
        verify(t.y >= anchor.y + anchor.height, "a below label must sit under the anchor");
    }

    function test_tooltip_clamps_inside_its_bounds() {
        var bounds = make(rectComponent, { width: 200, height: 200 });
        var anchor = make(rectComponent, { width: 40, height: 40, x: 180, y: 20 });
        var t = make(tooltipComponent, {
            anchorItem: anchor, bounds: bounds, text: "Wide label", placement: "above", open: true
        });
        tryCompare(t, "opacity", 1);
        verify(t.x >= 0, "the capsule must not spill past the leading edge");
        verify(t.x + t.width <= bounds.width + 0.5,
               "the capsule must be clamped at the trailing edge");
    }

    function test_tooltip_reduced_motion_opens_instantly() {
        var saved = Theme.reducedMotion;
        Theme.reducedMotion = true;
        var anchor = make(rectComponent, { width: 48, height: 48, x: 100, y: 120 });
        var t = make(tooltipComponent, { anchorItem: anchor, text: "Safari", open: false });
        compare(t.opacity, 0);
        t.open = true;
        waitForRendering(stage);
        compare(t.opacity, 1, "reduced motion must open with no fade");
        Theme.reducedMotion = saved;
    }

    // -- Skeleton (T-18.2) --------------------------------------------------
    //
    // The loading placeholder: a token grey rectangle with a slow-moving
    // highlight. The reduced-motion variant starts no animation at all and
    // rests the highlight centered, so both states are assertable from the
    // band's live x.

    function test_skeleton_is_a_token_grey_placeholder() {
        var s = make(skeletonComponent, {});
        compare(s.radius, Theme.controls.skeleton.radius);
        compare(String(s.color), String(Theme.color.skeletonBase));
        verify(s.shimmering, "a default skeleton animates its highlight");
        compare(s.Accessible.role, Accessible.Graphic);
        compare(s.Accessible.ignored, true,
                "an unnamed skeleton is presentational");
    }

    function test_skeleton_accessible_name_is_announced() {
        var s = make(skeletonComponent, { accessibleName: "Downloading wallpapers" });
        compare(s.Accessible.name, "Downloading wallpapers");
        compare(s.Accessible.ignored, false);
    }

    function test_skeleton_shimmer_moves() {
        var s = make(skeletonComponent, {});
        var first = s.highlightX;
        wait(150);
        waitForRendering(stage);
        verify(Math.abs(s.highlightX - first) > 0.5,
               "the shimmer highlight must advance, was " + first + " now " + s.highlightX);
    }

    function test_skeleton_reduced_motion_is_static() {
        var saved = Theme.reducedMotion;
        Theme.reducedMotion = true;
        var s = make(skeletonComponent, {});
        verify(!s.shimmering, "reduced motion must stop the shimmer");
        var first = s.highlightX;
        wait(150);
        waitForRendering(stage);
        compare(s.highlightX, first, "the static highlight must not move");
        compare(s.progress, 0.0, "no animation progress is recorded when reduced");
        Theme.reducedMotion = saved;
    }

    // -- Spinner (ADR 0240) -------------------------------------------------
    //
    // The small progress indicator used by the Wallpaper pane. Indeterminate
    // by default; a 0..1 `progress` draws a determinate arc and stops the
    // rotation. Reduced motion freezes the arc.

    function test_spinner_is_a_token_progress_indicator() {
        var s = make(spinnerComponent, {});
        compare(s.implicitWidth, Theme.controls.spinner.size);
        compare(s.implicitHeight, Theme.controls.spinner.size);
        compare(s.Accessible.role, Accessible.ProgressBar);
        compare(s.Accessible.ignored, true,
                "an unnamed spinner is presentational");
        verify(s.spinning, "an indeterminate spinner rotates");
        compare(s.sweepAngle, 270);
    }

    function test_spinner_accessible_name_is_announced() {
        var s = make(spinnerComponent, { accessibleName: "Applying Nature" });
        compare(s.Accessible.name, "Applying Nature");
        compare(s.Accessible.ignored, false);
    }

    function test_spinner_determinate_progress_stops_rotation() {
        var s = make(spinnerComponent, { progress: 0.5 });
        verify(!s.spinning, "a determinate spinner does not rotate");
        compare(s.sweepAngle, 180);
    }

    function test_spinner_reduced_motion_is_static() {
        var saved = Theme.reducedMotion;
        Theme.reducedMotion = true;
        var s = make(spinnerComponent, {});
        verify(!s.spinning, "reduced motion must stop the spinner");
        wait(150);
        waitForRendering(stage);
        compare(s.rotation, 0, "the static spinner must not rotate");
        Theme.reducedMotion = saved;
    }

    // -- PhosphorIcon (T-19.1a) ---------------------------------------------
    //
    // The glyph primitive: size and tint come from properties, an unknown
    // name/weight resolves to nothing (never a silent substitute), and a
    // missing glyph is a blank mark rather than a crash. The tint is asserted
    // on real grabbed pixels: a ShapePath fill renders under the headless
    // software scene graph, unlike MultiEffect colorization.

    function findPixel(img, color) {
        var r = channel(color.r), g = channel(color.g), b = channel(color.b);
        for (var y = 0; y < img.height; ++y) {
            for (var x = 0; x < img.width; ++x) {
                if (img.red(x, y) === r && img.green(x, y) === g && img.blue(x, y) === b)
                    return true;
            }
        }
        return false;
    }

    function test_phosphor_icon_sizes_and_tints_from_properties() {
        var icon = make(phosphorIconComponent, {
            name: "square", weight: "fill", size: 48, color: "#ff8800"
        });
        compare(icon.resolved, true);
        compare(icon.implicitWidth, 48);
        compare(icon.implicitHeight, 48);
        compare(icon.source, "qrc:/icons/phosphor/square-fill.svg");
        verify(findPixel(grabImage(icon), icon.color),
               "the glyph must be filled with the tint color");

        // Changing the tint recolors the mark; changing the size resizes it.
        icon.color = "#0033ff";
        icon.size = 64;
        waitForRendering(stage);
        compare(icon.implicitWidth, 64);
        verify(findPixel(grabImage(icon), icon.color),
               "the tint must follow the color property");
    }

    function test_phosphor_icon_name_and_weight_resolution() {
        var regular = make(phosphorIconComponent, { name: "gear", weight: "regular" });
        compare(regular.resolved, true);
        compare(regular.source, "qrc:/icons/phosphor/gear.svg");

        var fill = make(phosphorIconComponent, { name: "gear", weight: "fill" });
        compare(fill.resolved, true);
        compare(fill.source, "qrc:/icons/phosphor/gear-fill.svg");

        // A weight that is not vendored resolves to nothing; it does not
        // silently fall back to the regular glyph.
        var bold = make(phosphorIconComponent, { name: "gear", weight: "bold" });
        compare(bold.resolved, false);
        compare(bold.glyphPath, "");
    }

    function test_phosphor_icon_unknown_glyph_is_blank_not_a_crash() {
        var icon = make(phosphorIconComponent, { name: "definitely-not-a-real-glyph" });
        compare(icon.resolved, false);
        compare(icon.glyphPath, "");
        var img = grabImage(icon);
        compare(img.width, Math.round(icon.width));
        compare(img.height, Math.round(icon.height));
    }

    function test_phosphor_icon_default_color_comes_from_theme() {
        var icon = make(phosphorIconComponent, {});
        compare(String(icon.color), String(Theme.color.textPrimary));
        compare(icon.Accessible.role, Accessible.Graphic);
        compare(icon.Accessible.ignored, true);
        var named = make(phosphorIconComponent, { accessibleName: "Settings" });
        compare(named.Accessible.name, "Settings");
        compare(named.Accessible.ignored, false);
    }

    function test_phosphor_svg_resource_is_reachable() {
        // The raw vendored SVG ships at the stable prefix the registry is
        // generated from; both weights must resolve there.
        var xhr = new XMLHttpRequest();
        xhr.open("GET", "qrc:/icons/phosphor/gear.svg", false);
        xhr.send();
        compare(xhr.status, 200);
        verify(xhr.responseText.indexOf("<path") >= 0,
               "the vendored regular SVG must be reachable under /icons/phosphor");
        xhr.open("GET", "qrc:/icons/phosphor/gear-fill.svg", false);
        xhr.send();
        compare(xhr.status, 200);
        verify(xhr.responseText.indexOf("<path") >= 0,
               "the vendored fill SVG must be reachable under /icons/phosphor");
    }

    // -- TablerIcon (ADR 0185) ----------------------------------------------
    //
    // The line-glyph primitive: size, tint, weight, and stroke come from
    // properties; an unknown name/weight resolves to nothing (never a silent
    // substitute), and a missing glyph is a blank mark rather than a crash.

    function test_tabler_icon_sizes_and_tints_from_properties() {
        var icon = make(tablerIconComponent, {
            name: "folder", size: 48, color: "#ff8800"
        });
        compare(icon.resolved, true);
        compare(icon.implicitWidth, 48);
        compare(icon.implicitHeight, 48);
        compare(icon.source, "qrc:/icons/tabler/folder.svg");
        verify(findPixel(grabImage(icon), icon.color),
               "the outline must be stroked with the tint color");

        // Changing the tint recolors the mark; changing the size resizes it.
        icon.color = "#0033ff";
        icon.size = 64;
        waitForRendering(stage);
        compare(icon.implicitWidth, 64);
        verify(findPixel(grabImage(icon), icon.color),
               "the tint must follow the color property");
    }

    function test_tabler_icon_weight_resolution() {
        var outline = make(tablerIconComponent, { name: "folder", weight: "outline" });
        compare(outline.resolved, true);
        compare(outline.source, "qrc:/icons/tabler/folder.svg");

        var filled = make(tablerIconComponent, { name: "folder", weight: "filled" });
        compare(filled.resolved, true);
        compare(filled.source, "qrc:/icons/tabler/folder-filled.svg");

        // A weight that is not vendored resolves to nothing; it does not
        // silently fall back to the outline glyph.
        var bold = make(tablerIconComponent, { name: "folder", weight: "bold" });
        compare(bold.resolved, false);
        compare(bold.glyphPath, "");
    }

    function test_tabler_filled_icon_renders_solid() {
        var icon = make(tablerIconComponent, {
            name: "folder", weight: "filled", size: 48, color: "#00aa55"
        });
        compare(icon.resolved, true);
        verify(findPixel(grabImage(icon), icon.color),
               "the filled glyph must fill with the tint color");
    }

    function test_tabler_icon_unknown_glyph_is_blank_not_a_crash() {
        var icon = make(tablerIconComponent, { name: "definitely-not-a-real-glyph" });
        compare(icon.resolved, false);
        compare(icon.glyphPath, "");
        var img = grabImage(icon);
        compare(img.width, Math.round(icon.width));
        compare(img.height, Math.round(icon.height));
    }

    function test_tabler_icon_default_color_comes_from_theme() {
        var icon = make(tablerIconComponent, {});
        compare(String(icon.color), String(Theme.color.textPrimary));
        compare(icon.Accessible.role, Accessible.Graphic);
        compare(icon.Accessible.ignored, true);
        var named = make(tablerIconComponent, { accessibleName: "Folder" });
        compare(named.Accessible.name, "Folder");
        compare(named.Accessible.ignored, false);
    }

    function test_tabler_svg_resource_is_reachable() {
        // The raw vendored SVG ships at the stable prefix the registry is
        // generated from; both weights must resolve there.
        var xhr = new XMLHttpRequest();
        xhr.open("GET", "qrc:/icons/tabler/folder.svg", false);
        xhr.send();
        compare(xhr.status, 200);
        verify(xhr.responseText.indexOf("<path") >= 0,
               "the vendored outline SVG must be reachable under /icons/tabler");
        xhr.open("GET", "qrc:/icons/tabler/folder-filled.svg", false);
        xhr.send();
        compare(xhr.status, 200);
        verify(xhr.responseText.indexOf("<path") >= 0,
               "the vendored filled SVG must be reachable under /icons/tabler");
    }

    function test_icon_facade_maps_the_inherited_vocabulary() {
        // Files/Settings/shell still name the inherited vocabulary; the facade
        // must resolve each name to a vendored Tabler outline glyph.
        var close = make(legacyIconComponent, { name: "close" });
        compare(close.tablerName, "x");
        compare(close.resolved, true);
        close.destroy();

        var wifiError = make(legacyIconComponent, { name: "wifi-error" });
        compare(wifiError.tablerName, "wifi-off");
        compare(wifiError.resolved, true);
        wifiError.destroy();

        var unknown = make(legacyIconComponent, { name: "definitely-not-a-real-glyph" });
        compare(unknown.resolved, false);
        compare(unknown.tablerName, "");
    }

    function test_every_legacy_alias_target_is_vendored() {
        var facade = make(legacyIconComponent, {});
        var aliases = facade.aliases;
        for (var legacy in aliases) {
            verify(TablerGlyphs.has("outline", aliases[legacy]),
                   "legacy glyph " + legacy + " -> " + aliases[legacy]
                   + " must be vendored");
        }
        verify(Object.keys(aliases).length >= 60,
               "the facade must cover the shipped legacy vocabulary");
    }

    // -- SettingsCategoryIcon (T-19.1b) -------------------------------------
    //
    // The System Settings category tile: the container supplies the gradient,
    // highlight, and shadow and the wrapped PhosphorIcon supplies the glyph.
    // Size, tint, and gradient come from properties; an unknown glyph is a
    // blank mark, never a crash.

    function test_settings_category_tile_sizes_and_tints_from_properties() {
        var tile = make(settingsCategoryComponent, {
            source: "gear",
            size: 48,
            symbolColor: "#ff8800",
            // A flat fill makes the container color exact across the tile (a
            // two-stop gradient only reaches its end color asymptotically).
            gradientStart: "#0000ff",
            gradientEnd: "#0000ff"
        });
        compare(tile.implicitWidth, 48);
        compare(tile.implicitHeight, 48);
        compare(tile.radius, Math.round(48 * Theme.controls.settingsCategory.radiusRatio));

        var img = grabImage(tile);
        verify(findPixel(img, Qt.rgba(0, 0, 1, 1)), "the tile must paint the gradient");
        verify(findPixel(img, tile.symbolColor), "the glyph must be filled with symbolColor");

        // Changing the size and tint resizes and recolors the tile.
        tile.size = 32;
        tile.symbolColor = "#00cc44";
        waitForRendering(stage);
        compare(tile.implicitWidth, 32);
        compare(tile.implicitHeight, 32);
        verify(findPixel(grabImage(tile), tile.symbolColor),
               "the glyph tint must follow symbolColor");
    }

    function test_settings_category_unknown_glyph_is_blank_not_a_crash() {
        var tile = make(settingsCategoryComponent, {
            source: "definitely-not-a-real-glyph", size: 32,
            gradientStart: "#22aa55", gradientEnd: "#22aa55"
        });
        var img = grabImage(tile);
        compare(img.width, Math.round(tile.width));
        compare(img.height, Math.round(tile.height));
        // The container still paints even when the glyph is unknown.
        verify(findPixel(img, Qt.rgba(0x22 / 255, 0xaa / 255, 0x55 / 255, 1)),
               "the gradient container must render regardless of the glyph");
    }

    function test_settings_category_tile_follows_its_slot() {
        // A caller may lay the tile out at a size other than the implicit
        // default (the sidebar slot is 20px, the hero is 48px). The painted
        // container must fill that slot in both directions.
        var tile = make(settingsCategoryComponent, {
            size: 20, width: 40, height: 40, radius: 0,
            gradientStart: "#336699", gradientEnd: "#336699"
        });
        var img = grabImage(tile);
        compare(img.width, 40);
        verify(findPixel(img, Qt.rgba(0x33 / 255, 0x66 / 255, 0x99 / 255, 1)),
               "a slot larger than `size` must be filled by the container");

        // And it must not overflow a slot smaller than `size`.
        var slot = make(settingsCategorySlotComponent);
        var slotImg = grabImage(slot);
        var inside = slotImg.pixel(40, 40);
        verify(inside.a > 0.9 && inside.b > 0.9,
               "the container must paint inside the 20px slot");
        // The grabbed background is the light window fill, so test the hue:
        // a bleeding container would paint its blue past the slot edge.
        var outside = slotImg.pixel(61, 40);
        verify(outside.r > 0.5,
               "a slot smaller than `size` must clip the container to the slot");
    }

    function test_settings_category_default_is_a_token_gradient() {
        var tile = make(settingsCategoryComponent, { size: 40 });
        compare(String(tile.gradientStart),
                String(Theme.primitive.color.violet500));
        compare(String(tile.gradientEnd),
                String(Theme.primitive.color.violet700));
        compare(String(tile.symbolColor),
                String(Theme.primitive.color.neutral50));
    }

    // -- ScrollView (FR-1) --------------------------------------------------

    function test_scroll_view_keyboard_scroll() {
        var sv = make(scrollViewComponent, {});
        compare(sv.Accessible.role, Accessible.Pane);
        sv.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_End);
        verify(sv.flickable.contentY > 0, "End must scroll to the bottom");
    }
}
}
