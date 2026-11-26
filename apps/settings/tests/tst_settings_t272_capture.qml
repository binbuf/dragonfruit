// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T272 headless visual check (not part of ctest): the Network service list,
// per-service Details sheet, firewall options, the optional VPN pane, the Add
// Service sheet, and the NetworkManager-absent state (with the firewall still
// usable), in light and dark. Run through
// `scripts/capture-t272-network.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT272Capture"
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

        function serviceRow(pane, id) {
            for (var i = 0; i < pane.servicesRepeater.count; ++i) {
                var row = pane.servicesRepeater.itemAt(i);
                if (row && String(row.modelData.id) === id)
                    return row;
            }
            return null;
        }

        function capture(scheme) {
            reset();
            Settings.set("appearance.colorScheme", scheme);
            var shell = make();

            // Services list with status dots.
            var network = showPane(shell, "network");
            compare(network.ready, true);
            grabImage(shell).save("docs/captures/t272-network-services-" + scheme + ".png");

            // Per-service Details sheet.
            var wired = serviceRow(network, "Wired");
            wired.disclosureClicked();
            tryVerify(function() { return network.detailsDialog.open === true; });
            wait(300);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272-network-details-" + scheme + ".png");
            network.detailsDialog.hide();
            wait(300);
            waitForRendering(stage);

            // Firewall options.
            network.firewallOptionsRow.trailingButton.clicked();
            tryVerify(function() { return network.firewallDialog.open === true; });
            wait(300);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272-network-firewall-" + scheme + ".png");
            network.firewallDialog.hide();
            wait(300);
            waitForRendering(stage);

            // Add Service sheet.
            network.addServiceRow.trailingButton.clicked();
            tryVerify(function() { return network.addServiceDialog.open === true; });
            findChild(network, "networkAddServiceName").text = "Lab Ethernet";
            wait(300);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t272-network-add-service-" + scheme + ".png");
            network.addServiceDialog.hide();
            wait(300);
            waitForRendering(stage);

            // Optional top-level VPN pane.
            var vpn = showPane(shell, "vpn");
            compare(vpn.ready, true);
            grabImage(shell).save("docs/captures/t272-network-vpn-" + scheme + ".png");

            // NetworkManager absent: the firewall stays usable and the
            // sections keep their structure.
            var network2 = showPane(shell, "network");
            Settings.setNetworkFixture("nm-absent");
            tryVerify(function() { return network2.ready === false; });
            compare(network2.firewallToggle.enabled, true);
            grabImage(shell).save("docs/captures/t272-network-absence-" + scheme + ".png");
            Settings.setNetworkFixture("live");
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
