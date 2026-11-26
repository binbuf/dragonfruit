// SPDX-License-Identifier: GPL-3.0-only
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
            Settings.resetWifiFixture();
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
            // T288: the mockup intro opens the pane and the current row is an
            // icon row with a category tile.
            verify(pane.intro.text.indexOf("Set up Wi-Fi") >= 0);
            verify(pane.currentRow.category !== null);
            var row = knownRow(pane, "dragonfruit");
            verify(row !== null);
            verify(findChild(row, "wifiKnownDetailsButton") !== null);
            verify(row.category !== null);
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
            findChild(row, "wifiKnownDetailsButton").clicked();
            tryVerify(function() { return pane.detailsDialog.open === true; });
            pane.detailsForgetButton.clicked();
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
            var other = pane.otherButton;
            verify(other !== null);
            other.clicked();
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

        function test_the_absence_state_keeps_the_sections_and_disables_host_controls() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWifiFixture("absent");
            tryVerify(function() { return pane.ready === false; });
            compare(pane.available, false);
            compare(pane.powerToggle.enabled, false);
            compare(pane.absenceNote.visible, true);
            // T271: structure survives the absent daemon.
            compare(pane.currentGroup.visible, true);
            compare(pane.knownGroup.visible, true);
            compare(pane.availableGroup.visible, true);
            compare(pane.hotspotGroup.visible, true);
            compare(pane.advancedRow.visible, true);
            compare(pane.unavailableRow.visible, true);
            compare(pane.knownEmptyRow.visible, true);
            // The hidden-network join is a host write; it disables with the
            // radio and the list controls.
            compare(pane.otherButton.enabled, false);
            // The settingsd prompt rows stay live.
            verify(pane.askToJoinSelect.enabled);
            Settings.setWifiFixture("live");
            tryVerify(function() { return pane.ready === true; });
        }

        function test_scanning_state_shows_the_spinner_and_hides_the_list() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWifiFixture("scanning");
            tryVerify(function() { return pane.scanning === true; });
            compare(pane.scanningRow.visible, true);
            verify(pane.spinner !== null);
            compare(pane.availableRepeater.count, 0);
            // A read answers the scan.
            Settings.refreshWifi();
            tryVerify(function() { return pane.scanning === false; });
            compare(pane.scanningRow.visible, false);
        }

        function test_empty_state_is_an_explicit_row() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWifiFixture("empty");
            tryVerify(function() { return pane.otherNetworks.length === 0; });
            compare(pane.availableRepeater.count, 0);
            compare(pane.emptyRow.visible, true);
            compare(pane.errorRow.visible, false);
        }

        function test_read_error_keeps_the_structure_and_names_the_error() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setWifiFixture("error");
            tryVerify(function() { return pane.failed === true; });
            compare(pane.ready, false);
            compare(pane.errorRow.visible, true);
            compare(pane.absenceNote.visible, true);
            verify(pane.errorRow.text.length > 0);
            compare(pane.powerToggle.enabled, false);
        }

        function test_join_cancel_and_success_clear_the_password() {
            var shell = make();
            var pane = paneOf(shell);
            var row = rowWithSsid(pane, "neighbor-secure");
            verify(row !== null);
            findChild(row, "wifiJoinButton").clicked();
            tryVerify(function() { return pane.joinDialog.open === true; });
            var field = findChild(pane, "wifiJoinSecret");
            field.text = "hunter2";
            findChild(pane, "wifiJoinCancel").clicked();
            tryVerify(function() { return pane.joinDialog.open === false; });
            compare(field.text, "");
            compare(pane.pendingSsid, "");
            compare(Settings.wifi.activeSsid, "dragonfruit");

            // A successful join clears the field too.
            findChild(row, "wifiJoinButton").clicked();
            tryVerify(function() { return pane.joinDialog.open === true; });
            field.text = "hunter2";
            findChild(pane, "wifiJoinConfirm").clicked();
            tryVerify(function() { return Settings.wifi.activeSsid === "neighbor-secure"; });
            compare(field.text, "");
        }

        function test_known_details_offers_forget_and_disconnect() {
            var shell = make();
            var pane = paneOf(shell);
            var row = knownRow(pane, "dragonfruit");
            verify(row !== null);
            findChild(row, "wifiKnownDetailsButton").clicked();
            tryVerify(function() { return pane.detailsDialog.open === true; });
            compare(pane.detailsTitle.text, "dragonfruit");
            compare(pane.detailsForgetButton.visible, true);
            compare(pane.detailsDisconnectButton.visible, true);
            verify(findChild(pane, "wifiDetailsMetered") !== null);
            pane.detailsForgetButton.clicked();
            tryVerify(function() {
                for (var i = 0; i < Settings.wifi.knownNetworks.length; ++i)
                    if (Settings.wifi.knownNetworks[i].ssid === "dragonfruit")
                        return false;
                return true;
            });
        }

        function test_known_network_disconnects() {
            var shell = make();
            var pane = paneOf(shell);
            var row = knownRow(pane, "dragonfruit");
            verify(row !== null);
            findChild(row, "wifiKnownDetailsButton").clicked();
            tryVerify(function() { return pane.detailsDialog.open === true; });
            pane.detailsDisconnectButton.clicked();
            tryVerify(function() { return Settings.wifi.activeSsid === ""; });
            compare(pane.activeSsid, "");
        }

        function test_personal_hotspot_starts_and_stops() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.hotspotSupported, true);
            pane.hotspotRow.trailingButton.clicked();
            tryVerify(function() { return pane.hotspotDialog.open === true; });
            findChild(pane, "wifiHotspotSsid").text = "dragonfruit-hotspot";
            findChild(pane, "wifiHotspotSecret").text = "hunter2hunter2";
            findChild(pane, "wifiHotspotCreate").clicked();
            tryVerify(function() { return Settings.wifi.hotspotActive === true; });
            compare(pane.hotspotActive, true);
            compare(pane.hotspotRow.label, "dragonfruit-hotspot");
            pane.hotspotRow.trailingButton.clicked();
            tryVerify(function() { return Settings.wifi.hotspotActive === false; });
            compare(pane.hotspotActive, false);
        }

        function test_advanced_lists_the_linux_rows() {
            var shell = make();
            var pane = paneOf(shell);
            pane.advancedDialog.show();
            tryVerify(function() { return pane.advancedDialog.open === true; });
            compare(pane.advancedMetered.text, "Metered: Yes");
            compare(pane.advancedIpv6.text, "IPv6: auto");
            compare(pane.advancedDns.text, "DNS: 192.168.1.1, 1.1.1.1");
            compare(pane.advancedProxy.text, "Proxy: none");
            compare(pane.advancedMac.text, "MAC randomization: preserve");
            compare(pane.advancedAuth.text, "Authentication: WPA-PSK");
        }

        function test_a_failed_write_reports_and_reverts() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.failNextWifiWrite("power", "NetworkManager refused the change");
            pane.powerToggle.toggle();
            tryVerify(function() { return Settings.wifiWriteState === "failed"; });
            compare(Settings.wifiWriteError, "NetworkManager refused the change");
            compare(Settings.wifi.radioEnabled, true, "the view is the truth");
            compare(pane.powerToggle.checked, true, "the toggle reverts");
            compare(pane.writeErrorItem.visible, true);
        }

        function test_ask_to_join_popups_converge_after_an_external_change() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.set("network.wifi.askToJoin", "ask");
            tryCompare(pane.askToJoinSelect, "currentIndex", 2);
            Settings.set("network.wifi.askToJoin", "new");
            tryCompare(pane.askToJoinSelect, "currentIndex", 3);
            Settings.set("network.wifi.askToJoinHotspots", "off");
            tryCompare(pane.askHotspotsSelect, "currentIndex", 0);
        }

        // T288: the mockup's radio-off state is one note, not a dead list.
        function test_radio_off_shows_the_note_and_hides_the_lists() {
            var shell = make();
            var pane = paneOf(shell);
            pane.powerToggle.toggle();
            tryVerify(function() { return pane.radioEnabled === false; });
            compare(pane.radioOffNote.visible, true);
            compare(pane.currentGroup.visible, false);
            compare(pane.knownGroup.visible, false);
            compare(pane.availableGroup.visible, false);
            compare(pane.askGroup.visible, false);
            pane.powerToggle.toggle();
            tryVerify(function() { return pane.radioEnabled === true; });
            compare(pane.radioOffNote.visible, false);
            compare(pane.availableGroup.visible, true);
        }

        // T288: the ask-to-join rows are stored-only and the pane says so.
        function test_the_ask_to_join_rows_are_labeled_stored_only() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.askGroup.visible, true);
            compare(pane.askToJoinSelect.currentIndex, 1);
            verify(pane.askNote.text.indexOf("Stored only") >= 0);
            compare(pane.askNote.visible, true);
        }

        // T288: a scan is a real progress row, not a permanent spinner.
        function test_the_scan_row_is_a_progress_state() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.scanRow.visible, true);
            compare(pane.scanningRow.visible, false);
            Settings.setWifiFixture("scanning");
            tryVerify(function() { return pane.scanning === true; });
            compare(pane.scanningRow.visible, true);
            compare(pane.scanRow.visible, false);
            verify(pane.spinner !== null);
            Settings.refreshWifi();
            tryVerify(function() { return pane.scanning === false; });
            compare(pane.scanningRow.visible, false);
            compare(pane.scanRow.visible, true);
        }

        // T288: Personal Hotspot lives in the Advanced sheet now.
        function test_personal_hotspot_is_in_the_advanced_sheet() {
            var shell = make();
            var pane = paneOf(shell);
            verify(pane.hotspotGroup !== null);
            verify(pane.hotspotRow !== null);
            pane.advancedDialog.show();
            tryVerify(function() { return pane.advancedDialog.open === true; });
            verify(pane.hotspotGroup.parent.parent === pane.advancedDialog.content);
            compare(pane.hotspotRow.visible, true);
            pane.advancedDialog.hide();
        }

        // T288: the details sheet owns connect/forget/disconnect.
        function test_details_sheet_connects_a_saved_network() {
            var shell = make();
            var pane = paneOf(shell);
            var row = knownRow(pane, "dragonfruit");
            verify(row !== null);
            findChild(row, "wifiKnownDetailsButton").clicked();
            tryVerify(function() { return pane.detailsDialog.open === true; });
            // The active network offers Disconnect, not Connect.
            compare(pane.detailsConnectButton.visible, false);
            compare(pane.detailsDisconnectButton.visible, true);
            pane.detailsDialog.hide();
            var other = knownRow(pane, "home-office");
            verify(other !== null);
            findChild(other, "wifiKnownDetailsButton").clicked();
            tryVerify(function() { return pane.detailsDialog.open === true; });
            compare(pane.detailsConnectButton.visible, true);
            pane.detailsConnectButton.clicked();
            tryVerify(function() { return Settings.wifi.activeSsid === "home-office"; });
        }

        function test_the_pane_fits_the_measured_window() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            verify(shell !== null);
            waitForRendering(stage);
            shell.selectPane("wifi");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane !== null);
            verify(pane.width > 0);
            verify(pane.width <= 508,
                   "the pane fits the measured 508 pt detail content");
            verify(shell.detailScroll.maxY >= 0);
        }
    }
}
