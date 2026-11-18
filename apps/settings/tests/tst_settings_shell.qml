// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Settings app-shell tests (T-09.1a): the window opens with the sidebar and
// local search, search filters the pane list, keyboard navigation and history
// work, the window controls forward to the host, and the AT-SPI roles are
// present. Headless on the offscreen platform + software scene graph.
Item {
    id: stage
    width: 1000
    height: 1400

    TestCase {
        id: testCase
        name: "SettingsShell"
        when: windowShown

        // T263: the shell test opts into window-state persistence so the
        // restore/write path is real. Reset the remembered pane before each
        // case so one case's navigation cannot change the next case's default.
        function init() {
            Settings.set("settings.lastPane", "");
            Settings.set("settings.window.width", 740);
            Settings.set("settings.window.height", 820);
            Settings.set("settings.window.x", 0);
            Settings.set("settings.window.y", 0);
            Settings.set("settings.window.positioned", false);
        }

        Component { id: shellComponent; SettingsShell { } }
        Component { id: windowComponent; SettingsWindow { } }

        SignalSpy { id: closeSpy; signalName: "closeRequested" }
        SignalSpy { id: resizeSpy; signalName: "resizeRequested" }

        function make(props) {
            var merged = { width: stage.width, height: stage.height };
            for (var key in (props || {}))
                merged[key] = props[key];
            var obj = createTemporaryObject(shellComponent, stage, merged);
            waitForRendering(stage);
            return obj;
        }

        function makeSized(width, height) {
            return make({ width: width, height: height });
        }

        function ids(shell) {
            return JSON.stringify(shell.visiblePanes.map(function(pane) {
                return pane.id;
            }));
        }

        // -- Catalog ------------------------------------------------------------

        function test_catalog_is_the_reference_ia() {
            compare(SettingsPanes.catalog[0].id, "wifi");
            compare(SettingsPanes.catalog[SettingsPanes.catalog.length - 1].id,
                    "printers");
            // Only panes whose content has landed are advertised (no half
            // panes). Storage is hidden inside General, so 26 rows remain.
            compare(SettingsPanes.shippedPanes.length, 26);
            compare(SettingsPanes.indexOf("storage"), -1,
                    "Storage must not have a top-level sidebar row");
            compare(SettingsPanes.paneById("storage").shipped, true,
                    "Storage keeps its pane body and deep link");
        }

        // -- Category tiles (T-19.1b) -------------------------------------------

        function test_category_tiles_resolve_and_fall_back() {
            // Every mapping names a vendored fill glyph and carries a gradient.
            var styles = SettingsPanes.categoryStyles;
            var checked = 0;
            for (var icon in styles) {
                var style = styles[icon];
                verify(PhosphorGlyphs.has("fill", style.source),
                       "category " + icon + " -> " + style.source + " must be vendored");
                verify(String(style.gradientStart).length > 0,
                       "category " + icon + " needs a gradientStart");
                verify(String(style.gradientEnd).length > 0,
                       "category " + icon + " needs a gradientEnd");
                checked += 1;
            }
            verify(checked >= 20,
                   "the shipped categories must be mapped (got " + checked + ")");

            // A pane whose icon has no mapping would fall back to Icon.qml;
            // T263 mapped every shipped icon.
            verify(SettingsPanes.categoryStyle(SettingsPanes.paneById("trackpad")) !== null);
            verify(SettingsPanes.categoryStyle(SettingsPanes.paneById("general")) !== null);
            verify(SettingsPanes.categoryStyle(null) === null);

            // De-duplicated category identities: these pairs no longer share
            // one icon (T263).
            var pairs = [["general", "storage"], ["displays", "screen-saver"],
                         ["network", "online-accounts"]];
            for (var p = 0; p < pairs.length; ++p) {
                var left = SettingsPanes.paneById(pairs[p][0]);
                var right = SettingsPanes.paneById(pairs[p][1]);
                verify(left.icon !== right.icon,
                       pairs[p][0] + " and " + pairs[p][1]
                       + " must not share a category icon");
                verify(SettingsPanes.categoryStyle(left) !== null);
                verify(SettingsPanes.categoryStyle(right) !== null);
            }
        }

        function test_sidebar_rows_and_header_adopt_the_category_tile() {
            var shell = make();
            // The pane list is driven by SettingsPanes.filter; each item carries
            // the resolved category style (or null) for the sidebar delegate.
            var visible = shell.visiblePanes;
            verify(SettingsPanes.categoryStyle(visible[0]) !== null,
                   "Bluetooth must carry a category tile");
            // The Appearance pane has no header card but its icon is mapped, so
            // the header (when a header pane is current) resolves the tile too.
            shell.selectPane("general");
            waitForRendering(stage);
            verify(shell.header.visible);
            verify(shell.header.category !== null,
                   "the General header must resolve a category tile");
            compare(shell.header.category.source, "gear");
        }

        function test_header_card_is_only_where_the_reference_has_one() {
            // The Wave-1 captures (Desktop & Dock, Displays, Wallpaper) have
            // no header card; Appearance is uncaptured but follows the same
            // settings-list shape. Only these panes have one.
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("appearance")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("desktop-dock")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("displays")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("wallpaper")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("general")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("accessibility")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("notifications")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("privacy")), true);

            var shell = make();
            shell.selectPane("appearance");
            waitForRendering(stage);
            verify(!shell.header.visible,
                   "the Appearance pane must not draw a header card");
        }

        // -- Shell opens --------------------------------------------------------

        function test_shell_opens_with_sidebar_and_search() {
            var shell = makeSized(740, 820);
            compare(shell.currentPaneId, "general",
                    "first launch opens General");
            compare(shell.visiblePanes.length, 26);
            verify(shell.searchField !== null);
            verify(shell.sidebar !== null);
            verify(shell.titleBar !== null);
            compare(shell.titleBar.style, "unified",
                    "Settings must declare the unified window style");
            compare(shell.titleBar.height,
                    Theme.controls.settingsWindow.chromeHeight,
                    "the detail toolbar is the measured 52 pt chrome");
            compare(shell.sidebarTrafficLights.width,
                    WindowStyle.trafficLightSize * 3
                    + WindowStyle.trafficLightGap() * 2);
            compare(shell.sidebarPane.width,
                    Theme.controls.settingsWindow.sidebarWidth,
                    "the grouped sidebar is the measured 232 pt");
            compare(shell.detailSheet.width,
                    Theme.controls.settingsWindow.detailWidth,
                    "740 - 232 = 508 pt of detail content");
            compare(shell.detailScroll.height, 768,
                    "820 - 52 = 768 pt content viewport");
            compare(shell.header.pane.id, "general");
            // At the start of history both history affordances are dimmed.
            verify(!shell.backButton.enabled);
            verify(!shell.forwardButton.enabled);
        }

        // -- T206 app-local chrome ---------------------------------------------

        function test_first_party_chrome_uses_measured_app_material() {
            var savedScheme = Settings.values["appearance.colorScheme"];
            var savedReduce = Settings.values["accessibility.reduceTransparency"];
            var savedContrast = Settings.values["accessibility.increaseContrast"];
            var shell = make();
            try {
                shell.glassRollout = "replacement";
                shell.appChromeSupported = true;
                Settings.set("appearance.colorScheme", "light");
                Settings.set("accessibility.reduceTransparency", false);
                Settings.set("accessibility.increaseContrast", false);
                tryCompare(Theme, "dark", false);
                waitForRendering(stage);

                compare(shell.appWindow.chromeTransparent, true);
                fuzzyCompare(shell.sidebarPane.color.a, 0.986, 1e-4);
                fuzzyCompare(shell.sidebarPane.color.r, 0.857, 1e-4);
                fuzzyCompare(shell.detailToolbarSheet.color.a, 0.961, 1e-4);
                fuzzyCompare(shell.detailToolbarSheet.color.r, 0.965, 1e-4);
                fuzzyCompare(shell.detailSheet.color.a, 1.0, 1e-6);
                compare(shell.detailSheet.bottomRightRadius,
                        Theme.controls.window.radius);

                // Dark Settings is the explicit Finder-derived adaptation.
                Settings.set("appearance.colorScheme", "dark");
                tryCompare(Theme, "dark", true);
                waitForRendering(stage);
                fuzzyCompare(shell.sidebarPane.color.a, 0.935, 1e-4);
                fuzzyCompare(shell.sidebarPane.color.r, 0.218, 1e-4);
                fuzzyCompare(shell.detailToolbarSheet.color.a, 0.900, 1e-4);
                fuzzyCompare(shell.detailToolbarSheet.color.r, 0.247, 1e-4);

                // Reduce Transparency takes the shared opaque plate.
                Settings.set("accessibility.reduceTransparency", true);
                waitForRendering(stage);
                fuzzyCompare(shell.sidebarPane.color.a, 0.96, 1e-4);
                fuzzyCompare(shell.sidebarPane.color.r, 0.1, 1e-4);

                // A non-composited client keeps the opaque source-list plate.
                shell.appChromeSupported = false;
                waitForRendering(stage);
                fuzzyCompare(shell.sidebarPane.color.a, 1.0, 1e-6);
                fuzzyCompare(shell.sidebarPane.color.r,
                             Theme.color.surfaceMuted.r, 1e-6);
                fuzzyCompare(shell.detailToolbarSheet.color.a, 1.0, 1e-6);
                fuzzyCompare(shell.detailToolbarSheet.color.r,
                             Theme.color.chrome.r, 1e-6);

                // Focus is a controls/text state, not a material change.
                shell.windowActive = false;
                compare(shell.titleBar.active, false);
                shell.appChromeSupported = true;
                Settings.set("accessibility.reduceTransparency", false);
                waitForRendering(stage);
                fuzzyCompare(shell.sidebarPane.color.a, 0.935, 1e-4);
            } finally {
                Settings.set("appearance.colorScheme", savedScheme);
                Settings.set("accessibility.reduceTransparency", savedReduce);
                Settings.set("accessibility.increaseContrast", savedContrast);
                shell.glassRollout = "legacy";
                shell.appChromeSupported = true;
            }
        }

        // -- Search -------------------------------------------------------------

        function test_search_filters_panes() {
            var shell = make();
            shell.searchText = "wall";
            shell.commitSearch();
            compare(ids(shell), JSON.stringify(["wallpaper"]));

            shell.searchText = "dock";
            shell.commitSearch();
            compare(ids(shell), JSON.stringify(["desktop-dock"]));

            shell.searchText = "DISPLAY";
            shell.commitSearch();
            compare(ids(shell), JSON.stringify(["displays"]));

            shell.searchText = "no-such-pane";
            shell.commitSearch();
            compare(ids(shell), JSON.stringify([]));

            shell.searchText = "";
            shell.commitSearch();
            compare(ids(shell), JSON.stringify(
                ["wifi", "bluetooth", "network", "vpn", "battery", "general",
                 "accessibility", "appearance", "desktop-dock", "displays",
                 "menu-bar", "search", "wallpaper", "screen-saver",
                 "notifications", "sound", "focus", "lock-screen", "privacy",
                 "users-groups", "online-accounts", "keyboard", "mouse",
                 "trackpad", "game-controllers", "printers"]));
        }

        function test_search_keeps_the_current_pane() {
            var shell = make();
            shell.searchText = "wall";
            compare(shell.currentPaneId, "general");
            compare(shell.header.pane.id, "general");
        }

        // -- Keyboard navigation + history -------------------------------------

        function test_keyboard_navigates_and_activates() {
            var shell = make();
            shell.sidebar.forceActiveFocus();
            waitForRendering(stage);
            // The default pane is General. The sidebar starts with the update
            // status row (0), Connectivity's five rows, a spacer (6), then
            // General (7).
            compare(shell.sidebar.currentIndex, 7);
            keyClick(Qt.Key_Down);
            // The next row is Accessibility (8); the spacer is skipped.
            compare(shell.sidebar.currentIndex, 8);
            keyClick(Qt.Key_Return);
            compare(shell.currentPaneId, "accessibility");
            verify(shell.canGoBack);
        }

        function test_history_back_and_forward() {
            var shell = make();
            shell.selectPane("displays");
            shell.selectPane("wallpaper");
            compare(shell.currentPaneId, "wallpaper");
            verify(!shell.canGoForward);

            shell.back();
            compare(shell.currentPaneId, "displays");
            verify(shell.canGoBack);
            verify(shell.canGoForward);

            shell.forward();
            compare(shell.currentPaneId, "wallpaper");
            verify(!shell.canGoForward);

            // Forward is dimmed (disabled) at the end of history.
            verify(!shell.forwardButton.enabled);
            verify(shell.backButton.enabled);
        }

        // -- Selection highlight ------------------------------------------------

        function test_selected_sidebar_row_is_accent_tinted() {
            var shell = make();
            // The row highlight animates in over the hover duration; let it
            // settle before sampling pixels.
            wait(Theme.motion.hover.duration + 100);
            waitForRendering(stage);
            var img = grabImage(shell);
            var accent = Theme.color.accent;
            var count = 0;
            for (var y = 0; y < img.height; y += 2) {
                for (var x = 0; x < 240; x += 2) {
                    var c = img.pixel(x, y);
                    if (Math.abs(c.r - accent.r) < 0.02
                            && Math.abs(c.g - accent.g) < 0.02
                            && Math.abs(c.b - accent.b) < 0.02)
                        count += 1;
                }
            }
            verify(count > 20,
                   "the selected pane row must carry the accent highlight (count="
                   + count + " index=" + shell.sidebar.currentIndex + ")");
        }

        // -- Accessibility ------------------------------------------------------

        function test_accessibility_roles_are_present() {
            var shell = make();
            compare(shell.appWindow.Accessible.role, Accessible.Window);
            compare(shell.titleBar.Accessible.role, Accessible.TitleBar);
            compare(shell.searchField.Accessible.role, Accessible.EditableText);
            compare(shell.searchField.Accessible.searchEdit, true);
            compare(shell.sidebar.Accessible.role, Accessible.List);
            compare(shell.header.Accessible.role, Accessible.Grouping);
            compare(shell.header.Accessible.name, "General");
            compare(shell.backButton.Accessible.role, Accessible.Button);
            compare(shell.backButton.Accessible.name, "Back");
        }

        // -- Window-control intent forwarding -----------------------------------

        function test_traffic_lights_forward_close() {
            var shell = make();
            closeSpy.target = shell;
            closeSpy.clear();
            var s = Theme.controls.trafficLights.buttonSize;
            var lights = shell.sidebarTrafficLights;
            mouseClick(lights, s / 2, s / 2);
            compare(closeSpy.count, 1);
        }

        // -- Window-edge resize intent ------------------------------------------

        function test_edges_and_corners_report_resize_intent() {
            var shell = make();
            resizeSpy.target = shell;
            resizeSpy.clear();
            verify(shell.resizeEdges.thickness > 0);
            verify(shell.resizeEdges.corner > 0);

            // All four edges report their matching Qt.Edges flag.
            mouseClick(shell, shell.width / 2, 1);
            compare(resizeSpy.count, 1);
            compare(resizeSpy.signalArguments[0][0], Qt.TopEdge);

            mouseClick(shell, shell.width / 2, shell.height - 1);
            compare(resizeSpy.count, 2);
            compare(resizeSpy.signalArguments[1][0], Qt.BottomEdge);

            mouseClick(shell, 1, shell.height / 2);
            compare(resizeSpy.count, 3);
            compare(resizeSpy.signalArguments[2][0], Qt.LeftEdge);

            mouseClick(shell, shell.width - 1, shell.height / 2);
            compare(resizeSpy.count, 4);
            compare(resizeSpy.signalArguments[3][0], Qt.RightEdge);

            // A corner resizes both axes at once.
            mouseClick(shell, shell.width - 1, shell.height - 1);
            compare(resizeSpy.count, 5);
            compare(resizeSpy.signalArguments[4][0],
                    Qt.BottomEdge | Qt.RightEdge);

            // A press in the middle of the content is not a resize.
            mouseClick(shell, shell.width / 2, shell.height / 2);
            compare(resizeSpy.count, 5);
        }

        // -- T263 measured window, grouped sidebar, aliases -------------------

        function sectionIds(shell, index) {
            return shell.sidebarSections[index].items.filter(function(item) {
                return item.spacer !== true;
            }).map(function(item) {
                return item.id;
            }).join(",");
        }

        function firstPaneId(shell, index) {
            var items = shell.sidebarSections[index].items;
            for (var i = 0; i < items.length; ++i) {
                if (items[i].spacer !== true && items[i].status !== true)
                    return items[i].id;
            }
            return "";
        }

        function test_grouped_sidebar_has_three_logical_groups() {
            compare(SettingsPanes.groups.length, 3);
            compare(SettingsPanes.groups[0].title, "");
            compare(SettingsPanes.groups[1].title, "");
            compare(SettingsPanes.groups[2].title, "");
            compare(SettingsPanes.groups[0].accessibleTitle, "Connectivity");
            compare(SettingsPanes.groups[1].accessibleTitle, "Desktop");
            compare(SettingsPanes.groups[2].accessibleTitle, "Input");

            var shell = make();
            compare(shell.sidebarSections.length, 3);
            compare(sectionIds(shell, 0),
                    "general/software-update,wifi,bluetooth,network,vpn,battery");
            compare(sectionIds(shell, 1),
                    "general,accessibility,appearance,desktop-dock,displays,"
                    + "menu-bar,search,wallpaper,screen-saver");
            compare(sectionIds(shell, 2),
                    "notifications,sound,focus,lock-screen,privacy,"
                    + "users-groups,online-accounts,keyboard,mouse,trackpad,"
                    + "game-controllers,printers");
        }

        function test_measured_sizes_and_no_clipping() {
            var sizes = [[740, 820], [800, 640], [720, 480]];
            for (var i = 0; i < sizes.length; ++i) {
                var width = sizes[i][0];
                var height = sizes[i][1];
                var shell = makeSized(width, height);

                compare(shell.sidebarPane.width, 232,
                        "the sidebar is measured, not the 192 token");
                compare(shell.detailSheet.width, width - 232);
                compare(shell.detailContent.x, 30,
                        "the measured horizontal content inset");
                compare(shell.detailContent.width, width - 232 - 60);
                compare(shell.titleBar.width, width - 232);
                compare(shell.titleBar.height, 52);
                compare(shell.titleBar.mapToItem(shell, 0, 0).x, 232,
                        "the detail toolbar starts at the sidebar split");
                compare(shell.detailScroll.height, height - 52,
                        "820 - 52 = 768 pt content viewport");
                verify(shell.header.visible,
                       "General keeps its reference header at " + width + "x" + height);
                compare(shell.header.width, shell.detailContent.width);

                // The title is centred over the detail pane, not the window.
                var titleCenter = shell.titleBar.mapToItem(
                    shell, shell.titleBar.width / 2, 0).x;
                verify(titleCenter > width / 2,
                       "the title must sit over the detail pane at "
                       + width + "x" + height);
                compare(shell.titleBar.title, "General");

                // The grouped list scrolls instead of overflowing the card.
                verify(shell.sidebar.scrollable,
                       "the sidebar must scroll at " + width + "x" + height);
                verify(shell.sidebar.flickable.contentWidth
                       <= shell.sidebar.flickable.width,
                       "no sidebar row may be wider than the card");
                shell.destroy();
            }
        }

        function test_storage_is_general_storage_and_the_link_still_lands() {
            var shell = makeSized(740, 820);
            compare(SettingsPanes.indexOf("storage"), -1,
                    "Storage has no top-level sidebar row");
            verify(shell.deepLink("settings://pane/storage"));
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready;
            });
            tryVerify(function() {
                return shell.paneBody.item.subPaneId === "storage";
            });
            compare(shell.currentPaneId, "general");
            compare(shell.paneTitle, "Storage");
            compare(shell.windowTitle, "Storage");
            compare(shell.titleBar.title, "Storage");
            verify(!shell.header.visible,
                   "a General sub-pane replaces the header with its back chrome");

            // The General row opens the same in-place surface.
            shell.paneBody.item.closeSubPane();
            shell.paneBody.item.openSubPane("storage");
            tryVerify(function() {
                return shell.paneBody.item.subPaneId === "storage";
            });
        }

        function test_every_sidebar_row_opens_and_no_removed_pane_appears() {
            var shell = makeSized(740, 1400);
            var removedIds = SettingsPanes.removed.map(function(entry) {
                return entry.id;
            });
            for (var s = 0; s < shell.sidebarSections.length; ++s) {
                var items = shell.sidebarSections[s].items;
                for (var i = 0; i < items.length; ++i) {
                    var item = items[i];
                    if (item.spacer === true)
                        continue;
                    shell.activateSidebarItem(item.id);
                    if (item.status === true) {
                        tryVerify(function() {
                            return shell.currentPaneId === "general";
                        });
                    } else {
                        tryVerify(function() {
                            return shell.currentPaneId === item.id;
                        });
                    }
                    compare(removedIds.indexOf(shell.currentPaneId), -1,
                            "a removed pane must never open from the sidebar");
                }
            }
        }

        function test_status_row_opens_general_software_update() {
            var shell = makeSized(740, 820);
            verify(SettingsPanes.resolve("general/software-update") !== null);
            compare(SettingsPanes.resolve("general/software-update").paneId,
                    "general");
            compare(SettingsPanes.resolve("general/software-update").subPaneId,
                    "software-update");
            shell.activateSidebarItem("general/software-update");
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready;
            });
            tryVerify(function() {
                return shell.paneBody.item.subPaneId === "software-update";
            });
            compare(shell.currentPaneId, "general");
            compare(shell.paneTitle, "Software Update");
        }

        function test_moved_panes_land_on_their_new_home() {
            var shell = makeSized(740, 820);
            verify(shell.deepLink("settings://pane/control-center"));
            tryVerify(function() { return shell.currentPaneId === "menu-bar"; });
            compare(SettingsPanes.resolve("control-center").subPaneId,
                    "controls");

            verify(shell.deepLink("settings://pane/overview"));
            tryVerify(function() {
                return shell.currentPaneId === "desktop-dock";
            });
            compare(SettingsPanes.resolve("overview").anchor,
                    "Workspace Overview");

            verify(!shell.deepLink("settings://pane/biometrics"),
                   "the removed Biometrics pane must be refused");
            verify(shell.lastDeepLinkError.length > 0);
        }

        function test_online_accounts_is_always_listed() {
            var shell = make();
            var ids = shell.visiblePanes.map(function(pane) { return pane.id; });
            verify(ids.indexOf("online-accounts") >= 0,
                   "Online Accounts must stay in the sidebar when GOA is absent");
            compare(SettingsPanes.paneById("online-accounts").optional,
                    undefined);
        }

        function test_vpn_is_always_visible() {
            var vpn = SettingsPanes.paneById("vpn");
            verify(vpn !== null);
            compare(vpn.optional, undefined,
                    "T285 removed the optional-profile visibility gate");
            compare(vpn.shipped, true);

            var shell = make();
            var ids = shell.visiblePanes.map(function(pane) { return pane.id; });
            verify(ids.indexOf("vpn") >= 0,
                   "VPN is part of the 27-entry IA without a profile");
            verify(SettingsPanes.resolve("vpn").paneId === "vpn",
                   "VPN resolves to itself");
        }

        function test_last_pane_restores_and_a_deep_link_wins() {
            Settings.set("settings.lastPane", "displays");
            var shell = makeSized(740, 820);
            compare(shell.currentPaneId, "displays",
                    "the last visited pane is restored");

            verify(shell.deepLink("settings://pane/storage"));
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready;
            });
            tryVerify(function() {
                return shell.paneBody.item.subPaneId === "storage";
            });
            compare(shell.currentPaneId, "general",
                    "a deeplink wins over the restored pane");
            compare(Settings.values["settings.lastPane"], "general",
                    "the last pane follows navigation");
        }

        function test_stationary_first_click_selects_each_group() {
            // A tall shell keeps every group visible, so each first click is
            // made with no scroll and no prior pointer motion.
            var shell = makeSized(740, 1400);
            for (var g = 0; g < shell.sidebarSections.length; ++g) {
                var id = firstPaneId(shell, g);
                var index = shell.sidebarIndexOf(id);
                var row = shell.sidebar.rowAtIndex(index);
                verify(row !== null, "row for " + id);
                mouseClick(row, 20, row.height / 2);
                compare(shell.currentPaneId, id,
                        "the first stationary click must select " + id);
            }
        }

        function test_sidebar_keyboard_skips_spacers() {
            var shell = make();
            var spacerIndex = -1;
            for (var i = 0; i < shell.sidebar.entries.length; ++i) {
                if (shell.sidebar.entries[i].type === "spacer") {
                    spacerIndex = i;
                    break;
                }
            }
            verify(spacerIndex >= 0, "the sidebar must render its group spacer");
            shell.sidebar.currentIndex = spacerIndex - 1;
            shell.sidebar.move(1);
            verify(shell.sidebar.currentIndex !== spacerIndex,
                   "keyboard navigation must skip the spacer");
            compare(shell.sidebar.entries[shell.sidebar.currentIndex].type,
                    "item");
            // A spacer index is never a pane selection.
            compare(shell.sidebarIdAtIndex(spacerIndex), "");
        }

        function test_stationary_click_after_scroll_hits_the_row_under_the_pointer() {
            var shell = makeSized(720, 480);
            // The last sidebar row in the reduced IA is Printers & Scanners.
            var id = "printers";
            var index = shell.sidebarIndexOf(id);
            var row = shell.sidebar.rowAtIndex(index);
            verify(row !== null);
            // Scroll under a stationary pointer, then click the point where the
            // last row now sits. No motion event precedes the click.
            shell.sidebar.flickable.contentY =
                Math.max(0, shell.sidebar.flickable.contentHeight
                            - shell.sidebar.flickable.height);
            waitForRendering(stage);
            var point = row.mapToItem(shell, 20, row.height / 2);
            mouseClick(shell, point.x, point.y);
            compare(shell.currentPaneId, id,
                    "the click must hit the row now under the pointer");
        }

        function test_every_sidebar_group_switches_at_the_minimum() {
            var shell = makeSized(720, 480);
            for (var g = 0; g < shell.sidebarSections.length; ++g) {
                var section = shell.sidebarSections[g];
                var id = firstPaneId(shell, g);
                shell.activateSidebarItem(id);
                tryVerify(function() {
                    return shell.paneBody.status === Loader.Ready;
                });
                waitForRendering(stage);
                compare(shell.titleBar.title, shell.paneTitle,
                        "the title follows the pane in group "
                        + section.accessibleTitle);
                verify(shell.detailContent.width > 0);
                compare(shell.titleBar.width, 720 - 232);
                verify(shell.sidebar.scrollable);
            }
        }

        function test_window_geometry_restores_and_persists() {
            Settings.set("settings.window.width", 800);
            Settings.set("settings.window.height", 640);
            var win = createTemporaryObject(windowComponent, null, {});
            verify(win !== null);
            tryCompare(win, "width", 800);
            tryCompare(win, "height", 640);
            compare(win.title, "General");

            win.width = 760;
            win.height = 600;
            tryVerify(function() {
                return Number(Settings.values["settings.window.width"]) === 760
                    && Number(Settings.values["settings.window.height"]) === 600;
            });
            win.close();
        }

        // -- T264: sub-pane history, focus/scroll, About geometry, debug view --

        function ready(shell) {
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
        }

        function test_history_includes_general_subpanes() {
            var shell = make();
            ready(shell);
            shell.openSubPane("about");
            tryVerify(function() { return shell.currentSubPaneId === "about"; });
            verify(shell.canGoBack, "opening About is a history step");

            // Back closes the sub-pane before it changes pane.
            shell.back();
            tryVerify(function() { return shell.currentSubPaneId === ""; });
            compare(shell.currentPaneId, "general");
            verify(shell.canGoForward);

            // Forward restores it.
            shell.forward();
            tryVerify(function() { return shell.currentSubPaneId === "about"; });
            compare(shell.currentPaneId, "general");
        }

        function test_back_forward_restore_a_subpane_across_panes() {
            var shell = make();
            ready(shell);
            shell.selectPane("displays");
            ready(shell);
            shell.selectPane("general");
            ready(shell);
            shell.openSubPane("language");
            tryVerify(function() { return shell.currentSubPaneId === "language"; });
            shell.selectPane("displays");
            ready(shell);
            compare(shell.currentPaneId, "displays");

            shell.back();
            compare(shell.currentPaneId, "general");
            tryVerify(function() { return shell.currentSubPaneId === "language"; });

            shell.back();
            compare(shell.currentPaneId, "general");
            tryVerify(function() { return shell.currentSubPaneId === ""; });

            shell.forward();
            tryVerify(function() { return shell.currentSubPaneId === "language"; });
        }

        function test_reselecting_the_general_row_closes_its_subpane() {
            var shell = make();
            ready(shell);
            shell.openSubPane("about");
            tryVerify(function() { return shell.currentSubPaneId === "about"; });
            shell.activateSidebarItem("general");
            tryVerify(function() { return shell.currentSubPaneId === ""; });
            compare(shell.currentPaneId, "general");
        }

        function test_pane_change_resets_scroll_and_focus() {
            var shell = makeSized(740, 820);
            ready(shell);
            shell.selectPane("desktop-dock");
            ready(shell);
            shell.detailScroll.flickable.contentY = 80;
            shell.selectPane("displays");
            ready(shell);
            compare(shell.detailScroll.flickable.contentY, 0,
                    "a pane change starts at the top of the detail view");
            var target = shell.firstFocusable(shell.paneBody.item);
            verify(target !== null, "the new pane must expose a keyboard stop");
            tryVerify(function() { return target.activeFocus; });
        }

        function test_debug_view_fits_the_minimum_window_and_follows_the_pane() {
            var shell = makeSized(720, 480);
            ready(shell);
            shell.showDebugView("general");
            tryVerify(function() { return shell.debugView.open; });
            waitForRendering(stage);
            verify(shell.debugView.card.width <= 720,
                   "the debug card must fit 720 pt (got "
                   + shell.debugView.card.width + ")");
            verify(shell.debugView.card.height <= 480,
                   "the debug card must fit 480 pt (got "
                   + shell.debugView.card.height + ")");
            verify(shell.debugView.rowList.height > 0);
            verify(shell.debugView.rowList.contentHeight
                   > shell.debugView.rowList.height,
                   "the row list must scroll at the minimum window");

            shell.selectPane("displays");
            tryVerify(function() { return shell.debugView.paneId === "displays"; });
            compare(shell.debugView.paneSelect.currentIndex,
                    SettingsMap.paneIndex("displays"),
                    "the selector follows the shell's pane");
        }

        function test_about_page_restores_the_pre_about_geometry() {
            var win = createTemporaryObject(windowComponent, null, {});
            verify(win !== null);
            win.width = 900;
            win.height = 700;
            tryCompare(win, "width", 900);
            win.settingsShell.openAboutPage();
            tryVerify(function() { return win.settingsShell.aboutOpen; });

            // The About surface claims the frame; closing it must hand the
            // user's geometry back, not the 740x820 design default.
            win.width = 760;
            win.height = 640;
            win.settingsShell.closeSubPane();
            tryVerify(function() { return !win.settingsShell.aboutOpen; });
            tryCompare(win, "width", 900);
            tryCompare(win, "height", 700);
            win.close();
        }

        function test_about_panel_restores_the_pre_about_geometry() {
            var win = createTemporaryObject(windowComponent, null, {});
            verify(win !== null);
            win.width = 900;
            win.height = 700;
            tryCompare(win, "width", 900);
            win.showAboutPanel();
            tryVerify(function() { return win.aboutWindow.visible; });
            win.width = 760;
            win.height = 640;
            win.aboutWindow.close();
            tryVerify(function() { return !win.aboutWindow.visible; });
            tryCompare(win, "width", 900);
            tryCompare(win, "height", 700);
            win.close();
        }
    }
}