// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T222 headless visual check (not part of ctest): render the Settings shell
// with the owner debug view open and save the two stills. Run through
// `scripts/capture-t222-settings-map.sh`, which sets `DF_SETTINGS_DEBUG=1`.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsMapCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function test_capture_debug_and_sidebar() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            tryCompare(shell.debugView, "open", true);
            wait(300);
            waitForRendering(stage);

            // The sidebar order is the map's shipped-pane order.
            compare(shell.visiblePanes.length, 22);
            compare(shell.visiblePanes[0].id, "bluetooth");
            compare(shell.visiblePanes[5].id, "appearance");

            // A sample row's owner/absence state is visible in the debug view.
            compare(shell.debugView.pane.owner, "settingsd + compositor");
            compare(shell.debugView.pane.absence, "live-on-defaults");
            verify(shell.debugView.rowList.count > 0);

            var debug = grabImage(shell);
            debug.save("docs/captures/t222-settings-debug.png");

            shell.debugView.reject();
            wait(200);
            waitForRendering(stage);
            var sidebar = grabImage(shell);
            sidebar.save("docs/captures/t222-settings-sidebar.png");
        }
    }
}
