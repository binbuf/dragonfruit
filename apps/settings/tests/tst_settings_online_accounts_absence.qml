// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Online Accounts absence (T228/T263/T279): GOA is optional, but the pane is
// always present in the sidebar. With no bridge host (and therefore no GOA
// service) the pane shows its explicit absent-service state and a delegate
// action; it is not a dead entry and it does not silently disappear.
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

        function test_the_absent_pane_states_the_reason_and_delegates() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("online-accounts");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane);
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true);
            compare(findChild(pane.absenceNote, "onlineAccountsAbsenceText").text,
                    "Online accounts are not installed.");
            compare(pane.absenceButton.text, "How to install\u2026");
            pane.absenceButton.clicked();
            compare(pane.delegateDialog.open, true);
            compare(pane.delegateDialog.title, "Online accounts are not installed");
            pane.delegateDialog.accept();
        }
    }
}
