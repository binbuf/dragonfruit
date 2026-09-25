// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Displays-pane tests (T-09.5). Headless with `DF_SETTINGS_FIXTURE`, so the
// `Settings` singleton serves the schema defaults and keeps writes in memory:
// no bus, no daemon, deterministic. Every control is asserted to apply live to
// its `display.*` key on the same event-loop turn, and an external settingsd
// change flows back into the control (the T-09.1b converge rule).
Item {
    id: stage
    width: 900
    height: 720

    TestCase {
        id: testCase
        name: "SettingsDisplays"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global, so reset every key this pane
        // touches to its schema default before each case.
        function init() {
            Settings.set("display.scale", 1.0);
            Settings.set("display.rotation", "normal");
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("displays");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the displays body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_wired_controls() {
            var shell = make();
            compare(shell.currentPaneId, "displays");
            var pane = paneOf(shell);

            compare(pane.displayGroup.title, "Built-in Display");
            compare(pane.resolutionGroup.title, "Resolution");
            compare(pane.rotationGroup.title, "Rotation");
            compare(pane.previewLabel.text, "Built-in Display");
            compare(pane.tileItems.length, 5);
            compare(pane.tileItems[0].modelData.label, "Larger Text");
            compare(pane.tileItems[4].modelData.label, "Most Space");
            // Default (1.0) is the selected tile.
            verify(pane.tileItems[2].selected);
            compare(pane.rotationSelect.currentIndex, 0);
            verify(pane.footer.text.indexOf("scaled resolution") >= 0);
        }

        function test_resolution_tiles_apply_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.tileItems[0].choose(); // Larger Text -> 1.5
            verify(Math.abs(Settings.values["display.scale"] - 1.5) < 0.001);
            verify(pane.tileItems[0].selected);
            verify(!pane.tileItems[2].selected);

            pane.tileItems[4].choose(); // Most Space -> 0.75
            verify(Math.abs(Settings.values["display.scale"] - 0.75) < 0.001);

            // An external change converges back into the selected tile.
            Settings.set("display.scale", 1.25);
            verify(pane.tileItems[1].selected);
            verify(!pane.tileItems[4].selected);
        }

        function test_rotation_select_applies_live() {
            var shell = make();
            var pane = paneOf(shell);

            pane.rotationSelect.activateIndex(3); // 270°
            compare(Settings.values["display.rotation"], "270");
            compare(pane.rotationSelect.currentIndex, 3);

            pane.rotationSelect.activateIndex(1); // 90°
            compare(Settings.values["display.rotation"], "90");

            // External change converges.
            Settings.set("display.rotation", "normal");
            compare(pane.rotationSelect.currentIndex, 0);
        }

        // The SettingsRow lays the label on the left and the control on the
        // right; assert the rotation row so a width regression fails here.
        function test_rotation_row_control_is_right_aligned() {
            var shell = make();
            var pane = paneOf(shell);
            var row = pane.rotationGroup.rows.children[0];
            verify(row);
            verify(row.control.x > row.width * 0.55,
                   "rotation control must sit at the right, x=" + row.control.x
                   + " row.width=" + row.width);
        }
    }
}