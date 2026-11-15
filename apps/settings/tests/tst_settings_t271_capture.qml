// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T271 headless visual check (not part of ctest): the Wi-Fi pane's live,
// scanning, connected, off, details, hotspot, and absence states plus the
// Bluetooth pane's live, device-sheet, and absence states, in light and dark.
// Run through `scripts/capture-t271-wifi-bluetooth.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT271Capture"
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
            Settings.resetWifiFixture();
            Settings.resetBluetoothFixture();
            Settings.setWifiFixture("live");
            Settings.setBluetoothFixture("live");
            Settings.set("network.wifi.askToJoin", "notify");
            Settings.set("network.wifi.askToJoinHotspots", "ask");
        }

        function capture(scheme) {
            reset();
            Settings.set("appearance.colorScheme", scheme);
            var shell = make();

            // Wi-Fi live.
            var wifi = showPane(shell, "wifi");
            compare(wifi.ready, true);
            grabImage(shell).save("docs/captures/t271-wifi-live-" + scheme + ".png");

            // Wi-Fi scanning.
            Settings.setWifiFixture("scanning");
            tryVerify(function() { return wifi.scanning === true; });
            grabImage(shell).save("docs/captures/t271-wifi-scanning-" + scheme + ".png");

            // Wi-Fi connected: join the open guest network.
            Settings.setWifiFixture("live");
            tryVerify(function() { return wifi.ready === true; });
            for (var i = 0; i < wifi.availableRepeater.count; ++i) {
                var row = wifi.availableRepeater.itemAt(i);
                if (row && String(row.modelData.ssid) === "cafe-guest") {
                    findChild(row, "wifiJoinButton").clicked();
                    break;
                }
            }
            tryVerify(function() { return Settings.wifi.activeSsid === "cafe-guest"; });
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t271-wifi-connected-" + scheme + ".png");

            // Wi-Fi off, then absent — before any dialog opens, so the stills
            // never carry a closing dialog's overlay frame.
            Settings.setWifiFixture("off");
            tryVerify(function() { return wifi.radioEnabled === false; });
            grabImage(shell).save("docs/captures/t271-wifi-off-" + scheme + ".png");
            Settings.setWifiFixture("absent");
            tryVerify(function() { return wifi.ready === false; });
            grabImage(shell).save("docs/captures/t271-wifi-absence-" + scheme + ".png");

            // Current-network Details sheet.
            Settings.setWifiFixture("live");
            tryVerify(function() { return wifi.ready === true; });
            findChild(wifi, "wifiCurrentRow").trailingButton.clicked();
            tryVerify(function() { return wifi.detailsDialog.open === true; });
            wait(300);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t271-wifi-details-" + scheme + ".png");
            wifi.detailsDialog.hide();
            wait(300);
            waitForRendering(stage);

            // Personal hotspot sheet.
            wifi.hotspotRow.trailingButton.clicked();
            tryVerify(function() { return wifi.hotspotDialog.open === true; });
            wait(300);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t271-wifi-hotspot-" + scheme + ".png");
            wifi.hotspotDialog.hide();
            wait(300);
            waitForRendering(stage);

            // Bluetooth live.
            var bluetooth = showPane(shell, "bluetooth");
            compare(bluetooth.ready, true);
            grabImage(shell).save("docs/captures/t271-bluetooth-live-" + scheme + ".png");

            // Device info sheet.
            bluetooth.showDeviceInfo("AA:BB:CC:DD:EE:FF");
            tryVerify(function() { return bluetooth.deviceDialog.open === true; });
            wait(300);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t271-bluetooth-device-" + scheme + ".png");
            bluetooth.deviceDialog.hide();
            wait(300);
            waitForRendering(stage);

            // Bluetooth absent.
            Settings.setBluetoothFixture("absent");
            tryVerify(function() { return bluetooth.ready === false; });
            grabImage(shell).save("docs/captures/t271-bluetooth-absence-" + scheme + ".png");
            Settings.setBluetoothFixture("live");
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
