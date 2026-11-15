// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// The optional top-level VPN pane (T272, hardened by T272a). The sidebar
// shows it when NetworkManager reports a profile; Network -> VPN remains the
// fallback. Add is capability-driven: native WireGuard is not a plugin, a
// plugin profile is only offered when the editor can finish it, and an
// incomplete profile is labelled and cannot be presented as ready.
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
            // A plugin profile is created without credentials and is not
            // presented as ready to connect.
            var added = Settings.vpn.connections.filter(function(entry) {
                return entry.id === "Lab VPN";
            })[0];
            compare(added.needsSetup, true);
            compare(added.connected, false);
            var row = pane.connectionRepeater.itemAt(
                pane.connectionRepeater.count - 1);
            compare(findChild(row, "vpnConnectToggle").visible, false);
            verify(findChild(row, "vpnCompleteSetup") !== null);
        }

        function test_complete_setup_marks_the_profile_ready() {
            var shell = make();
            var pane = shell.paneBody.item;
            Settings.setVpnFixture("incomplete");
            tryVerify(function() {
                return Settings.vpn.connections[0].needsSetup === true;
            });
            var row = pane.connectionRepeater.itemAt(0);
            compare(findChild(row, "vpnConnectToggle").visible, false);
            findChild(row, "vpnCompleteSetup").clicked();
            tryVerify(function() {
                return Settings.vpn.connections[0].needsSetup === false;
            });
            var fresh = pane.connectionRepeater.itemAt(0);
            compare(findChild(fresh, "vpnConnectToggle").visible, true);
            compare(findChild(fresh, "vpnConnectToggle").enabled, true);
        }

        function test_no_plugin_keeps_native_wireguard_as_an_add_path() {
            var shell = make();
            var pane = shell.paneBody.item;
            Settings.setVpnFixture("no-plugin");
            tryVerify(function() { return Settings.vpn.plugins.length === 0; });
            compare(Settings.vpn.wireguardNative, true);
            compare(pane.canAdd, true);
            compare(pane.addVpnRow.trailingAction, "Add\u2026");
        }

        function test_a_host_with_no_add_path_names_the_reason() {
            var shell = make();
            var pane = shell.paneBody.item;
            Settings.setVpnFixture("no-add");
            tryVerify(function() {
                return Settings.vpn.plugins.length === 0
                    && Settings.vpn.wireguardNative === false;
            });
            compare(pane.canAdd, false);
            compare(pane.addVpnRow.trailingAction, "");
            compare(pane.addVpnRow.enabled, false);
            verify(pane.addVpnRow.description.indexOf("No NetworkManager VPN plugin") >= 0);
        }

        function test_a_plugin_without_an_editor_is_not_offered() {
            var shell = make();
            var pane = shell.paneBody.item;
            Settings.setVpnFixture("no-editor");
            tryVerify(function() { return Settings.vpn.editorAvailable === false; });
            // Native WireGuard still works; the plugin entry is not offered.
            compare(pane.canAdd, true);
            pane.addVpnRow.trailingButton.clicked();
            tryVerify(function() { return pane.addVpnDialog.open === true; });
            compare(pane.addVpnDialog.options.length, 1);
            compare(pane.addVpnDialog.options[0].kind, "wireguard");
            pane.addVpnDialog.hide();

            Settings.setVpnFixture("no-wireguard");
            tryVerify(function() { return Settings.vpn.wireguardNative === false; });
            // A plugin is installed and the editor can finish it, so the
            // plugin Add path remains.
            compare(pane.canAdd, true);
            compare(pane.addVpnDialog.options.length, 2);
            compare(pane.addVpnDialog.options[0].kind, "plugin");
        }

        function test_plugin_without_editor_or_wireguard_names_the_install() {
            var shell = make();
            var pane = shell.paneBody.item;
            Settings.setVpnFixture("no-editor-no-wireguard");
            tryVerify(function() {
                return Settings.vpn.plugins.length === 2
                    && Settings.vpn.editorAvailable === false
                    && Settings.vpn.wireguardNative === false;
            });
            compare(pane.canAdd, false);
            verify(pane.addVpnRow.description
                   .indexOf("Install nm-connection-editor") >= 0);
        }

        function test_native_wireguard_form_creates_a_complete_profile() {
            var shell = make();
            var pane = shell.paneBody.item;
            pane.addVpnRow.trailingButton.clicked();
            tryVerify(function() { return pane.addVpnDialog.open === true; });
            var kind = findChild(pane, "vpnAddKind");
            verify(kind !== null);
            // Plugins come first; the native WireGuard entry is last.
            kind.currentIndex = pane.addVpnDialog.options.length - 1;
            compare(pane.addVpnDialog.selectedKind, "wireguard");
            findChild(pane, "vpnAddName").text = "Home WG";
            findChild(pane, "vpnAddPrivateKey").text = "private-key";
            findChild(pane, "vpnAddAddress").text = "10.7.0.2/32";
            findChild(pane, "vpnAddPeerKey").text = "peer-key";
            findChild(pane, "vpnAddEndpoint").text = "vpn.example:51820";
            findChild(pane, "vpnAddDns").text = "1.1.1.1";
            findChild(pane, "vpnAddConfirm").clicked();
            tryVerify(function() {
                for (var i = 0; i < Settings.vpn.connections.length; ++i)
                    if (Settings.vpn.connections[i].id === "Home WG")
                        return true;
                return false;
            });
            var added = Settings.vpn.connections.filter(function(entry) {
                return entry.id === "Home WG";
            })[0];
            compare(added.kind, "wireguard");
            compare(added.needsSetup, false);
            // No `vpn`/WireGuard service-type profile was created.
            compare(added.kindLabel, "WireGuard");
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
