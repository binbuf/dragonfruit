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
        SignalSpy { id: storageMountSpy; signalName: "storageMountRequested" }
        SignalSpy { id: storageUnmountSpy; signalName: "storageUnmountRequested" }
        SignalSpy { id: storageEjectSpy; signalName: "storageEjectRequested" }
        SignalSpy { id: soundSettingsSpy; signalName: "soundSettingsRequested" }
        SignalSpy { id: keyboardSettingsSpy; signalName: "keyboardSettingsRequested" }
        SignalSpy { id: missionControlSettingsSpy; signalName: "missionControlSettingsRequested" }
        SignalSpy { id: batterySettingsSpy; signalName: "batterySettingsRequested" }
        SignalSpy { id: lockScreenSettingsSpy; signalName: "lockScreenSettingsRequested" }
        SignalSpy { id: menuBarSettingsSpy; signalName: "menuBarSettingsRequested" }
        SignalSpy { id: updatesCheckSpy; signalName: "updatesCheckRequested" }
        SignalSpy { id: updatesInstallSpy; signalName: "updatesInstallRequested" }
        SignalSpy { id: updatesRebootSpy; signalName: "updatesRebootRequested" }
        SignalSpy { id: updatesSettingsSpy; signalName: "updatesSettingsRequested" }
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

        function audioRoutingModel(defaultName) {
            var speakersDefault = defaultName === "Built-in Speakers";
            return {
                kind: "audio",
                state: "available",
                visible: true,
                enabled: true,
                volume: 0.6,
                muted: false,
                defaultSink: speakersDefault ? "speakers" : "headphones",
                sinkCount: 2,
                sinks: [
                    { id: 7, name: "speakers", description: "Built-in Speakers",
                      volume: 0.6, percent: 60, muted: false, default: speakersDefault },
                    { id: 9, name: "headphones", description: "Headphones",
                      volume: 0.6, percent: 60, muted: false, default: !speakersDefault }
                ]
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

        function storageModel(state, present, mounted, volumes, drives) {
            return {
                kind: "storage",
                state: state,
                present: present,
                visible: state === "available" && present,
                enabled: state === "available" && present,
                label: (state !== "available" || !present) ? "Storage unavailable"
                     : mounted ? "Storage 1 mounted" : "Storage",
                mountedCount: mounted ? 1 : 0,
                volumes: volumes !== undefined ? volumes : [],
                drives: drives !== undefined ? drives : []
            };
        }

        function inputModel(state, present, devices) {
            return {
                kind: "input",
                state: state,
                present: present,
                visible: state === "available" && present,
                enabled: state === "available" && present,
                label: (state !== "available" || !present)
                    ? "No input devices" : "1 keyboards, 2 pointing devices",
                deviceCount: devices !== undefined ? devices.length : 0,
                devices: devices !== undefined ? devices : []
            };
        }

        function focusModel(mode, batchedCount) {
            return {
                mode: mode,
                allowList: [],
                batchedCount: batchedCount !== undefined ? batchedCount : 0
            };
        }

        // The shell-projected Mission Control summary (T-15.5b), shaped by
        // `controlcenterpolicy.cpp`'s `missionControlView`.
        function missionControlModel(gesture, corners, label) {
            var resolved = label !== undefined ? label
                : gesture && corners === 0 ? "Gesture"
                : gesture ? "Gesture, " + corners + " corner(s)"
                : corners === 0 ? "No trigger" : corners + " corner(s)";
            return {
                state: "available",
                glyph: "overview",
                label: resolved,
                reachable: gesture || corners > 0,
                gesture: gesture,
                cornerCount: corners
            };
        }

        // The shell's decoded battery / power-profiles view (T-15.6b), shaped by
        // `SystemStatusModel`.
        function batteryModel(percent, profile, profilesAvailable, present) {
            var list = [];
            if (profilesAvailable) {
                list = [
                    { id: "power-saver", label: "Power Saver",
                      glyph: "power-saver", active: profile === "power-saver" },
                    { id: "balanced", label: "Balanced",
                      glyph: "power-balanced", active: profile === "balanced" },
                    { id: "performance", label: "Performance",
                      glyph: "power-performance", active: profile === "performance" }
                ];
            }
            var label = profile === "power-saver" ? "Power Saver"
                : profile === "performance" ? "Performance" : "Balanced";
            return {
                kind: "battery",
                state: "available",
                present: present,
                visible: present,
                enabled: true,
                percent: percent,
                profilesAvailable: profilesAvailable,
                activeProfile: profile,
                profileLabel: label,
                profiles: list
            };
        }

        // The shell-projected Lock Screen summary (T-15.8b), shaped by
        // `controlcenterpolicy.cpp`'s `lockPolicyView`.
        function lockPolicyModel(lockSeconds) {
            var label;
            if (lockSeconds <= 0)
                label = "No password required";
            else if (lockSeconds % 3600 === 0)
                label = "Password after " + (lockSeconds / 3600) + " h";
            else if (lockSeconds % 60 === 0)
                label = "Password after " + (lockSeconds / 60) + " min";
            else
                label = "Password after " + lockSeconds + " s";
            return {
                state: "available",
                glyph: "lock",
                label: label,
                requirePassword: lockSeconds > 0,
                lockSeconds: lockSeconds
            };
        }

        // The shell-projected Menu Bar summary (T-15.9b), shaped by
        // `controlcenterpolicy.cpp`'s `menuBarView`.
        function updatesModel(label, phase, count) {
            return {
                kind: "updates",
                state: "available",
                visible: true,
                enabled: true,
                glyph: "software-update",
                label: label !== undefined ? label : "1 Update Available",
                updatesAvailable: true,
                phase: phase !== undefined ? phase : "available",
                busy: false,
                rebootRequired: phase === "reboot-required",
                updateCount: count !== undefined ? count : 1
            };
        }

        function menuBarModel(autoHide, showBackground, globalMenu) {
            var label = autoHide === "never" ? "Never"
                : autoHide === "always" ? "Always"
                : "In Full Screen Only";
            return {
                state: "available",
                glyph: "menu-bar",
                label: label,
                autoHide: autoHide !== undefined ? autoHide : "full-screen",
                showBackground: showBackground !== false,
                globalMenu: globalMenu !== false
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
            // The shell sets this from the bridge host's Storage interface
            // (T-15.2b); the tile is live when it is true.
            panel.storageWritable = true;
            waitForRendering(stage);
            return panel;
        }

        function test_tiles_expose_all_ten_controls() {
            var panel = make({
                wifi: wifiModel("available", true, "home"),
                bluetooth: bluetoothModel("available", true, true, false),
                storage: storageModel("available", true, false),
                audio: audioModel("available", 0.6, false),
                input: inputModel("available", true,
                                  [{ name: "AT keyboard", kind: "keyboard" }]),
                missionControl: missionControlModel(true, 1),
                battery: batteryModel(71, "balanced", true, true),
                lockPolicy: lockPolicyModel(600),
                menuBar: menuBarModel("full-screen"),
                updates: updatesModel("1 Update Available", "available", 1),
                brightness: 0.8,
                focusPolicy: focusModel("off"),
                dark: true
            });
            compare(panel.tiles.length, 13);
            compare(panel.tiles[0].id, "wifi");
            compare(panel.tiles[0].kind, "toggle");
            compare(panel.tiles[0].checked, true);
            compare(panel.tiles[1].id, "bluetooth");
            compare(panel.tiles[1].kind, "toggle");
            compare(panel.tiles[1].checked, true);
            compare(panel.tiles[2].id, "storage");
            compare(panel.tiles[2].kind, "storage");
            compare(panel.tiles[3].id, "focus");
            compare(panel.tiles[3].kind, "toggle");
            compare(panel.tiles[4].id, "volume");
            compare(panel.tiles[4].kind, "slider");
            compare(Math.abs(panel.tiles[4].value - 0.6) < 0.0001, true);
            compare(panel.tiles[5].id, "brightness");
            compare(Math.abs(panel.tiles[5].value - 0.8) < 0.0001, true);
            compare(panel.tiles[6].id, "dark");
            compare(panel.tiles[6].kind, "toggle");
            compare(panel.tiles[6].checked, true);
            compare(panel.tiles[7].id, "keyboard");
            compare(panel.tiles[7].kind, "info");
            compare(panel.tiles[7].subtitle, "1 keyboards, 2 pointing devices");
            compare(panel.tiles[8].id, "mission-control");
            compare(panel.tiles[8].kind, "info");
            compare(panel.tiles[8].subtitle, "Gesture, 1 corner(s)");
            compare(panel.tiles[9].id, "battery");
            compare(panel.tiles[9].kind, "info");
            compare(panel.tiles[9].subtitle, "71% \u00b7 Balanced");
            compare(panel.tiles[10].id, "lock-screen");
            compare(panel.tiles[10].kind, "info");
            compare(panel.tiles[11].id, "menu-bar");
            compare(panel.tiles[11].kind, "info");
            compare(panel.tiles[11].subtitle, "In Full Screen Only");
            compare(panel.tiles[12].id, "software-update");
            compare(panel.tiles[12].kind, "info");
            compare(panel.tiles[12].subtitle, "1 Update Available");
            compare(panel.wifiLabel, "home");
        }

        function test_panel_content_fits_the_shell_surface() {
            // The shell sizes the Control Center surface to 360x1160
            // (kControlCenterWidth/Height). With the Bluetooth tile's device
            // rows, the Storage tile, the Sound tile's routing subtitle, the
            // Keyboard tile, the Mission Control tile, the Battery tile, the
            // Lock Screen tile, and the Menu Bar tile the content must still
            // fit, or the lower tiles are clipped.
            var panel = make({
                wifi: wifiModel("available", true, "home"),
                bluetooth: bluetoothModel("available", true, true, false,
                                          [{ address: "AA:BB:CC:DD:EE:FF",
                                             name: "WF-1000XM6", connected: true },
                                           { address: "11:22:33:44:55:66",
                                             name: "WH-1000XM6", connected: false }]),
                storage: storageModel("available", true, false),
                audio: audioRoutingModel("Built-in Speakers"),
                input: inputModel("available", true,
                                  [{ name: "AT keyboard", kind: "keyboard" }]),
                missionControl: missionControlModel(true, 1),
                battery: batteryModel(71, "balanced", true, true),
lockPolicy: lockPolicyModel(600),
                menuBar: menuBarModel("full-screen"),
                updates: updatesModel("1 Update Available", "available", 1),
                brightness: 1.0,
                focusPolicy: focusModel("off"),
                dark: false
            });
            panel.width = 360;
            panel.height = 1160;
            waitForRendering(stage);
            var content = findChild(panel, "controlCenterContent");
            verify(content !== null);
            verify(content.childrenRect.height <= 1160,
                   "Control Center content must fit the 1160px surface, height="
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

        function test_storage_tile_reflects_state() {
            var panel = make({
                storage: storageModel("available", true, true,
                                      [{ path: "/dev/sdb1", drivePath: "/drives/usb",
                                         name: "Photos", mounted: true,
                                         removable: true, ejectable: true }])
            });
            compare(panel.storageVisible, true);
            compare(panel.storageMountedCount, 1);
            compare(panel.storageVolumes.length, 1);
            compare(panel.storageLabel, "Storage 1 mounted");
            compare(panel.tiles[2].enabled, true);

            panel.storage = storageModel("available", true, false);
            compare(panel.storageMountedCount, 0);
            compare(panel.storageLabel, "Storage");
        }

        function test_storage_tile_hides_on_absence_and_no_volumes() {
            var panel = make({ storage: storageModel("unavailable", false, false) });
            compare(panel.storageVisible, false);
            compare(panel.tiles[2].visible, false);
            var tile = findChild(panel, "storageTile");
            verify(tile !== null);
            compare(tile.visible, false);

            // UDisks2 present but no mountable volume: available, yet hidden.
            panel.storage = storageModel("available", false, false);
            compare(panel.storageVisible, false);
        }

        function test_storage_volume_row_raises_mount_or_unmount() {
            var panel = make({
                storage: storageModel("available", true, false,
                                      [{ path: "/dev/sdb1", drivePath: "/drives/usb",
                                         name: "Photos", mounted: false,
                                         removable: true, ejectable: true }])
            });
            storageMountSpy.target = panel;
            storageMountSpy.clear();
            storageUnmountSpy.target = panel;
            storageUnmountSpy.clear();
            panel.toggleStorageVolume("/dev/sdb1");
            compare(storageMountSpy.count, 1);
            compare(storageMountSpy.signalArguments[0][0], "/dev/sdb1");

            panel.storage = storageModel("available", true, true,
                                         [{ path: "/dev/sdb1", drivePath: "/drives/usb",
                                            name: "Photos", mounted: true,
                                            removable: true, ejectable: true }]);
            panel.toggleStorageVolume("/dev/sdb1");
            compare(storageUnmountSpy.count, 1);
            compare(storageUnmountSpy.signalArguments[0][0], "/dev/sdb1");
        }

        function test_storage_eject_raises_the_drive() {
            var panel = make({
                storage: storageModel("available", true, true,
                                      [{ path: "/dev/sdb1", drivePath: "/drives/usb",
                                         name: "Photos", mounted: true,
                                         removable: true, ejectable: true }])
            });
            storageEjectSpy.target = panel;
            storageEjectSpy.clear();
            panel.ejectStorageVolume("/dev/sdb1");
            compare(storageEjectSpy.count, 1);
            compare(storageEjectSpy.signalArguments[0][0], "/drives/usb");
        }

        function test_storage_settings_link_is_accessible() {
            var panel = make({
                storage: storageModel("available", true, false)
            });
            var link = findChild(panel, "storageSettingsLink");
            verify(link !== null);
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Storage Settings");
        }

        function test_keyboard_tile_reflects_the_inventory() {
            var panel = make({
                input: inputModel("available", true,
                                  [{ name: "AT keyboard", kind: "keyboard" },
                                   { name: "TouchPad", kind: "touchpad" }])
            });
            compare(panel.inputVisible, true);
            compare(panel.inputLabel, "1 keyboards, 2 pointing devices");
            compare(panel.tiles[7].visible, true);

            panel.input = inputModel("available", false);
            compare(panel.inputVisible, false);
            compare(panel.tiles[7].visible, false);
        }

        function test_keyboard_tile_hides_on_absence() {
            var panel = make({ input: inputModel("unavailable", false) });
            compare(panel.inputVisible, false);
            var tile = findChild(panel, "keyboardTile");
            verify(tile !== null);
            compare(tile.visible, false);
        }

        function test_keyboard_settings_link_raises_the_request() {
            var panel = make({
                input: inputModel("available", true,
                                  [{ name: "AT keyboard", kind: "keyboard" }])
            });
            keyboardSettingsSpy.target = panel;
            keyboardSettingsSpy.clear();
            var link = findChild(panel, "keyboardSettingsLink");
            verify(link !== null, "the Keyboard Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Keyboard Settings");
            mouseClick(link, link.width / 2, link.height / 2);
            compare(keyboardSettingsSpy.count, 1);
        }

        function test_mission_control_tile_reflects_the_trigger_summary() {
            var panel = make({ missionControl: missionControlModel(true, 1) });
            compare(panel.missionControlVisible, true);
            compare(panel.missionControlLabel, "Gesture, 1 corner(s)");
            compare(panel.tiles[8].visible, true);
            compare(panel.tiles[8].enabled, true);

            // No gesture and no corner is a valid "No trigger" state.
            panel.missionControl = missionControlModel(false, 0);
            compare(panel.missionControlLabel, "No trigger");
            compare(panel.missionControl.reachable, false);

            // With no projection at all the tile hides.
            panel.missionControl = ({});
            compare(panel.missionControlVisible, false);
            compare(panel.tiles[8].visible, false);
        }

        function test_mission_control_settings_link_raises_the_request() {
            var panel = make({ missionControl: missionControlModel(true, 1) });
            missionControlSettingsSpy.target = panel;
            missionControlSettingsSpy.clear();
            var link = findChild(panel, "missionControlSettingsLink");
            verify(link !== null, "the Mission Control Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Mission Control Settings");
            mouseClick(link, link.width / 2, link.height / 2);
            compare(missionControlSettingsSpy.count, 1);
        }

        function test_battery_tile_reflects_charge_and_profile() {
            var panel = make({ battery: batteryModel(71, "balanced", true, true) });
            compare(panel.batteryVisible, true);
            compare(panel.batteryPercent, 71);
            compare(panel.batteryGlyph, "power-balanced");
            compare(panel.batteryLabel, "71% \u00b7 Balanced");
            compare(panel.tiles[9].visible, true);
            compare(panel.tiles[9].enabled, true);

            // A profile change pushed by the host converges, and the tile glyph
            // follows the active profile.
            panel.battery = batteryModel(70, "performance", true, true);
            compare(panel.batteryLabel, "70% \u00b7 Performance");
            compare(panel.batteryGlyph, "power-performance");

            // A desktop with no battery but selectable profiles still shows.
            panel.battery = batteryModel(0, "power-saver", true, false);
            compare(panel.batteryVisible, true);
            compare(panel.batteryLabel, "Power Saver");

            // No battery and no profiles hides the tile.
            panel.battery = batteryModel(0, "balanced", false, false);
            compare(panel.batteryVisible, false);
            compare(panel.tiles[9].visible, false);
        }

        function test_battery_tile_hides_on_absence() {
            var panel = make({ battery: ({ state: "unavailable" }) });
            compare(panel.batteryAvailable, false);
            compare(panel.batteryVisible, false);
            var tile = findChild(panel, "batteryTile");
            verify(tile !== null);
            compare(tile.visible, false);
        }

        function test_lock_screen_tile_reflects_the_policy() {
            var panel = make({ lockPolicy: lockPolicyModel(600) });
            compare(panel.lockScreenVisible, true);
            compare(panel.lockScreenLabel, "Password after 10 min");
            compare(panel.tiles[10].visible, true);
            compare(panel.tiles[10].enabled, true);

            // The delay converges when the settings-pane projection moves.
            panel.lockPolicy = lockPolicyModel(30);
            compare(panel.lockScreenLabel, "Password after 30 s");

            // `Never` (0) disables the lock stage, so no password is required.
            panel.lockPolicy = lockPolicyModel(0);
            compare(panel.lockScreenLabel, "No password required");
            compare(panel.lockPolicy.requirePassword, false);

            // No projection hides the tile.
            panel.lockPolicy = ({});
            compare(panel.lockScreenVisible, false);
            compare(panel.tiles[10].visible, false);
        }

        function test_lock_screen_settings_link_raises_the_request() {
            var panel = make({ lockPolicy: lockPolicyModel(600) });
            lockScreenSettingsSpy.target = panel;
            lockScreenSettingsSpy.clear();
            var link = findChild(panel, "lockScreenSettingsLink");
            verify(link !== null, "the Lock Screen Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Lock Screen Settings");
            mouseClick(link, link.width / 2, link.height / 2);
            compare(lockScreenSettingsSpy.count, 1);
        }

        function test_menu_bar_tile_reflects_the_auto_hide_mode() {
            var panel = make({ menuBar: menuBarModel("full-screen") });
            compare(panel.menuBarVisible, true);
            compare(panel.menuBarLabel, "In Full Screen Only");
            compare(panel.tiles[11].visible, true);
            compare(panel.tiles[11].enabled, true);

            // The mode converges when the settings-pane projection moves.
            panel.menuBar = menuBarModel("always");
            compare(panel.menuBarLabel, "Always");
            panel.menuBar = menuBarModel("never");
            compare(panel.menuBarLabel, "Never");

            // No projection hides the tile.
            panel.menuBar = ({});
            compare(panel.menuBarVisible, false);
            compare(panel.tiles[11].visible, false);
        }

        function test_menu_bar_settings_link_raises_the_request() {
            var panel = make({ menuBar: menuBarModel("full-screen") });
            menuBarSettingsSpy.target = panel;
            menuBarSettingsSpy.clear();
            var link = findChild(panel, "menuBarSettingsLink");
            verify(link !== null, "the Menu Bar Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Menu Bar Settings");
            mouseClick(link, link.width / 2, link.height / 2);
            compare(menuBarSettingsSpy.count, 1);
        }

        function test_software_update_tile_reflects_state() {
            var panel = make({ updates: updatesModel("1 Update Available", "available", 1) });
            compare(panel.updatesAvailable, true);
            compare(panel.updatesProvider, true);
            compare(panel.updatesCount, 1);
            compare(panel.updatesLabel, "1 Update Available");
            compare(panel.updatesAction, "Install");
            compare(panel.tiles[12].visible, true);
            compare(panel.tiles[12].enabled, true);

            // A host with no provider keeps the tile visible but inert.
            panel.updates = { kind: "updates", state: "available",
                              visible: true, enabled: true,
                              updatesAvailable: false,
                              label: "Software Update Unavailable" };
            compare(panel.updatesAvailable, true);
            compare(panel.updatesProvider, false);
            compare(panel.updatesLabel, "Software Update Unavailable");
            compare(panel.updatesAction, "Unavailable");

            // No host at all hides the tile.
            panel.updates = ({});
            compare(panel.updatesAvailable, false);
            compare(panel.tiles[12].visible, false);
        }

        function test_software_update_action_link_raises_the_state_action() {
            var panel = make({ updates: updatesModel("1 Update Available", "available", 1) });
            updatesInstallSpy.target = panel;
            updatesInstallSpy.clear();
            updatesCheckSpy.target = panel;
            updatesCheckSpy.clear();
            updatesRebootSpy.target = panel;
            updatesRebootSpy.clear();

            panel.performUpdatesAction();
            compare(updatesInstallSpy.count, 1);
            compare(updatesCheckSpy.count, 0);

            // Up to date: the action is a check.
            panel.updates = { kind: "updates", state: "available", visible: true,
                              enabled: true, updatesAvailable: true,
                              label: "Up to Date", updateCount: 0 };
            compare(panel.updatesAction, "Check for Updates");
            panel.performUpdatesAction();
            compare(updatesCheckSpy.count, 1);

            // Reboot required: the action is a restart.
            panel.updates = updatesModel("Restart Required", "reboot-required", 0);
            compare(panel.updatesAction, "Restart");
            panel.performUpdatesAction();
            compare(updatesRebootSpy.count, 1);
        }

        function test_software_update_settings_link_raises_the_request() {
            var panel = make({ updates: updatesModel("Up to Date", "up-to-date", 0) });
            updatesSettingsSpy.target = panel;
            updatesSettingsSpy.clear();
            var link = findChild(panel, "updatesSettingsLink");
            verify(link !== null, "the General Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open General Settings");
            mouseClick(link, link.width / 2, link.height / 2);
            compare(updatesSettingsSpy.count, 1);
        }

        function test_battery_settings_link_raises_the_request() {
            var panel = make({ battery: batteryModel(71, "balanced", true, true) });
            batterySettingsSpy.target = panel;
            batterySettingsSpy.clear();
            var link = findChild(panel, "batterySettingsLink");
            verify(link !== null, "the Battery Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Battery Settings");
            mouseClick(link, link.width / 2, link.height / 2);
            compare(batterySettingsSpy.count, 1);
        }

        function test_sound_tile_reflects_the_default_output_device() {
            var panel = make({ audio: audioRoutingModel("Built-in Speakers") });
            compare(panel.audioOutputName, "Built-in Speakers");
            compare(panel.audioOutputs.length, 2);
            compare(panel.tiles[4].subtitle, "Built-in Speakers");

            // A routing change pushed by the host (the pane's write) converges.
            panel.audio = audioRoutingModel("Headphones");
            compare(panel.audioOutputName, "Headphones");
        }

        function test_sound_settings_link_raises_the_request() {
            var panel = make({ audio: audioModel("available", 0.5, false) });
            soundSettingsSpy.target = panel;
            soundSettingsSpy.clear();
            var link = findChild(panel, "soundSettingsLink");
            verify(link !== null, "the Sound Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Sound Settings");
            mouseClick(link, link.width / 2, link.height / 2);
            compare(soundSettingsSpy.count, 1);
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
            compare(panel.tiles[3].checked, true);
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
            compare(panel.tiles[6].checked, false);
            compare(panel.tiles[6].subtitle, "Off");
            panel.dark = true;
            compare(panel.tiles[6].checked, true);
            compare(panel.tiles[6].subtitle, "On");
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