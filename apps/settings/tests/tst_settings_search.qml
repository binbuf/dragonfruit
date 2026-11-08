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

            // T227 adds a Quick Settings row literally named "Dark mode"; an
            // exact row label outranks the Appearance pane's synonym. The
            // Appearance pane still owns the theme query.
            var dark = SettingsSearch.search("dark mode");
            verify(dark.length > 0);
            compare(dark[0].kind, "row");
            compare(dark[0].deepLink,
                    "settings://pane/control-center/row/darkMode");

            var theme = SettingsSearch.search("theme");
            verify(theme.length > 0);
            compare(theme[0].kind, "pane");
            compare(theme[0].paneId, "appearance");

            var size = SettingsSearch.search("size");
            verify(size.length > 0);
            compare(size[0].kind, "row");
            compare(size[0].deepLink, "settings://pane/desktop-dock/row/size");

            var exact = SettingsSearch.search("Show previews");
            compare(exact[0].label, "Show previews");
            compare(SettingsSearch.search("").length, 0);
            compare(SettingsSearch.search("no-such-setting").length, 0);
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

            // Accent/diacritic folding keeps a localized query useful.
            compare(SettingsSearch.normalize("Wi-Fi"), "wi fi");
            compare(SettingsSearch.normalize("  Größe  "), "große");
        }

        // -- Search UI + keyboard ----------------------------------------------

        function test_search_results_activate_a_deep_link() {
            var shell = make();
            shell.searchText = "show previews";
            waitForRendering(stage);
            verify(shell.searchActive);
            verify(shell.searchResultsList.visible);
            verify(shell.searchResults.length > 0);
            compare(shell.searchResultsList.currentIndex, 0);

            shell.searchResultsList.activateCurrent();
            waitForRendering(stage);
            compare(shell.searchText, "");
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

        function test_result_list_scrolls_with_keyboard() {
            var shell = make();
            shell.searchText = "e";
            waitForRendering(stage);
            verify(shell.searchResults.length > 25,
                   "the query must produce enough results to scroll");
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
            shell.searchText = "zzzz-no-result";
            waitForRendering(stage);
            verify(shell.searchResultsList.visible);
            compare(shell.searchResults.length, 0);
            verify(shell.searchResultsList.emptyText.visible);
            compare(shell.searchResultsList.emptyText.text,
                    "No results for \u201czzzz-no-result\u201d.");

            shell.searchResultsList.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            compare(shell.searchText, "");
            verify(!shell.searchResultsList.visible);
            verify(shell.sidebar.visible);
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
            verify(!shell.deepLink("settings://pane/online-accounts"));
            verify(!shell.deepLink("settings://pane/appearance/row/no-such-row"));
            verify(!shell.deepLink("https://example.org/pane/appearance"));
            compare(shell.currentPaneId, "appearance");
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
            compare(shell.helpButton.Accessible.name, "Help for Appearance");
        }

        function test_pane_headers_stay_reference_scoped() {
            compare(SettingsPanes.headerPanes.length, 4);
            compare(SettingsPanes.hasHeader(SettingsPanes.paneById("general")), true);
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
