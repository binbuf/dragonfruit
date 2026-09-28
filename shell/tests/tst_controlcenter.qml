// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.ControlCenter

// Control Center panel tests (T-11.3a/T-11.3b): the normalized tile model,
// live tile gestures (volume/brightness/Focus/dark mode), the read-only Wi-Fi
// degradation, accessible roles, and Escape dismissal. Runs headless on the
// offscreen platform.
Item {
    id: stage
    width: 480
    height: 640

    TestCase {
        id: testCase
        name: "ControlCenter"
        when: windowShown

        Component { id: panelComponent; ControlCenter { } }
        SignalSpy { id: volumeSpy; signalName: "volumeSetRequested" }
        SignalSpy { id: brightnessSpy; signalName: "brightnessSetRequested" }
        SignalSpy { id: muteSpy; signalName: "muteToggleRequested" }
        SignalSpy { id: bluetoothToggleSpy; signalName: "bluetoothToggleRequested" }
        SignalSpy { id: bluetoothDeviceSpy; signalName: "bluetoothDeviceToggled" }
        SignalSpy { id: focusSpy; signalName: "focusToggleRequested" }
        SignalSpy { id: darkSpy; signalName: "darkModeToggleRequested" }
        SignalSpy { id: closedSpy; signalName: "closed" }
        SignalSpy { id: clipboardCopySpy; signalName: "clipboardCopyRequested" }
        SignalSpy { id: clipboardPinSpy; signalName: "clipboardPinToggled" }
        SignalSpy { id: clipboardClearSpy; signalName: "clipboardClearRequested" }

        function wifiModel(state, radioEnabled, label) {
            return {
                kind: "wifi",
                state: state,
                visible: state === "available",
                enabled: state === "available",
                radioEnabled: radioEnabled,
                label: label !== undefined ? label : ""
            };
        }

        function audioModel(state, volume, muted) {
            return {
                kind: "audio",
                state: state,
                visible: state === "available",
                enabled: state === "available",
                volume: volume,
                muted: muted
            };
        }

        function bluetoothModel(state, present, powered, discovering, devices) {
            var view = {
                kind: "bluetooth",
                state: state,
                present: present,
                visible: state === "available" && present,
                enabled: state === "available" && present,
                powered: powered,
                label: (state !== "available" || !present) ? "Bluetooth unavailable"
                     : discovering === true ? "Bluetooth discovering"
                     : powered ? "Bluetooth on" : "Bluetooth off"
            };
            if (discovering !== undefined)
                view.discovering = discovering;
            if (devices !== undefined)
                view.knownDevices = devices;
            return view;
        }

        function focusModel(mode, batchedCount) {
            return {
                mode: mode,
                allowList: [],
                batchedCount: batchedCount !== undefined ? batchedCount : 0
            };
        }

        function make(props) {
            var panel = createTemporaryObject(panelComponent, stage, props || {});
            // The shell sizes the panel to its surface; do the same here so
            // pointer coordinates land on the controls.
            panel.width = 360;
            panel.height = 520;
            // The shell sets this from the bridge host's Bluetooth interface
            // (T-15.1b); the tile is live when it is true.
            panel.bluetoothWritable = true;
            waitForRendering(stage);
            return panel;
        }

        function test_tiles_expose_all_six_controls() {
            var panel = make({
                wifi: wifiModel("available", true, "home"),
                bluetooth: bluetoothModel("available", true, true, false),
                audio: audioModel("available", 0.6, false),
                brightness: 0.8,
                focusPolicy: focusModel("off"),
                dark: true
            });
            compare(panel.tiles.length, 6);
            compare(panel.tiles[0].id, "wifi");
            compare(panel.tiles[0].kind, "toggle");
            compare(panel.tiles[0].checked, true);
            compare(panel.tiles[1].id, "bluetooth");
            compare(panel.tiles[1].kind, "toggle");
            compare(panel.tiles[1].checked, true);
            compare(panel.tiles[2].id, "focus");
            compare(panel.tiles[2].kind, "toggle");
            compare(panel.tiles[3].id, "volume");
            compare(panel.tiles[3].kind, "slider");
            compare(Math.abs(panel.tiles[3].value - 0.6) < 0.0001, true);
            compare(panel.tiles[4].id, "brightness");
            compare(Math.abs(panel.tiles[4].value - 0.8) < 0.0001, true);
            compare(panel.tiles[5].id, "dark");
            compare(panel.tiles[5].kind, "toggle");
            compare(panel.tiles[5].checked, true);
            compare(panel.wifiLabel, "home");
        }

        function test_panel_content_fits_the_shell_surface() {
            // The shell sizes the Control Center surface to 360x780
            // (kControlCenterWidth/Height). With the Bluetooth tile's device
            // rows the content must still fit, or the lower tiles are clipped.
            var panel = make({
                wifi: wifiModel("available", true, "home"),
                bluetooth: bluetoothModel("available", true, true, false,
                                          [{ address: "AA:BB:CC:DD:EE:FF",
                                             name: "WF-1000XM6", connected: true },
                                           { address: "11:22:33:44:55:66",
                                             name: "WH-1000XM6", connected: false }]),
                audio: audioModel("available", 0.6, false),
                brightness: 1.0,
                focusPolicy: focusModel("off"),
                dark: false
            });
            panel.width = 360;
            panel.height = 780;
            waitForRendering(stage);
            var content = findChild(panel, "controlCenterContent");
            verify(content !== null);
            verify(content.childrenRect.height <= 780,
                   "Control Center content must fit the 780px surface, height="
                   + content.childrenRect.height);
        }

        function test_bluetooth_tile_reflects_state() {
            var panel = make({
                bluetooth: bluetoothModel("available", true, true, false,
                                          [{ address: "AA:BB:CC:DD:EE:FF",
                                             name: "WF-1000XM6", connected: true }])
            });
            compare(panel.bluetoothVisible, true);
            compare(panel.bluetoothOn, true);
            compare(panel.tiles[1].enabled, true);
            compare(panel.bluetoothDevices.length, 1);
            compare(panel.bluetoothLabel, "Bluetooth on");

            panel.bluetooth = bluetoothModel("available", true, false, false);
            compare(panel.bluetoothOn, false);
            compare(panel.bluetoothLabel, "Bluetooth off");
        }

        function test_bluetooth_tile_hides_on_absence_and_no_controller() {
            var panel = make({ bluetooth: bluetoothModel("unavailable", false, false) });
            compare(panel.bluetoothVisible, false);
            compare(panel.tiles[1].visible, false);
            var tile = findChild(panel, "bluetoothTile");
            verify(tile !== null);
            compare(tile.visible, false);

            // A running daemon with no controller is available but not present.
            panel.bluetooth = bluetoothModel("available", false, false);
            compare(panel.bluetoothVisible, false);
        }

        function test_bluetooth_toggle_raises_a_write() {
            var panel = make({
                bluetooth: bluetoothModel("available", true, true, false)
            });
            bluetoothToggleSpy.target = panel;
            bluetoothToggleSpy.clear();
            var toggle = findChild(panel, "bluetoothToggle");
            verify(toggle !== null);
            toggle.toggle();
            compare(bluetoothToggleSpy.count, 1);
            compare(bluetoothToggleSpy.signalArguments[0][0], false);

            panel.toggleBluetooth();
            compare(bluetoothToggleSpy.count, 2);
            compare(bluetoothToggleSpy.signalArguments[1][0], false);
        }

        function test_bluetooth_device_row_raises_a_connect() {
            var panel = make({
                bluetooth: bluetoothModel("available", true, true, false,
                                          [{ address: "AA:BB:CC:DD:EE:FF",
                                             name: "WF-1000XM6", connected: false }])
            });
            bluetoothDeviceSpy.target = panel;
            bluetoothDeviceSpy.clear();
            panel.toggleBluetoothDevice("AA:BB:CC:DD:EE:FF");
            compare(bluetoothDeviceSpy.count, 1);
            compare(bluetoothDeviceSpy.signalArguments[0][0], "AA:BB:CC:DD:EE:FF");
            compare(bluetoothDeviceSpy.signalArguments[0][1], true);
        }

        function test_bluetooth_settings_link_is_accessible() {
            var panel = make({
                bluetooth: bluetoothModel("available", true, true, false)
            });
            var link = findChild(panel, "bluetoothSettingsLink");
            verify(link !== null);
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Bluetooth Settings");
        }

        function test_wifi_tile_is_read_only_until_the_adapter_can_write() {
            // The T-07 adapter exposes no radio write (T-15); the toggle
            // reflects state and is inert, so its model entry is disabled.
            var panel = make({ wifi: wifiModel("available", true, "home") });
            compare(panel.tiles[0].enabled, false);
            var toggle = findChild(panel, "wifiToggle");
            verify(toggle !== null);
            compare(toggle.enabled, false);
        }

        function test_brightness_change_emits_a_settings_request() {
            var panel = make({ brightness: 1.0 });
            brightnessSpy.target = panel;
            brightnessSpy.clear();
            panel.setBrightness(0.4);
            compare(brightnessSpy.count, 1);
            verify(Math.abs(brightnessSpy.signalArguments[0][0] - 0.4) < 0.0001);
            compare(Math.abs(panel.brightness - 0.4) < 0.0001, true);
            // Clamps to the unit range.
            panel.setBrightness(1.5);
            compare(panel.brightness, 1.0);
        }

        function test_brightness_slider_moves_live() {
            var panel = make({ brightness: 1.0 });
            brightnessSpy.target = panel;
            brightnessSpy.clear();
            var slider = findChild(panel, "brightnessSlider");
            verify(slider !== null);
            slider.setValue(0.25);
            slider.commit();
            verify(brightnessSpy.count >= 1);
            verify(Math.abs(panel.brightness - 0.25) < 0.0001);
        }

        function test_volume_change_emits_a_host_request() {
            var panel = make({ audio: audioModel("available", 0.5, false) });
            volumeSpy.target = panel;
            volumeSpy.clear();
            panel.setVolume(0.7);
            compare(volumeSpy.count, 1);
            verify(Math.abs(volumeSpy.signalArguments[0][0] - 0.7) < 0.0001);
            panel.setVolume(-1.0);
            compare(panel.volume, 0.0);
        }

        function test_mute_button_raises_the_toggle() {
            var panel = make({ audio: audioModel("available", 0.5, false) });
            muteSpy.target = panel;
            muteSpy.clear();
            var button = findChild(panel, "muteButton");
            verify(button !== null);
            mouseClick(button, button.width / 2, button.height / 2);
            compare(muteSpy.count, 1);
        }

        function test_focus_tile_reflects_the_service_policy() {
            var panel = make({ focusPolicy: focusModel("dnd", 3) });
            compare(panel.focusMode, "dnd");
            compare(panel.focusOn, true);
            compare(panel.focusLabel, "Do Not Disturb");
            compare(panel.tiles[2].checked, true);
            verify(panel.focusSubtitle.indexOf("3") >= 0);
            verify(panel.focusSubtitle.indexOf("Do Not Disturb") >= 0);

            panel.focusPolicy = focusModel("focus");
            compare(panel.focusOn, true);
            compare(panel.focusLabel, "Focus");

            // An absent policy is the safe off default.
            panel.focusPolicy = ({});
            compare(panel.focusMode, "off");
            compare(panel.focusOn, false);
        }

        function test_focus_toggle_raises_the_requested_mode() {
            var panel = make({ focusPolicy: focusModel("off") });
            focusSpy.target = panel;
            focusSpy.clear();

            // Off -> on requests DND.
            panel.toggleFocus();
            compare(focusSpy.count, 1);
            compare(focusSpy.signalArguments[0][0], true);

            // On -> off clears the policy.
            panel.focusPolicy = focusModel("dnd");
            panel.toggleFocus();
            compare(focusSpy.count, 2);
            compare(focusSpy.signalArguments[1][0], false);
        }

        function test_focus_switch_toggles_through_the_control() {
            var panel = make({ focusPolicy: focusModel("off") });
            focusSpy.target = panel;
            focusSpy.clear();
            var toggle = findChild(panel, "focusToggle");
            verify(toggle !== null);
            mouseClick(toggle, toggle.width / 2, toggle.height / 2);
            compare(focusSpy.count, 1);
            compare(focusSpy.signalArguments[0][0], true);
        }

        function test_dark_switch_toggles_through_the_control() {
            var panel = make({ dark: false });
            darkSpy.target = panel;
            darkSpy.clear();
            var toggle = findChild(panel, "darkToggle");
            verify(toggle !== null);
            toggle.toggle();
            compare(darkSpy.count, 1);
            compare(darkSpy.signalArguments[0][0], true);
        }

        function test_dark_mode_tile_reflects_the_scheme() {
            var panel = make({ dark: false });
            compare(panel.tiles[5].checked, false);
            compare(panel.tiles[5].subtitle, "Off");
            panel.dark = true;
            compare(panel.tiles[5].checked, true);
            compare(panel.tiles[5].subtitle, "On");
        }

        function test_dark_mode_toggle_raises_the_absolute_scheme() {
            var panel = make({ dark: false });
            darkSpy.target = panel;
            darkSpy.clear();
            panel.toggleDarkMode();
            compare(darkSpy.count, 1);
            compare(darkSpy.signalArguments[0][0], true);

            panel.dark = true;
            panel.toggleDarkMode();
            compare(darkSpy.count, 2);
            compare(darkSpy.signalArguments[1][0], false);
        }

        function test_tiles_carry_accessible_roles_and_names() {
            var panel = make({ focusPolicy: focusModel("off"), dark: false });
            compare(panel.Accessible.role, Accessible.Pane);
            compare(panel.Accessible.name, "Control Center");

            var focusToggle = findChild(panel, "focusToggle");
            verify(focusToggle !== null);
            compare(focusToggle.Accessible.role, Accessible.Switch);
            compare(focusToggle.Accessible.name, "Do Not Disturb");

            var darkToggle = findChild(panel, "darkToggle");
            verify(darkToggle !== null);
            compare(darkToggle.Accessible.role, Accessible.Switch);
            compare(darkToggle.Accessible.name, "Dark Mode");

            var focusTile = findChild(panel, "focusTile");
            verify(focusTile !== null);
            compare(focusTile.Accessible.role, Accessible.Grouping);
            compare(focusTile.Accessible.name, "Focus");

            var darkTile = findChild(panel, "darkTile");
            verify(darkTile !== null);
            compare(darkTile.Accessible.role, Accessible.Grouping);
            compare(darkTile.Accessible.name, "Dark Mode");
        }

        function test_text_links_are_accessible_buttons() {
            var panel = make({ focusPolicy: focusModel("off") });
            var link = findChild(panel, "focusSettingsLink");
            verify(link !== null);
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Focus Settings");
        }

        function test_escape_closes_the_panel() {
            var panel = make({ brightness: 1.0 });
            panel.forceActiveFocus();
            waitForRendering(stage);
            closedSpy.target = panel;
            closedSpy.clear();
            keyClick(Qt.Key_Escape);
            compare(closedSpy.count, 1);
        }

        function test_unknown_wifi_shows_unavailable() {
            var panel = make({ wifi: wifiModel("unavailable", false) });
            compare(panel.wifiAvailable, false);
            compare(panel.wifiLabel, "Unavailable");
            compare(panel.tiles[0].enabled, false);
        }

        function test_clipboard_rows_render_and_raise_requests() {
            var panel = make({
                clipboardEntries: [
                    { kind: "text", preview: "first item", pinned: true, index: 0 },
                    { kind: "files", preview: "a.txt, b.txt", pinned: false, index: 1 }
                ]
            });
            // The clipboard section is below the tiles; size the panel to the
            // shell's fixed surface so pointer coordinates land on it.
            panel.height = 900;
            waitForRendering(stage);
            compare(panel.clipboardShown, 2);

            var preview = findChild(panel, "clipboardRowPreview");
            verify(preview !== null);
            clipboardCopySpy.target = panel;
            clipboardCopySpy.clear();
            mouseClick(preview, preview.width / 2, preview.height / 2);
            compare(clipboardCopySpy.count, 1);
            compare(clipboardCopySpy.signalArguments[0][0], 0);

            var pin = findChild(panel, "clipboardRowPin");
            verify(pin !== null);
            clipboardPinSpy.target = panel;
            clipboardPinSpy.clear();
            mouseClick(pin, pin.width / 2, pin.height / 2);
            compare(clipboardPinSpy.count, 1);
            compare(clipboardPinSpy.signalArguments[0][0], 0);
            compare(clipboardPinSpy.signalArguments[0][1], false);

            var clear = findChild(panel, "clipboardClearLink");
            verify(clear !== null);
            clipboardClearSpy.target = panel;
            clipboardClearSpy.clear();
            mouseClick(clear, clear.width / 2, clear.height / 2);
            compare(clipboardClearSpy.count, 1);
        }

        function test_clipboard_empty_state() {
            var panel = make({ clipboardEntries: [] });
            var empty = findChild(panel, "clipboardEmpty");
            verify(empty !== null);
            compare(empty.visible, true);
            var clear = findChild(panel, "clipboardClearLink");
            verify(clear !== null);
            compare(clear.visible, false);
        }
    }
}