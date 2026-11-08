// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Wi-Fi pane tests (T225). Headless with `DF_SETTINGS_FIXTURE` and
// `DF_WIFI_FIXTURE`, so the `Settings` singleton and the NetworkManager
// Wi-Fi client are deterministic and in-process with no bus. The cases cover
// the live model, the radio toggle, open/secured/hidden joins, forget,
// details, the settingsd prompt policy, and the absence state.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsWifi"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.setWifiFixture("live");
            Settings.set("network.wifi.askToJoin", "notify");
            Settings.set("network.wifi.askToJoinHotspots", "ask");
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("wifi");
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

        function rowWithSsid(pane, ssid, objectName) {
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

        function test_pane_opens_with_the_live_model() {
            var shell = make();
            compare(shell.currentPaneId, "wifi");
            compare(shell.currentPane.title, "Wi-Fi");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.available, true);
            compare(pane.radioEnabled, true);
            compare(pane.activeSsid, "dragonfruit");
            compare(pane.powerToggle.checked, true);
            compare(pane.knownRepeater.count, 3);
            compare(pane.availableRepeater.count, 3);
            compare(pane.absenceNote.visible, false);
            var row = knownRow(pane, "dragonfruit");
            verify(row !== null);
            verify(findChild(row, "wifiForgetButton") !== null);
        }

        function test_the_radio_toggle_writes_once_and_follows_the_view() {
            var shell = make();
            var pane = paneOf(shell);
            pane.powerToggle.toggle();
            tryVerify(function() { return Settings.wifi.radioEnabled === false; });
            compare(pane.powerToggle.checked, false);
            compare(pane.availableRepeater.count, 0);
            wait(200);
            pane.powerToggle.toggle();
            tryVerify(function() { return Settings.wifi.radioEnabled === true; });
            compare(pane.powerToggle.checked, true);
        }

        function test_join_an_open_network() {
            var shell = make();
            var pane = paneOf(shell);
            var row = rowWithSsid(pane, "cafe-guest");
            verify(row !== null);
            findChild(row, "wifiJoinButton").clicked();
            tryVerify(function() { return Settings.wifi.activeSsid === "cafe-guest"; });
            compare(pane.activeSsid, "cafe-guest");
        }

        function test_join_a_secured_network_prompts_for_a_password() {
            var shell = make();
            var pane = paneOf(shell);
            var row = rowWithSsid(pane, "neighbor-secure");
            verify(row !== null);
            findChild(row, "wifiJoinButton").clicked();
            tryVerify(function() { return pane.joinDialog.open === true; });
            var field = findChild(pane, "wifiJoinSecret");
            verify(field !== null);
            field.text = "hunter2";
            findChild(pane, "wifiJoinConfirm").clicked();
            tryVerify(function() { return Settings.wifi.activeSsid === "neighbor-secure"; });
            compare(pane.joinDialog.open, false);
        }

        function test_forget_a_known_network() {
            var shell = make();
            var pane = paneOf(shell);
            var row = knownRow(pane, "dragonfruit");
            verify(row !== null);
            findChild(row, "wifiForgetButton").clicked();
            tryVerify(function() {
                for (var i = 0; i < Settings.wifi.knownNetworks.length; ++i)
                    if (Settings.wifi.knownNetworks[i].ssid === "dragonfruit")
                        return false;
                return true;
            });
            compare(Settings.wifi.activeSsid, "");
        }

        function test_join_a_hidden_network() {
            var shell = make();
            var pane = paneOf(shell);
            var hidden = findChild(pane, "wifiHiddenRow");
            verify(hidden !== null);
            hidden.trailingButton.clicked();
            tryVerify(function() { return pane.hiddenDialog.open === true; });
            findChild(pane, "wifiHiddenSsid").text = "lab-network";
            findChild(pane, "wifiHiddenSecret").text = "secret";
            findChild(pane, "wifiHiddenConfirm").clicked();
            tryVerify(function() { return Settings.wifi.activeSsid === "lab-network"; });
            tryVerify(function() {
                for (var i = 0; i < Settings.wifi.knownNetworks.length; ++i)
                    if (Settings.wifi.knownNetworks[i].ssid === "lab-network")
                        return true;
                return false;
            });
        }

        function test_details_shows_the_live_network() {
            var shell = make();
            var pane = paneOf(shell);
            var current = findChild(pane, "wifiCurrentRow");
            verify(current !== null);
            current.trailingButton.clicked();
            tryVerify(function() { return pane.detailsDialog.open === true; });
            compare(pane.detailsTitle.text, "dragonfruit");
            verify(pane.detailsBody.children.length > 0);
        }

        function test_advanced_shows_the_active_service() {
            var shell = make();
            var pane = paneOf(shell);
            pane.advancedDialog.show();
            tryVerify(function() { return pane.advancedDialog.open === true; });
            var iface = findChild(pane, "wifiAdvancedInterface");
            var hardware = findChild(pane, "wifiAdvancedHardware");
            verify(iface !== null);
            verify(hardware !== null);
            // The saved connection id ("Home Wi-Fi") differs from the SSID
            // ("dragonfruit"); the UUID join must still find the device.
            compare(iface.text, "Interface: wlan0");
            compare(hardware.text, "Hardware address: aa:bb:cc:dd:ee:01");
        }

        function test_the_prompt_policy_writes_the_settingsd_keys() {
            var shell = make();
            var pane = paneOf(shell);
            pane.askToJoinSelect.activateIndex(0);
            tryVerify(function() {
                return Settings.values["network.wifi.askToJoin"] === "off";
            });
            pane.askHotspotsSelect.activateIndex(0);
            tryVerify(function() {
                return Settings.values["network.wifi.askToJoinHotspots"] === "off";
            });
        }

        function test_the_absence_state_disables_the_controls() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWifiFixture("absent");
            tryVerify(function() { return pane.ready === false; });
            compare(pane.available, false);
            compare(pane.powerToggle.enabled, false);
            compare(pane.absenceNote.visible, true);
            Settings.setWifiFixture("live");
            tryVerify(function() { return pane.ready === true; });
        }
    }
}
