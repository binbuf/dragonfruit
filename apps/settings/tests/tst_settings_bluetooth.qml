// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Bluetooth-pane tests (T-15.1b). Headless with `DF_SETTINGS_FIXTURE` and
// `DF_BLUETOOTH_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton
// serves a deterministic in-process adapter with no bus. The cases cover the
// toggle card and caption, the known-device rows and connect round-trip, the
// Nearby Devices searching state, live convergence, and the absence note.
Item {
    id: stage
    width: 900
    height: 720

    TestCase {
        id: testCase
        name: "SettingsBluetooth"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; reset the adapter before every
        // case so the pane always opens on a known state.
        function init() {
            Settings.resetBluetoothFixture();
            Settings.setBluetoothConnected("AA:BB:CC:DD:EE:FF", false);
            Settings.setBluetoothPowered(true);
            Settings.setBluetoothDiscovering(false);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("bluetooth");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Bluetooth body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_toggle_card_and_caption() {
            var shell = make();
            compare(shell.currentPaneId, "bluetooth");
            var pane = paneOf(shell);
            verify(pane.powerToggle, "the Bluetooth toggle must exist");
            compare(pane.powerToggle.checked, true);
            compare(pane.powered, true);
            compare(pane.discoverableCaption.indexOf("workstation") >= 0, true,
                    "the caption names the computer");
            compare(pane.discoverableCaption.indexOf("11:22:33:44:55:66"), -1,
                    "the caption never shows the adapter id");
            compare(pane.caption.visible, true,
                    "the discoverable caption shows while the pane is open and on");
            // T288: the mockup intro opens the pane.
            verify(pane.intro.text.indexOf("Connect to accessories") >= 0);
            verify(pane.myDevicesGroup.visible, "My Devices is shown");
            verify(pane.nearbyGroup.visible, "Nearby Devices is shown");
            verify(pane.discoverableRow.visible, "Discoverable As is shown");
        }

        function test_toggle_applies_live_through_the_adapter() {
            var shell = make();
            var pane = paneOf(shell);
            pane.powerToggle.toggle();
            compare(Settings.bluetooth.powered, false);
            compare(pane.powered, false);
            compare(pane.caption.visible, false,
                    "the discoverable caption hides when Bluetooth is off");

            pane.powerToggle.toggle();
            compare(Settings.bluetooth.powered, true);
        }

        function test_known_device_row_connects_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.knownDevices.length, 2);
            compare(pane.knownDevices[0].name, "WF-1000XM6");
            compare(pane.knownDevices[0].connected, false);

            pane.toggleDevice("AA:BB:CC:DD:EE:FF", false);
            compare(pane.knownDevices[0].connected, true);

            var info = findChild(pane, "bluetoothInfoButton");
            verify(info !== null, "an info action is present");
            info.clicked();
            tryVerify(function() { return pane.deviceDialog.open === true; });
            compare(pane.deviceTitle.text, "WF-1000XM6");
            pane.deviceDialog.hide();
        }

        function test_nearby_devices_show_the_searching_state() {
            var shell = make();
            var pane = paneOf(shell);
            // Opening the pane starts an inquiry; the fixture then reports a
            // nearby device.
            compare(pane.searching, true);
            compare(pane.searchingRow.visible, true);
            verify(pane.spinner !== null, "the searching spinner is present");

            Settings.setBluetoothDiscovering(false);
            waitForRendering(stage);
            compare(pane.searching, false);
            compare(pane.searchingRow.visible, false);
            compare(pane.nearbyDevices.length, 0);
        }

        function test_power_off_hides_the_caption_and_asks_for_power() {
            var shell = make();
            var pane = paneOf(shell);
            pane.powerToggle.toggle();
            compare(pane.powered, false);
            compare(pane.caption.visible, false);
            compare(pane.searching, false);
        }

        function test_external_change_converges() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setBluetoothPowered(false);
            compare(pane.powered, false);
            Settings.setBluetoothPowered(true);
            compare(pane.powered, true);
        }

        function test_info_opens_the_device_sheet_and_connects() {
            var shell = make();
            var pane = paneOf(shell);
            var info = findChild(pane, "bluetoothInfoButton");
            verify(info !== null);
            info.clicked();
            tryVerify(function() { return pane.deviceDialog.open === true; });
            compare(pane.deviceTitle.text, "WF-1000XM6");
            compare(pane.deviceAddress.text.indexOf("AA:BB:CC:DD:EE:FF") >= 0, true);
            compare(pane.deviceType.text, "Type: Audio");
            compare(pane.deviceBattery.text, "Battery: 82%");
            compare(pane.deviceConnectButton.text, "Connect");
            pane.deviceConnectButton.clicked();
            tryVerify(function() { return Settings.bluetooth.knownDevices[0].connected === true; });
            compare(pane.deviceConnectButton.text, "Disconnect");
            pane.deviceConnectButton.clicked();
            tryVerify(function() { return Settings.bluetooth.knownDevices[0].connected === false; });
        }

        function test_forget_from_the_device_sheet_removes_the_device() {
            var shell = make();
            var pane = paneOf(shell);
            var info = findChild(pane, "bluetoothInfoButton");
            verify(info !== null);
            info.clicked();
            tryVerify(function() { return pane.deviceDialog.open === true; });
            pane.deviceForgetButton.clicked();
            tryVerify(function() {
                for (var i = 0; i < Settings.bluetooth.knownDevices.length; ++i)
                    if (Settings.bluetooth.knownDevices[i].address === "AA:BB:CC:DD:EE:FF")
                        return false;
                return true;
            });
            compare(pane.deviceDialog.open, false);
        }

        function test_power_on_starts_discovery() {
            var shell = make();
            var pane = paneOf(shell);
            pane.powerToggle.toggle(); // off
            tryVerify(function() { return Settings.bluetooth.powered === false; });
            compare(pane.discovering, false);
            pane.powerToggle.toggle(); // on
            tryVerify(function() { return Settings.bluetooth.powered === true; });
            tryVerify(function() { return Settings.bluetooth.discovering === true; });
            compare(pane.searching, true);
        }

        function test_absent_adapter_keeps_the_sections() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setBluetoothFixture("absent");
            tryVerify(function() { return pane.ready === false; });
            compare(pane.powerToggle.enabled, false);
            compare(pane.caption.visible, false);
            compare(pane.absenceNote.visible, true);
            compare(pane.myDevicesGroup.visible, true);
            compare(pane.nearbyGroup.visible, true);
            // T288: the discoverable note is conditional on a live,
            // discoverable adapter; an absent host keeps the other sections.
            compare(pane.discoverableRow.visible, false);
            compare(pane.advancedRow.visible, true);
            compare(pane.myDevicesEmpty.visible, true);
            compare(pane.nearbyEmpty.visible, true);
            Settings.setBluetoothFixture("live");
            tryVerify(function() { return pane.ready === true; });
        }

        function test_empty_device_list_is_an_explicit_row() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setBluetoothFixture("empty");
            tryVerify(function() { return pane.knownDevices.length === 0; });
            compare(pane.myDevicesEmpty.visible, true);
            compare(pane.knownRepeater.count, 0);
        }

        function test_a_failed_write_reports_and_reverts() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.failNextBluetoothWrite("power", "BlueZ refused the change");
            pane.powerToggle.toggle();
            tryVerify(function() { return Settings.bluetoothWriteState === "failed"; });
            compare(Settings.bluetoothWriteError, "BlueZ refused the change");
            compare(Settings.bluetooth.powered, true, "the view is the truth");
            compare(pane.powerToggle.checked, true, "the toggle reverts");
            compare(pane.writeErrorItem.visible, true);
        }

        // T288: the device lists use the shared device row and the scan row is
        // a real progress state.
        function test_device_rows_and_the_searching_row() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.knownRepeater.count, 2);
            var row = pane.knownRepeater.itemAt(0);
            compare(row.objectName, "bluetoothDeviceRow");
            verify(row.deviceGlyph.length > 0);
            verify(findChild(row, "bluetoothInfoButton") !== null);
            compare(pane.searchingRow.objectName, "bluetoothSearchingRow");
            verify(pane.searchingRow.spinner !== null);
            Settings.setBluetoothDiscovering(false);
            waitForRendering(stage);
            compare(pane.searchingRow.visible, false);
        }

        // T288: the discoverable note only exists while the pane is open, the
        // radio is on, and the adapter reports itself discoverable.
        function test_the_discoverable_note_is_conditional() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.discoverableRow.visible, true);
            verify(pane.discoverableRow.description.indexOf("workstation") >= 0);
            pane.powerToggle.toggle();
            tryVerify(function() { return pane.powered === false; });
            compare(pane.discoverableRow.visible, false);
        }

        function test_the_pane_fits_the_measured_window() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            verify(shell !== null);
            waitForRendering(stage);
            shell.selectPane("bluetooth");
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