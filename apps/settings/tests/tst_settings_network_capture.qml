// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings
import Dragonfruit.MenuBar

// T225 headless visual check (not part of ctest): the Wi-Fi pane, the Network
// pane's firewall/VPN states, the shared Wi-Fi quick sheet, and the absence
// states. Run through `scripts/capture-t225-network-panes.sh`.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsNetworkCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width,
                                                height: stage.height,
                                                highlightHoldMs: 50 });
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

        function wifiRow(pane, ssid) {
            for (var i = 0; i < pane.availableRepeater.count; ++i) {
                var row = pane.availableRepeater.itemAt(i);
                if (row && String(row.modelData.ssid) === ssid)
                    return row;
            }
            return null;
        }

        function knownRow(pane, ssid) {
            for (var i = 0; i < pane.knownRepeater.count; ++i) {
                var row = pane.knownRepeater.itemAt(i);
                if (row && String(row.modelData.ssid) === ssid)
                    return row;
            }
            return null;
        }

        function scrollToBottom(shell) {
            tryVerify(function() { return shell.detailScroll.maxY > 0; });
            shell.detailScroll.flickable.contentY = shell.detailScroll.maxY;
            waitForRendering(stage);
        }

        function init() {
            Settings.setWifiFixture("live");
            Settings.setNetworkFixture("live");
            Settings.setVpnFixture("live");
        }

        function test_capture_panes_and_absence() {
            var shell = make();

            // 1. Wi-Fi pane, live.
            var wifi = showPane(shell, "wifi");
            compare(wifi.ready, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-wifi-pane.png");

            // 2. Join a network (the open guest network) and capture the
            //    resulting current-network row.
            var guest = wifiRow(wifi, "cafe-guest");
            verify(guest !== null);
            findChild(guest, "wifiJoinButton").clicked();
            tryVerify(function() { return Settings.wifi.activeSsid === "cafe-guest"; });
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-wifi-join.png");

            // 3. Forget a saved network.
            var saved = knownRow(wifi, "dragonfruit");
            verify(saved !== null);
            findChild(saved, "wifiForgetButton").clicked();
            tryVerify(function() {
                for (var i = 0; i < Settings.wifi.knownNetworks.length; ++i)
                    if (Settings.wifi.knownNetworks[i].ssid === "dragonfruit")
                        return false;
                return true;
            });
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-wifi-forget.png");

            // 4. The Network pane with the firewall on, then off.
            var network = showPane(shell, "network");
            compare(network.ready, true);
            network.firewallToggle.toggle();
            tryVerify(function() { return Settings.network.firewall.enabled === true; });
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-network-firewall-on.png");
            network.firewallToggle.toggle();
            tryVerify(function() { return Settings.network.firewall.enabled === false; });
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-network-firewall-off.png");

            // 5. Connect the idle VPN and capture the live tunnel.
            Settings.connectVpn("22222222-2222-2222-2222-222222222222");
            tryVerify(function() { return Settings.vpn.connectedCount === 1; });
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-vpn-connected.png");

            // 6. The shared Wi-Fi quick sheet, using the same `Settings.wifi`
            //    model as the pane.
            var quick = createTemporaryObject(quickSheetComponent, stage,
                                              { width: stage.width,
                                                height: stage.height });
            waitForRendering(stage);
            quick.openSheet();
            wait(300);
            waitForRendering(stage);
            // The quick sheet is a child of the stage, so the grab shows it
            // over the same Settings shell the pane uses.
            grabImage(stage).save("docs/captures/t225-wifi-quick-sheet.png");
            quick.destroy();

            // 7. Wi-Fi absence.
            Settings.setWifiFixture("absent");
            wifi = showPane(shell, "wifi");
            tryVerify(function() { return wifi.ready === false; });
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-wifi-absence.png");
            Settings.setWifiFixture("live");

            // 8. Network absence and nftables' unavailable status.
            network = showPane(shell, "network");
            Settings.setNetworkFixture("absent");
            tryVerify(function() { return network.ready === false; });
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-network-absence.png");
            Settings.setNetworkFixture("nftables");
            tryVerify(function() { return network.ready === true; });
            compare(network.firewallStatusKnown, false);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t225-firewall-unavailable.png");

            // 9. No VPN plugin installed. Scroll to the VPN group so the
            //    hidden Add affordance and its reason are in the still.
            Settings.setNetworkFixture("live");
            Settings.setVpnFixture("no-plugin");
            tryVerify(function() { return Settings.vpn.plugins.length === 0; });
            scrollToBottom(shell);
            grabImage(shell).save("docs/captures/t225-vpn-plugin-absence.png");
        }

        Component {
            id: quickSheetComponent
            Item {
                id: quickRoot
                property alias menu: quickMenu
                width: 420
                height: 420

                function openSheet() {
                    quickMenu.popup.open = true;
                }

                // Transparent: the stage grab shows the quick sheet over the
                // Settings shell, the way the menu-bar popover appears.
                Rectangle {
                    anchors.fill: parent
                    color: "transparent"
                }

                Item {
                    id: anchor
                    x: quickRoot.width - 440
                    y: 40
                    width: 1
                    height: 1
                }

                WifiMenu {
                    id: quickMenu
                    objectName: "t225QuickSheet"
                    model: Settings.wifi
                    anchorItem: anchor
                    materialBlurActive: false
                    materialTier: 0
                    materialFallback: 0
                }
            }
        }
    }
}
