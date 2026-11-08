// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Game Controllers-pane tests (T229). Headless with `DF_SETTINGS_FIXTURE`
// and `DF_INPUT_FIXTURE`, so the shared input fixture supplies one controller
// and no host bus is touched. The test read and the mapping sheet are the
// pane's two actions; the no-controller absence state is covered by
// tst_settings_absence's per-pane sweep.
Item {
    id: stage
    width: 900
    height: 1000

    TestCase {
        id: testCase
        name: "SettingsControllers"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("game-controllers");
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

        function test_controllers_list_and_actions() {
            var shell = make();
            compare(shell.currentPaneId, "game-controllers");
            var pane = paneOf(shell);
            verify(pane.present, "the fixture input view is available");
            compare(pane.controllerRepeater.count, 1);
            compare(pane.controllers[0].name, "Microsoft X-Box 360 pad");
            compare(pane.controllers[0].identity, "045E:028E");
            verify(!pane.emptyNote.visible);
        }

        function test_controller_test_reads_one_event() {
            var shell = make();
            var pane = paneOf(shell);
            pane.openTest(pane.controllers[0]);
            tryVerify(function() { return pane.testDialog.open; });
            compare(Settings.controllerTest.outcome, "event");
            compare(pane.testDialog.title, "Test Microsoft X-Box 360 pad");
            pane.testDialog.accept();
            wait(250);
        }

        function test_mapping_sheet_is_read_only_and_explains_absence() {
            var shell = make();
            var pane = paneOf(shell);
            pane.openMapping(pane.controllers[0]);
            tryVerify(function() { return pane.mappingDialog.open; });
            verify(pane.mappingText.text.indexOf("No SDL mapping") >= 0);
            pane.mappingDialog.accept();
            wait(250);
        }
    }
}
