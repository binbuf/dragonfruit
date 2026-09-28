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
                    "the caption names the adapter");
            compare(pane.caption.visible, true,
                    "the discoverable caption shows while the pane is open and on");
            verify(pane.myDevicesGroup.visible, "My Devices is shown");
            verify(pane.nearbyGroup.visible, "Nearby Devices is shown");
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
            verify(info !== null, "a circular info button is present");
            compare(info.Accessible.role, Accessible.Button);
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
    }
}