// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.ControlCenter

// Control Center panel tests (T-11.3a): the normalized tile model, live tile
// gestures (volume/brightness), the read-only Wi-Fi degradation, and
// Escape dismissal. Runs headless on the offscreen platform.
Item {
    id: stage
    width: 480
    height: 520

    TestCase {
        id: testCase
        name: "ControlCenter"
        when: windowShown

        Component { id: panelComponent; ControlCenter { } }
        SignalSpy { id: volumeSpy; signalName: "volumeSetRequested" }
        SignalSpy { id: brightnessSpy; signalName: "brightnessSetRequested" }
        SignalSpy { id: muteSpy; signalName: "muteToggleRequested" }
        SignalSpy { id: closedSpy; signalName: "closed" }

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

        function make(props) {
            var panel = createTemporaryObject(panelComponent, stage, props || {});
            waitForRendering(stage);
            return panel;
        }

        function test_tiles_expose_wifi_volume_and_brightness() {
            var panel = make({
                wifi: wifiModel("available", true, "home"),
                audio: audioModel("available", 0.6, false),
                brightness: 0.8
            });
            compare(panel.tiles.length, 3);
            compare(panel.tiles[0].id, "wifi");
            compare(panel.tiles[0].kind, "toggle");
            compare(panel.tiles[0].checked, true);
            compare(panel.tiles[1].id, "volume");
            compare(panel.tiles[1].kind, "slider");
            compare(Math.abs(panel.tiles[1].value - 0.6) < 0.0001, true);
            compare(panel.tiles[2].id, "brightness");
            compare(Math.abs(panel.tiles[2].value - 0.8) < 0.0001, true);
            compare(panel.wifiLabel, "home");
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
    }
}