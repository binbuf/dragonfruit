// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Accessibility pane tests (T275). Headless with `DF_SETTINGS_FIXTURE`,
// `DF_ACCESSIBILITY_FIXTURE=1`, and `DF_ACCESSIBILITY_DELEGATE_FIXTURE=present`
// (see CMakeLists.txt), so the `Settings` singleton, the AT-SPI view, and the
// three external delegates are deterministic and in-process with no bus.
//
// The cases cover the five-section inventory, the map/row lockstep, the
// read-only screen-reader status plus its real Open Orca action, the live
// display/motion/magnifier/shortcut keys, the hearing/motor/speech delegates,
// absence structure, keyboard operation, and the 740×820 window.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsAccessibility"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.resetAccessibilityFixture();
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("accessibility.differentiateWithoutColor", false);
            Settings.set("accessibility.textSize", "default");
            Settings.set("accessibility.dimFlashingLights", false);
            Settings.set("accessibility.magnifier", false);
            Settings.set("accessibility.magnifierZoom", 2.0);
            Settings.set("accessibility.magnifierMode", "follow-focus");
            Settings.set("accessibility.shortcut", "");
            Settings.set("sound.balance", 0.5);
            Settings.set("input.doubleClickSpeed", 400);
            Settings.set("appearance.cursorSize", 24);
        }

        function make(width, height) {
            var shell = createTemporaryObject(shellComponent, stage, {
                width: width !== undefined ? width : stage.width,
                height: height !== undefined ? height : stage.height
            });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Accessibility pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function collectRows(item, out) {
            if (!item)
                return;
            if (item.settingsId !== undefined && String(item.settingsId).length > 0)
                out.push(item);
            var children = item.children;
            for (var i = 0; i < children.length; ++i)
                collectRows(children[i], out);
        }

        // -- The five-section information architecture -----------------------

        function test_the_five_sections_render_in_macos_order() {
            var shell = make();
            compare(shell.currentPaneId, "accessibility");
            compare(shell.currentPane.title, "Accessibility");
            var pane = paneOf(shell);

            compare(pane.visionGroup.visible, true);
            compare(pane.visionGroup.title, "Vision");
            compare(pane.displayGroup.visible, true);
            compare(pane.magnifierGroup.visible, true);
            compare(pane.motionGroup.visible, true);
            compare(pane.readSpeakGroup.visible, true);
            compare(pane.hearingGroup.visible, true);
            compare(pane.hearingGroup.title, "Hearing");
            compare(pane.motorGroup.visible, true);
            compare(pane.motorGroup.title, "Motor");
            compare(pane.speechGroup.visible, true);
            compare(pane.speechGroup.title, "Speech");
            compare(pane.shortcutGroup.visible, true);
            compare(pane.shortcutGroup.title, "Accessibility Shortcut");

            // The section order is the reference order.
            var content = pane.visionGroup.parent;
            var order = [];
            for (var i = 0; i < content.children.length; ++i) {
                var child = content.children[i];
                if (child.title !== undefined && child.title.length > 0)
                    order.push(child.title);
            }
            compare(order[0], "Vision");
            verify(order.indexOf("Hearing") > order.indexOf("Vision"));
            verify(order.indexOf("Motor") > order.indexOf("Hearing"));
            verify(order.indexOf("Speech") > order.indexOf("Motor"));
            verify(order.indexOf("Accessibility Shortcut") > order.indexOf("Speech"));
        }

        function test_every_rendered_row_has_a_map_owner_and_key() {
            var shell = make();
            var pane = paneOf(shell);
            var rendered = [];
            collectRows(pane, rendered);
            verify(rendered.length >= 40,
                   "the pane must render the full five-section inventory");

            for (var i = 0; i < rendered.length; ++i) {
                var row = rendered[i];
                verify(String(row.owner).length > 0,
                       "rendered row " + row.settingsId + " has no owner");
                verify(String(row.settingsKey).length > 0,
                       "rendered row " + row.settingsId + " has no key/API");
                var mapped = SettingsMap.rowFor("accessibility", row.settingsId);
                verify(mapped !== null,
                       "rendered row " + row.settingsId + " has no map row");
                verify(mapped.state === "shipped" || mapped.state === "omitted",
                       "map row " + row.settingsId + " is not rendered");
                if (mapped.state === "shipped") {
                    verify(String(mapped.owner).length > 0 && mapped.owner !== "omit");
                    verify(String(mapped.key).length > 0 && mapped.key !== "omit");
                }
            }

            // Every shipped/omitted map row has a rendered target.
            var mapRows = SettingsMap.rowsFor("accessibility").filter(function(entry) {
                return entry.state === "shipped" || entry.state === "omitted";
            });
            compare(rendered.length, mapRows.length,
                    "the rendered rows and the map rows must be the same set");
        }

        // -- Screen reader: read-only status + real Open action --------------

        function test_the_screen_reader_row_is_a_status_plus_open_action() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.screenReaderOn, true);
            compare(pane.enabledOn, true);
            compare(pane.screenReaderLabel, "On");
            compare(pane.bridgeLabel, "On");
            compare(findChild(pane, "accessibilityScreenReaderValue").text, "On");
            compare(findChild(pane, "accessibilityBridgeValue").text, "On");

            // The row is a status, not a toggle: the map says delegated and
            // the action launches the external screen reader.
            compare(SettingsMap.rowFor("accessibility", "vision.screenReader").absence,
                    "delegated");
            compare(pane.screenReaderRow.trailingAction, "Open Orca");
            compare(pane.screenReaderRow.trailingButton.enabled, true);
            pane.screenReaderRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "screen-reader");
        }

        // -- Display accommodations -----------------------------------------

        function test_display_keys_round_trip_and_apply_to_the_theme() {
            var shell = make();
            var pane = paneOf(shell);

            pane.reduceTransparencyToggle.toggle();
            compare(Settings.values["accessibility.reduceTransparency"], true);
            compare(Glass.reduceTransparency, true);

            pane.increaseContrastToggle.toggle();
            compare(Settings.values["accessibility.increaseContrast"], true);
            compare(Glass.increaseContrast, true);

            pane.differentiateWithoutColorToggle.toggle();
            compare(Settings.values["accessibility.differentiateWithoutColor"], true);
            compare(Theme.differentiateWithoutColor, true);

            pane.textSizeSelect.activateIndex(2);
            compare(Settings.values["accessibility.textSize"], "large");
            compare(Theme.textSize, "large");
            verify(Theme.textScale > 1.0, "large text must raise the type scale");
            verify(Theme.primitive.font.sizeMd > 13,
                   "the primitive type scale must follow the text size");

            pane.textSizeSelect.activateIndex(1);
            compare(Settings.values["accessibility.textSize"], "default");
            compare(Theme.textScale, 1.0);
        }

        function test_pointer_size_is_the_shared_cursor_key() {
            var shell = make();
            var pane = paneOf(shell);
            pane.pointerSizeSlider.setValue(32);
            pane.pointerSizeSlider.commit();
            compare(Settings.values["appearance.cursorSize"], 32);
            // The map marks the row stored-only: the compositor applies it at
            // the next session start, not live.
            compare(SettingsMap.rowFor("accessibility", "vision.display.pointerSize").storedOnly,
                    true);
        }

        // -- Motion ----------------------------------------------------------

        function test_motion_keys_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            pane.reduceMotionToggle.toggle();
            compare(Settings.values["accessibility.reduceMotion"], true);
            compare(Theme.reducedMotion, true);

            pane.dimFlashingLightsToggle.toggle();
            compare(Settings.values["accessibility.dimFlashingLights"], true);
            compare(Theme.dimFlashingLights, true);

            compare(Settings.values["accessibility.reduceTransparency"], false);
            compare(Settings.values["accessibility.increaseContrast"], false);
        }

        // -- Magnifier -------------------------------------------------------

        function test_magnifier_keys_round_trip_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.magnifier, false);
            compare(pane.magnifierToggle.checked, false);

            pane.magnifierToggle.toggle();
            compare(Settings.values["accessibility.magnifier"], true);
            compare(pane.magnifier, true);

            pane.magnifierZoomSlider.setValue(3.5);
            pane.magnifierZoomSlider.commit();
            compare(Settings.values["accessibility.magnifierZoom"], 3.5);

            pane.magnifierModeSelect.activateIndex(1);
            compare(Settings.values["accessibility.magnifierMode"], "follow-caret");
            compare(pane.magnifierMode, "follow-caret");
        }

        // -- Hearing / Motor / Speech delegates ------------------------------

        function test_hearing_balance_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            pane.balanceSlider.setValue(0.75);
            pane.balanceSlider.commit();
            compare(Settings.values["sound.balance"], 0.75);
            compare(SettingsMap.rowFor("accessibility", "hearing.balance").storedOnly,
                    true);
        }

        function test_speech_and_on_screen_keyboard_delegates_launch() {
            var shell = make();
            var pane = paneOf(shell);

            compare(pane.speechAvailable, true);
            pane.readSpeakRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "speech");

            pane.liveSpeechRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "speech");

            compare(pane.onScreenKeyboardAvailable, true);
            pane.onScreenKeyboardRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "on-screen-keyboard");
        }

        function test_motor_double_click_speed_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            pane.doubleClickSpeedSlider.setValue(650);
            pane.doubleClickSpeedSlider.commit();
            compare(Settings.values["input.doubleClickSpeed"], 650);
        }

        // -- Accessibility shortcut recorder + engine consumer ---------------

        function test_the_shortcut_recorder_writes_the_canonical_chord() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.accessibilityShortcut, "");

            pane.shortcutRecorder.startRecording();
            compare(pane.shortcutRecorder.recording, true);
            keyClick(Qt.Key_A, Qt.ControlModifier | Qt.ShiftModifier);
            compare(Settings.values["accessibility.shortcut"], "Ctrl+Shift+A");
            compare(pane.shortcutRecorder.chord, "Ctrl+Shift+A");
            compare(pane.shortcutRecorder.recording, false);

            // Backspace with no modifiers clears the recorded chord.
            pane.shortcutRecorder.startRecording();
            keyClick(Qt.Key_Backspace);
            compare(Settings.values["accessibility.shortcut"], "");
        }

        // -- Keyboard / AT-SPI ----------------------------------------------

        function test_the_live_controls_are_keyboard_operable() {
            var shell = make();
            var pane = paneOf(shell);

            pane.reduceTransparencyToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["accessibility.reduceTransparency"], true);

            pane.increaseContrastToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["accessibility.increaseContrast"], true);

            pane.differentiateWithoutColorToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["accessibility.differentiateWithoutColor"], true);
        }

        // -- 740×820 ---------------------------------------------------------

        function test_the_pane_fits_the_measured_window() {
            var shell = make(740, 820);
            var pane = paneOf(shell);
            verify(pane.width > 0 && pane.width <= 740,
                   "the pane must not exceed the 740 px window");
            verify(pane.implicitHeight > 0);
            // The detail scroll owns the overflow; the pane itself is one
            // column, so there is no horizontal overflow.
            verify(pane.implicitWidth <= 740);
        }

        // -- The live glass preview (retained from T-15.14b) -----------------

        function test_reduce_transparency_and_contrast_drive_the_preview() {
            var shell = make();
            shell.glassRollout = "legacy";
            var pane = paneOf(shell);
            var plainEdge = pane.previewResolved.edge;

            pane.reduceTransparencyToggle.toggle();
            pane.increaseContrastToggle.toggle();
            compare(Glass.reduceTransparency, true);
            compare(Glass.increaseContrast, true);
            compare(pane.previewResolved.renderMode, "solid");
            verify(pane.previewResolved.edge > plainEdge);
            compare(pane.previewResolved.sliderApplied, false);
        }
    }
}
