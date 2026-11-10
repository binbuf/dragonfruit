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
        Component { id: editComponent; EditControls { } }
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
        SignalSpy { id: usersSettingsSpy; signalName: "usersSettingsRequested" }
        SignalSpy { id: quickSettingsSettingsSpy; signalName: "quickSettingsSettingsRequested" }
        SignalSpy { id: quickSettingsWriteSpy; signalName: "quickSettingsWriteRequested" }
        SignalSpy { id: editOpenedSpy; signalName: "editControlsRequested" }
        SignalSpy { id: editClosedSpy; signalName: "editControlsClosed" }
        SignalSpy { id: screenshotSpy; signalName: "screenshotRequested" }
        SignalSpy { id: screencastSpy; signalName: "screencastRequested" }
        SignalSpy { id: focusModeSpy; signalName: "focusModeSetRequested" }        SignalSpy { id: printersSettingsSpy; signalName: "printersSettingsRequested" }
        SignalSpy { id: privacySettingsSpy; signalName: "privacySettingsRequested" }
        SignalSpy { id: accessibilitySettingsSpy; signalName: "accessibilitySettingsRequested" }
        SignalSpy { id: vpnSettingsSpy; signalName: "vpnSettingsRequested" }
        SignalSpy { id: focusSpy; signalName: "focusToggleRequested" }
        SignalSpy { id: darkSpy; signalName: "darkModeToggleRequested" }
        SignalSpy { id: closedSpy; signalName: "closed" }
        SignalSpy { id: clipboardCopySpy; signalName: "clipboardCopyRequested" }
        SignalSpy { id: clipboardPinSpy; signalName: "clipboardPinToggled" }
        SignalSpy { id: clipboardClearSpy; signalName: "clipboardClearRequested" }

        // These tests exercise the legacy sampled/fallback rail unless a
        // case explicitly selects the replacement look.
        function init() {
            Glass.rollout = "legacy";
        }

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

        function focusModel(mode, batchedCount, profileLabel) {
            return {
                mode: mode,
                allowList: [],
                batchedCount: batchedCount !== undefined ? batchedCount : 0,
                profileLabel: profileLabel !== undefined ? profileLabel : ""
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

        // The shell's decoded Users and Groups view (T-15.11b), shaped by
        // `SystemStatusModel`.
        function usersModel(label, humanCount, adminCount) {
            return {
                kind: "accounts",
                state: "available",
                visible: true,
                enabled: true,
                glyph: "users",
                label: label !== undefined ? label : "2 Users",
                present: true,
                humanCount: humanCount !== undefined ? humanCount : 2,
                adminCount: adminCount !== undefined ? adminCount : 1,
                lockedCount: 1,
                groupsAvailable: true,
                groupCount: 1,
                automaticLogin: "Dan Doe",
                automaticLoginUser: "dan",
                automaticLoginUid: 1000,
                users: [],
                groups: []
            };
        }

        // The shell's decoded Printers and Scanners view (T-15.12b), shaped by
        // `SystemStatusModel`.
        function printersModel(label, printerCount, scannerCount) {
            return {
                kind: "printers",
                state: "available",
                visible: true,
                enabled: true,
                glyph: "printer",
                label: label !== undefined ? label : "2 Printers, 1 Scanner",
                present: true,
                printersAvailable: true,
                scannersAvailable: true,
                printerCount: printerCount !== undefined ? printerCount : 2,
                scannerCount: scannerCount !== undefined ? scannerCount : 1,
                queuedJobCount: 0,
                defaultPrinter: "Canon_MF230"
            };
        }

        // The shell's decoded Privacy and Security view (T-15.13b), shaped by
        // `SystemStatusModel`.
        function privacyModel(label, appCount) {
            return {
                kind: "privacy",
                state: "available",
                visible: true,
                enabled: true,
                glyph: "privacy",
                label: label !== undefined ? label : "3 Apps",
                present: true,
                appCount: appCount !== undefined ? appCount : 3,
                grantedCount: 1,
                deniedCount: 1,
                categoryCount: 14
            };
        }

        // The shell's decoded Accessibility view (T-15.14b), shaped by
        // `SystemStatusModel`.
        function accessibilityModel(label, screenReader, enabled) {
            var sr = screenReader !== undefined ? screenReader : true;
            var en = enabled !== undefined ? enabled : true;
            return {
                kind: "accessibility",
                state: "available",
                visible: true,
                enabled: true,
                glyph: "accessibility",
                label: label !== undefined ? label
                    : (sr ? "Screen Reader On" : (en ? "On" : "Off")),
                present: sr || en,
                enabledValue: en,
                enabledLabel: en ? "On" : "Off",
                screenReader: sr,
                screenReaderLabel: sr ? "On" : "Off"
            };
        }

        // The shell's decoded Network advanced (VPN) view (T-15.15b), shaped by
        // `SystemStatusModel`.
        function vpnModel(label, connected, present) {
            var conn = connected !== undefined ? connected : true;
            var pres = present !== undefined ? present : true;
            return {
                kind: "vpn",
                state: "available",
                visible: pres,
                enabled: true,
                glyph: conn ? "vpn" : "vpn-off",
                label: label !== undefined ? label
                    : (conn ? "Work VPN" : "Not Connected"),
                present: pres,
                connectionCount: 2,
                connectedCount: conn ? 1 : 0,
                activeUuid: conn ? "11111111-1111-1111-1111-111111111111" : "",
                activeName: conn ? "Work VPN" : "",
                connections: []
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

        readonly property var legacyModules: [
            "wifi", "bluetooth", "storage", "focus", "volume", "brightness",
            "dark", "keyboard", "mission-control", "battery", "lock-screen",
            "menu-bar", "software-update", "users", "printers", "privacy",
            "accessibility", "vpn"
        ]

        function make(props) {
            props = props || {};
            var panel = createTemporaryObject(panelComponent, stage, props);
            // Most cases exercise one service module; keep the pre-T235
            // inventory visible unless the case is about the measured default
            // panel or an explicit customization.
            if (props.quickSettingsModules === undefined
                    && props.quickSettingsPage2 === undefined
                    && props.quickSettingsPage3 === undefined) {
                panel.quickSettingsModules = legacyModules.slice();
            }
            // The shell sizes the panel to its surface; do the same here so
            // pointer coordinates land on the controls.
            panel.width = 360;
            // The shell sizes the surface to 360x1164; keep the default
            // tall enough that an expanded sheet is inside the viewport.
            panel.height = 1164;
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
                accounts: usersModel("2 Users", 2, 1),
                printers: printersModel("2 Printers, 1 Scanner", 2, 1),
                privacy: privacyModel("3 Apps", 3),
                accessibility: accessibilityModel("Screen Reader On", true, true),
                vpn: vpnModel("Work VPN", true, true),
                brightness: 0.8,
                focusPolicy: focusModel("off"),
                dark: true
            });
            compare(panel.tiles.length, 18);
            compare(panel.tileById("wifi").id, "wifi");
            compare(panel.tileById("wifi").kind, "toggle");
            compare(panel.tileById("wifi").checked, true);
            compare(panel.tileById("bluetooth").id, "bluetooth");
            compare(panel.tileById("bluetooth").kind, "toggle");
            compare(panel.tileById("bluetooth").checked, true);
            compare(panel.tileById("storage").id, "storage");
            compare(panel.tileById("storage").kind, "storage");
            compare(panel.tileById("focus").id, "focus");
            compare(panel.tileById("focus").kind, "toggle");
            compare(panel.tileById("volume").id, "volume");
            compare(panel.tileById("volume").kind, "slider");
            compare(Math.abs(panel.tileById("volume").value - 0.6) < 0.0001, true);
            compare(panel.tileById("brightness").id, "brightness");
            compare(Math.abs(panel.tileById("brightness").value - 0.8) < 0.0001, true);
            compare(panel.tileById("dark").id, "dark");
            compare(panel.tileById("dark").kind, "toggle");
            compare(panel.tileById("dark").checked, true);
            compare(panel.tileById("keyboard").id, "keyboard");
            compare(panel.tileById("keyboard").kind, "info");
            compare(panel.tileById("keyboard").subtitle, "1 keyboards, 2 pointing devices");
            compare(panel.tileById("mission-control").id, "mission-control");
            compare(panel.tileById("mission-control").kind, "info");
            compare(panel.tileById("mission-control").subtitle, "Gesture, 1 corner(s)");
            compare(panel.tileById("battery").id, "battery");
            compare(panel.tileById("battery").kind, "info");
            compare(panel.tileById("battery").subtitle, "71% \u00b7 Balanced");
            compare(panel.tileById("lock-screen").id, "lock-screen");
            compare(panel.tileById("lock-screen").kind, "info");
            compare(panel.tileById("menu-bar").id, "menu-bar");
            compare(panel.tileById("menu-bar").kind, "info");
            compare(panel.tileById("menu-bar").subtitle, "In Full Screen Only");
            compare(panel.tileById("software-update").id, "software-update");
            compare(panel.tileById("software-update").kind, "info");
            compare(panel.tileById("software-update").subtitle, "1 Update Available");
            compare(panel.tileById("users").id, "users");
            compare(panel.tileById("users").kind, "info");
            compare(panel.tileById("users").subtitle, "2 Users");
            compare(panel.tileById("printers").id, "printers");
            compare(panel.tileById("printers").kind, "info");
            compare(panel.tileById("printers").subtitle, "2 Printers, 1 Scanner");
            compare(panel.tileById("privacy").id, "privacy");
            compare(panel.tileById("privacy").kind, "info");
            compare(panel.tileById("privacy").subtitle, "3 Apps");
            compare(panel.tileById("accessibility").id, "accessibility");
            compare(panel.tileById("accessibility").kind, "info");
            compare(panel.tileById("accessibility").subtitle, "Screen Reader On");
            compare(panel.tileById("vpn").id, "vpn");
            compare(panel.tileById("vpn").kind, "info");
            compare(panel.tileById("vpn").subtitle, "Work VPN");
            compare(panel.wifiLabel, "home");
        }

        function test_panel_content_fits_the_shell_surface() {
            // The shell sizes the Control Center surface to 360x1160
            // (kControlCenterWidth/Height). With the Bluetooth tile's device
            // rows, the Storage tile, the Sound tile's routing subtitle, the
            // Keyboard tile, the Mission Control tile, the Battery tile, the
            // Lock Screen tile, the Menu Bar tile, the Software Update tile, the
            // Users tile, the Printers tile, the Privacy tile, and the compact
            // Accessibility tile the content must still fit, or the lower tiles
            // are clipped.
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
                accounts: usersModel("2 Users", 2, 1),
                printers: printersModel("2 Printers, 1 Scanner", 2, 1),
                privacy: privacyModel("3 Apps", 3),
                accessibility: accessibilityModel("Screen Reader On", true, true),
                vpn: vpnModel("Work VPN", true, true),
                brightness: 1.0,
                focusPolicy: focusModel("off"),
                dark: false
            });
            panel.width = 360;
            panel.height = 1164;
            waitForRendering(stage);
            var content = findChild(panel, "controlCenterContent");
            verify(content !== null);
            verify(content.childrenRect.height <= 1164,
                   "Control Center content must fit the 1164px surface, height="
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
            compare(panel.tileById("bluetooth").enabled, true);
            compare(panel.bluetoothDevices.length, 1);
            compare(panel.bluetoothLabel, "Bluetooth on");

            panel.bluetooth = bluetoothModel("available", true, false, false);
            compare(panel.bluetoothOn, false);
            compare(panel.bluetoothLabel, "Bluetooth off");
        }

        function test_bluetooth_tile_hides_on_absence_and_no_controller() {
            var panel = make({ bluetooth: bluetoothModel("unavailable", false, false) });
            compare(panel.bluetoothVisible, false);
            compare(panel.tileById("bluetooth").visible, false);
            var tile = findChild(panel, "bluetoothTile");
            verify(tile === null || tile.visible === false);

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
            panel.openModule("bluetooth");
            waitForRendering(stage);
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
            compare(panel.tileById("storage").enabled, true);

            panel.storage = storageModel("available", true, false);
            compare(panel.storageMountedCount, 0);
            compare(panel.storageLabel, "Storage");
        }

        function test_storage_tile_hides_on_absence_and_no_volumes() {
            var panel = make({ storage: storageModel("unavailable", false, false) });
            compare(panel.storageVisible, false);
            compare(panel.tileById("storage").visible, false);
            var tile = findChild(panel, "storageTile");
            verify(tile === null || tile.visible === false);

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
            panel.openModule("storage");
            waitForRendering(stage);
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
            compare(panel.tileById("keyboard").visible, true);

            panel.input = inputModel("available", false);
            compare(panel.inputVisible, false);
            compare(panel.tileById("keyboard").visible, false);
        }

        function test_keyboard_tile_hides_on_absence() {
            var panel = make({ input: inputModel("unavailable", false) });
            compare(panel.inputVisible, false);
            var tile = findChild(panel, "keyboardTile");
            verify(tile === null || tile.visible === false);
        }

        function test_keyboard_settings_link_raises_the_request() {
            var panel = make({
                input: inputModel("available", true,
                                  [{ name: "AT keyboard", kind: "keyboard" }])
            });
            keyboardSettingsSpy.target = panel;
            keyboardSettingsSpy.clear();
            panel.openModule("keyboard");
            waitForRendering(stage);
            var link = findChild(panel, "keyboardSettingsLink");
            verify(link !== null, "the Keyboard Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Keyboard Settings");
            link.activated();
            compare(keyboardSettingsSpy.count, 1);
        }

        function test_mission_control_tile_reflects_the_trigger_summary() {
            var panel = make({ missionControl: missionControlModel(true, 1) });
            compare(panel.missionControlVisible, true);
            compare(panel.missionControlLabel, "Gesture, 1 corner(s)");
            compare(panel.tileById("mission-control").visible, true);
            compare(panel.tileById("mission-control").enabled, true);

            // No gesture and no corner is a valid "No trigger" state.
            panel.missionControl = missionControlModel(false, 0);
            compare(panel.missionControlLabel, "No trigger");
            compare(panel.missionControl.reachable, false);

            // With no projection at all the tile hides.
            panel.missionControl = ({});
            compare(panel.missionControlVisible, false);
            compare(panel.tileById("mission-control").visible, false);
        }

        function test_mission_control_settings_link_raises_the_request() {
            var panel = make({ missionControl: missionControlModel(true, 1) });
            missionControlSettingsSpy.target = panel;
            missionControlSettingsSpy.clear();
            panel.openModule("mission-control");
            waitForRendering(stage);
            var link = findChild(panel, "missionControlSettingsLink");
            verify(link !== null, "the Mission Control Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Overview Settings");
            link.activated();
            compare(missionControlSettingsSpy.count, 1);
        }

        function test_battery_tile_reflects_charge_and_profile() {
            var panel = make({ battery: batteryModel(71, "balanced", true, true) });
            compare(panel.batteryVisible, true);
            compare(panel.batteryPercent, 71);
            compare(panel.batteryGlyph, "power-balanced");
            compare(panel.batteryLabel, "71% \u00b7 Balanced");
            compare(panel.tileById("battery").visible, true);
            compare(panel.tileById("battery").enabled, true);

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
            compare(panel.tileById("battery").visible, false);
        }

        function test_battery_tile_hides_on_absence() {
            var panel = make({ battery: ({ state: "unavailable" }) });
            compare(panel.batteryAvailable, false);
            compare(panel.batteryVisible, false);
            var tile = findChild(panel, "batteryTile");
            verify(tile === null || tile.visible === false);
        }

        function test_lock_screen_tile_reflects_the_policy() {
            var panel = make({ lockPolicy: lockPolicyModel(600) });
            compare(panel.lockScreenVisible, true);
            compare(panel.lockScreenLabel, "Password after 10 min");
            compare(panel.tileById("lock-screen").visible, true);
            compare(panel.tileById("lock-screen").enabled, true);

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
            compare(panel.tileById("lock-screen").visible, false);
        }

        function test_lock_screen_settings_link_raises_the_request() {
            var panel = make({ lockPolicy: lockPolicyModel(600) });
            lockScreenSettingsSpy.target = panel;
            lockScreenSettingsSpy.clear();
            panel.openModule("lock-screen");
            waitForRendering(stage);
            var link = findChild(panel, "lockScreenSettingsLink");
            verify(link !== null, "the Lock Screen Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Lock Screen Settings");
            link.activated();
            compare(lockScreenSettingsSpy.count, 1);
        }

        function test_menu_bar_tile_reflects_the_auto_hide_mode() {
            var panel = make({ menuBar: menuBarModel("full-screen") });
            compare(panel.menuBarVisible, true);
            compare(panel.menuBarLabel, "In Full Screen Only");
            compare(panel.tileById("menu-bar").visible, true);
            compare(panel.tileById("menu-bar").enabled, true);

            // The mode converges when the settings-pane projection moves.
            panel.menuBar = menuBarModel("always");
            compare(panel.menuBarLabel, "Always");
            panel.menuBar = menuBarModel("never");
            compare(panel.menuBarLabel, "Never");

            // No projection hides the tile.
            panel.menuBar = ({});
            compare(panel.menuBarVisible, false);
            compare(panel.tileById("menu-bar").visible, false);
        }

        function test_menu_bar_settings_link_raises_the_request() {
            var panel = make({ menuBar: menuBarModel("full-screen") });
            menuBarSettingsSpy.target = panel;
            menuBarSettingsSpy.clear();
            panel.openModule("menu-bar");
            waitForRendering(stage);
            var link = findChild(panel, "menuBarSettingsLink");
            verify(link !== null, "the Menu Bar Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Menu Bar Settings");
            link.activated();
            compare(menuBarSettingsSpy.count, 1);
        }

        function test_software_update_tile_reflects_state() {
            var panel = make({ updates: updatesModel("1 Update Available", "available", 1) });
            compare(panel.updatesAvailable, true);
            compare(panel.updatesProvider, true);
            compare(panel.updatesCount, 1);
            compare(panel.updatesLabel, "1 Update Available");
            compare(panel.updatesAction, "Install");
            compare(panel.tileById("software-update").visible, true);
            compare(panel.tileById("software-update").enabled, true);

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
            compare(panel.tileById("software-update").visible, false);
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
            panel.openModule("software-update");
            waitForRendering(stage);
            var link = findChild(panel, "updatesSettingsLink");
            verify(link !== null, "the General Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open General Settings");
            link.activated();
            compare(updatesSettingsSpy.count, 1);
        }

        function test_users_tile_reflects_state() {
            var panel = make({ accounts: usersModel("2 Users", 2, 1) });
            compare(panel.accountsAvailable, true);
            compare(panel.accountsHumanCount, 2);
            compare(panel.accountsAdminCount, 1);
            compare(panel.accountsGlyph, "users");
            compare(panel.accountsLabel, "2 Users");
            compare(panel.tileById("users").visible, true);
            compare(panel.tileById("users").enabled, true);

            // A host with no group provider keeps the tile visible: the user
            // list is still live.
            panel.accounts = { kind: "accounts", state: "available", visible: true,
                               enabled: true, groupsAvailable: false,
                               label: "1 User", humanCount: 1, adminCount: 0 };
            compare(panel.accountsAvailable, true);
            compare(panel.accountsLabel, "1 User");

            // No host at all hides the tile.
            panel.accounts = ({});
            compare(panel.accountsAvailable, false);
            compare(panel.tileById("users").visible, false);
        }

        function test_users_tile_hides_on_absence() {
            var panel = make({ accounts: ({ state: "unavailable" }) });
            compare(panel.accountsAvailable, false);
            var tile = findChild(panel, "usersTile");
            verify(tile === null || tile.visible === false);
        }

        function test_users_settings_link_raises_the_request() {
            var panel = make({ accounts: usersModel("2 Users", 2, 1) });
            usersSettingsSpy.target = panel;
            usersSettingsSpy.clear();
            panel.openModule("users");
            waitForRendering(stage);
            var link = findChild(panel, "usersSettingsLink");
            verify(link !== null, "the Users & Groups Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Users & Groups Settings");
            link.activated();
            compare(usersSettingsSpy.count, 1);
        }

        function test_printers_tile_reflects_state() {
            var panel = make({ printers: printersModel("2 Printers, 1 Scanner", 2, 1) });
            compare(panel.printersAvailable, true);
            compare(panel.printersVisible, true);
            compare(panel.printersPrinterCount, 2);
            compare(panel.printersScannerCount, 1);
            compare(panel.printersGlyph, "printer");
            compare(panel.printersLabel, "2 Printers, 1 Scanner");
            compare(panel.tileById("printers").visible, true);
            compare(panel.tileById("printers").enabled, true);

            // A running host with no queue and no scanner (`present: false`)
            // hides the tile; the model normalizes it that way.
            panel.printers = {
                kind: "printers", state: "available", visible: false, enabled: true,
                present: false, printerCount: 0, scannerCount: 0, label: "No Printers or Scanners"
            };
            compare(panel.printersVisible, false);
            compare(panel.tileById("printers").visible, false);

            // No host at all hides the tile.
            panel.printers = ({});
            compare(panel.printersAvailable, false);
            compare(panel.tileById("printers").visible, false);
        }

        function test_printers_tile_hides_on_absence() {
            var panel = make({ printers: ({ state: "unavailable" }) });
            compare(panel.printersAvailable, false);
            var tile = findChild(panel, "printersTile");
            verify(tile === null || tile.visible === false);
        }

        function test_printers_settings_link_raises_the_request() {
            var panel = make({ printers: printersModel("2 Printers, 1 Scanner", 2, 1) });
            printersSettingsSpy.target = panel;
            printersSettingsSpy.clear();
            panel.openModule("printers");
            waitForRendering(stage);
            var link = findChild(panel, "printersSettingsLink");
            verify(link !== null, "the Printers & Scanners Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Printers & Scanners Settings");
            link.activated();
            compare(printersSettingsSpy.count, 1);
        }

        function test_privacy_tile_reflects_state() {
            var panel = make({ privacy: privacyModel("3 Apps", 3) });
            compare(panel.privacyAvailable, true);
            compare(panel.privacyVisible, true);
            compare(panel.privacyAppCount, 3);
            compare(panel.privacyGlyph, "privacy");
            compare(panel.privacyLabel, "3 Apps");
            compare(panel.tileById("privacy").visible, true);
            compare(panel.tileById("privacy").enabled, true);

            // A running store with no application permission (`present: false`)
            // hides the tile; the model normalizes it that way.
            panel.privacy = {
                kind: "privacy", state: "available", visible: false, enabled: true,
                present: false, appCount: 0, label: "No App Permissions"
            };
            compare(panel.privacyVisible, false);
            compare(panel.tileById("privacy").visible, false);

            // No host at all hides the tile.
            panel.privacy = ({});
            compare(panel.privacyAvailable, false);
            compare(panel.tileById("privacy").visible, false);
        }

        function test_privacy_tile_hides_on_absence() {
            var panel = make({ privacy: ({ state: "unavailable" }) });
            compare(panel.privacyAvailable, false);
            var tile = findChild(panel, "privacyTile");
            verify(tile === null || tile.visible === false);
        }

        function test_privacy_settings_link_raises_the_request() {
            var panel = make({ privacy: privacyModel("3 Apps", 3) });
            privacySettingsSpy.target = panel;
            privacySettingsSpy.clear();
            panel.openModule("privacy");
            waitForRendering(stage);
            var link = findChild(panel, "privacySettingsLink");
            verify(link !== null, "the Privacy & Security Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Privacy & Security Settings");
            link.activated();
            compare(privacySettingsSpy.count, 1);
        }

        function test_accessibility_tile_reflects_state() {
            var panel = make({ accessibility: accessibilityModel("Screen Reader On", true, true) });
            compare(panel.accessibilityAvailable, true);
            compare(panel.accessibilityVisible, true);
            compare(panel.accessibilityScreenReader, true);
            compare(panel.accessibilityGlyph, "accessibility");
            compare(panel.accessibilityLabel, "Screen Reader On");
            compare(panel.tileById("accessibility").visible, true);
            compare(panel.tileById("accessibility").enabled, true);

            // A bridge-only session labels itself `On`.
            panel.accessibility = accessibilityModel("On", false, true);
            compare(panel.accessibilityScreenReader, false);
            compare(panel.accessibilityLabel, "On");
            compare(panel.accessibilityVisible, true);

            // A bus with every feature off (`present: false`) hides the tile.
            panel.accessibility = {
                kind: "accessibility", state: "available", visible: false,
                enabled: true, present: false, label: "Off"
            };
            compare(panel.accessibilityVisible, false);
            compare(panel.tileById("accessibility").visible, false);
        }

        function test_accessibility_tile_hides_on_absence() {
            var panel = make({ accessibility: ({ state: "unavailable" }) });
            compare(panel.accessibilityAvailable, false);
            var tile = findChild(panel, "accessibilityTile");
            verify(tile === null || tile.visible === false);
        }

        function test_accessibility_tile_opens_the_pane() {
            var panel = make({ accessibility: accessibilityModel("Screen Reader On", true, true) });
            accessibilitySettingsSpy.target = panel;
            accessibilitySettingsSpy.clear();
            var tile = findChild(panel, "accessibilityTile");
            verify(tile !== null, "the Accessibility tile is present");
            compare(tile.Accessible.role, Accessible.Button);
            compare(tile.Accessible.name, "Accessibility");
            panel.openModule("accessibility");
            waitForRendering(stage);
            var link = findChild(panel, "accessibilitySettingsLink");
            verify(link !== null);
            link.activated();
            compare(accessibilitySettingsSpy.count, 1);
        }

        function test_vpn_tile_reflects_state() {
            var panel = make({ vpn: vpnModel("Work VPN", true, true) });
            compare(panel.vpnAvailable, true);
            compare(panel.vpnVisible, true);
            compare(panel.vpnConnected, true);
            compare(panel.vpnGlyph, "vpn");
            compare(panel.vpnLabel, "Work VPN");
            compare(panel.tileById("vpn").visible, true);
            compare(panel.tileById("vpn").enabled, true);

            // A disconnect round-trip converges into the tile.
            panel.vpn = vpnModel("Not Connected", false, true);
            compare(panel.vpnConnected, false);
            compare(panel.vpnGlyph, "vpn-off");
            compare(panel.vpnLabel, "Not Connected");
            compare(panel.vpnVisible, true);

            // NetworkManager with no VPN configured is `present: false`.
            panel.vpn = {
                kind: "vpn", state: "available", visible: false,
                enabled: true, present: false, label: "No VPN",
                connectionCount: 0, connectedCount: 0
            };
            compare(panel.vpnVisible, false);
            compare(panel.tileById("vpn").visible, false);
        }

        function test_vpn_tile_hides_on_absence() {
            var panel = make({ vpn: ({ state: "unavailable" }) });
            compare(panel.vpnAvailable, false);
            var tile = findChild(panel, "vpnTile");
            verify(tile === null || tile.visible === false);
        }

        function test_vpn_tile_opens_the_pane() {
            var panel = make({ vpn: vpnModel("Work VPN", true, true) });
            vpnSettingsSpy.target = panel;
            vpnSettingsSpy.clear();
            var tile = findChild(panel, "vpnTile");
            verify(tile !== null, "the VPN tile is present");
            compare(tile.Accessible.role, Accessible.Button);
            compare(tile.Accessible.name, "VPN");
            panel.openModule("vpn");
            waitForRendering(stage);
            var link = findChild(panel, "vpnSettingsLink");
            verify(link !== null);
            link.activated();
            compare(vpnSettingsSpy.count, 1);
        }

        function test_battery_settings_link_raises_the_request() {
            var panel = make({ battery: batteryModel(71, "balanced", true, true) });
            batterySettingsSpy.target = panel;
            batterySettingsSpy.clear();
            panel.openModule("battery");
            waitForRendering(stage);
            var link = findChild(panel, "batterySettingsLink");
            verify(link !== null, "the Battery Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Battery Settings");
            link.activated();
            compare(batterySettingsSpy.count, 1);
        }

        function test_sound_tile_reflects_the_default_output_device() {
            var panel = make({ audio: audioRoutingModel("Built-in Speakers") });
            compare(panel.audioOutputName, "Built-in Speakers");
            compare(panel.audioOutputs.length, 2);
            compare(panel.tileById("volume").subtitle, "Built-in Speakers");

            // A routing change pushed by the host (the pane's write) converges.
            panel.audio = audioRoutingModel("Headphones");
            compare(panel.audioOutputName, "Headphones");
        }

        function test_sound_settings_link_raises_the_request() {
            var panel = make({ audio: audioModel("available", 0.5, false) });
            soundSettingsSpy.target = panel;
            soundSettingsSpy.clear();
            panel.openModule("volume");
            waitForRendering(stage);
            var link = findChild(panel, "soundSettingsLink");
            verify(link !== null, "the Sound Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Sound Settings");
            link.activated();
            compare(soundSettingsSpy.count, 1);
        }

        function test_quick_settings_settings_link_raises_the_request() {
            var panel = make({});
            quickSettingsSettingsSpy.target = panel;
            quickSettingsSettingsSpy.clear();
            var link = findChild(panel, "quickSettingsSettingsLink");
            verify(link !== null, "the Quick Settings link is present");
            compare(link.Accessible.role, Accessible.Button);
            compare(link.Accessible.name, "Open Quick Settings Settings");
            link.activated();
            compare(quickSettingsSettingsSpy.count, 1);
        }

        function test_wifi_tile_is_read_only_until_the_adapter_can_write() {
            // The T-07 adapter exposes no radio write (T-15); the toggle
            // reflects state and is inert, so its model entry is disabled.
            var panel = make({ wifi: wifiModel("available", true, "home") });
            compare(panel.tileById("wifi").enabled, false);
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
            panel.openModule("brightness");
            waitForRendering(stage);
            var slider = findChild(panel, "brightnessSlider");
            verify(slider !== null);
            slider.setValue(0.25);
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
            panel.openModule("volume");
            waitForRendering(stage);
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
            compare(panel.tileById("focus").checked, true);
            verify(panel.focusSubtitle.indexOf("3") >= 0);
            verify(panel.focusSubtitle.indexOf("Do Not Disturb") >= 0);

            panel.focusPolicy = focusModel("focus");
            compare(panel.focusOn, true);
            compare(panel.focusLabel, "Focus");

            // An absent policy is the safe off default.
            // T234: the named profile is the tile label.
            panel.focusPolicy = focusModel("focus", 0, "Personal");
            compare(panel.focusLabel, "Personal");
            compare(panel.focusSubtitle, "Personal");
            panel.focusPolicy = focusModel("dnd", 2, "Sleep");
            compare(panel.focusLabel, "Sleep");
            verify(panel.focusSubtitle.indexOf("2") >= 0);

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
            toggle.toggle();
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
            compare(panel.tileById("dark").checked, false);
            compare(panel.tileById("dark").subtitle, "Off");
            panel.dark = true;
            compare(panel.tileById("dark").checked, true);
            compare(panel.tileById("dark").subtitle, "On");
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
            compare(panel.Accessible.name, "Quick Settings");

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
            compare(panel.tileById("wifi").enabled, false);
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

        function compareColor(actual, expected, label) {
            fuzzyCompare(actual.r, expected.r, 0.001, label + " red");
            fuzzyCompare(actual.g, expected.g, 0.001, label + " green");
            fuzzyCompare(actual.b, expected.b, 0.001, label + " blue");
            fuzzyCompare(actual.a, expected.a, 0.001, label + " alpha");
        }

        function test_control_center_material_regions_are_distinct_and_stable() {
            var savedRollout = Glass.rollout;
            var savedReduce = Glass.reduceTransparency;
            var savedContrast = Glass.increaseContrast;
            try {
                Glass.rollout = "replacement";
                Glass.reduceTransparency = false;
                Glass.increaseContrast = false;
                Theme.dark = false;
                var panel = make({
                    wifi: { state: "available", radioEnabled: true, label: "home",
                            networks: [
                                { ssid: "home", strength: 82, security: "WPA2",
                                  secured: true, active: true, band: "5 GHz" },
                                { ssid: "guest", strength: 61, security: "Open",
                                  secured: false, active: false, band: "2.4 GHz" }
                            ] },
                    bluetooth: bluetoothModel("available", true, true, false,
                                              [{ address: "AA:BB:CC:DD:EE:FF",
                                                 name: "WF-1000XM6", connected: true }]),
                    materialBlurActive: true,
                    materialTier: 0,
                    materialFallback: 0
                });
                compare(panel.sampledGlass, true);
                compare(panel.outerGlass.preset, "controlCenterOuter");
                compare(panel.moduleGlass.preset, "controlCenterModule");
                compare(panel.expandedGlass.preset, "controlCenterExpanded");
                fuzzyCompare(panel.outerGlass.alpha, 0.265, 0.001, "outer alpha");
                fuzzyCompare(panel.moduleGlass.alpha, 0.391, 0.001,
                             "module effective alpha");
                compare(panel.outerFill.a, 0.0);
                compare(panel.moduleFill.a, 0.0);
                compare(panel.expandedFill.a, 0.0);

                var regions = panel.materialRegions;
                verify(regions.length >= 3, "outer + modules expected");
                compare(regions[0].key, "outer");
                compare(regions[0].preset, "controlCenterOuter");
                var seen = {};
                var modules = 0;
                for (var i = 0; i < regions.length; ++i) {
                    var region = regions[i];
                    verify(region.key.length > 0);
                    verify(!seen[region.key], "duplicate region " + region.key);
                    seen[region.key] = true;
                    verify(region.w > 0 && region.h > 0, region.key);
                    if (region.preset === "controlCenterModule")
                        ++modules;
                }
                verify(modules >= 2, "visible modules are separate regions");
                verify(regions.length <= 32, "region bound");
                verify(seen["module-wifi"] === true);
                verify(seen["module-bluetooth"] === true);
            } finally {
                Glass.rollout = savedRollout;
                Glass.reduceTransparency = savedReduce;
                Glass.increaseContrast = savedContrast;
                Theme.dark = false;
            }
        }

        function test_control_center_solid_fallback_and_accessibility_variants() {
            var savedRollout = Glass.rollout;
            var savedReduce = Glass.reduceTransparency;
            var savedContrast = Glass.increaseContrast;
            try {
                Glass.rollout = "replacement";
                Glass.reduceTransparency = true;
                Glass.increaseContrast = false;
                Theme.dark = false;
                var solid = make({
                    wifi: { state: "available", radioEnabled: true, label: "home" },
                    materialBlurActive: false,
                    materialTier: 2,
                    materialFallback: 0
                });
                compare(solid.sampledGlass, false);
                compare(solid.outerGlass.renderMode, "solid");
                verify(solid.outerFill.a >= 0.95, "outer solid plate");
                verify(solid.moduleFill.a >= 0.9, "module solid plate");
                verify(solid.moduleBorderWidth >= 1, "module rim on solid path");
                compare(solid.outerBorderWidth >= 1, true);

                Glass.reduceTransparency = false;
                Glass.increaseContrast = true;
                var contrast = make({
                    wifi: { state: "available", radioEnabled: true, label: "home" },
                    materialBlurActive: true,
                    materialTier: 0,
                    materialFallback: 0
                });
                verify(contrast.outerGlass.edge > 0.3,
                       "Increase Contrast raises the outer rim");
                fuzzyCompare(contrast.outerGlass.edge, 0.65, 0.001,
                             "contrast outer rim");
                verify(contrast.outerGlass.edgeWidth > 1.0,
                       "contrast widens the compositor-painted rim");
            } finally {
                Glass.rollout = savedRollout;
                Glass.reduceTransparency = savedReduce;
                Glass.increaseContrast = savedContrast;
                Theme.dark = false;
            }
        }

        function test_expanded_wifi_and_bluetooth_lists_add_regions() {
            var panel = make({
                wifi: { state: "available", radioEnabled: true, label: "home",
                        networks: [
                            { ssid: "home", strength: 82, security: "WPA2",
                              secured: true, active: true, band: "5 GHz" },
                            { ssid: "guest", strength: 61, security: "Open",
                              secured: false, active: false, band: "2.4 GHz" }
                        ] },
                bluetooth: bluetoothModel("available", true, true, false,
                                          [{ address: "AA:BB:CC:DD:EE:FF",
                                             name: "WF-1000XM6", connected: true }])
            });
            compare(panel.wifiNetworks.length, 2);
            compare(panel.expandedId, "");

            panel.openModule("wifi");
            waitForRendering(stage);
            compare(panel.expandedId, "wifi");
            var wifiList = findChild(panel, "wifiSheet");
            verify(wifiList !== null);
            verify(findChild(wifiList, "wifiNetworkName") !== null);
            var wifiKeys = panel.materialRegions.map(function(region) {
                return region.key;
            });
            verify(wifiKeys.indexOf("expanded-wifi") >= 0);

            panel.openModule("bluetooth");
            waitForRendering(stage);
            compare(panel.expandedId, "bluetooth");
            var bluetoothList = findChild(panel, "bluetoothSheet");
            verify(bluetoothList !== null);
            verify(findChild(bluetoothList, "bluetoothDeviceName") !== null);
            var keys = panel.materialRegions.map(function(region) {
                return region.key;
            });
            verify(keys.indexOf("expanded-bluetooth") >= 0);
            compare(keys.indexOf("expanded-wifi"), -1);

            panel.closeSheet();
            waitForRendering(stage);
            compare(panel.expandedId, "");
            var collapsedKeys = panel.materialRegions.map(function(region) {
                return region.key;
            });
            compare(collapsedKeys.indexOf("expanded-wifi"), -1);
            compare(collapsedKeys.indexOf("expanded-bluetooth"), -1);
        }

        function test_expanded_lists_stay_inside_scroll_bounds() {
            var panel = make({
                wifi: { state: "available", radioEnabled: true, label: "home",
                        networks: [
                            { ssid: "home", strength: 82, security: "WPA2",
                              secured: true, active: true, band: "5 GHz" }
                        ] },
                bluetooth: bluetoothModel("available", true, true, false,
                                          [{ address: "AA:BB:CC:DD:EE:FF",
                                             name: "WF-1000XM6", connected: true }])
            });
            panel.width = 360;
            panel.height = 400;
            panel.openModule("wifi");
            waitForRendering(stage);
            var scroll = findChild(panel, "controlCenterScroll");
            verify(scroll !== null);
            compare(scroll.boundsBehavior, Flickable.StopAtBounds);
            verify(scroll.contentHeight > scroll.height,
                   "expanded content scrolls in a short surface");
            var regions = panel.materialRegions;
            for (var i = 1; i < regions.length; ++i) {
                var region = regions[i];
                verify(region.x >= scroll.x - 0.5, region.key + " left bound");
                verify(region.y >= scroll.y - 0.5, region.key + " top bound");
                verify(region.x + region.w <= scroll.x + scroll.width + 0.5,
                       region.key + " right bound");
                verify(region.y + region.h <= scroll.y + scroll.height + 0.5,
                       region.key + " bottom bound");
            }
        }

        function test_expanded_disclosures_are_accessible_actions() {
            var panel = make({
                wifi: { state: "available", radioEnabled: true, label: "home",
                        networks: [] },
                bluetooth: bluetoothModel("available", true, true, false)
            });
            panel.openModule("wifi");
            waitForRendering(stage);
            var wifiDisclosure = findChild(panel, "wifiDisclosure");
            verify(wifiDisclosure !== null);
            compare(wifiDisclosure.Accessible.role, Accessible.Button);
            compare(wifiDisclosure.Accessible.name, "Show other Wi-Fi networks");
            compare(wifiDisclosure.activeFocusOnTab, true);
            wifiDisclosure.activated();
            compare(panel.wifiSettingsRequested !== undefined, true);

            panel.openModule("bluetooth");
            waitForRendering(stage);
            var bluetoothDisclosure = findChild(panel, "bluetoothDisclosure");
            verify(bluetoothDisclosure !== null);
            compare(bluetoothDisclosure.Accessible.role, Accessible.Button);
            compare(bluetoothDisclosure.Accessible.name, "Hide Bluetooth devices");
            compare(bluetoothDisclosure.activeFocusOnTab, true);
        }

        function test_light_sampled_modules_switch_to_light_foreground() {
            var savedRollout = Glass.rollout;
            var savedReduce = Glass.reduceTransparency;
            try {
                Glass.rollout = "replacement";
                Glass.reduceTransparency = false;
                Theme.dark = false;
                var sampled = make({ materialBlurActive: true, materialTier: 0,
                                     materialFallback: 0 });
                verify(sampled.moduleTextPrimary.r > 0.8,
                       "light-on-glass text on the sampled light module");
                verify(sampled.moduleAccent.r > 0.8,
                       "light accent on the sampled light module");

                Glass.reduceTransparency = true;
                var solid = make({ materialBlurActive: false, materialTier: 2,
                                   materialFallback: 0 });
                verify(solid.moduleTextPrimary.r < 0.3,
                       "scheme text on the light solid plate");
            } finally {
                Glass.rollout = savedRollout;
                Glass.reduceTransparency = savedReduce;
                Theme.dark = false;
            }
        }

        function test_popup_role_material_covers_the_control_center() {
            // T-21.2c: Control Center is the transient Popup role. The panel
            // keeps its geometry and rim; only the fill switches between the
            // sampled-glass scrim and the milky fallback.
            var states = [
                { tier: 0, blur: true, fallback: 0, reason: "none", label: "full" },
                { tier: 1, blur: true, fallback: 0, reason: "none", label: "reduced" },
                { tier: 2, blur: false, fallback: 0, reason: "minimal", label: "minimal" },
                { tier: 2, blur: false, fallback: 2, reason: "software", label: "software" }
            ];
            for (var scheme = 0; scheme < 2; ++scheme) {
                Theme.dark = scheme === 1;
                for (var i = 0; i < states.length; ++i) {
                    var state = states[i];
                    var panel = make({
                        materialTier: state.tier,
                        materialBlurActive: state.blur,
                        materialFallback: state.fallback
                    });
                    compare(panel.materialTier, state.tier, state.label);
                    compare(panel.materialBlurActive, state.blur, state.label);
                    compare(panel.materialFallback, state.fallback, state.label);
                    compare(panel.materialFallbackReason, state.reason,
                            state.label);
                    var surface = findChild(panel, "controlCenterSurface");
                    verify(surface !== null);
                    compareColor(surface.color, Material.popupFill(state.blur),
                                 state.label);
                    verify(surface.color.a >= 0.9 || state.blur,
                           state.label + " fallback must read as a plate");
                }
            }
            Theme.dark = false;
        }

        // T227: the Quick Settings pane writes the ordered module list and
        // the menu-bar-only set; the live panel filters and orders from them.
        function test_quick_settings_customization_changes_the_live_panel() {
            var panel = make({
                wifi: wifiModel("available", true, "home"),
                bluetooth: bluetoothModel("available", true, true, false),
                audio: audioModel("available", 0.6, false),
                dark: true
            });
            compare(panel.tiles.length, 18);

            panel.quickSettingsModules = ["dark", "wifi", "bogus"];
            panel.quickSettingsMenuBarOnly = [];
            waitForRendering(stage);
            compare(panel.tiles.length, 2);
            compare(panel.tiles[0].id, "dark");
            compare(panel.tiles[1].id, "wifi");

            // The live scene hides a removed module and keeps the ordered one.
            var wifiTile = findChild(panel, "wifiTile");
            var darkTile = findChild(panel, "darkTile");
            var volumeTile = findChild(panel, "volumeTile");
            verify(wifiTile !== null && darkTile !== null);
            compare(wifiTile.visible, true);
            compare(darkTile.visible, true);
            verify(volumeTile === null || volumeTile.visible === false);

            // A menu-bar-only module leaves the panel even when it is in the
            // ordered list.
            panel.quickSettingsMenuBarOnly = ["wifi"];
            compare(panel.tiles.length, 1);
            compare(panel.tiles[0].id, "dark");
            verify(wifiTile === null || wifiTile.visible === false);

            // Restoring the built-in order restores every module.
            panel.quickSettingsModules = [];
            panel.quickSettingsMenuBarOnly = [];
            compare(panel.tiles.length, 13);
            compare(panel.tiles[0].id, "wifi");
            compare(panel.tiles[1].id, "bluetooth");
        }

        // ── T235: the measured grid, catalogue and customization ─────────

        function defaultPanel(props) {
            var merged = props || {};
            merged.quickSettingsModules = [];
            return make(merged);
        }

        function test_default_panel_is_the_measured_inventory() {
            var panel = defaultPanel({
                wifi: wifiModel("available", true, "home"),
                bluetooth: bluetoothModel("available", true, true, false),
                audio: audioModel("available", 0.6, false)
            });
            var expected = [
                "wifi", "bluetooth", "now-playing", "airdrop", "stage-manager",
                "screen-mirroring", "dark", "capture-screen", "focus",
                "brightness", "volume", "airplay-audio", "edit-controls"
            ];
            compare(panel.tiles.length, expected.length);
            for (var i = 0; i < expected.length; ++i)
                compare(panel.tiles[i].id, expected[i]);

            // The measured spans: 1×1=64, 2×1=140×64, 2×2=140×140, 4×1=292×64.
            var wifi = panel.tileById("wifi");
            compare(wifi.span, "2x1");
            compare(wifi.w, 140);
            compare(wifi.h, 64);
            var now = panel.tileById("now-playing");
            compare(now.span, "2x2");
            compare(now.w, 140);
            compare(now.h, 140);
            var display = panel.tileById("brightness");
            compare(display.span, "4x1");
            compare(display.w, 292);
            compare(display.h, 64);
            compare(display.w, 4 * 64 + 3 * 12);
            var stage = panel.tileById("stage-manager");
            compare(stage.w, 64);
            compare(stage.h, 64);
            compare(panel.tileById("edit-controls").footer, true);

            // The grid is 292 pt wide and modules do not overlap.
            compare(ControlCenterRegistry.moduleFieldWidth, 292);
            var seen = {};
            for (var j = 0; j < panel.gridTiles.length; ++j) {
                var tile = panel.gridTiles[j];
                for (var x = tile.x; x < tile.x + tile.w; x += 8) {
                    for (var y = tile.y; y < tile.y + tile.h; y += 8) {
                        var key = x + "," + y;
                        verify(!seen[key], "overlap at " + key);
                        seen[key] = true;
                    }
                }
            }
        }

        function test_registry_catalogue_has_the_22_categories() {
            compare(ControlCenterRegistry.categories.length, 22);
            var expected = [
                "All Controls", "Battery", "Clock", "Connectivity",
                "Desktop & Files", "Display & Brightness",
                "Fast User Switching", "Focus", "Home", "Keyboards",
                "Lock Screen", "Notes", "Now Playing", "Screenshot",
                "Shortcuts", "Sounds", "Utilities", "Voice Memos",
                "Accessibility", "Hearing Accessibility",
                "Motor Accessibility", "Vision Accessibility"
            ];
            for (var i = 0; i < expected.length; ++i)
                compare(ControlCenterRegistry.categories[i].label, expected[i]);

            var all = ControlCenterRegistry.categoryModules("all-controls");
            verify(all.length >= 30);
            verify(all.indexOf("battery") >= 0);
            verify(all.indexOf("keyboard-brightness") >= 0);
            verify(all.indexOf("accessibility") >= 0);
            verify(all.indexOf("fast-user-switching") >= 0);
            verify(all.indexOf("screen-mirroring") >= 0);

            // The default panel is a subset of the catalogue.
            for (var j = 0; j < ControlCenterRegistry.defaultModules.length; ++j)
                verify(all.indexOf(ControlCenterRegistry.defaultModules[j]) >= 0);
            verify(ControlCenterRegistry.defaultModules.length
                   < all.length);
        }

        function test_span_size_math_is_the_measured_grid() {
            var one = ControlCenterRegistry.spanSize("1x1");
            compare(one.w, 64); compare(one.h, 64);
            var two = ControlCenterRegistry.spanSize("2x1");
            compare(two.w, 140); compare(two.h, 64);
            var square = ControlCenterRegistry.spanSize("2x2");
            compare(square.w, 140); compare(square.h, 140);
            var wide = ControlCenterRegistry.spanSize("4x1");
            compare(wide.w, 292); compare(wide.h, 64);
            compare(ControlCenterRegistry.nextSpan("1x1"), "2x1");
            compare(ControlCenterRegistry.nextSpan("2x1"), "2x2");
            compare(ControlCenterRegistry.nextSpan("2x2"), "4x1");
            compare(ControlCenterRegistry.nextSpan("4x1"), "1x1");
        }

        function test_expanded_module_grows_the_panel_and_uses_measured_sizes() {
            var savedRollout = Glass.rollout;
            Glass.rollout = "replacement";
            var panel = defaultPanel({
                wifi: { state: "available", radioEnabled: true, label: "home",
                        networks: [
                            { ssid: "home", strength: 82, security: "WPA2",
                              secured: true, active: true, band: "5 GHz" }
                        ] },
                bluetooth: bluetoothModel("available", true, true, false,
                                          [{ address: "AA:BB:CC:DD:EE:FF",
                                             name: "WF-1000XM6", connected: true }])
            });
            var collapsed = panel.panelHeight;
            panel.openModule("wifi");
            waitForRendering(stage);
            compare(panel.expandedId, "wifi");
            verify(panel.panelHeight > collapsed, "the outer panel grows");
            var wifiSheet = findChild(panel, "wifiSheet");
            verify(wifiSheet !== null);
            compare(wifiSheet.width, 308);
            compare(panel.outerGlass.preset, "controlCenterExpandedOuter");
            fuzzyCompare(panel.outerGlass.alpha, 0.286, 0.001, "expanded outer alpha");
            fuzzyCompare(panel.outerGlass.tint, 0.257, 0.001, "expanded outer tint");

            panel.openModule("bluetooth");
            waitForRendering(stage);
            var bluetoothSheet = findChild(panel, "bluetoothSheet");
            verify(bluetoothSheet !== null);
            compare(bluetoothSheet.width, 308);
            panel.closeSheet();
            waitForRendering(stage);
            compare(panel.outerGlass.preset, "controlCenterOuter");
            Glass.rollout = savedRollout;
        }

        function test_every_default_module_opens_its_sheet() {
            var panel = defaultPanel({
                wifi: wifiModel("available", true, "home"),
                bluetooth: bluetoothModel("available", true, true, false),
                audio: audioModel("available", 0.6, false)
            });
            for (var i = 0; i < ControlCenterRegistry.defaultModules.length; ++i) {
                var id = ControlCenterRegistry.defaultModules[i];
                if (id === "edit-controls")
                    continue;
                panel.openModule(id);
                waitForRendering(stage);
                compare(panel.expandedId, id);
                panel.closeSheet();
            }
        }

        function test_absence_is_explicit_for_every_module() {
            var panel = defaultPanel({
                wifi: ({ state: "unavailable" }),
                bluetooth: bluetoothModel("unavailable", false, false),
                audio: audioModel("unavailable", 0, false)
            });
            compare(panel.tileById("wifi").available, false);
            compare(panel.tileById("wifi").subtitle, "Unavailable");
            compare(panel.tileById("airdrop").available, false);
            verify(panel.tileById("airdrop").reason.indexOf("Nearby Transfer") >= 0);
            compare(panel.tileById("stage-manager").available, false);
            verify(panel.tileById("stage-manager").reason.length > 0);
            compare(panel.tileById("now-playing").available, false);
            verify(panel.tileById("now-playing").reason.length > 0);
            compare(panel.tileById("airplay-audio").available, false);

            panel.openModule("wifi");
            waitForRendering(stage);
            var empty = findChild(panel, "wifiNetworksEmpty");
            verify(empty !== null);
            compare(empty.text, "Wi-Fi unavailable");
        }

        function test_pages_switch_and_write_the_active_page() {
            var panel = defaultPanel({ dark: true });
            panel.quickSettingsPageCount = 2;
            panel.quickSettingsPage2 = ["dark"];
            panel.quickSettingsActivePage = 1;
            waitForRendering(stage);
            compare(panel.pageCount, 2);
            compare(panel.activePage, 1);
            compare(panel.tiles.length, 1);
            compare(panel.tiles[0].id, "dark");

            quickSettingsWriteSpy.target = panel;
            quickSettingsWriteSpy.clear();
            panel.setActivePage(0);
            compare(quickSettingsWriteSpy.count, 1);
            compare(quickSettingsWriteSpy.signalArguments[0][0],
                    "quickSettings.activePage");
            compare(quickSettingsWriteSpy.signalArguments[0][1], 0);
            compare(panel.activePage, 0);
        }

        function test_edit_controls_geometry_and_22_categories() {
            var editor = createTemporaryObject(editComponent, stage, {
                panelModules: ControlCenterRegistry.defaultModules.slice(),
                pageCount: 1,
                activePage: 0
            });
            verify(editor !== null);
            compare(editor.width, 736);
            compare(editor.height, 900);
            compare(editor.sidebarWidth, 245);
            var sidebar = findChild(editor, "editControlsSidebar");
            verify(sidebar !== null);
            compare(sidebar.width, 245);
            var list = findChild(editor, "editControlsList");
            verify(list !== null);
            compare(list.count, ControlCenterRegistry.categoryModules("all-controls").length);
            compare(ControlCenterRegistry.categories.length, 22);
        }

        function test_edit_controls_add_remove_resize_and_pin() {
            var editor = createTemporaryObject(editComponent, stage, {
                panelModules: ControlCenterRegistry.defaultModules.slice(),
                pageCount: 1,
                activePage: 0
            });
            verify(editor !== null);
            quickSettingsWriteSpy.target = editor;
            quickSettingsWriteSpy.clear();

            // Resize cycles the measured span.
            editor.cycleSpan("wifi");
            compare(quickSettingsWriteSpy.count, 1);
            compare(quickSettingsWriteSpy.signalArguments[0][0],
                    "quickSettings.spans");
            var spans = quickSettingsWriteSpy.signalArguments[0][1];
            verify(spans.indexOf("wifi=2x2") >= 0);

            // Add/remove writes the active page list.
            quickSettingsWriteSpy.clear();
            editor.setEnabled("storage", true);
            compare(quickSettingsWriteSpy.count, 1);
            compare(quickSettingsWriteSpy.signalArguments[0][0],
                    "quickSettings.modules");
            verify(quickSettingsWriteSpy.signalArguments[0][1].indexOf("storage") >= 0);
            quickSettingsWriteSpy.clear();
            editor.setEnabled("wifi", false);
            verify(quickSettingsWriteSpy.signalArguments[0][1].indexOf("wifi") < 0);

            // Menu-bar-only removes the control from the panel and writes the
            // separate mark.
            quickSettingsWriteSpy.clear();
            compare(editor.menuBarOnly.length, 0);
            editor.setMenuBarOnly("bluetooth", true);
            verify(editor.menuBarOnly.length === 1);
            var keys = [];
            for (var i = 0; i < quickSettingsWriteSpy.count; ++i)
                keys.push(quickSettingsWriteSpy.signalArguments[i][0]);
            verify(keys.indexOf("quickSettings.menuBarOnly") >= 0);
            verify(keys.indexOf("quickSettings.modules") >= 0);
            verify(keys.indexOf("menu.control.bluetooth") >= 0);
            verify(editor.isMenuBarOnly("bluetooth"));
        }

        function test_edit_controls_pages_and_reset() {
            var editor = createTemporaryObject(editComponent, stage, {
                panelModules: ControlCenterRegistry.defaultModules.slice(),
                pageCount: 1,
                activePage: 0
            });
            verify(editor !== null);
            quickSettingsWriteSpy.target = editor;
            quickSettingsWriteSpy.clear();
            editor.addPage();
            var keys = [];
            for (var i = 0; i < quickSettingsWriteSpy.count; ++i)
                keys.push(quickSettingsWriteSpy.signalArguments[i][0]);
            verify(keys.indexOf("quickSettings.pageCount") >= 0);
            verify(keys.indexOf("quickSettings.activePage") >= 0);

            quickSettingsWriteSpy.clear();
            editor.reset();
            keys = [];
            for (var j = 0; j < quickSettingsWriteSpy.count; ++j)
                keys.push(quickSettingsWriteSpy.signalArguments[j][0]);
            verify(keys.indexOf("quickSettings.modules") >= 0);
            verify(keys.indexOf("quickSettings.spans") >= 0);
            verify(keys.indexOf("quickSettings.perDisplay") >= 0);
        }

        function test_keyboard_and_ax_on_the_grid() {
            var panel = defaultPanel({ dark: false });
            var wifi = findChild(panel, "wifiTile");
            verify(wifi !== null);
            compare(wifi.activeFocusOnTab, true);
            compare(wifi.Accessible.name, "Wi-Fi");
            compare(wifi.Accessible.role, Accessible.Grouping);
            var dark = findChild(panel, "darkTile");
            verify(dark !== null);
            compare(dark.activeFocusOnTab, true);
            compare(dark.Accessible.name, "Dark Mode");
            var page = findChild(panel, "controlCenterPages");
            verify(page !== null);
            compare(panel.reduceMotion, Theme.reducedMotion);
        }

        // T235: headless evidence for the measured grid, expanded sheets,
        // Edit Controls catalogue, pages, resizing and absence. Skipped unless
        // `DF_CONTROL_CENTER_CAPTURE_DIR` is set (make t235-control-center-capture).
        function test_t235_capture_states() {
            if (typeof TestEnvironment === "undefined"
                    || TestEnvironment.captureDir.length === 0)
                skip("DF_CONTROL_CENTER_CAPTURE_DIR not set");
            var dir = TestEnvironment.captureDir;
            var savedRollout = Glass.rollout;
            var savedReduce = Glass.reduceTransparency;
            var savedWidth = stage.width;
            var savedHeight = stage.height;
            try {
                Glass.rollout = "replacement";
                Glass.reduceTransparency = true;
                Theme.dark = false;
                stage.width = 760;
                stage.height = 1200;

                var panel = defaultPanel({
                    wifi: { state: "available", radioEnabled: true, label: "home",
                            networks: [
                                { ssid: "home", strength: 82, security: "WPA2",
                                  secured: true, active: true, band: "5 GHz" },
                                { ssid: "guest", strength: 61, security: "Open",
                                  secured: false, active: false, band: "2.4 GHz" }
                            ] },
                    bluetooth: bluetoothModel("available", true, true, false,
                                              [{ address: "AA:BB:CC:DD:EE:FF",
                                                 name: "WF-1000XM6", connected: true },
                                               { address: "11:22:33:44:55:66",
                                                 name: "WH-1000XM6", connected: false }]),
                    audio: audioRoutingModel("Built-in Speakers"),
                    battery: batteryModel(71, "balanced", true, true),
                    focusPolicy: focusModel("dnd", 3),
                    dark: false
                });
                panel.width = 360;
                panel.height = 1164;
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-grid-light.png", "PNG");

                Theme.dark = true;
                panel.dark = true;
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-grid-dark.png", "PNG");

                Theme.dark = false;
                panel.dark = false;
                panel.openModule("wifi");
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-expanded-wifi.png", "PNG");
                panel.openModule("bluetooth");
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-expanded-bluetooth.png", "PNG");
                panel.openModule("focus");
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-expanded-focus.png", "PNG");
                panel.openModule("now-playing");
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-expanded-now-playing.png", "PNG");
                panel.openModule("volume");
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-expanded-sound.png", "PNG");
                panel.openModule("brightness");
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-expanded-display.png", "PNG");
                panel.openModule("battery");
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-expanded-battery.png", "PNG");
                panel.closeSheet();

                panel.quickSettingsPageCount = 2;
                panel.quickSettingsPage2 = ["storage", "keyboard", "mission-control"];
                panel.quickSettingsActivePage = 1;
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-page-2.png", "PNG");

                panel.quickSettingsActivePage = 0;
                panel.quickSettingsSpans = ["wifi=2x2", "bluetooth=4x1",
                                            "dark=1x1", "focus=1x1"];
                waitForRendering(stage);
                grabImage(panel).save(dir + "/t235-control-center-resized.png", "PNG");

                var absent = defaultPanel({
                    wifi: ({ state: "unavailable" }),
                    bluetooth: bluetoothModel("unavailable", false, false),
                    audio: audioModel("unavailable", 0, false),
                    battery: ({ state: "unavailable" })
                });
                absent.width = 360;
                absent.height = 1164;
                waitForRendering(stage);
                grabImage(absent).save(dir + "/t235-control-center-absent.png", "PNG");

                var editor = createTemporaryObject(editComponent, stage, {
                    panelModules: ControlCenterRegistry.defaultModules.slice(),
                    pageCount: 2,
                    activePage: 0,
                    menuBarOnly: ["accessibility"]
                });
                editor.width = 736;
                editor.height = 900;
                waitForRendering(stage);
                grabImage(editor).save(dir + "/t235-edit-controls-light.png", "PNG");
                Theme.dark = true;
                waitForRendering(stage);
                grabImage(editor).save(dir + "/t235-edit-controls-dark.png", "PNG");
            } finally {
                Theme.dark = false;
                Glass.rollout = savedRollout;
                Glass.reduceTransparency = savedReduce;
                stage.width = savedWidth;
                stage.height = savedHeight;
            }
        }

        function test_module_actions_route_to_the_owner() {
            var panel = defaultPanel({});
            screenshotSpy.target = panel;
            screenshotSpy.clear();
            panel.requestModuleAction("capture-screen");
            compare(screenshotSpy.count, 1);
            screencastSpy.target = panel;
            screencastSpy.clear();
            panel.requestModuleAction("screen-mirroring");
            compare(screencastSpy.count, 1);
            focusModeSpy.target = panel;
            focusModeSpy.clear();
            panel.focusModeSetRequested("focus");
            compare(focusModeSpy.count, 1);
            compare(focusModeSpy.signalArguments[0][0], "focus");
        }
    }
}