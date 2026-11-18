// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Network and optional VPN pane tests (T225, T272). Headless with
// `DF_SETTINGS_FIXTURE`, `DF_NETWORK_FIXTURE`, and `DF_VPN_FIXTURE`, so the
// `Settings` singleton and the NetworkManager bridge client are deterministic
// and in-process with no bus.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsNetwork"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture is process-global; reset it before every case so each
        // starts on the known wired/Wi-Fi/bridge + OpenVPN-up store.
        function init() {
            Settings.resetVpnFixture();
            Settings.resetNetworkFixture();
            Settings.resetWifiFixture();
            Settings.setNetworkFixture("live");
            Settings.setVpnFixture("live");
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

        function rowFor(pane, id) {
            for (var i = 0; i < pane.servicesRepeater.count; ++i) {
                var row = pane.servicesRepeater.itemAt(i);
                if (row && String(row.modelData.id) === id)
                    return row;
            }
            return null;
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
            var first = pane.connectionRepeater.itemAt(0);
            var firstToggle = findChild(first, "networkVpnToggle");
            firstToggle.toggle();
            tryVerify(function() { return Settings.vpn.connectedCount === 0; });
            compare(Settings.vpn.label, "Not Connected");
            compare(Settings.vpn.glyph, "vpn-off");

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
            compare(pane.absenceNote.visible, false);
            compare(pane.vpnGroup.visible, true);
        }

        // T272: each service row carries a status dot and a Details sheet with
        // the TCP/IP, DNS, search-domain, hardware, and Linux-only rows.
        function test_services_have_status_dots_and_details() {
            var shell = make();
            var pane = paneOf(shell);
            var wired = rowFor(pane, "Wired");
            verify(wired !== null);
            var dot = findChild(wired, "networkServiceStatusDot");
            verify(dot !== null);
            compare(dot.color, Theme.color.success);

            findChild(wired, "networkServiceDetails").clicked();
            tryVerify(function() { return pane.detailsDialog.open === true; });
            compare(pane.detailsService.id, "Wired");
            var labels = pane.detailRows.map(function(entry) { return entry.label; });
            verify(labels.indexOf("Subnet Mask") >= 0);
            verify(labels.indexOf("Router") >= 0);
            verify(labels.indexOf("DNS Servers") >= 0);
            verify(labels.indexOf("Search Domains") >= 0);
            verify(labels.indexOf("IPv6") >= 0);
            verify(labels.indexOf("MAC Randomization") >= 0);
            verify(labels.indexOf("Metered") >= 0);
            verify(labels.indexOf("802.1X") >= 0);
            var values = pane.detailRows.map(function(entry) { return entry.value; });
            verify(values.indexOf("255.255.255.0") >= 0);
            verify(values.indexOf("192.168.1.1") >= 0);
            pane.detailsDialog.hide();
        }

        // T272: the actions menu owns activate/deactivate, order, make
        // inactive, delete, and add. The row no longer carries a pair of Up /
        // Down buttons.
        function test_the_service_actions_menu_reorders_and_deletes() {
            var shell = make();
            var pane = paneOf(shell);
            var wired = rowFor(pane, "Wired");
            verify(wired !== null);
            compare(findChild(wired, "networkServiceActions") !== null, true);
            pane.openServiceMenu(wired.modelData, wired);
            var actions = pane.serviceMenu.model.map(function(entry) {
                return entry.action || entry.type;
            });
            verify(actions.indexOf("up") >= 0);
            verify(actions.indexOf("down") >= 0);
            verify(actions.indexOf("inactive") >= 0);
            verify(actions.indexOf("delete") >= 0);
            verify(actions.indexOf("add") >= 0);

            pane.runServiceAction("down");
            tryVerify(function() {
                var services = Settings.network.services;
                for (var i = 0; i < services.length; ++i)
                    if (services[i].id === "Wired")
                        return services[i].order > 0;
                return false;
            });
            pane.serviceMenu.hide();

            var fresh = rowFor(pane, "Wired");
            pane.openServiceMenu(fresh.modelData, fresh);
            pane.runServiceAction("delete");
            tryVerify(function() {
                for (var i = 0; i < Settings.network.services.length; ++i)
                    if (Settings.network.services[i].id === "Wired")
                        return false;
                return true;
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

        // T272: validation is visible and a failed add does not create a row.
        function test_add_service_validation() {
            var shell = make();
            var pane = paneOf(shell);
            pane.addServiceRow.trailingButton.clicked();
            tryVerify(function() { return pane.addServiceDialog.open === true; });
            var before = Settings.network.services.length;
            findChild(pane, "networkAddServiceConfirm").clicked();
            tryVerify(function() {
                return findChild(pane, "networkAddServiceResult").text.length > 0;
            });
            compare(Settings.network.services.length, before);
            verify(findChild(pane, "networkAddServiceResult").text
                   .indexOf("required") >= 0);
            pane.addServiceDialog.hide();
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
            findChild(pane, "vpnAddName").text = "Lab VPN";
            findChild(pane, "vpnAddConfirm").clicked();
            tryVerify(function() { return Settings.vpn.connections.length === 3; });
            var added = Settings.vpn.connections.filter(function(entry) {
                return entry.id === "Lab VPN";
            })[0];
            compare(added.needsSetup, true);
        }

        function test_no_vpn_plugin_keeps_native_wireguard_and_no_add_is_honest() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setVpnFixture("no-plugin");
            tryVerify(function() { return Settings.vpn.plugins.length === 0; });
            compare(pane.vpnPlugins.length, 0);
            // Native WireGuard is a separate capability from a VPN plugin.
            compare(Settings.vpn.wireguardNative, true);
            compare(pane.vpnCanAdd, true);
            compare(pane.addVpnRow.trailingAction, "Add\u2026");
            compare(pane.vpnGroup.visible, true);

            Settings.setVpnFixture("no-add");
            tryVerify(function() { return Settings.vpn.wireguardNative === false; });
            compare(pane.vpnCanAdd, false);
            compare(pane.addVpnRow.trailingAction, "");
            verify(pane.addVpnRow.description.indexOf("No NetworkManager VPN plugin") >= 0);
            Settings.setVpnFixture("live");
            tryVerify(function() { return Settings.vpn.plugins.length === 2; });
        }

        function test_an_incomplete_vpn_row_is_not_presented_as_ready() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setVpnFixture("incomplete");
            tryVerify(function() {
                return Settings.vpn.connections[0].needsSetup === true;
            });
            var row = pane.connectionRepeater.itemAt(0);
            compare(findChild(row, "networkVpnToggle").visible, false);
            verify(findChild(row, "networkVpnCompleteSetup") !== null);
            findChild(row, "networkVpnCompleteSetup").clicked();
            tryVerify(function() {
                return Settings.vpn.connections[0].needsSetup === false;
            });
            var fresh = pane.connectionRepeater.itemAt(0);
            compare(findChild(fresh, "networkVpnToggle").visible, true);
            compare(findChild(fresh, "networkVpnToggle").enabled, true);
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

        // T272a: a normal user on Ubuntu can read ufw's on/off state and toggle
        // it, but the rule table needs the host's own tool.
        function test_ufw_as_a_normal_user_reads_status_and_names_the_rules_tool() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setNetworkFixture("ufw");
            tryVerify(function() { return Settings.network.firewall.backend === "ufw"; });
            compare(pane.firewallDetected, true);
            compare(pane.firewallStatusKnown, true);
            compare(pane.firewallEnabled, true);
            compare(pane.firewallToggle.enabled, true);
            compare(pane.firewallRulesPresent, false);
            compare(pane.firewallManageWith, "ufw");
            verify(pane.firewallNote.indexOf("sudo ufw status verbose") >= 0);
            // The rule sheet controls cannot complete, so they are disabled.
            pane.firewallOptionsRow.trailingButton.clicked();
            tryVerify(function() { return pane.firewallDialog.open === true; });
            compare(findChild(pane, "networkFirewallOpenPort").enabled, false);
            pane.firewallDialog.hide();
        }

        // T272a: iptables-only is named, never "No firewall detected".
        function test_iptables_only_is_named_and_read_only() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setNetworkFixture("iptables");
            tryVerify(function() { return Settings.network.firewall.backend === "iptables"; });
            compare(pane.firewallDetected, true);
            compare(pane.firewallStatusKnown, false);
            compare(pane.firewallBackendLabel, "iptables");
            compare(pane.firewallToggle.enabled, false);
            compare(pane.firewallRulesPresent, true);
            compare(pane.firewallManageWith, "iptables");
            verify(pane.firewallNote.indexOf("iptables rules present") >= 0);
        }

        // T272a: the only "No firewall detected" state is a host with no
        // manager at all.
        function test_no_manager_is_the_only_no_firewall_state() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setNetworkFixture("no-firewall");
            tryVerify(function() { return Settings.network.firewall.detected === false; });
            compare(pane.firewallDetected, false);
            compare(pane.firewallBackendLabel, "No firewall detected");
            compare(pane.firewallToggle.enabled, false);
        }

        // T272: a missing NetworkManager leaves the firewall usable; the pane
        // keeps its sections and only disables the NetworkManager controls.
        function test_the_firewall_is_independent_of_networkmanager() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setNetworkFixture("nm-absent");
            tryVerify(function() { return pane.ready === false; });
            compare(pane.absenceNote.visible, true);
            compare(pane.firewallGroup.visible, true);
            compare(pane.firewallDetected, true);
            compare(pane.firewallToggle.enabled, true);
            compare(pane.addServiceRow.enabled, false);
            compare(pane.vpnGroup.visible, true);

            pane.firewallToggle.toggle();
            tryVerify(function() { return Settings.network.firewall.enabled === true; });
            compare(pane.firewallToggle.checked, true);
        }

        // T272: a read-only VPN adapter does not disable the firewall.
        function test_a_read_only_vpn_does_not_disable_the_firewall() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setVpnFixture("read-only");
            tryVerify(function() { return Settings.vpn.readOnly === true; });
            compare(pane.vpnReadOnly, true);
            compare(pane.firewallReadOnly, false);
            compare(pane.firewallToggle.enabled, true);
            pane.firewallToggle.toggle();
            tryVerify(function() { return Settings.network.firewall.enabled === true; });
        }

        // T272: every write reports its outcome; a denied write reverts the
        // control and shows the reason.
        function test_a_failed_write_is_reported() {
            var shell = make();
            var pane = paneOf(shell);
            var wired = rowFor(pane, "Wired");
            Settings.failNextNetworkWrite("deactivate", "not authorized");
            Settings.deactivateNetworkService(String(wired.modelData.uuid));
            tryVerify(function() { return Settings.networkWriteState === "failed"; });
            tryVerify(function() { return pane.writeNote.visible === true; });
            verify(findChild(pane, "networkWriteNote").text
                   .indexOf("not authorized") >= 0);
            // The fixture did not change because the write failed.
            compare(Settings.network.services[0].connected, true);
        }

        // T272: the optional VPN pane appears in Connectivity only when a
        // profile exists; Network -> VPN stays available otherwise.
        function test_the_optional_vpn_pane_follows_the_profiles() {
            var shell = make();
            var ids = shell.visiblePanes.map(function(entry) { return entry.id; });
            verify(ids.indexOf("vpn") >= 0);
            verify(ids.indexOf("network") >= 0);

            shell.selectPane("vpn");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            var vpn = shell.paneBody.item;
            compare(vpn.ready, true);
            compare(vpn.connections.length, 2);
            compare(vpn.connectionRepeater.count, 2);

            // Info is a real sheet.
            var row = vpn.connectionRepeater.itemAt(0);
            findChild(row, "vpnInfoButton").clicked();
            tryVerify(function() { return vpn.infoDialog.open === true; });
            vpn.infoDialog.hide();

            // T285: VPN is part of the stable 27-entry IA, so an empty
            // profile list keeps the pane in the sidebar. T288 owns the
            // pane's empty-state body.
            Settings.setVpnFixture("empty");
            tryVerify(function() {
                var visible = shell.visiblePanes.map(function(entry) { return entry.id; });
                return visible.indexOf("vpn") >= 0;
            });
            compare(SettingsPanes.optionalVisible(
                        SettingsPanes.paneById("vpn"),
                        { connections: [] }), true);
        }

        // T272: the pane is measured at the 740x820 window and the service
        // rows stay inside the detail column.
        function test_the_pane_fits_the_measured_window() {
            var shell = make();
            compare(shell.width, 740);
            compare(shell.height, 820);
            var pane = paneOf(shell);
            verify(pane.width > 0);
            verify(pane.width <= shell.width);
            var wired = rowFor(pane, "Wired");
            verify(wired !== null);
            verify(wired.width <= pane.width);
        }
    }
}
