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
