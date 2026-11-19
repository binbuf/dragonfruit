// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T223 Settings search, deep-link, row-anatomy, keyboard, and long-string
// tests. Headless offscreen + software scene graph; `DF_SETTINGS_FIXTURE`
// keeps the cases off the host bus.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsSearch"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        Component {
            id: longRowComponent
            SettingsRow {
                width: 340
                label: "Barrierefreiheit und Bedienungshilfen für Menschen mit besonderen Anforderungen"
                description: "Diese Beschreibung ist absichtlich sehr lang, damit die Zeile über mehrere Zeilen umbricht und das Steuerelement trotzdem vollständig sichtbar bleibt."
                settingsId: "long-string"
                owner: "settingsd"
                settingsKey: "test.long"
                controlData: Toggle { checked: true }
            }
        }

        Component {
            id: anatomyRowComponent
            SettingsRow {
                width: 420
                label: "Anatomy"
                description: "One row, every anatomy slot."
                settingsId: "anatomy"
                owner: "settingsd"
                settingsKey: "test.anatomy"
                disclosure: true
                trailingAction: "Details..."
                help: "Anatomy help"
                controlData: Toggle { checked: true }
            }
        }

        SignalSpy { id: disclosureSpy; signalName: "disclosureClicked" }
        SignalSpy { id: trailingSpy; signalName: "trailingActionClicked" }
        SignalSpy { id: helpSpy; signalName: "helpRequested" }

        function make(props) {
            var merged = { width: stage.width, height: stage.height,
                           highlightHoldMs: 60000 };
            for (var key in (props || {}))
                merged[key] = props[key];
            var shell = createTemporaryObject(shellComponent, stage, merged);
            waitForRendering(stage);
            return shell;
        }

        // Drive the field without waiting for the 120 ms debounce. The debounce
        // itself is covered by `test_keyboard_only_search_and_activate`.
        function runSearch(shell, text) {
            shell.searchText = text;
            shell.commitSearch();
            waitForRendering(stage);
        }

        function childByName(item, name) {
            if (item === null || item === undefined)
                return null;
            if (item.objectName === name)
                return item;
            var children = item.children || [];
            for (var i = 0; i < children.length; ++i) {
                var found = childByName(children[i], name);
                if (found !== null)
                    return found;
            }
            return null;
        }

        function rowOf(shell, paneId, rowId) {
            shell.selectPane(paneId);
            waitForRendering(stage);
            var row = SettingsMap.rowFor(paneId, rowId);
            return shell.findRowTarget(shell.paneBody.item, row);
        }

        // -- The local index ---------------------------------------------------

        function test_index_is_local_and_deep_linkable() {
            verify(SettingsSearch.entries.length > 100,
                   "the index must cover panes, sections, and rows");
            var kinds = { pane: 0, section: 0, row: 0 };
            for (var i = 0; i < SettingsSearch.entries.length; ++i) {
                var entry = SettingsSearch.entries[i];
                verify(kinds[entry.kind] !== undefined,
                       "unknown result kind " + entry.kind);
                kinds[entry.kind] += 1;
                var pane = SettingsPanes.paneById(entry.paneId);
                verify(pane !== null && pane.shipped,
                       "entry " + entry.label + " must belong to a shipped pane");
                verify(String(entry.deepLink).indexOf("settings://pane/") === 0,
                       "entry " + entry.label + " must carry a deep link");
                if (entry.kind === "row" || entry.kind === "section") {
                    var row = SettingsMap.rowFor(entry.paneId, entry.targetRow);
                    verify(row !== null, "entry " + entry.label + " must target a map row");
                    compare(row.state, "shipped",
                            "entry " + entry.label + " must target a shipped row");
                }
            }
            verify(kinds.pane >= 20, "every shipped pane must be indexed");
            verify(kinds.section > 10, "sections must be indexed");
            verify(kinds.row > 50, "rows must be indexed");

            // Unshipped panes are never search results (no half panes).
            for (var p = 0; p < SettingsMap.panes.length; ++p) {
                var mapped = SettingsMap.panes[p];
                if (mapped.shipped)
                    continue;
                var results = SettingsSearch.search(mapped.title);
                for (var r = 0; r < results.length; ++r)
                    verify(results[r].paneId !== mapped.id,
                           "unshipped pane " + mapped.id + " must not be a result");
            }
        }

        function test_removed_panes_are_absent_from_the_index() {
            // T285 removed three top-level panes. No entry may name one of
            // them, even through an old label or synonym.
            var removedIds = ["biometrics", "mission-control", "control-center"];
            for (var i = 0; i < SettingsSearch.entries.length; ++i) {
                compare(removedIds.indexOf(SettingsSearch.entries[i].paneId), -1,
                        "entry " + SettingsSearch.entries[i].label
                        + " names a removed pane");
            }
            var queries = ["biometrics", "mission control", "control center",
                           "quick settings", "overview"];
            for (var q = 0; q < queries.length; ++q) {
                var results = SettingsSearch.search(queries[q]);
                for (var r = 0; r < results.length; ++r) {
                    if (results[r].kind === "showAll")
                        continue;
                    compare(removedIds.indexOf(results[r].paneId), -1,
                            "'" + queries[q] + "' must not reach a removed pane");
                    verify(String(results[r].deepLink).indexOf("settings://pane/") === 0);
                }
            }
            // The moved names still reach their new homes.
            compare(SettingsSearch.firstOfKind("quick settings", "pane").paneId,
                    "menu-bar");
            compare(SettingsSearch.firstOfKind("mission control", "pane").paneId,
                    "desktop-dock");
        }

        function test_search_ranks_pane_section_and_row() {
            var wallpaper = SettingsSearch.search("wallpaper");
            verify(wallpaper.length > 0);
            compare(wallpaper[0].kind, "pane");
            compare(wallpaper[0].paneId, "wallpaper");

            var previews = SettingsSearch.search("show previews");
            verify(previews.length > 0);
            compare(previews[0].kind, "row");
            compare(previews[0].deepLink,
                    "settings://pane/notifications/row/previews");

            // T285 removed the top-level Quick Settings pane and its "Dark
            // mode" row; the Appearance pane owns the query through its
            // synonym again.
            var dark = SettingsSearch.search("dark mode");
            verify(dark.length > 0);
            compare(dark[0].kind, "pane");
            compare(dark[0].paneId, "appearance");

            // T290d: the Appearance pane now has a Theme section, so an exact
            // "theme" query lands on that section (and deep-links to its first
            // row) while the pane stays in the same result group.
            var theme = SettingsSearch.search("theme");
            verify(theme.length > 0);
            compare(theme[0].paneId, "appearance");
            compare(theme[0].kind, "section");
            compare(theme[0].label, "Theme");
            var themePane = false;
            for (var t = 0; t < theme.length; ++t) {
                if (theme[t].kind === "pane" && theme[t].paneId === "appearance")
                    themePane = true;
            }
            verify(themePane, "the Appearance pane must remain a 'theme' result");

            var size = SettingsSearch.search("size");
            verify(size.length > 0);
            compare(size[0].kind, "row");
            compare(size[0].deepLink, "settings://pane/desktop-dock/row/size");

            var exact = SettingsSearch.search("Show previews");
            compare(exact[0].label, "Show previews");
            compare(SettingsSearch.search("").length, 0);
            compare(SettingsSearch.search("no-such-setting").length, 0);
        }

        // T290b: the Appearance tiles are searchable by their three values,
        // not only by the old "light mode"/"dark mode" phrases.
        function test_appearance_scheme_finds_light_dark_and_auto() {
            var terms = ["light", "dark", "auto"];
            for (var t = 0; t < terms.length; ++t) {
                var results = SettingsSearch.search(terms[t]);
                var found = false;
                for (var i = 0; i < results.length; ++i) {
                    if (results[i].paneId === "appearance"
                            && String(results[i].deepLink)
                               === "settings://pane/appearance/row/scheme") {
                        found = true;
                        break;
                    }
                }
                verify(found, "search for '" + terms[t]
                       + "' must reach appearance/scheme");
            }
        }

        // T290c: the Appearance tint is called "Glass tint"; the old
        // "attached menu transparency" wording still resolves.
        function test_appearance_glass_tint_is_searchable() {
            var terms = ["glass tint", "attached menu transparency", "opacity"];
            for (var t = 0; t < terms.length; ++t) {
                var results = SettingsSearch.search(terms[t]);
                var found = false;
                for (var i = 0; i < results.length; ++i) {
                    if (results[i].paneId === "appearance"
                            && String(results[i].deepLink)
                               === "settings://pane/appearance/row/transparency") {
                        found = true;
                        break;
                    }
                }
                verify(found, "search for '" + terms[t]
                       + "' must reach appearance/transparency");
            }
        }

        // T290d: the Theme rows are findable by their labels and the words
        // users use for them.
        function test_appearance_theme_rows_are_searchable() {
            var cases = [
                { terms: ["accent"], row: "accent" },
                { terms: ["text highlight", "selection color", "highlight"],
                  row: "highlight" },
                { terms: ["icon style", "widget style"], row: "iconStyle" },
                { terms: ["folder color", "folder tint"], row: "folderColor" }
            ];
            for (var c = 0; c < cases.length; ++c) {
                for (var t = 0; t < cases[c].terms.length; ++t) {
                    var results = SettingsSearch.search(cases[c].terms[t]);
                    var found = false;
                    for (var i = 0; i < results.length; ++i) {
                        if (results[i].paneId === "appearance"
                                && String(results[i].deepLink)
                                   === "settings://pane/appearance/row/"
                                      + cases[c].row) {
                            found = true;
                            break;
                        }
                    }
                    verify(found, "search for '" + cases[c].terms[t]
                           + "' must reach appearance/" + cases[c].row);
                }
            }
        }


        // T290e: the Windows and scroll-bar rows are findable by the mockup
        // labels and by the phrases users type.
        function test_appearance_windows_and_scroll_rows_are_searchable() {
            var cases = [
                { terms: ["sidebar icon size", "sidebar icons"],
                  row: "sidebarIconSize" },
                { terms: ["wallpaper tint", "tint window background"],
                  row: "wallpaperTint" },
                { terms: ["scroll bars", "scroll bar visibility"],
                  row: "scrollBars" },
                { terms: ["scroll bar click", "click in the scroll bar",
                          "jump to the spot"],
                  row: "scrollBarClick" }
            ];
            for (var c = 0; c < cases.length; ++c) {
                for (var t = 0; t < cases[c].terms.length; ++t) {
                    var results = SettingsSearch.search(cases[c].terms[t]);
                    var found = false;
                    for (var i = 0; i < results.length; ++i) {
                        if (results[i].paneId === "appearance"
                                && String(results[i].deepLink)
                                   === "settings://pane/appearance/row/"
                                      + cases[c].row) {
                            found = true;
                            break;
                        }
                    }
                    verify(found, "search for '" + cases[c].terms[t]
                           + "' must reach appearance/" + cases[c].row);
                }
            }
        }


        // T290f: the Advanced Appearance sheet's six rows are indexed by their
        // rendered labels and synonyms, and every appearance row is landable.
        function test_appearance_advanced_rows_are_searchable() {
            var cases = [
                { terms: ["icon theme", "installed icon theme"],
                  row: "iconTheme" },
                { terms: ["cursor theme", "pointer theme"], row: "cursorTheme" },
                { terms: ["cursor size", "pointer size"], row: "cursorSize" },
                { terms: ["window controls", "traffic lights"],
                  row: "windowControls" },
                { terms: ["material effects", "glass effects"], row: "material" },
                { terms: ["accent from wallpaper", "dynamic accent"],
                  row: "accentFromWallpaper" }
            ];
            for (var c = 0; c < cases.length; ++c) {
                for (var t = 0; t < cases[c].terms.length; ++t) {
                    var results = SettingsSearch.search(cases[c].terms[t],
                                                        { perPane: 50 });
                    var found = false;
                    for (var i = 0; i < results.length; ++i) {
                        if (results[i].paneId === "appearance"
                                && String(results[i].deepLink)
                                   === "settings://pane/appearance/row/"
                                      + cases[c].row) {
                            found = true;
                            break;
                        }
                    }
                    verify(found, "search for '" + cases[c].terms[t]
                           + "' must reach appearance/" + cases[c].row);
                }
            }

            // The sheet's own heading is one section result that deep-links
            // into the sheet.
            var section = SettingsSearch.search("advanced appearance",
                                                { perPane: 50 });
            var sectionFound = false;
            for (var s = 0; s < section.length; ++s) {
                if (section[s].kind === "section"
                        && section[s].paneId === "appearance"
                        && section[s].label === "Advanced Appearance") {
                    sectionFound = true;
                    break;
                }
            }
            verify(sectionFound, "the Advanced Appearance section must be indexed");
        }

        // T290f: the audit in one place - every shipped Appearance row is
        // landable and its rendered label finds the row it deep-links to.
        function test_every_appearance_row_is_landable_and_searchable() {
            var rows = SettingsMap.visibleRowsFor("appearance");
            verify(rows.length > 0);
            for (var r = 0; r < rows.length; ++r) {
                verify(SettingsMap.isRowLandable("appearance", rows[r].id),
                       rows[r].id + " must be landable");
                var results = SettingsSearch.search(rows[r].label,
                                                    { perPane: 50 });
                var found = false;
                for (var i = 0; i < results.length; ++i) {
                    if (results[i].paneId === "appearance"
                            && String(results[i].deepLink)
                               === "settings://pane/appearance/row/"
                                  + rows[r].id) {
                        found = true;
                        break;
                    }
                }
                verify(found, "the label '" + rows[r].label
                       + "' must reach appearance/" + rows[r].id);
            }
        }

        function test_synonyms_and_help_text_are_indexed() {
            var bluez = SettingsSearch.search("bluez");
            verify(bluez.length > 0);
            var foundBluetooth = false;
            for (var i = 0; i < bluez.length; ++i) {
                if (bluez[i].paneId === "bluetooth")
                    foundBluetooth = true;
            }
            verify(foundBluetooth, "the BlueZ owner help must find Bluetooth");

            var manager = SettingsSearch.search("NetworkManager");
            verify(manager.length > 0);
            var foundNetwork = false;
            for (var n = 0; n < manager.length; ++n) {
                if (manager[n].paneId === "network")
                    foundNetwork = true;
            }
            verify(foundNetwork, "the NetworkManager owner must find Network");

            var absence = SettingsSearch.search("disabled with reason");
            verify(absence.length > 0);
            var foundAbsence = false;
            for (var a = 0; a < absence.length; ++a) {
                if (absence[a].paneId === "bluetooth")
                    foundAbsence = true;
            }
            verify(foundAbsence, "the absence help text must be searchable");

            // Pane-level absence kinds are indexed too (rows carry their own).
            var delegated = SettingsSearch.search("delegated");
            var foundDelegated = false;
            for (var d = 0; d < delegated.length; ++d) {
                if (delegated[d].paneId === "online-accounts")
                    foundDelegated = true;
            }
            verify(foundDelegated, "a pane absence kind must be searchable");

            // Accent/diacritic folding keeps a localized query useful.
            compare(SettingsSearch.normalize("Wi-Fi"), "wi fi");
            compare(SettingsSearch.normalize("  Größe  "), "große");
        }

        // -- T265: local discovery 2.0 ----------------------------------------

        function test_colloquial_names_find_live_targets() {
            var cases = [
                { query: "wifi", pane: "wifi" },
                { query: "wi-fi", pane: "wifi" },
                { query: "Wi Fi", pane: "wifi" },
                { query: "control center", pane: "menu-bar" },
                { query: "quick settings", pane: "menu-bar" },
                { query: "menu bar controls", pane: "menu-bar" },
                { query: "mission control", pane: "desktop-dock" },
                { query: "workspace overview", pane: "desktop-dock" },
                { query: "time machine", pane: "general", row: "backup." },
                { query: "touch id", pane: "users-groups" },
                { query: "internet accounts", pane: "online-accounts" },
                { query: "passwords & keys", pane: "general" },
                { query: "backups", pane: "general" },
                { query: "keyring", pane: "general" },
                { query: "restic", pane: "general" },
                { query: "timeshift", pane: "general" },
                { query: "date and time", pane: "general" },
                { query: "language", pane: "general" },
                { query: "screen saver", pane: "screen-saver" },
                { query: "autofill", pane: "general" }
            ];
            for (var i = 0; i < cases.length; ++i) {
                var test = cases[i];
                var results = SettingsSearch.search(test.query);
                verify(results.length > 0,
                       "query '" + test.query + "' must return a result");
                var matched = false;
                var rowMatched = test.row === undefined;
                for (var r = 0; r < results.length; ++r) {
                    var result = results[r];
                    if (result.kind === "showAll")
                        continue;
                    verify(String(result.deepLink).indexOf("settings://pane/") === 0,
                           "'" + test.query + "' must never return a dead result");
                    if (result.paneId === test.pane)
                        matched = true;
                    if (test.row !== undefined && result.kind === "row"
                            && result.paneId === test.pane
                            && result.targetRow.indexOf(test.row) === 0)
                        rowMatched = true;
                }
                verify(matched, "query '" + test.query + "' must reach "
                       + test.pane);
                verify(rowMatched, "query '" + test.query + "' must reach row "
                       + test.row);
            }

            // A pane id is an alias too: the moved panes keep their old macOS
            // names as search words and land on their new home.
            compare(SettingsSearch.firstOfKind("control center", "pane").paneId,
                    "menu-bar");
            compare(SettingsSearch.firstOfKind("mission control", "pane").paneId,
                    "desktop-dock");
        }

        function test_pane_subtitle_is_truthful_not_an_absence_note() {
            var wifi = SettingsSearch.firstOfKind("wifi", "pane");
            verify(wifi !== null);
            verify(wifi.subtitle.length > 0);
            verify(wifi.subtitle.indexOf("unavailable") < 0,
                   "an available pane must not use its absence note as subtitle");
            var mapPane = SettingsMap.panes.filter(function(pane) {
                return pane.id === "wifi";
            })[0];
            verify(wifi.subtitle !== mapPane.absenceNote);

            var row = SettingsSearch.firstOfKind("show previews", "row");
            verify(row !== null);
            compare(row.subtitle, "Notifications");
            verify(row.subtitle.indexOf("unavailable") < 0);
        }

        function test_technical_terms_stay_out_of_the_primary_rank() {
            // An owner query still finds the pane, but only in the lower band.
            var bluez = SettingsSearch.search("bluez");
            verify(bluez.length > 0);
            for (var i = 0; i < bluez.length; ++i) {
                if (bluez[i].kind !== "showAll")
                    verify(bluez[i].technicalOnly,
                           "an owner-only match must be technical-only");
            }
            // A user word always sorts above the technical band.
            var wifi = SettingsSearch.search("wifi");
            verify(wifi.length > 0);
            verify(wifi[0].primaryMatch,
                   "the Wi-Fi pane must be a primary match");
            var network = SettingsSearch.search("network");
            verify(network[0].primaryMatch,
                   "a user-word match must outrank owner/key help text");
        }

        function test_results_are_grouped_capped_and_show_all() {
            var capped = SettingsSearch.search("network");
            verify(SettingsSearch.groupCount(capped) > 1,
                   "results must be grouped by pane");
            var networkRows = capped.filter(function(result) {
                return result.paneId === "network" && result.kind !== "showAll";
            });
            compare(networkRows.length, SettingsSearch.paneLimit);
            var showAll = capped.filter(function(result) {
                return result.paneId === "network" && result.kind === "showAll";
            });
            compare(showAll.length, 1);
            verify(showAll[0].hiddenCount > 0);
            verify(showAll[0].label.indexOf("Show all") === 0);
            // Every group's first row carries the pane header marker.
            var groups = 0;
            for (var g = 0; g < capped.length; ++g) {
                if (capped[g].groupStart === true)
                    groups += 1;
            }
            compare(groups, SettingsSearch.groupCount(capped));

            // "Show all" expands in place and keeps the query.
            var expanded = SettingsSearch.search("network",
                                                 { expanded: ["network"] });
            var expandedRows = expanded.filter(function(result) {
                return result.paneId === "network" && result.kind !== "showAll";
            });
            verify(expandedRows.length > SettingsSearch.paneLimit);
            var stillCapped = expanded.filter(function(result) {
                return result.paneId === "network" && result.kind === "showAll";
            });
            compare(stillCapped.length, 0);

            // An expanded group is never dropped by the global list limit.
            var limited = SettingsSearch.search("network",
                                                { limit: 5,
                                                  expanded: ["wifi"] });
            var wifiGroup = limited.filter(function(result) {
                return result.paneId === "wifi";
            });
            verify(wifiGroup.length > 0,
                   "an expanded group must survive the global list limit");
        }

        function test_no_match_and_long_localized_query() {
            compare(SettingsSearch.search("zzzz-no-result").length, 0);
            var longQuery = "Barrierefreiheit und Bedienungshilfen für Menschen "
                    + "mit besonderen Anforderungen an die Darstellung";
            var longResults = SettingsSearch.search(longQuery);
            verify(longResults.length >= 0, "a long localized query must not throw");
            verify(SettingsSearch.search("Größe").length >= 0);
            compare(SettingsSearch.compact("Wi-Fi"), "wifi");
            compare(SettingsSearch.compact("Wi Fi"), "wifi");
            compare(SettingsSearch.compact("wifi"), "wifi");
        }

        function test_show_all_expands_the_group_in_place() {
            var shell = make();
            runSearch(shell, "network");
            var showIndex = -1;
            for (var i = 0; i < shell.searchResults.length; ++i) {
                if (shell.searchResults[i].kind === "showAll"
                        && shell.searchResults[i].paneId === "network")
                    showIndex = i;
            }
            verify(showIndex >= 0);
            var before = shell.searchResults.length;
            shell.searchResultsList.currentIndex = showIndex;
            shell.searchResultsList.activateCurrent();
            tryVerify(function() {
                for (var r = 0; r < shell.searchResults.length; ++r) {
                    if (shell.searchResults[r].kind === "showAll"
                            && shell.searchResults[r].paneId === "network")
                        return false;
                }
                return true;
            });
            compare(shell.searchText, "network",
                    "Show all must keep the query and the field text");
            verify(shell.searchResults.length >= before);
            verify(shell.searchResultsList.visible);
        }

        function test_keyboard_focus_moves_to_the_activated_row() {
            var shell = make();
            runSearch(shell, "show previews");
            shell.searchResultsList.forceActiveFocus();
            shell.searchResultsList.currentIndex = 0;
            keyClick(Qt.Key_Return);
            tryVerify(function() { return shell.highlightTarget !== null; });
            compare(shell.currentPaneId, "notifications");
            verify(!shell.searchField.activeFocus,
                   "activation must not leave focus in the search field");
            verify(!shell.searchResultsList.activeFocus,
                   "activation must leave the result list");
            compare(shell.highlightTarget.settingsId, "previews");
        }

        function test_grouped_results_fit_740x820_and_720x480() {
            var sizes = [[740, 820], [720, 480]];
            for (var s = 0; s < sizes.length; ++s) {
                var shell = make({ width: sizes[s][0], height: sizes[s][1] });
                runSearch(shell, "network");
                verify(shell.searchResultsList.visible);
                verify(shell.searchResultsList.countText.visible);
                verify(shell.searchResultsList.countText.width
                       <= shell.sidebar.width);
                verify(shell.searchResultsList.width <= shell.sidebar.width);
                verify(shell.searchResultsList.listView.height > 0,
                       "the grouped list must keep a visible body at "
                       + sizes[s][0] + "x" + sizes[s][1]);
                compare(shell.searchResultsList.Accessible.role, Accessible.List);
                shell.destroy();
                wait(50);
            }
        }

        function test_result_list_names_pane_and_section() {
            var shell = make();
            runSearch(shell, "show previews");
            var item = shell.searchResultsList.listView.itemAtIndex(0);
            verify(item !== null);
            compare(item.Accessible.role, Accessible.ListItem);
            compare(item.Accessible.name, "Show previews");
            verify(item.Accessible.description.indexOf("Notifications") >= 0,
                   "the item must name its pane (got "
                   + item.Accessible.description + ")");
            verify(shell.searchResultsList.Accessible.name.indexOf("results")
                   >= 0);
        }

        function test_search_clear_button_has_an_accessible_name() {
            var shell = make();
            var clear = childByName(shell.searchField, "searchClearButton");
            verify(clear !== null, "the field must expose its clear affordance");
            compare(clear.Accessible.role, Accessible.Button);
            compare(clear.Accessible.name, "Clear search");
            verify(clear.Accessible.description.length > 0);
        }

        // -- Search UI + keyboard ----------------------------------------------

        function test_search_results_activate_a_deep_link() {
            var shell = make();
            runSearch(shell, "show previews");
            verify(shell.searchActive);
            verify(shell.searchResultsList.visible);
            verify(shell.searchResults.length > 0);
            compare(shell.searchResultsList.currentIndex, 0);

            shell.searchResultsList.activateCurrent();
            waitForRendering(stage);
            // T286: the query stays in the field after a result opens; the
            // ranked list hides so the pane is visible.
            compare(shell.searchText, "show previews");
            compare(shell.searchField.text, "show previews");
            verify(!shell.searchResultsList.visible);
            compare(shell.currentPaneId, "notifications");
            verify(shell.highlightTarget !== null);
            compare(shell.highlightTarget.label, "Show previews");
            compare(shell.highlightTarget.settingsId, "previews");
        }

        function test_keyboard_only_search_and_activate() {
            var shell = make();
            shell.searchField.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_S);
            keyClick(Qt.Key_I);
            keyClick(Qt.Key_Z);
            keyClick(Qt.Key_E);
            tryCompare(shell, "searchText", "size");
            // T265: the index scan is debounced, so the list settles a beat
            // after the last keystroke.
            tryVerify(function() { return shell.searchResults.length > 0; });
            keyClick(Qt.Key_Down);
            waitForRendering(stage);
            verify(shell.searchResultsList.activeFocus,
                   "Down must hand focus to the result list");
            compare(shell.searchResultsList.currentIndex, 0);
            keyClick(Qt.Key_Return);
            waitForRendering(stage);
            compare(shell.currentPaneId, "desktop-dock");
            verify(shell.highlightTarget !== null);
            compare(shell.highlightTarget.label, "Size");
        }

        function test_down_enters_the_list_within_the_debounce_window() {
            var shell = make();
            shell.searchField.forceActiveFocus();
            waitForRendering(stage);
            // Set the field text without committing: Down must apply the
            // pending query and enter the list in one key press.
            shell.searchText = "wifi";
            keyClick(Qt.Key_Down);
            waitForRendering(stage);
            verify(shell.searchResultsList.activeFocus);
            compare(shell.searchResultsList.currentIndex, 0);
            verify(shell.searchResults.length > 0);
        }

        function test_result_list_scrolls_with_keyboard() {            var shell = make();
            runSearch(shell, "e");
            verify(shell.searchResults.length > 25,
                   "the grouped query must produce enough results to scroll");
            shell.searchResultsList.forceActiveFocus();
            for (var i = 0; i < 25; ++i)
                keyClick(Qt.Key_Down);
            compare(shell.searchResultsList.currentIndex, 25);
            waitForRendering(stage);
            verify(shell.searchResultsList.listView.contentY > 0,
                   "keyboard navigation must keep the current result visible");
        }

        function test_empty_state_and_escape() {
            var shell = make();
            runSearch(shell, "zzzz-no-result");
            verify(shell.searchResultsList.visible);
            compare(shell.searchResults.length, 0);
            verify(shell.searchResultsList.emptyText.visible);
            compare(shell.searchResultsList.emptyText.text,
                    "No results for \u201czzzz-no-result\u201d.");
            // The no-match state is announced, not only painted.
            compare(shell.searchResultsList.emptyText.Accessible.role,
                    Accessible.StaticText);
            compare(shell.searchResultsList.emptyText.Accessible.name,
                    "No results for \u201czzzz-no-result\u201d.");

            shell.searchResultsList.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            compare(shell.searchText, "");
            verify(!shell.searchResultsList.visible);
            verify(shell.sidebar.visible);
            verify(shell.searchField.activeFocus,
                   "Escape from the list returns focus to the field");
            keyClick(Qt.Key_Escape);
            verify(shell.sidebar.activeFocus,
                   "a second Escape returns focus to the sidebar");
        }

        // -- Deep links --------------------------------------------------------

        function test_deep_link_scrolls_and_highlights() {
            var shell = make();
            verify(shell.deepLink(
                "settings://pane/desktop-dock/row/minimizeReaction"));
            waitForRendering(stage);
            compare(shell.currentPaneId, "desktop-dock");
            verify(shell.highlightTarget !== null);
            compare(shell.highlightTarget.label,
                    "Bounce app icons when windows minimize");
            compare(shell.highlightTarget.highlighted, true);
            verify(shell.detailScroll.flickable.contentY >= 0);
        }

        function test_three_rows_across_panes_deep_link() {
            var shell = make();
            var links = [
                "settings://pane/appearance/row/scheme",
                "settings://pane/desktop-dock/row/size",
                "settings://pane/notifications/row/previews"
            ];
            var labels = ["Appearance", "Size", "Show previews"];
            for (var i = 0; i < links.length; ++i) {
                verify(shell.deepLink(links[i]), "deep link " + links[i]);
                waitForRendering(stage);
                compare(shell.currentPaneId,
                        links[i].split("/")[3]);
                verify(shell.highlightTarget !== null,
                       "deep link " + links[i] + " must land on a row");
                compare(shell.highlightTarget.label, labels[i]);
                compare(shell.highlightTarget.highlighted, true);
            }
        }

        function test_deep_link_rejects_unknown_targets() {
            var shell = make();
            verify(!shell.deepLink("settings://pane/no-such-pane"));
            verify(!shell.deepLink("settings://pane/appearance/row/no-such-row"));
            verify(!shell.deepLink("https://example.org/pane/appearance"));
            compare(shell.currentPaneId, "general");

            // T285: Biometrics was removed, not hidden. Its link is refused
            // with a reason; the moved panes land on their new home.
            verify(!shell.deepLink("settings://pane/biometrics"));
            verify(shell.lastDeepLinkError.length > 0,
                   "a refused link must surface a reason");
            compare(shell.currentPaneId, "general");
            verify(shell.deepLink("settings://pane/control-center"));
            waitForRendering(stage);
            compare(shell.currentPaneId, "menu-bar");

            // Online Accounts is always present now; its deep link is valid
            // even when the host service is absent (T263/T279).
            verify(shell.deepLink("settings://pane/online-accounts"));
            waitForRendering(stage);
            compare(shell.currentPaneId, "online-accounts");
        }

        // -- T264: deep links into General sub-panes --------------------------

        function test_deep_link_opens_the_general_subpane_and_lands_on_the_row() {
            var shell = make();
            verify(shell.deepLink(
                "settings://pane/general/row/language.region"));
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready;
            });
            tryVerify(function() {
                return shell.paneBody.item.subPaneId === "language";
            });
            tryVerify(function() { return shell.highlightTarget !== null; });
            compare(shell.currentPaneId, "general");
            compare(shell.paneTitle, "Language & Region");
            compare(shell.highlightTarget.settingsId, "language.region");
            compare(shell.highlightTarget.label, "Region");
            compare(shell.highlightTarget.highlighted, true);

            verify(shell.deepLink(
                "settings://pane/general/row/about.deviceName"));
            tryVerify(function() {
                return shell.paneBody.item.subPaneId === "about";
            });
            tryVerify(function() {
                return shell.highlightTarget !== null
                    && shell.highlightTarget.settingsId === "about.deviceName";
            });
            compare(shell.highlightTarget.settingsId, "about.deviceName");
            compare(shell.highlightTarget.highlighted, true);
            compare(shell.paneTitle, "About");
        }

        function test_deep_link_to_an_update_row_opens_the_subpane() {
            var shell = make();
            verify(shell.deepLink("settings://pane/general/row/updates.auto"));
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready;
            });
            tryVerify(function() {
                return shell.paneBody.item.subPaneId === "software-update";
            });
            compare(shell.currentPaneId, "general");
            compare(shell.paneTitle, "Software Update");
        }

        function test_every_general_subpane_has_a_resolvable_row() {
            var cases = [
                { link: "about.deviceName", sub: "about" },
                { link: "updates.auto", sub: "software-update" },
                { link: "passwords.source", sub: "passwords" },
                { link: "time.zone", sub: "date-time" },
                { link: "language.region", sub: "language" },
                { link: "login.start", sub: "login" },
                { link: "sharing.file", sub: "sharing" },
                { link: "backup.auto", sub: "backup" },
                { link: "reset.startupDisk", sub: "startup" },
                { link: "reset.reset", sub: "reset" },
                { link: "reset.transfer", sub: "reset" }
            ];
            var shell = make();
            for (var i = 0; i < cases.length; ++i) {
                compare(SettingsMap.subPaneForRow("general", cases[i].link),
                        cases[i].sub);
                verify(SettingsMap.isRowLandable("general", cases[i].link),
                       cases[i].link + " must be landable");
                verify(shell.deepLink(
                    "settings://pane/general/row/" + cases[i].link),
                    cases[i].link + " must be accepted");
                tryVerify(function() {
                    return shell.paneBody.item.subPaneId === cases[i].sub;
                });
            }
        }

        function test_back_forward_follows_pane_history() {
            var shell = make();
            verify(shell.deepLink("settings://pane/appearance/row/scheme"));
            waitForRendering(stage);
            verify(shell.deepLink("settings://pane/notifications/row/previews"));
            waitForRendering(stage);
            compare(shell.currentPaneId, "notifications");

            shell.back();
            compare(shell.currentPaneId, "appearance");
            verify(shell.canGoForward);
            shell.forward();
            compare(shell.currentPaneId, "notifications");
            verify(!shell.canGoForward);
        }

        function test_back_forward_restores_a_deep_link_row() {
            var shell = make();
            verify(shell.deepLink(
                "settings://pane/general/row/language.region"));
            tryVerify(function() {
                return shell.currentSubPaneId === "language"
                    && shell.highlightTarget !== null
                    && shell.highlightTarget.settingsId === "language.region";
            });
            shell.back();
            tryVerify(function() { return shell.currentSubPaneId === ""; });
            compare(shell.currentPaneId, "general");
            shell.forward();
            tryVerify(function() { return shell.currentSubPaneId === "language"; });
            tryVerify(function() {
                return shell.highlightTarget !== null
                    && shell.highlightTarget.settingsId === "language.region";
            });
        }

        // T290f: a dialog row is part of history; Back closes the sheet with
        // the pane, Forward reopens it on the row.
        function test_back_forward_restores_a_dialog_row() {
            var shell = make();
            verify(shell.deepLink(
                "settings://pane/appearance/row/iconTheme"));
            tryVerify(function() {
                return shell.paneBody.item.advancedDialog.open
                    && shell.highlightTarget !== null
                    && shell.highlightTarget.settingsId === "iconTheme";
            });
            verify(shell.deepLink("settings://pane/notifications/row/previews"));
            waitForRendering(stage);
            compare(shell.currentPaneId, "notifications");

            shell.back();
            tryVerify(function() {
                return shell.currentPaneId === "appearance"
                    && shell.paneBody.item.advancedDialog.open
                    && shell.highlightTarget !== null
                    && shell.highlightTarget.settingsId === "iconTheme";
            });
        }

        // -- Row anatomy and long strings --------------------------------------

        function test_row_anatomy_slots_are_one_component() {
            var row = createTemporaryObject(anatomyRowComponent, stage, {});
            waitForRendering(stage);
            verify(row.control.visible);
            verify(row.disclosureArea.visible);
            verify(row.trailingButton.visible);
            verify(row.helpArea.visible);
            compare(row.settingsId, "anatomy");
            compare(row.owner, "settingsd");
            compare(row.settingsKey, "test.anatomy");

            disclosureSpy.target = row;
            disclosureSpy.clear();
            mouseClick(row.disclosureArea, row.disclosureArea.width / 2,
                       row.disclosureArea.height / 2);
            compare(disclosureSpy.count, 1);

            trailingSpy.target = row;
            trailingSpy.clear();
            mouseClick(row.trailingButton, row.trailingButton.width / 2,
                       row.trailingButton.height / 2);
            compare(trailingSpy.count, 1);

            helpSpy.target = row;
            helpSpy.clear();
            mouseClick(row.helpArea, row.helpArea.width / 2,
                       row.helpArea.height / 2);
            compare(helpSpy.count, 1);
            compare(helpSpy.signalArguments[0][0], "Anatomy help");
        }

        function test_long_strings_do_not_clip_controls() {
            var row = createTemporaryObject(longRowComponent, stage, {});
            waitForRendering(stage);
            compare(row.labelText.elide, Text.ElideRight);
            verify(row.labelText.truncated,
                   "the long localized label must elide instead of pushing the control out");
            compare(row.descriptionText.wrapMode, Text.WordWrap);
            verify(row.height > Theme.controls.settingsRow.height,
                   "the wrapped description must grow the row");
            verify(row.control.x + row.control.width <= row.width,
                   "the control must stay inside the row");
            verify(row.control.visible);
        }

        function test_map_wiring_on_rendered_rows() {
            var shell = make();
            var checks = [
                { pane: "appearance", id: "scheme" },
                { pane: "desktop-dock", id: "size" },
                { pane: "notifications", id: "previews" }
            ];
            for (var i = 0; i < checks.length; ++i) {
                var row = rowOf(shell, checks[i].pane, checks[i].id);
                verify(row !== null, "row " + checks[i].pane + "/" + checks[i].id);
                var mapped = SettingsMap.rowFor(checks[i].pane, checks[i].id);
                compare(row.settingsId, mapped.id);
                compare(row.owner, mapped.owner);
                compare(row.settingsKey, mapped.key);
            }
        }

        // -- Help and headers --------------------------------------------------

        function test_help_opens_the_pane_anchor() {
            var shell = make();
            var url = Settings.helpUrl("pane-appearance");
            verify(url.toString().indexOf("file://") === 0);
            verify(url.toString().indexOf("#pane-appearance") >= 0);
            verify(shell.helpButton.enabled);
            compare(shell.helpButton.Accessible.name, "Help for General");
            verify(shell.helpButton.Accessible.description.length > 0,
                   "the help control explains itself when enabled");

            // The live control and the Help menu share one action, and the
            // fragment is the map's `#pane-<id>` anchor.
            shell.selectPane("appearance");
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready;
            });
            verify(shell.paneHelpUrl("appearance")
                   .indexOf("#pane-appearance") >= 0);
            mouseClick(shell.helpButton,
                       shell.helpButton.width / 2,
                       shell.helpButton.height / 2);
            verify(shell.lastHelpUrl.indexOf("#pane-appearance") >= 0,
                   "the button must open the pane anchor (got "
                   + shell.lastHelpUrl + ")");
        }

        function test_pane_headers_stay_reference_scoped() {
            compare(SettingsPanes.headerPanes.length, 3);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("general")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("accessibility")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("notifications")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("privacy")), true);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("appearance")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("desktop-dock")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("displays")), false);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("wallpaper")), false);
        }
    }
}
