// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Online Accounts pane tests (T228). Headless with `DF_SETTINGS_FIXTURE` and
// `DF_ONLINE_ACCOUNTS_FIXTURE`: the deterministic in-process client serves the
// GOA provider/account fixture with no session bus.
Item {
    id: stage
    width: 1000
    height: 1000

    TestCase {
        id: testCase
        name: "SettingsOnlineAccounts"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.resetOnlineAccountsFixture();
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("online-accounts");
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

        function test_pane_lists_the_goa_providers_and_accounts() {
            var shell = make();
            compare(shell.currentPaneId, "online-accounts");
            compare(Settings.onlineAccountsAvailable, true);
            compare(SettingsPanes.indexOf("online-accounts") >= 0, true);
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.providers.length, 2);
            compare(pane.accounts.length, 1);
            compare(pane.accountRepeater.count, 1);
            compare(pane.emptyNote.visible, false);
            compare(pane.absenceNote.visible, false);
        }

        function test_sync_toggle_writes_and_converges() {
            var shell = make();
            var pane = paneOf(shell);
            var account = pane.accountRepeater.itemAt(0);
            verify(account);
            var toggle = findChild(account, "onlineAccountsSyncToggle");
            verify(toggle);
            compare(toggle.checked, true);
            toggle.toggled(false);
            tryVerify(function() {
                return pane.accounts[0].capabilities[0].enabled === false;
            });
        }

        function test_add_and_remove_an_account() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addButton.clicked();
            compare(pane.addDialog.open, true);
            pane.providerSelect.activateIndex(1); // Nextcloud
            pane.identityField.text = "dan@cloud.example";
            pane.submitAdd();
            tryVerify(function() { return pane.accounts.length === 2; });
            compare(pane.accounts[1].provider, "nextcloud");

            var account = pane.accountRepeater.itemAt(1);
            var remove = findChild(account, "onlineAccountsRemoveButton");
            verify(remove);
            remove.clicked();
            compare(pane.removeDialog.open, true);
            var confirm = findChild(pane.removeDialog, "onlineAccountsRemoveConfirm");
            verify(confirm);
            confirm.clicked();
            tryVerify(function() { return pane.accounts.length === 1; });
        }

        function test_remove_dialog_confirms_before_writing() {
            var shell = make();
            var pane = paneOf(shell);
            var account = pane.accountRepeater.itemAt(0);
            var remove = findChild(account, "onlineAccountsRemoveButton");
            remove.clicked();
            compare(pane.removeDialog.open, true);
            pane.removeDialog.reject();
            compare(pane.accounts.length, 1);
        }
    }
}
