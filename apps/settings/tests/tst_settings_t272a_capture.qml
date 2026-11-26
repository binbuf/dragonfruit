// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T272a headless visual check (not part of ctest): the firewall backend
// states (ufw as a normal user, nftables, iptables-only, no manager) and the
// VPN Add-path capability states (plugin + editor, native WireGuard form,
// plugin without an editor, no Add path, an incomplete profile), in light and
// dark. Run through `scripts/capture-t272a-network-capability.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT272aCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function reset() {
            Settings.resetNetworkFixture();
            Settings.resetVpnFixture();
            Settings.setNetworkFixture("live");
            Settings.setVpnFixture("live");
        }

        function captureFirewall(shell, pane, mode, scheme, name) {
            Settings.setNetworkFixture(mode);
            tryVerify(function() {
                return Settings.network.firewall.backend === mode
                    || (mode === "no-firewall"
                        && Settings.network.firewall.detected === false);
            });
            wait(200);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272a-firewall-" + name + "-"
                                  + scheme + ".png");
        }

        function capture(scheme) {
            reset();
            Settings.set("appearance.colorScheme", scheme);
            var shell = make();
            var network = showPane(shell, "network");
            compare(network.ready, true);

            captureFirewall(shell, network, "ufw", scheme, "ufw-normal-user");
            captureFirewall(shell, network, "iptables", scheme, "iptables-only");
            captureFirewall(shell, network, "nftables", scheme, "nftables-normal-user");
            captureFirewall(shell, network, "no-firewall", scheme, "no-manager");

            // VPN Add path: plugins first, native WireGuard last.
            Settings.setNetworkFixture("live");
            Settings.setVpnFixture("live");
            var vpn = showPane(shell, "vpn");
            compare(vpn.ready, true);
            vpn.addButton.clicked();
            tryVerify(function() { return vpn.addVpnDialog.open === true; });
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272a-vpn-add-plugin-" + scheme
                                  + ".png");
            var kind = findChild(vpn, "vpnAddKind");
            kind.currentIndex = vpn.addVpnDialog.options.length - 1;
            compare(vpn.addVpnDialog.selectedKind, "wireguard");
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272a-vpn-wireguard-form-" + scheme
                                  + ".png");
            vpn.addVpnDialog.hide();
            wait(200);

            // A plugin installed without an editor: only native WireGuard is
            // offered.
            Settings.setVpnFixture("no-editor");
            tryVerify(function() { return Settings.vpn.editorAvailable === false; });
            vpn.addButton.clicked();
            tryVerify(function() { return vpn.addVpnDialog.open === true; });
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272a-vpn-no-editor-" + scheme
                                  + ".png");
            vpn.addVpnDialog.hide();
            wait(200);

            // No plugin, no native WireGuard, no editor: a named reason.
            Settings.setVpnFixture("no-add");
            tryVerify(function() { return Settings.vpn.canAdd === false; });
            wait(200);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272a-vpn-no-add-" + scheme
                                  + ".png");

            // An incomplete profile is labelled and cannot be connected.
            Settings.setVpnFixture("incomplete");
            tryVerify(function() {
                return Settings.vpn.connections[0].needsSetup === true;
            });
            wait(200);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272a-vpn-needs-setup-" + scheme
                                  + ".png");

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
