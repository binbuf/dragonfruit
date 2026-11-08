// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Online Accounts absence (T228): GOA is optional. With no bridge host (and
// therefore no `org.gnome.OnlineAccounts`) the pane is hidden from the sidebar
// and a deep link is rejected; it is not a dead entry.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsOnlineAccountsAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function test_goa_absent_hides_the_pane() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            compare(Settings.onlineAccountsAvailable, false);
            compare(SettingsPanes.indexOf("online-accounts"), -1);
            var visible = shell.visiblePanes.map(function(pane) { return pane.id; });
            compare(visible.indexOf("online-accounts"), -1);
            verify(!shell.deepLink("settings://pane/online-accounts"));
            // The catalog still knows the pane and its body; only the optional
            // visibility rule hides it.
            verify(SettingsPanes.paneById("online-accounts") !== null);
            verify(shell.paneComponent("online-accounts") !== null);
        }
    }
}
