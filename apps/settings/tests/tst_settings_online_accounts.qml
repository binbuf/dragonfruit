// SPDX-License-Identifier: GPL-3.0-only
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
            Settings.setOnlineAttentionFixture(false);
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

        function test_details_sheet_lists_the_projection() {
            var shell = make();
            var pane = paneOf(shell);
            var account = pane.accountRepeater.itemAt(0);
            var details = findChild(account, "onlineAccountsDetailsButton");
            verify(details);
            details.clicked();
            compare(pane.detailsDialog.open, true);
            compare(pane.detailsProvider.text, "Google");
            compare(pane.detailsIdentity.text, "dan@example.com");
            compare(pane.detailsCapabilityRepeater.count, 3);
            // A signed-in account has no sign-in action.
            compare(pane.detailsSignInButton.visible, false);
            pane.detailsDialog.accept();
        }

        function test_sign_in_again_is_the_goa_delegate() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setOnlineAttentionFixture(true);
            tryVerify(function() { return pane.accounts[0].attentionNeeded === true; });
            var account = pane.accountRepeater.itemAt(0);
            var signIn = findChild(account, "onlineAccountsSignInButton");
            verify(signIn);
            compare(signIn.visible, true);
            signIn.clicked();
            tryVerify(function() { return pane.accounts[0].attentionNeeded === false; });
        }

        // T279: the measured 740×820 window keeps the account rows reachable.
        function test_measured_window_740x820_keeps_the_accounts() {
            var measured = createTemporaryObject(shellComponent, stage,
                                                 { width: 740, height: 820 });
            waitForRendering(stage);
            measured.selectPane("online-accounts");
            tryVerify(function() { return measured.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var measuredPane = measured.paneBody.item;
            verify(measuredPane.width > 0 && measuredPane.width <= 740);
            verify(measuredPane.accountRepeater.count === 1);
            verify(measuredPane.accountRepeater.itemAt(0).width <= 740);
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
