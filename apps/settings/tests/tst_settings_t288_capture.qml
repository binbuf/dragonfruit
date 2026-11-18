// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T288 headless visual check (not part of ctest): the four connectivity panes
// in their connected, empty, and absent states, light and dark. Run through
// `scripts/capture-t288-settings-connectivity.sh`.
Item {
    id: stage
    width: 1000
    height: 900
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT288Capture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function setScheme(scheme) {
            stage.currentScheme = scheme;
            Settings.set("appearance.colorScheme", scheme);
            waitForRendering(stage);
        }

        function resetFixtures() {
            Settings.resetWifiFixture();
            Settings.setWifiFixture("live");
            Settings.resetBluetoothFixture();
            Settings.setBluetoothFixture("live");
            Settings.resetNetworkFixture();
            Settings.setNetworkFixture("live");
            Settings.resetVpnFixture();
            Settings.setVpnFixture("live");
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function capture(shell, name) {
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t288-" + name + "-"
                                  + stage.currentScheme + ".png");
        }

        function run(scheme) {
            resetFixtures();
            setScheme(scheme);
            var shell = make();

            // Wi-Fi: connected, empty scan, absent NetworkManager.
            Settings.setWifiFixture("live");
            showPane(shell, "wifi");
            capture(shell, "wifi-connected");
            Settings.setWifiFixture("empty");
            showPane(shell, "wifi");
            capture(shell, "wifi-empty");
            Settings.setWifiFixture("absent");
            showPane(shell, "wifi");
            capture(shell, "wifi-absent");
            Settings.setWifiFixture("live");

            // Bluetooth: devices, no devices, absent BlueZ.
            Settings.setBluetoothFixture("live");
            showPane(shell, "bluetooth");
            capture(shell, "bluetooth-connected");
            Settings.setBluetoothFixture("empty");
            showPane(shell, "bluetooth");
            capture(shell, "bluetooth-empty");
            Settings.setBluetoothFixture("absent");
            showPane(shell, "bluetooth");
            capture(shell, "bluetooth-absent");
            Settings.setBluetoothFixture("live");

            // Network: live services, no firewall detected, absent NM.
            Settings.setNetworkFixture("live");
            showPane(shell, "network");
            capture(shell, "network-connected");
            Settings.setNetworkFixture("no-firewall");
            showPane(shell, "network");
            capture(shell, "network-no-firewall");
            Settings.setNetworkFixture("nm-absent");
            Settings.setVpnFixture("absent");
            showPane(shell, "network");
            capture(shell, "network-absent");
            Settings.setNetworkFixture("live");
            Settings.setVpnFixture("live");

            // VPN: configured, empty, absent VPN service.
            Settings.setVpnFixture("live");
            showPane(shell, "vpn");
            capture(shell, "vpn-configured");
            Settings.setVpnFixture("empty");
            showPane(shell, "vpn");
            capture(shell, "vpn-empty");
            Settings.setVpnFixture("absent");
            showPane(shell, "vpn");
            capture(shell, "vpn-absent");
            Settings.setVpnFixture("live");

            shell.destroy();
        }

        function test_capture_light() {
            run("light");
        }

        function test_capture_dark() {
            run("dark");
        }
    }
}
