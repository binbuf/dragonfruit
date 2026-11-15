// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Online Accounts absence (T228/T263): GOA is optional, but the pane is always
// present in the sidebar. With no bridge host (and therefore no GOA service)
// the pane shows its absent-service state; it is not a dead entry and it does
// not silently disappear. T279 owns the full absent-service surface.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsOnlineAccountsAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function test_goa_absent_keeps_the_pane_visible() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            compare(Settings.onlineAccountsAvailable, false);
            verify(SettingsPanes.indexOf("online-accounts") >= 0);
            var visible = shell.visiblePanes.map(function(pane) { return pane.id; });
            verify(visible.indexOf("online-accounts") >= 0);
            verify(shell.deepLink("settings://pane/online-accounts"));
            verify(SettingsPanes.paneById("online-accounts") !== null);
            verify(shell.paneComponent("online-accounts") !== null);
        }
    }
}
