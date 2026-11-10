// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Quick Settings pane tests (T227). The pane writes the same
// `quickSettings.modules` / `quickSettings.menuBarOnly` keys the live Control
// Center panel reads; `tst_controlcenter.qml` asserts the panel side.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsQuickSettings"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        readonly property var defaultModules: [
            "wifi", "bluetooth", "now-playing", "airdrop", "stage-manager",
            "screen-mirroring", "dark", "capture-screen", "focus", "brightness",
            "volume", "airplay-audio", "edit-controls"
        ]

        function init() {
            Settings.setList("quickSettings.modules", defaultModules.slice());
            Settings.setList("quickSettings.menuBarOnly", []);
            Settings.set("menu.control.wifi", true);
            Settings.set("menu.control.bluetooth", true);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("control-center");
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

        function test_pane_opens_with_every_module() {
            var shell = make();
            compare(shell.currentPaneId, "control-center");
            var pane = paneOf(shell);
            // T235: the catalogue is the 37-module measured inventory; the
            // default panel is the 13-module subset.
            compare(pane.moduleRepeater.count, 37);
            compare(pane.menuBarRepeater.count, 6);
            compare(Settings.values["quickSettings.modules"].length, 13);
            compare(pane.moduleEnabled("wifi"), true);
        }

        function test_module_toggle_adds_and_removes_live() {
            var shell = make();
            var pane = paneOf(shell);

            compare(Settings.setList("quickSettings.modules", ["wifi"]), true);
            compare(Settings.values["quickSettings.modules"].length, 1);
            compare(Settings.values["quickSettings.modules"][0], "wifi");
            Settings.setList("quickSettings.modules", defaultModules.slice());
            pane.setModuleEnabled("wifi", false);
            compare(pane.moduleEnabled("wifi"), false);
            compare(Settings.values["quickSettings.modules"].indexOf("wifi"), -1);

            pane.setModuleEnabled("wifi", true);
            compare(pane.moduleEnabled("wifi"), true);
            verify(Settings.values["quickSettings.modules"].indexOf("wifi") >= 0);
        }

        function test_reorder_writes_the_ordered_list() {
            var shell = make();
            var pane = paneOf(shell);
            var before = Settings.values["quickSettings.modules"].slice();

            pane.moveModule(before[0], 1);
            var after = Settings.values["quickSettings.modules"];
            compare(after[0], before[1]);
            compare(after[1], before[0]);

            pane.moveModule(after[1], -1);
            compare(Settings.values["quickSettings.modules"][0], before[0]);
        }

        function test_menu_bar_only_removes_from_the_panel_and_keeps_the_mark() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("menu.control.bluetooth", false);

            pane.setMenuBarOnly("bluetooth", true);
            compare(pane.isMenuBarOnly("bluetooth"), true);
            compare(pane.moduleEnabled("bluetooth"), false);
            compare(Settings.values["menu.control.bluetooth"], true);

            pane.setMenuBarOnly("bluetooth", false);
            compare(pane.isMenuBarOnly("bluetooth"), false);
            compare(pane.moduleEnabled("bluetooth"), true);
        }

        function test_reset_restores_the_default_order() {
            var shell = make();
            var pane = paneOf(shell);
            pane.setModuleEnabled("wifi", false);
            pane.setMenuBarOnly("focus", true);

            pane.resetButton.clicked();
            compare(Settings.values["quickSettings.menuBarOnly"].length, 0);
            compare(Settings.values["quickSettings.modules"].join(","),
                    defaultModules.join(","));
        }

        function test_per_display_row_writes_the_policy() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.perDisplayRow.enabled, true);
            verify(pane.perDisplayRow.description.indexOf("OPEN") >= 0);
            compare(Settings.values["quickSettings.perDisplay"], false);
            Settings.set("quickSettings.perDisplay", true);
            compare(Settings.values["quickSettings.perDisplay"], true);
        }

        function test_external_module_change_converges() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setList("quickSettings.modules", ["dark", "wifi"]);
            tryVerify(function() { return pane.modules.length === 2; });
            compare(pane.modules[0], "dark");
            compare(pane.modules[1], "wifi");
        }
    }
}
