// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// The optional top-level VPN pane (T272). The sidebar shows it when
// NetworkManager reports a profile; Network -> VPN remains the fallback.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsVpn"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.resetVpnFixture();
            Settings.resetNetworkFixture();
            Settings.setVpnFixture("live");
        }

        function make(paneId) {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane(paneId || "vpn");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function test_the_pane_lists_connections_and_connects() {
            var shell = make();
            var pane = shell.paneBody.item;
            compare(pane.ready, true);
            compare(pane.connections.length, 2);
            compare(pane.connectionRepeater.count, 2);
            var row = pane.connectionRepeater.itemAt(1);
            compare(row.label, "Home");
            var toggle = findChild(row, "vpnConnectToggle");
            compare(toggle.checked, false);
            toggle.toggle();
            tryVerify(function() { return Settings.vpn.activeName === "Home"; });
        }

        function test_info_and_auto_connect() {
            var shell = make();
            var pane = shell.paneBody.item;
            var row = pane.connectionRepeater.itemAt(0);
            findChild(row, "vpnInfoButton").clicked();
            tryVerify(function() { return pane.infoDialog.open === true; });
            compare(pane.infoConnection.id, "Work VPN");
            pane.infoDialog.hide();

            var auto = findChild(row, "vpnAutoToggle");
            auto.toggle();
            tryVerify(function() { return Settings.vpn.connections[0].autoconnect === false; });
        }

        function test_add_vpn_validates_and_adds() {
            var shell = make();
            var pane = shell.paneBody.item;
            pane.addVpnRow.trailingButton.clicked();
            tryVerify(function() { return pane.addVpnDialog.open === true; });
            findChild(pane, "vpnAddConfirm").clicked();
            verify(findChild(pane, "vpnAddResult").text
                   .indexOf("required") >= 0);
            findChild(pane, "vpnAddName").text = "Lab VPN";
            findChild(pane, "vpnAddConfirm").clicked();
            tryVerify(function() { return Settings.vpn.connections.length === 3; });
        }

        function test_no_plugin_disables_add() {
            var shell = make();
            var pane = shell.paneBody.item;
            Settings.setVpnFixture("no-plugin");
            tryVerify(function() { return Settings.vpn.plugins.length === 0; });
            compare(pane.addVpnRow.trailingAction, "");
            compare(pane.addVpnRow.enabled, false);
        }

        function test_a_failed_write_is_reported() {
            var shell = make();
            var pane = shell.paneBody.item;
            Settings.failNextVpnWrite("connect", "not authorized");
            Settings.connectVpn("22222222-2222-2222-2222-222222222222");
            tryVerify(function() { return Settings.vpnWriteState === "failed"; });
            tryVerify(function() { return pane.writeNote.visible === true; });
            verify(findChild(pane, "vpnWriteNote").text
                   .indexOf("not authorized") >= 0);
        }

        function test_the_optional_pane_is_gated_on_profiles() {
            var shell = make();
            var ids = shell.visiblePanes.map(function(entry) { return entry.id; });
            verify(ids.indexOf("vpn") >= 0);
            Settings.setVpnFixture("empty");
            tryVerify(function() {
                var visible = shell.visiblePanes.map(function(entry) { return entry.id; });
                return visible.indexOf("vpn") < 0;
            });
        }
    }
}
