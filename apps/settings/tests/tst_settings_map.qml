// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// The executable Settings business-logic map (T222).
//
// `docs/design/settings-business-logic-map.md` is the human contract and
// `SettingsMap.qml` is its runtime projection. These cases prove the catalog
// and the map agree pane-for-pane, every visible row has an owner/key/absence
// path, an unowned row is rejected, omitted panes are absent from the sidebar
// and documented, and the debug view exposes the current pane's owners.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsMap"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            return shell;
        }

        function ids(list) {
            return list.map(function(entry) { return entry.id; }).join(",");
        }

        // -- The catalog and the map agree ------------------------------------

        function test_catalog_and_map_agree_pane_for_pane() {
            compare(SettingsPanes.catalog.length, SettingsMap.panes.length,
                    "the catalog and map must list the same panes");
            for (var i = 0; i < SettingsPanes.catalog.length; ++i) {
                var pane = SettingsPanes.catalog[i];
                var mapped = SettingsMap.panes[i];
                compare(pane.id, mapped.id, "pane id at index " + i);
                compare(pane.title, mapped.title, "pane title for " + pane.id);
                compare(pane.icon, mapped.icon, "pane icon for " + pane.id);
                compare(pane.group, mapped.group, "group for " + pane.id);
                compare(pane.shipped, mapped.shipped, "shipped flag for " + pane.id);
                compare(pane.owner, mapped.owner, "owner for " + pane.id);
                compare(pane.deepLink, mapped.deepLink, "deep link for " + pane.id);
                compare(pane.absence, mapped.absence, "absence for " + pane.id);
            }
        }

        function test_pane_ids_and_deep_links_are_unique() {
            var seenIds = {};
            var seenLinks = {};
            for (var i = 0; i < SettingsMap.panes.length; ++i) {
                var pane = SettingsMap.panes[i];
                verify(seenIds[pane.id] === undefined, "duplicate pane id " + pane.id);
                verify(seenLinks[pane.deepLink] === undefined,
                       "duplicate deep link " + pane.deepLink);
                seenIds[pane.id] = true;
                seenLinks[pane.deepLink] = true;
                compare(pane.deepLink, "settings://pane/" + pane.id);
            }
        }

        // -- The no-half-panes rule -------------------------------------------

        function test_every_shipped_pane_has_a_body_and_no_other_does() {
            var shell = make();
            for (var i = 0; i < SettingsPanes.catalog.length; ++i) {
                var pane = SettingsPanes.catalog[i];
                var body = shell.paneComponent(pane.id);
                if (pane.shipped)
                    verify(body !== null, "shipped pane " + pane.id + " must have a body");
                else
                    verify(body === null, "unshipped pane " + pane.id
                           + " must not advertise a body");
            }
        }

        function test_shipped_panes_have_shipped_rows() {
            for (var i = 0; i < SettingsMap.panes.length; ++i) {
                var pane = SettingsMap.panes[i];
                if (!pane.shipped)
                    continue;
                verify(SettingsMap.visibleRowsFor(pane.id).length > 0,
                       "shipped pane " + pane.id + " must expose at least one visible row");
            }
        }

        // T290b: the Appearance mode row is three visual tiles, not the old
        // segmented control. The section/label/key stay as the map contract.
        function test_appearance_scheme_row_is_a_tile_control() {
            var row = SettingsMap.rowFor("appearance", "scheme");
            verify(row !== null);
            compare(row.control, "tiles");
            compare(row.section, "Appearance");
            compare(row.label, "Appearance");
            compare(row.key, "appearance.colorScheme");
            compare(row.state, "shipped");
        }

        // T290c: the transparency slider is the Glass tint row in the Glass
        // section; the settings key and slider control are unchanged.
        function test_appearance_transparency_row_is_the_glass_tint() {
            var row = SettingsMap.rowFor("appearance", "transparency");
            verify(row !== null);
            compare(row.section, "Glass");
            compare(row.label, "Glass tint");
            compare(row.control, "slider");
            compare(row.key, "appearance.transparency");
            compare(row.state, "shipped");
            compare(SettingsMap.rowsFor("appearance").length > 0, true);
        }

        // T290d/T290e: the accent, Text highlight, icon style, and folder
        // color rows are one Theme section; the sidebar icon size and
        // wallpaper tint rows moved to Windows, and the scroll-bar rows are
        // their own mockup sections with radio controls.
        function test_appearance_theme_and_windows_rows_use_their_sections() {
            var accent = SettingsMap.rowFor("appearance", "accent");
            verify(accent !== null);
            compare(accent.section, "Theme");
            compare(accent.label, "Accent color");
            compare(accent.control, "swatches");
            compare(accent.key, "appearance.accent");

            var highlight = SettingsMap.rowFor("appearance", "highlight");
            verify(highlight !== null);
            compare(highlight.section, "Theme");
            compare(highlight.label, "Text highlight color");
            compare(highlight.control, "popup");
            compare(highlight.key, "appearance.highlight");

            var iconStyle = SettingsMap.rowFor("appearance", "iconStyle");
            verify(iconStyle !== null);
            compare(iconStyle.section, "Theme");
            compare(iconStyle.label, "Icon & widget style");
            compare(iconStyle.control, "tiles");
            compare(iconStyle.key, "appearance.iconStyle");
            compare(iconStyle.state, "shipped");

            var folderColor = SettingsMap.rowFor("appearance", "folderColor");
            verify(folderColor !== null);
            compare(folderColor.section, "Theme");
            compare(folderColor.label, "Folder color");
            compare(folderColor.control, "popup");
            compare(folderColor.key, "appearance.folderColor");

            var sidebar = SettingsMap.rowFor("appearance", "sidebarIconSize");
            verify(sidebar !== null);
            compare(sidebar.section, "Windows");
            compare(sidebar.label, "Sidebar icon size");
            compare(sidebar.control, "popup");
            compare(sidebar.key, "appearance.sidebarIconSize");

            var tint = SettingsMap.rowFor("appearance", "wallpaperTint");
            verify(tint !== null);
            compare(tint.section, "Windows");
            compare(tint.label, "Tint window background with wallpaper color");
            compare(tint.control, "toggle");
            compare(tint.key, "appearance.wallpaperTint");

            var scrollBars = SettingsMap.rowFor("appearance", "scrollBars");
            verify(scrollBars !== null);
            compare(scrollBars.section, "Show scroll bars");
            compare(scrollBars.label, "Show scroll bars");
            compare(scrollBars.control, "radio");
            compare(scrollBars.key, "appearance.scrollBars");

            var scrollClick = SettingsMap.rowFor("appearance", "scrollBarClick");
            verify(scrollClick !== null);
            compare(scrollClick.section, "Click in the scroll bar to");
            compare(scrollClick.label, "Click in the scroll bar to");
            compare(scrollClick.control, "radio");
            compare(scrollClick.key, "appearance.scrollBarClick");

            // T290e: the Motion row left the pane; Accessibility owns the key.
            compare(SettingsMap.rowFor("appearance", "motion"), null);

            // The old section names are gone from the map, not duplicated.
            var rows = SettingsMap.rowsFor("appearance");
            for (var i = 0; i < rows.length; ++i) {
                verify(rows[i].section !== "Color");
                verify(rows[i].section !== "Icon & widget style");
                verify(rows[i].section !== "Windows and scroll bars");
                verify(rows[i].section !== "Desktop integration");
            }
        }

        // T290f: the six Linux-only Appearance rows live in the Advanced
        // Appearance sheet; the map answers "dialog" for exactly those ids and
        // every other appearance row stays on the pane surface.
        function test_appearance_advanced_rows_are_dialog_rows() {
            var cases = [
                { id: "iconTheme", label: "Icon theme", control: "popup",
                  key: "appearance.iconTheme" },
                { id: "cursorTheme", label: "Cursor theme", control: "popup",
                  key: "appearance.cursorTheme" },
                { id: "cursorSize", label: "Cursor size", control: "slider",
                  key: "appearance.cursorSize" },
                { id: "windowControls", label: "Window control layout",
                  control: "popup", key: "appearance.windowControls" },
                { id: "material", label: "Material effects", control: "popup",
                  key: "appearance.material" },
                { id: "accentFromWallpaper", label: "Accent from wallpaper",
                  control: "toggle", key: "appearance.accentFromWallpaper" }
            ];
            for (var i = 0; i < cases.length; ++i) {
                var row = SettingsMap.rowFor("appearance", cases[i].id);
                verify(row !== null, cases[i].id + " must be in the map");
                compare(row.section, "Advanced Appearance");
                compare(row.label, cases[i].label);
                compare(row.control, cases[i].control);
                compare(row.key, cases[i].key);
                compare(row.state, "shipped");
                compare(SettingsMap.rowSurface("appearance", cases[i].id),
                        "dialog");
                verify(SettingsMap.isRowLandable("appearance", cases[i].id),
                       cases[i].id + " must be landable");
            }

            var dialogIds = cases.map(function(entry) { return entry.id; });
            var rows = SettingsMap.rowsFor("appearance");
            var seen = {};
            for (var r = 0; r < rows.length; ++r) {
                verify(seen[rows[r].id] === undefined,
                       "duplicate appearance row " + rows[r].id);
                seen[rows[r].id] = true;
                var surface = SettingsMap.rowSurface("appearance", rows[r].id);
                if (dialogIds.indexOf(rows[r].id) >= 0) {
                    compare(surface, "dialog");
                } else {
                    compare(surface, "pane",
                            rows[r].id + " must stay on the pane surface");
                }
                verify(SettingsMap.isRowLandable("appearance", rows[r].id),
                       rows[r].id + " must be landable");
            }
            compare(rows.length, 16);
        }

        // T293a: the Menu Bar rows follow the mockup order; the three
        // Linux-only rows live in the Advanced Menu Bar sheet; the omitted
        // mockup rows carry their reason and are never landable; and the
        // Quick Settings Controls… action lands on the alias sub-pane.
        function test_menu_bar_rows_follow_the_mockup_order() {
            var rows = SettingsMap.rowsFor("menu-bar");
            // The three footer actions live in the Menu Bar Controls group
            // and follow the Allow section, so they form a second block with
            // the same section title.
            var sections = [];
            for (var i = 0; i < rows.length; ++i) {
                if (sections[sections.length - 1] !== rows[i].section)
                    sections.push(rows[i].section);
            }
            // T293b: the footer actions follow the Allow block under the
            // Menu Bar Controls title, then the in-place sub-pane's own rows
            // (same title, `controls` surface), then the Advanced sheet.
            compare(sections.join("|"),
                    "Menu Bar Behaviour|OS Menu Glyph|Menu Bar Controls|"
                    + "Allow in the Menu Bar|Menu Bar Controls|Allow in the Menu Bar|"
                    + "Menu Bar Controls|Advanced Menu Bar");

            var glyph = SettingsMap.rowFor("menu-bar", "osGlyph");
            verify(glyph !== null);
            compare(glyph.section, "OS Menu Glyph");
            compare(glyph.control, "radio");
            compare(glyph.key, "menu.osGlyph");
            compare(SettingsMap.rowSurface("menu-bar", "osGlyph"), "pane");

            // The controls list uses the mockup labels, not the internal
            // names. T293b: the list is the status-item section (its rows own
            // `menu.control.*`); the moved Quick Settings rows reuse the same
            // ids on the `controls` surface and are filtered out here, then
            // asserted below.
            // Some ids exist on both surfaces: the status item here and the
            // moved control on `controls`. They are disambiguated by key —
            // the sub-pane rows write `quickSettings.*` (its Wi-Fi etc. are
            // `quickSettings.modules`) — so the list keeps exactly the rows
            // that render on this pane.
            var labels = SettingsMap.rowsFor("menu-bar")
                .filter(function(row) {
                    return row.section === "Menu Bar Controls"
                            // A moved id is the sub-pane one only when its
                            // key is a `quickSettings.*` key (the status
                            // items own `menu.control.*` / `menu.clock.*`).
                            && String(row.key).indexOf("quickSettings.") !== 0
                            && row.control !== "omit"
                            && row.id !== "addControls"
                            && row.id !== "customizeControls"
                            && row.id !== "reset";
                }).map(function(row) { return row.label; }).join(",");
            // `controlCenter` renders on the pane inside the Allow section
            // (`rowSurface` is "pane"), so it is not in this list; the Allow
            // section's own literal is asserted below.
            compare(labels, "Clock,Search,Wi-Fi,Bluetooth,Battery,Focus,Sound,"
                            + "Accessibility,Display,Workspaces,"
                            + "Now Playing,Text Input,Backups,Timer");
            var cc = SettingsMap.rowFor("menu-bar", "controlCenter");
            verify(cc !== null);
            compare(cc.label, "Quick Settings");
            compare(cc.section, "Allow in the Menu Bar");
            compare(SettingsMap.rowSurface("menu-bar", "controlCenter"), "pane");
            // The sub-pane rows carry the same title but a different surface.
            compare(SettingsMap.subPaneForRow("menu-bar", "pages"), "controls");
            compare(SettingsMap.subPaneForRow("menu-bar", "controlCenter"), "");
            // No top-level Quick Settings row list and no live Screen Casting
            // row: the latter is a recorded omission on the sub-pane.
            compare(SettingsMap.rowsFor("control-center").length, 0);
            var casting = SettingsMap.rowFor("menu-bar", "screenMirroring");
            verify(casting !== null);
            compare(casting.state, "omitted");
            compare(casting.reason.length > 0, true);

            var advanced = ["overflow", "globalMenu", "perDisplayPanel"];
            for (var a = 0; a < advanced.length; ++a) {
                var row = SettingsMap.rowFor("menu-bar", advanced[a]);
                verify(row !== null, advanced[a] + " must stay in the map");
                compare(row.section, "Advanced Menu Bar");
                compare(SettingsMap.rowSurface("menu-bar", advanced[a]),
                        "dialog");
                verify(SettingsMap.isRowLandable("menu-bar", advanced[a]));
                verify(row.owner.length > 0 && row.key.length > 0);
            }

            var omitted = ["suggestions", "nowPlaying", "textInput", "backups",
                           "timer"];
            for (var o = 0; o < omitted.length; ++o) {
                var gone = SettingsMap.rowFor("menu-bar", omitted[o]);
                verify(gone !== null, omitted[o] + " must be recorded");
                compare(gone.state, "omitted");
                compare(gone.label.length > 0, true);
                compare(gone.reason.length > 0, true);
                compare(SettingsMap.isRowLandable("menu-bar", omitted[o]),
                        false);
            }
            // Display is an omitted row that still renders its exact reason.
            var display = SettingsMap.rowFor("menu-bar", "display");
            compare(display.state, "omitted");
            verify(display.reason.indexOf("Quick Settings") >= 0);
        }

        // T293b: every former Quick Settings row now lives under `menu-bar`
        // as the in-place `controls` sub-pane, and the aliases land there.
        function test_menu_bar_controls_rows_are_landable() {
            var rows = SettingsMap.rowsFor("menu-bar");
            var controls = 0;
            for (var i = 0; i < rows.length; ++i) {
                if (SettingsMap.subPaneForRow("menu-bar", rows[i].id) !== "controls")
                    continue;
                controls += 1;
                if (rows[i].state !== "shipped")
                    continue;
                compare(SettingsMap.rowSurface("menu-bar", rows[i].id),
                        "subpane", rows[i].id);
            }
            verify(controls >= 18, "the sub-pane carries the moved rows");
            // No top-level Quick Settings row list survives.
            compare(SettingsMap.rowsFor("control-center").length, 0);
            var shell = make();
            verify(shell.deepLink("settings://pane/menu-bar"));
            tryVerify(function() { return shell.currentPaneId === "menu-bar"; });
            compare(shell.currentSubPaneId, "");
            verify(shell.deepLink("settings://pane/quick-settings"));
            tryVerify(function() { return shell.currentSubPaneId === "controls"; });
            compare(shell.currentPaneId, "menu-bar");
            compare(shell.paneTitle, "Menu Bar Controls");
        }

        // -- Every visible row has an owner/key/absence path -------------------

        function test_every_visible_row_has_an_owner() {
            compare(SettingsMap.validate().join("; "), "",
                    "the executable map must validate cleanly");
            for (var i = 0; i < SettingsMap.panes.length; ++i) {
                var pane = SettingsMap.panes[i];
                var rows = SettingsMap.rowsFor(pane.id);
                verify(rows.length > 0, "pane " + pane.id + " must have row metadata");
                for (var r = 0; r < rows.length; ++r) {
                    var row = rows[r];
                    if (row.state !== "shipped")
                        continue;
                    verify(row.owner.length > 0 && row.owner !== "omit",
                           "visible row " + pane.id + "/" + row.id + " has no owner");
                    verify(row.key.length > 0 && row.key !== "omit",
                           "visible row " + pane.id + "/" + row.id + " has no key/API");
                    verify(SettingsMap.absenceKinds.indexOf(row.absence) >= 0,
                           "visible row " + pane.id + "/" + row.id
                           + " has an unknown absence path");
                }
            }
        }

        // T271: a durable preference with no consumer is marked stored-only.
        function test_stored_only_rows_are_labeled() {
            var ask = SettingsMap.rowFor("wifi", "ask");
            verify(ask !== null);
            compare(ask.storedOnly, true);
            var hotspots = SettingsMap.rowFor("wifi", "askHotspots");
            verify(hotspots !== null);
            compare(hotspots.storedOnly, true);
            var power = SettingsMap.rowFor("wifi", "power");
            verify(power !== null);
            compare(power.storedOnly, false);
        }

        // -- T281: the Linux-specific audit -----------------------------------

        function test_linux_specific_additions_are_audited_and_unique() {
            verify(SettingsMap.linuxAdditions.length >= 40,
                   "the Linux-specific audit must cover every §5 area");
            compare(SettingsMap.validateLinuxAdditions().join("; "), "",
                    "the Linux-specific audit must match the executable rows");

            var keys = {};
            for (var i = 0; i < SettingsMap.linuxAdditions.length; ++i) {
                var entry = SettingsMap.linuxAdditions[i];
                verify(entry.setting.length > 0);
                verify(entry.task.length > 0);
                compare(SettingsMap.pane(entry.pane) !== null, true,
                        entry.setting + " must name a real pane");
                for (var r = 0; r < entry.rows.length; ++r) {
                    var row = SettingsMap.rowFor(entry.pane, entry.rows[r]);
                    verify(row !== null, entry.setting + " names a missing row");
                    verify(keys[row.key] === undefined,
                           "linux additions must not duplicate key " + row.key);
                    keys[row.key] = true;
                }
            }
        }

        function test_linux_specific_live_rows_have_consumers_or_stored_only() {
            for (var i = 0; i < SettingsMap.linuxAdditions.length; ++i) {
                var entry = SettingsMap.linuxAdditions[i];
                for (var r = 0; r < entry.rows.length; ++r) {
                    var row = SettingsMap.rowFor(entry.pane, entry.rows[r]);
                    verify(row.consumer.length > 0,
                           entry.pane + "/" + row.id + " must record a consumer");
                    if (row.storedOnly) {
                        compare(row.consumer.indexOf("stored-only"), 0,
                                row.id + " is stored-only and must say so");
                        verify(row.absence !== "disabled-with-reason",
                               row.id + " cannot be both stored-only and disabled");
                    } else if (row.absence === "live-on-defaults") {
                        verify(row.consumer.indexOf("stored-only") !== 0,
                               row.id + " is live and must not claim stored-only");
                    }
                }
            }
        }

        function test_linux_specific_rows_live_in_macos_familiar_panes() {
            for (var i = 0; i < SettingsMap.linuxAdditions.length; ++i) {
                var entry = SettingsMap.linuxAdditions[i];
                var pane = SettingsMap.pane(entry.pane);
                verify(String(pane.title).toLowerCase().indexOf("linux") < 0,
                       "Linux rows must not create a separate Linux pane");
                verify(String(pane.id).toLowerCase().indexOf("linux") < 0,
                       "Linux rows must not create a separate Linux pane");
            }
        }

        function test_linux_specific_labels_are_user_facing() {
            var banned = ["OPEN", "T2", "stored-only", "consumer", "settingsd"];
            for (var i = 0; i < SettingsMap.linuxAdditions.length; ++i) {
                var entry = SettingsMap.linuxAdditions[i];
                for (var r = 0; r < entry.rows.length; ++r) {
                    var row = SettingsMap.rowFor(entry.pane, entry.rows[r]);
                    for (var b = 0; b < banned.length; ++b) {
                        verify(row.label.indexOf(banned[b]) < 0,
                               "row label " + row.id + " leaks " + banned[b]);
                    }
                }
            }
        }

        function test_an_unowned_row_fails_validation() {            var errors = SettingsMap.validateRow({
                id: "unowned", label: "Unowned", control: "toggle",
                owner: "", key: "", absence: "live-on-defaults", state: "shipped"
            });
            verify(errors.join(" ").indexOf("no owner") >= 0,
                   "an unowned row must fail validation");
            verify(errors.join(" ").indexOf("no key/API") >= 0,
                   "a visible row without a key/API must fail validation");

            var badAbsence = SettingsMap.validateRow({
                id: "bad-absence", label: "Bad absence", owner: "settingsd",
                key: "test.key", absence: "pretend-it-works", state: "shipped"
            });
            verify(badAbsence.join(" ").indexOf("unknown absence") >= 0,
                   "an unknown absence path must fail validation");
        }

        // -- Omitted panes are absent and documented ---------------------------

        function test_omitted_panes_are_absent_from_the_sidebar_and_documented() {
            var shell = make();
            var visible = shell.visiblePanes.map(function(entry) { return entry.id; });
            for (var i = 0; i < SettingsMap.omittedPanes.length; ++i) {
                var omitted = SettingsMap.omittedPanes[i];
                verify(omitted.reason.length > 0,
                       "omitted pane " + omitted.id + " must document its reason");
                compare(SettingsPanes.paneById(omitted.id), null,
                        "omitted pane " + omitted.id + " must not be in the catalog");
                compare(visible.indexOf(omitted.id), -1,
                        "omitted pane " + omitted.id + " must not be in the sidebar");
            }
        }

        // -- Sidebar order -----------------------------------------------------

        function test_sidebar_order_matches_the_map() {
            var shell = make();
            var expected = SettingsMap.panes.filter(function(pane) {
                if (pane.hidden === true)
                    return false;
                return pane.shipped;
            }).map(function(pane) { return pane.id; });
            var actual = shell.visiblePanes.map(function(pane) { return pane.id; });
            compare(ids(actual), ids(expected),
                    "the sidebar must follow the map's grouped pane order");
        }

        // -- T285 reduced IA, aliases, and removals ---------------------------

        function test_every_pane_has_a_group_and_the_groups_are_ordered() {
            var order = SettingsPanes.groups.map(function(group) {
                return group.id;
            }).join(",");
            compare(order, "connectivity,desktop,input");
            for (var i = 0; i < SettingsPanes.groups.length; ++i) {
                compare(SettingsPanes.groups[i].title, "",
                        "group headings are not painted");
                verify(SettingsPanes.groups[i].accessibleTitle.length > 0,
                       "a logical group must keep an AT-SPI title");
            }
            for (var p = 0; p < SettingsMap.panes.length; ++p) {
                var pane = SettingsMap.panes[p];
                verify(order.indexOf(pane.group) >= 0,
                       "pane " + pane.id + " has an unknown group " + pane.group);
            }
            // The map order is the grouped order: groups never interleave.
            var lastGroup = -1;
            for (var q = 0; q < SettingsMap.panes.length; ++q) {
                var at = order.indexOf(SettingsMap.panes[q].group);
                verify(at >= lastGroup,
                       "pane " + SettingsMap.panes[q].id
                       + " is out of grouped order");
                lastGroup = at;
            }
        }

        function test_the_catalog_has_the_twenty_seven_entry_sidebar_order() {
            var shell = make();
            var expected = [
                "general/software-update",
                "wifi", "bluetooth", "network", "vpn", "battery",
                "general", "accessibility", "appearance", "desktop-dock",
                "displays", "menu-bar", "search", "wallpaper", "screen-saver",
                "notifications", "sound", "focus", "lock-screen", "privacy",
                "users-groups", "online-accounts", "keyboard", "mouse",
                "trackpad", "game-controllers", "printers"
            ];
            var actual = [];
            for (var s = 0; s < shell.sidebarSections.length; ++s) {
                var items = shell.sidebarSections[s].items;
                for (var i = 0; i < items.length; ++i) {
                    if (items[i].spacer !== true)
                        actual.push(items[i].id);
                }
            }
            compare(actual.join(","), expected.join(","),
                    "the sidebar must have the mockup's 27 entries in order");
            compare(actual.length, 27);
            verify(actual.indexOf("biometrics") < 0);
            verify(actual.indexOf("mission-control") < 0);
            verify(actual.indexOf("control-center") < 0);
        }

        function test_groups_and_spacers_keep_their_order() {
            var shell = make();
            compare(shell.sidebarSections.length, 3);
            compare(shell.sidebarSections[0].spacerAfter, true);
            compare(shell.sidebarSections[1].spacerAfter, true);
            compare(shell.sidebarSections[2].spacerAfter, false);
            // The status row is prepended to the first group, not a pane.
            compare(shell.sidebarSections[0].items[0].id,
                    "general/software-update");
            compare(shell.sidebarSections[0].items[0].status, true);
            // The spacer after the first group is a non-selectable entry.
            var firstSpacer = shell.sidebarSections[0].items[
                shell.sidebarSections[0].items.length - 1];
            compare(firstSpacer.spacer, true);
            compare(shell.sidebarIdAtIndex(shell.sidebarIndexOf("wifi")),
                    "wifi");
            compare(shell.sidebarIdAtIndex(
                        shell.sidebarIndexOf("general/software-update")),
                    "general/software-update");
            compare(shell.sidebarIndexOf("biometrics"), -1);
        }

        function test_storage_is_a_general_alias_and_vpn_is_always_visible() {
            var storage = SettingsMap.panes.filter(function(pane) {
                return pane.id === "storage";
            })[0];
            compare(storage.hidden, true);
            compare(SettingsPanes.resolve("storage").paneId, "general");
            compare(SettingsPanes.resolve("storage").subPaneId, "storage");
            compare(SettingsMap.aliases["storage"].subPaneId, "storage");

            var vpn = SettingsMap.panes.filter(function(pane) {
                return pane.id === "vpn";
            })[0];
            compare(vpn.shipped, true);
            compare(vpn.optional, undefined);
            compare(SettingsPanes.resolve("vpn").paneId, "vpn");
            compare(SettingsPanes.indexOf("vpn") >= 0, true);
        }

        function test_aliases_land_on_the_new_home() {
            compare(SettingsPanes.resolve("mission-control").paneId,
                    "desktop-dock");
            compare(SettingsPanes.resolve("mission-control").anchor,
                    "Workspace Overview");
            compare(SettingsPanes.resolve("overview").paneId, "desktop-dock");
            compare(SettingsPanes.resolve("control-center").paneId,
                    "menu-bar");
            compare(SettingsPanes.resolve("control-center").subPaneId,
                    "controls");
            compare(SettingsPanes.resolve("quick-settings").subPaneId,
                    "controls");
            compare(SettingsPanes.resolve("internet-accounts").paneId,
                    "online-accounts");
            // The visible category label is Energy; the id stays `battery`.
            compare(SettingsPanes.paneById("battery").title, "Energy");
            compare(SettingsPanes.resolve("energy").paneId, "battery");
            compare(SettingsPanes.resolve("general/software-update").paneId,
                    "general");
            compare(SettingsPanes.resolve("general/software-update").subPaneId,
                    "software-update");

            // The map mirrors the catalog's alias table one-for-one.
            var catalogIds = Object.keys(SettingsPanes.aliases).sort().join(",");
            var mapIds = Object.keys(SettingsMap.aliases).sort().join(",");
            compare(mapIds, catalogIds, "catalog and map aliases must agree");
            for (var id in SettingsPanes.aliases) {
                compare(SettingsMap.aliases[id].paneId,
                        SettingsPanes.aliases[id].paneId,
                        "alias target for " + id);
                compare(SettingsMap.aliases[id].subPaneId,
                        SettingsPanes.aliases[id].subPaneId,
                        "alias sub-pane for " + id);
            }
        }

        function test_removed_panes_are_documented_and_biometrics_is_refused() {
            var shell = make();
            var ids = SettingsMap.removed.map(function(entry) { return entry.id; });
            compare(ids.join(","), "biometrics,mission-control,control-center");
            for (var i = 0; i < SettingsMap.removed.length; ++i) {
                verify(SettingsMap.removed[i].reason.length > 0,
                       "removed pane " + SettingsMap.removed[i].id
                       + " must document its reason");
                compare(SettingsPanes.paneById(SettingsMap.removed[i].id), null,
                        "removed pane " + SettingsMap.removed[i].id
                        + " must not be in the catalog");
            }
            compare(SettingsPanes.resolve("biometrics"), null);
            verify(!shell.deepLink("settings://pane/biometrics"),
                   "a removed id must be refused");
            verify(shell.lastDeepLinkError.length > 0,
                   "a refusal must carry a reason");
            verify(shell.deepLink("settings://pane/mission-control"));
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            verify(shell.deepLink("settings://pane/internet-accounts"));
            tryVerify(function() { return shell.currentPaneId === "online-accounts"; });
        }

        // -- The debug view ----------------------------------------------------

        function test_debug_view_shows_pane_and_row_owners() {
            var shell = make();
            shell.selectPane("bluetooth");
            waitForRendering(stage);
            shell.showDebugView();
            tryCompare(shell.debugView, "open", true);
            waitForRendering(stage);

            compare(shell.debugView.paneId, "bluetooth");
            verify(shell.debugView.pane !== null);
            compare(shell.debugView.pane.owner, "BlueZ");
            compare(shell.debugView.pane.absence, "disabled-with-reason");
            compare(shell.debugView.pane.deepLink, "settings://pane/bluetooth");
            verify(shell.debugView.rowList.count > 0,
                   "the debug view must list the pane's rows");

            var sample = SettingsMap.rowFor("bluetooth", "power");
            verify(sample !== null);
            compare(sample.owner, "BlueZ");
            compare(sample.key, "Adapter1.Powered");
            compare(sample.absence, "disabled-with-reason");
            compare(sample.state, "shipped");

            // The map document resolves from the dev/test tree.
            verify(Settings.mapDocPath.indexOf("settings-business-logic-map.md") >= 0);
            verify(Settings.mapDocUrl.toString().length > 0,
                   "the debug view's map link must resolve");
            verify(shell.debugView.mapButton.enabled);
            verify(shell.debugView.removedText.visible,
                   "the debug view must show the removed-pane reasons");
            verify(shell.debugView.removedText.text.indexOf("Biometrics") >= 0);
            shell.debugView.reject();
        }

        function test_debug_view_switches_pane_and_rows() {
            var shell = make();
            shell.showDebugView("appearance");
            tryCompare(shell.debugView, "open", true);
            compare(shell.debugView.paneId, "appearance");
            verify(shell.debugView.rowList.count > 0);
            var firstCount = shell.debugView.rowList.count;

            var index = SettingsMap.paneIndex("printers");
            shell.debugView.paneSelect.activateIndex(index);
            compare(shell.debugView.paneId, "printers");
            tryCompare(shell.debugView.rowList, "count",
                       SettingsMap.rowsFor("printers").length);
            verify(shell.debugView.rowList.count !== firstCount
                   || SettingsMap.rowsFor("appearance").length
                      === SettingsMap.rowsFor("printers").length);
            shell.debugView.reject();
        }

        function test_help_menu_links_the_debug_view_and_map() {
            var help = SettingsMenu.menus[4];
            compare(help.title, "Help");
            compare(help.items[0].action, "help");
            compare(help.items[1].type, "separator");
            compare(help.items[2].action, "help.debug");
            compare(help.items[3].action, "help.map");
        }
    }
}
