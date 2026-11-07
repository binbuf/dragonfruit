// SPDX-License-Identifier: MIT
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

        function test_an_unowned_row_fails_validation() {
            var errors = SettingsMap.validateRow({
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
                return pane.shipped;
            }).map(function(pane) { return pane.id; });
            var actual = shell.visiblePanes.map(function(pane) { return pane.id; });
            compare(ids(actual), ids(expected),
                    "the sidebar must follow the map's shipped pane order");
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
