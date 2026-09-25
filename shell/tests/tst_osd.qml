// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Osd

// OSD overlay tests (T-11.4a): the volume/brightness card renders the right
// glyph, level track, muted state, and accessible name from the properties the
// shell controller drives. The fade is the model's presentation curve, so the
// view only mirrors it to `opacity`. Runs headless on the offscreen platform.
Item {
    id: stage
    width: 240
    height: 240

    TestCase {
        id: testCase
        name: "Osd"
        when: windowShown

        Component { id: osdComponent; Osd { } }

        function make(props) {
            var osd = createTemporaryObject(osdComponent, stage, props || {});
            osd.width = 220;
            osd.height = 220;
            waitForRendering(stage);
            return osd;
        }

        function test_volume_card_shows_the_level_and_label() {
            var osd = make({ kind: "volume", value: 0.6, muted: false, fade: 1.0 });
            compare(osd.iconName, "volume");
            compare(osd.isBrightness, false);
            compare(osd.accessibleName, "Volume 60%");

            var track = findChild(osd, "osdTrack");
            verify(track !== null);
            var fill = findChild(osd, "osdFill");
            verify(fill !== null);
            verify(Math.abs(fill.width - track.width * 0.6) < 0.5);

            var label = findChild(osd, "osdLabel");
            verify(label !== null);
            compare(label.text, "60%");
        }

        function test_brightness_uses_the_brightness_glyph() {
            var osd = make({ kind: "brightness", value: 0.75 });
            compare(osd.iconName, "brightness");
            compare(osd.isBrightness, true);
            compare(osd.accessibleName, "Brightness 75%");

            var label = findChild(osd, "osdLabel");
            compare(label.text, "75%");
        }

        function test_muted_empties_the_track_and_labels_muted() {
            var osd = make({ kind: "volume", value: 0.6, muted: true });
            var fill = findChild(osd, "osdFill");
            verify(fill !== null);
            compare(fill.width, 0);
            var label = findChild(osd, "osdLabel");
            compare(label.text, "Muted");
            compare(osd.accessibleName, "Volume muted");
        }

        function test_fade_drives_the_opacity() {
            var osd = make({ kind: "volume", value: 0.5, fade: 0.0 });
            compare(osd.opacity, 0.0);
            osd.fade = 1.0;
            compare(osd.opacity, 1.0);
            osd.fade = 0.4;
            verify(Math.abs(osd.opacity - 0.4) < 0.001);
        }

        function test_card_is_an_accessible_alert() {
            var osd = make({ kind: "volume", value: 0.5 });
            compare(osd.Accessible.role, Accessible.Alert);
            compare(osd.Accessible.name, "Volume 50%");
        }
    }
}