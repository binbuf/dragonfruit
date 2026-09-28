// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Sound-pane tests (T-15.3b). Headless with `DF_SETTINGS_FIXTURE` and
// `DF_SOUND_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton serves
// a deterministic in-process audio adapter with no bus. The cases cover the
// Sound Effects rows round-tripping through settingsd's defaults, the
// Output/Input device selection round-tripping through the adapter, and the
// Output volume/Mute/Balance controls.
Item {
    id: stage
    width: 900
    height: 900

    TestCase {
        id: testCase
        name: "SettingsSound"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; reset routing, volume, and mute
        // before every case so the pane always opens on a known state.
        function init() {
            Settings.setSoundVolume(0.6);
            Settings.setSoundMute(false);
            Settings.setSoundDefaultSink(7);
            Settings.setSoundDefaultSource(20);
            Settings.set("sound.alertSound", "Chime");
            Settings.set("sound.alertVolume", 0.8);
            Settings.set("sound.playOnStartup", true);
            Settings.set("sound.uiEffects", true);
            Settings.set("sound.volumeFeedback", false);
            Settings.set("sound.balance", 0.5);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("sound");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Sound body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_on_output_with_devices_and_effects() {
            var shell = make();
            compare(shell.currentPaneId, "sound");
            var pane = paneOf(shell);
            verify(pane.ready, "the audio view must be present");
            verify(pane.effectsGroup.visible, "the Sound Effects group is shown");
            verify(pane.outputInputGroup.visible, "the Output & Input group is shown");
            compare(pane.tabIndex, 0, "the pane opens on Output");
            compare(pane.outputs.length, 2);
            compare(pane.inputs.length, 2);
            compare(pane.deviceRepeater.count, 2);
            compare(pane.devices[0].description, "Built-in Speakers");
            compare(pane.devices[0].default, true);
        }

        function test_device_rows_are_labelled_with_a_type() {
            var shell = make();
            var pane = paneOf(shell);
            var type = findChild(pane, "soundDeviceType");
            verify(type !== null, "the Type column is present");
            compare(type.text, "Built-in");
        }

        function test_selecting_an_output_row_routes_the_default_sink() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.sound.defaultSink, "speakers");
            pane.selectDevice(9);
            compare(Settings.sound.defaultSink, "headphones");
            compare(Settings.sound.sinks[1].default, true);
            compare(Settings.sound.sinks[0].default, false);
        }

        function test_selecting_an_input_row_routes_the_default_source() {
            var shell = make();
            var pane = paneOf(shell);
            pane.tabIndex = 1;
            waitForRendering(stage);
            compare(pane.devices.length, 2);
            compare(Settings.sound.defaultSource, "microphone");
            pane.selectDevice(21);
            compare(Settings.sound.defaultSource, "usb-mic");
            compare(Settings.sound.sources[1].default, true);
        }

        function test_output_volume_and_mute_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.sound.volume, 0.6);
            compare(Settings.sound.muted, false);

            pane.outputVolumeSlider.setValue(0.25);
            pane.outputVolumeSlider.commit();
            verify(Math.abs(Settings.sound.volume - 0.25) < 0.001);
            compare(pane.volume, 0.25);

            Settings.setSoundMute(true);
            compare(Settings.sound.muted, true);
            compare(pane.muted, true);
            pane.muteToggle.toggle();
            compare(Settings.sound.muted, false);
        }

        function test_alert_sound_and_effects_rows_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.alertSound, "Chime");

            pane.alertSoundSelect.activateIndex(2); // Pulse
            compare(Settings.values["sound.alertSound"], "Pulse");

            pane.alertVolumeSlider.setValue(0.3);
            pane.alertVolumeSlider.commit();
            verify(Math.abs(Settings.values["sound.alertVolume"] - 0.3) < 0.001);

            pane.playOnStartupToggle.toggle();
            compare(Settings.values["sound.playOnStartup"], false);
            pane.uiEffectsToggle.toggle();
            compare(Settings.values["sound.uiEffects"], false);
            pane.volumeFeedbackToggle.toggle();
            compare(Settings.values["sound.volumeFeedback"], true);

            pane.balanceSlider.setValue(0.25);
            pane.balanceSlider.commit();
            verify(Math.abs(Settings.values["sound.balance"] - 0.25) < 0.001);
        }

        function test_external_change_converges_into_the_pane() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.setSoundDefaultSink(9);
            compare(pane.devices[0].default, false);
            compare(pane.devices[1].default, true);
            Settings.set("sound.alertSound", "Breeze");
            compare(pane.alertSound, "Breeze");
            compare(pane.alertSoundSelect.currentIndex, 4);
        }
    }
}