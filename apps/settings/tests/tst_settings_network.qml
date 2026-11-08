// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Network advanced (VPN) pane tests (T-15.15b). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_VPN_FIXTURE` (see CMakeLists.txt), so the
// `Settings` singleton and the NetworkManager bridge client are deterministic
// and in-process with no bus. The cases cover the live connection list, the
// connect/disconnect round-trip, and the absence/empty gating.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsNetwork"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture is process-global; reset it before every case so each
        // starts on the known OpenVPN-up / WireGuard-idle store.
        function init() {
            Settings.resetVpnFixture();
            Settings.resetNetworkFixture();
            Settings.resetWifiFixture();
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("network");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Network pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_live_connections() {
            var shell = make();
            compare(shell.currentPaneId, "network");
            compare(shell.currentPane.title, "Network");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.available, true);
            compare(pane.connectionCount, 2);
            compare(pane.connectionRepeater.count, 2);
            compare(pane.vpnGroup.visible, true);
            compare(pane.absenceNote.visible, false);
            compare(findChild(pane, "networkEmptyNote").visible, false);
            // The first connection is the connected OpenVPN link.
            var row = pane.connectionRepeater.itemAt(0);
            compare(row.label, "Work VPN");
            var toggle = findChild(row, "networkVpnToggle");
            verify(toggle !== null);
            compare(toggle.checked, true);
        }

        function test_connect_and_disconnect_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            // Work VPN starts connected; disconnect it.
            var first = pane.connectionRepeater.itemAt(0);
            var firstToggle = findChild(first, "networkVpnToggle");
            firstToggle.toggle();
            tryVerify(function() { return Settings.vpn.connectedCount === 0; });
            compare(Settings.vpn.label, "Not Connected");
            compare(Settings.vpn.glyph, "vpn-off");

            // Connect the idle WireGuard link.
            var second = pane.connectionRepeater.itemAt(1);
            compare(second.label, "Home");
            var secondToggle = findChild(second, "networkVpnToggle");
            secondToggle.toggle();
            tryVerify(function() { return Settings.vpn.connectedCount === 1; });
            compare(Settings.vpn.activeName, "Home");
        }

        function test_the_toggle_follows_the_live_state() {
            var shell = make();
            var pane = paneOf(shell);
            var second = pane.connectionRepeater.itemAt(1);
            var secondToggle = findChild(second, "networkVpnToggle");
            compare(secondToggle.checked, false);
            // A change from elsewhere (the host's pushed state) converges. The
            // Repeater rebuilds its delegates when the view changes, so re-fetch
            // the live row rather than hold the old object.
            Settings.connectVpn("22222222-2222-2222-2222-222222222222");
            tryVerify(function() {
                var row = pane.connectionRepeater.itemAt(1);
                var toggle = row ? findChild(row, "networkVpnToggle") : null;
                return toggle !== null && toggle.checked === true;
            });
        }

        function test_the_absence_and_empty_notes_are_gated() {
            var shell = make();
            var pane = paneOf(shell);
            // The host answers, so the absence note is hidden.
            compare(pane.absenceNote.visible, false);
            compare(pane.vpnGroup.visible, true);
        }

        function serviceRow(pane, id) {
            for (var i = 0; i < pane.servicesRepeater.count; ++i) {
                var row = pane.servicesRepeater.itemAt(i);
                if (row && String(row.modelData.id) === id)
                    return row;
            }
            return null;
        }

        function test_services_are_listed_and_reordered() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.ready, true);
            // The fixture's Other Services: Wired and Thunderbolt Bridge
            // (Wi-Fi and VPN have their own surfaces).
            compare(pane.otherServices.length, 2);
            var wired = serviceRow(pane, "Wired");
            verify(wired !== null);
            compare(wired.modelData.connected, true);
            findChild(wired, "networkServiceDown").clicked();
            tryVerify(function() {
                var services = Settings.network.services;
                for (var i = 0; i < services.length; ++i)
                    if (services[i].id === "Wired")
                        return services[i].order > 0;
                return false;
            });
        }

        function test_a_service_can_be_added() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addServiceRow.trailingButton.clicked();
            tryVerify(function() { return pane.addServiceDialog.open === true; });
            findChild(pane, "networkAddServiceName").text = "Lab Ethernet";
            findChild(pane, "networkAddServiceConfirm").clicked();
            tryVerify(function() {
                for (var i = 0; i < Settings.network.services.length; ++i)
                    if (Settings.network.services[i].id === "Lab Ethernet")
                        return true;
                return false;
            });
        }

        function test_the_firewall_toggle_and_rules() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.firewallGroup.visible, true);
            compare(pane.firewallToggle.checked, false);
            pane.firewallToggle.toggle();
            tryVerify(function() { return Settings.network.firewall.enabled === true; });
            compare(pane.firewallToggle.checked, true);
            pane.firewallToggle.toggle();
            tryVerify(function() { return Settings.network.firewall.enabled === false; });

            pane.firewallOptionsRow.trailingButton.clicked();
            tryVerify(function() { return pane.firewallDialog.open === true; });
            findChild(pane, "networkFirewallPortField").text = "8080";
            findChild(pane, "networkFirewallOpenPort").clicked();
            tryVerify(function() {
                var ports = Settings.network.firewall.ports;
                for (var i = 0; i < ports.length; ++i)
                    if (String(ports[i].id) === "8080/tcp")
                        return true;
                return false;
            });
            findChild(pane, "networkFirewallServiceField").text = "mdns";
            findChild(pane, "networkFirewallOpenService").clicked();
            tryVerify(function() {
                return Settings.network.firewall.services.indexOf("mdns") >= 0;
            });
        }

        function test_vpn_plugins_and_add() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.vpn.plugins.length, 2);
            compare(pane.vpnPlugins.length, 2);
            pane.addVpnRow.trailingButton.clicked();
            tryVerify(function() { return pane.addVpnDialog.open === true; });
            findChild(pane, "networkAddVpnName").text = "Lab VPN";
            findChild(pane, "networkAddVpnConfirm").clicked();
            tryVerify(function() { return Settings.vpn.connections.length === 3; });
        }

        function test_no_vpn_plugin_is_an_honest_absence() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setVpnFixture("no-plugin");
            tryVerify(function() { return Settings.vpn.plugins.length === 0; });
            compare(pane.vpnPlugins.length, 0);
            compare(pane.addVpnRow.trailingAction, "");
            Settings.setVpnFixture("live");
            tryVerify(function() { return Settings.vpn.plugins.length === 2; });
        }

        function test_firewall_absence_and_unavailable_are_distinct() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setNetworkFixture("nftables");
            tryVerify(function() { return Settings.network.firewall.backend === "nftables"; });
            compare(pane.firewallDetected, true);
            compare(pane.firewallStatusKnown, false);
            compare(pane.firewallToggle.enabled, false);

            Settings.setNetworkFixture("absent");
            tryVerify(function() { return pane.ready === false; });
            compare(pane.absenceNote.visible, true);
            compare(pane.firewallToggle.enabled, false);

            Settings.setNetworkFixture("live");
            tryVerify(function() { return pane.ready === true; });
        }
    }
}