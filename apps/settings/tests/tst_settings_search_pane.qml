// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Search pane tests (T227). The pane writes the durable search-service keys;
// the indexing status/rebuild rows stay disabled until T246.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsSearchPane"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.setList("search.categories", [
                "applications", "files", "folders", "actions", "calculations",
                "definitions", "web"
            ]);
            Settings.setList("search.indexLocations", ["home"]);
            Settings.set("search.shortcut", "Super+Space");
            Settings.set("search.web", true);
            Settings.set("menu.control.search", true);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("search");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready);
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_service_defaults() {
            var shell = make();
            compare(shell.currentPaneId, "search");
            var pane = paneOf(shell);
            compare(pane.categoryRepeater.count, 7);
            compare(pane.locationRepeater.count, 1);
            compare(pane.shortcut, "Super+Space");
            verify(pane.webToggle.checked);
            verify(pane.menuBarToggle.checked);
        }

        function test_category_toggle_writes_the_key() {
            var shell = make();
            var pane = paneOf(shell);
            pane.setCategory("files", false);
            compare(Settings.values["search.categories"].indexOf("files"), -1);
            pane.setCategory("files", true);
            verify(Settings.values["search.categories"].indexOf("files") >= 0);
        }

        function test_index_locations_add_and_remove() {
            var shell = make();
            var pane = paneOf(shell);

            pane.locationDraft = "/tmp/dragonfruit-index";
            pane.addLocation();
            verify(Settings.values["search.indexLocations"]
                   .indexOf("/tmp/dragonfruit-index") >= 0);
            compare(pane.locationDraft, "");

            pane.removeLocation("home");
            compare(Settings.values["search.indexLocations"].indexOf("home"), -1);
        }

        function test_shortcut_applies_and_rejects_empty() {
            var shell = make();
            var pane = paneOf(shell);

            pane.shortcutDraft = "Ctrl+Alt+Space";
            pane.applyShortcut();
            compare(Settings.values["search.shortcut"], "Ctrl+Alt+Space");

            pane.shortcutDraft = "   ";
            pane.applyShortcut();
            compare(Settings.values["search.shortcut"], "Ctrl+Alt+Space");
        }

        function test_web_and_menu_bar_toggles_apply() {
            var shell = make();
            var pane = paneOf(shell);
            pane.webToggle.toggled(false);
            compare(Settings.values["search.web"], false);
            pane.menuBarToggle.toggled(false);
            compare(Settings.values["menu.control.search"], false);

            Settings.set("search.web", true);
            tryVerify(function() { return pane.webToggle.checked; });
        }

        function test_indexing_rows_are_disabled_with_a_reason() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.statusRow.enabled, false);
            verify(pane.statusRow.description.indexOf("T246") >= 0);
            compare(pane.rebuildButton.enabled, false);
        }
    }
}
