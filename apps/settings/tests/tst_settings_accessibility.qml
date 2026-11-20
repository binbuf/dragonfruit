// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Accessibility pane tests (T291a). Headless with `DF_SETTINGS_FIXTURE`,
// `DF_ACCESSIBILITY_FIXTURE=1`, and `DF_ACCESSIBILITY_DELEGATE_FIXTURE=present`
// (see CMakeLists.txt), so the `Settings` singleton, the AT-SPI view, and the
// external delegates are deterministic and in-process with no bus.
//
// The cases cover the mockup intro and big list, the in-place Vision and
// Physical and Motor sub-panes, both row inventories (rendered and omitted),
// the read-only screen-reader status plus its real Open Orca action, the live
// display/motion/magnifier keys, the speech and on-screen-keyboard delegates,
// the keyboard-navigation and pointer keys, absence structure, search/deep
// links, keyboard operation, and the 740×820 window.
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
            Settings.set("input.keyboardNavigation", false);
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

        // Open the one in-place Vision sub-pane and return its body.
        function openVision(shell) {
            var pane = paneOf(shell);
            pane.openSubPane("vision");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var vision = pane.subPaneLoader.item;
            verify(vision, "the Vision sub-pane body must be loaded");
            return vision;
        }

        // Open the in-place Physical and Motor sub-pane and return its body.
        function openMotor(shell) {
            var pane = paneOf(shell);
            pane.openSubPane("motor");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var motor = pane.subPaneLoader.item;
            verify(motor, "the Physical and Motor sub-pane body must be loaded");
            return motor;
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

        function renderedIds(vision) {
            var rows = [];
            collectRows(vision, rows);
            return rows.map(function(row) { return String(row.settingsId); });
        }

        // -- The mockup intro and top-level big list -------------------------

        function test_the_pane_opens_with_the_mockup_intro_and_big_list() {
            var shell = make();
            compare(shell.currentPaneId, "accessibility");
            compare(shell.currentPane.title, "Accessibility");

            // The intro is the shell's SettingsIntro from the catalog
            // description (doc 25 §3.3); the body starts at the big list.
            verify(shell.hasIntro, "the Accessibility intro must render");
            verify(shell.introText.indexOf("accessibility") >= 0,
                   "the intro names the accessibility features");

            var pane = paneOf(shell);
            compare(pane.subPaneId, "");
            compare(pane.listColumn.visible, true);
            compare(pane.bigList.rows.children.length, 2,
                    "T291a/T291b ship Vision and Physical and Motor; later "
                    + "children add one each");
            compare(pane.visionRow.visible, true);
            compare(pane.visionRow.label, "Vision");
            compare(pane.motorRow.visible, true);
            compare(pane.motorRow.label, "Physical and Motor");

            // The mockup order: Vision, then Physical and Motor.
            compare(pane.bigList.rows.children[0].label, "Vision");
            compare(pane.bigList.rows.children[1].label, "Physical and Motor");

            verify(pane.visionRow.height === 52,
                   "the big row is the 52 pt token, not 53");
            verify(pane.motorRow.height === 52,
                   "the Physical and Motor row uses the same 52 pt token");
            verify(pane.visionRow.category !== null,
                   "the Vision row carries a category tile");
            compare(pane.visionRow.category.source, "scan");
            verify(pane.motorRow.category !== null,
                   "the Physical and Motor row carries a category tile");
            compare(pane.motorRow.category.source, "hand-pointing");

            // No disabled placeholders for later sections.
            for (var i = 0; i < pane.bigList.rows.children.length; ++i) {
                var label = pane.bigList.rows.children[i].label;
                verify(label !== "Hearing");
                verify(label !== "Speech");
                verify(label !== "General");
            }
        }

        function test_the_vision_row_opens_and_closes_the_in_place_sub_pane() {
            var shell = make();
            var pane = paneOf(shell);

            pane.visionRow.disclosureClicked();
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);

            compare(pane.subPaneId, "vision");
            compare(pane.subPaneTitle, "Vision");
            compare(shell.paneTitle, "Vision");
            compare(pane.listColumn.visible, false);
            compare(pane.backButton.text, "‹ Accessibility");
            verify(pane.subPaneLoader.item !== null);

            pane.backButton.clicked();
            tryVerify(function() { return pane.subPaneId === ""; });
            waitForRendering(stage);
            compare(pane.listColumn.visible, true);
            compare(pane.subPaneLoader.status, Loader.Null);
            compare(shell.paneTitle, "Accessibility");
        }

        // -- Vision rows ----------------------------------------------------

        function test_the_vision_groups_render_in_mockup_order() {
            var shell = make();
            var vision = openVision(shell);

            var content = vision.screenReaderGroup.parent;
            var order = [];
            for (var i = 0; i < content.children.length; ++i) {
                var child = content.children[i];
                if (child.title !== undefined && child.title.length > 0)
                    order.push(child.title);
            }
            compare(order.join(","),
                    "Screen Reader,Zoom,Display,Motion,Read & Speak");
            compare(vision.screenReaderGroup.visible, true);
            compare(vision.zoomGroup.visible, true);
            compare(vision.displayGroup.visible, true);
            compare(vision.motionGroup.visible, true);
            compare(vision.readSpeakGroup.visible, true);
        }

        function test_every_rendered_vision_row_has_a_map_owner_and_key() {
            var shell = make();
            var vision = openVision(shell);
            var rendered = [];
            collectRows(vision, rendered);
            verify(rendered.length >= 13,
                   "the Vision sub-pane renders every shipped Vision row");

            for (var i = 0; i < rendered.length; ++i) {
                var row = rendered[i];
                verify(String(row.owner).length > 0,
                       "rendered row " + row.settingsId + " has no owner");
                verify(String(row.settingsKey).length > 0,
                       "rendered row " + row.settingsId + " has no key/API");
                var mapped = SettingsMap.rowFor("accessibility", row.settingsId);
                verify(mapped !== null,
                       "rendered row " + row.settingsId + " has no map row");
                compare(mapped.state, "shipped",
                        "rendered row " + row.settingsId + " must be shipped");
                verify(String(mapped.owner).length > 0 && mapped.owner !== "omit");
                verify(String(mapped.key).length > 0 && mapped.key !== "omit");
                compare(SettingsMap.subPaneForRow("accessibility", mapped.id),
                        "vision");
                compare(SettingsMap.rowSurface("accessibility", mapped.id),
                        "subpane");
                verify(SettingsMap.isRowLandable("accessibility", mapped.id));
            }

            // The rendered rows and the shipped Vision map rows are the same
            // set (Physical and Motor has its own body).
            var expected = SettingsMap.visibleRowsFor("accessibility")
                    .filter(function(entry) {
                        return SettingsMap.subPaneForRow("accessibility", entry.id)
                                === "vision";
                    })
                    .map(function(entry) { return String(entry.id); });
            var actual = rendered.map(function(row) { return String(row.settingsId); });
            compare(actual.sort().join(","), expected.sort().join(","));
        }

        function test_ownerless_vision_rows_are_omitted_with_a_reason() {
            var omitted = [
                "vision.hoverText",
                "vision.display.invertColors",
                "vision.display.titleIcons",
                "vision.display.toolbarShapes",
                "vision.display.displayContrast",
                "vision.display.menuBarSize",
                "vision.display.pointerColors",
                "vision.display.colorFilters",
                "vision.motion.autoPlayImages",
                "vision.motion.nonBlinkingCursor",
                "vision.audioDescriptions"
            ];

            var shell = make();
            var vision = openVision(shell);
            var ids = renderedIds(vision);
            for (var i = 0; i < omitted.length; ++i) {
                var id = omitted[i];
                var row = SettingsMap.rowFor("accessibility", id);
                verify(row !== null, id + " must stay in the map as an omission");
                compare(row.state, "omitted", id + " must not be shipped");
                compare(row.absence, "omitted", id + " must carry the omitted absence");
                verify(row.reason.length > 0, id + " must document its reason");
                compare(ids.indexOf(id), -1, id + " must not be rendered");
                compare(SettingsMap.isRowLandable("accessibility", id), false,
                        id + " must not be landable");
                verify(SettingsMap.visibleRowsFor("accessibility")
                       .map(function(entry) { return entry.id; }).indexOf(id) < 0,
                       id + " must not be in the search index");
            }

            // The mockup labels stay searchable only through the omission
            // reason text, never as a row result.
            var results = SettingsSearch.search("hover text");
            for (var r = 0; r < results.length; ++r) {
                verify(!(results[r].kind === "row"
                         && results[r].paneId === "accessibility"
                         && results[r].targetRow === "vision.hoverText"),
                       "an omitted row must never be a search result");
            }
        }

        // -- Screen reader: read-only status + real Open action --------------

        function test_the_screen_reader_row_is_a_status_plus_open_action() {
            var shell = make();
            var vision = openVision(shell);
            compare(vision.ready, true);
            compare(vision.screenReaderOn, true);
            compare(vision.enabledOn, true);
            compare(vision.screenReaderLabel, "On");
            compare(vision.bridgeLabel, "On");
            compare(findChild(vision, "accessibilityScreenReaderValue").text, "On");
            compare(findChild(vision, "accessibilityBridgeValue").text, "On");

            // The row is a status, not a toggle: the map says delegated and
            // the action launches the external screen reader.
            compare(SettingsMap.rowFor("accessibility", "vision.screenReader").absence,
                    "delegated");
            compare(vision.screenReaderRow.trailingAction, "Open Orca");
            compare(vision.screenReaderRow.trailingButton.enabled, true);
            vision.screenReaderRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "screen-reader");
        }

        // -- Display accommodations -----------------------------------------

        function test_display_keys_round_trip_and_apply_to_the_theme() {
            var shell = make();
            var vision = openVision(shell);

            vision.reduceTransparencyToggle.toggle();
            compare(Settings.values["accessibility.reduceTransparency"], true);
            compare(Glass.reduceTransparency, true);

            vision.increaseContrastToggle.toggle();
            compare(Settings.values["accessibility.increaseContrast"], true);
            compare(Glass.increaseContrast, true);

            vision.differentiateWithoutColorToggle.toggle();
            compare(Settings.values["accessibility.differentiateWithoutColor"], true);
            compare(Theme.differentiateWithoutColor, true);

            vision.textSizeSelect.activateIndex(2);
            compare(Settings.values["accessibility.textSize"], "large");
            compare(Theme.textSize, "large");
            verify(Theme.textScale > 1.0, "large text must raise the type scale");
            verify(Theme.primitive.font.sizeMd > 13,
                   "the primitive type scale must follow the text size");

            vision.textSizeSelect.activateIndex(1);
            compare(Settings.values["accessibility.textSize"], "default");
            compare(Theme.textScale, 1.0);
        }

        function test_pointer_size_is_the_shared_cursor_key() {
            var shell = make();
            var vision = openVision(shell);
            vision.pointerSizeSlider.setValue(32);
            vision.pointerSizeSlider.commit();
            compare(Settings.values["appearance.cursorSize"], 32);
            // The map marks the row stored-only: the compositor applies it at
            // the next session start, not live.
            compare(SettingsMap.rowFor("accessibility", "vision.display.pointerSize").storedOnly,
                    true);
        }

        // -- Motion ----------------------------------------------------------

        function test_motion_keys_round_trip() {
            var shell = make();
            var vision = openVision(shell);
            vision.reduceMotionToggle.toggle();
            compare(Settings.values["accessibility.reduceMotion"], true);
            compare(Theme.reducedMotion, true);

            vision.dimFlashingLightsToggle.toggle();
            compare(Settings.values["accessibility.dimFlashingLights"], true);
            compare(Theme.dimFlashingLights, true);

            compare(Settings.values["accessibility.reduceTransparency"], false);
            compare(Settings.values["accessibility.increaseContrast"], false);
        }

        // -- Magnifier -------------------------------------------------------

        function test_magnifier_keys_round_trip_live() {
            var shell = make();
            var vision = openVision(shell);
            compare(vision.magnifier, false);
            compare(vision.magnifierToggle.checked, false);

            vision.magnifierToggle.toggle();
            compare(Settings.values["accessibility.magnifier"], true);
            compare(vision.magnifier, true);

            vision.magnifierZoomSlider.setValue(3.5);
            vision.magnifierZoomSlider.commit();
            compare(Settings.values["accessibility.magnifierZoom"], 3.5);

            vision.magnifierModeSelect.activateIndex(1);
            compare(Settings.values["accessibility.magnifierMode"], "follow-caret");
            compare(vision.magnifierMode, "follow-caret");
        }

        // -- Read & Speak delegate -------------------------------------------

        function test_read_and_speak_delegate_launches() {
            var shell = make();
            var vision = openVision(shell);
            compare(vision.speechAvailable, true);
            vision.readSpeakRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "speech");
        }

        // -- Physical and Motor ----------------------------------------------

        function test_the_motor_row_opens_and_closes_the_in_place_sub_pane() {
            var shell = make();
            var pane = paneOf(shell);

            pane.motorRow.disclosureClicked();
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);

            compare(pane.subPaneId, "motor");
            compare(pane.subPaneTitle, "Physical and Motor");
            compare(shell.paneTitle, "Physical and Motor");
            compare(pane.listColumn.visible, false);
            verify(pane.subPaneLoader.item !== null);

            pane.backButton.clicked();
            tryVerify(function() { return pane.subPaneId === ""; });
            waitForRendering(stage);
            compare(pane.listColumn.visible, true);
            compare(pane.subPaneLoader.status, Loader.Null);
            compare(shell.paneTitle, "Accessibility");
        }

        function test_the_motor_groups_render_in_mockup_order() {
            var shell = make();
            var motor = openMotor(shell);

            var content = motor.keyboardGroup.parent;
            var order = [];
            for (var i = 0; i < content.children.length; ++i) {
                var child = content.children[i];
                if (child.title !== undefined && child.title.length > 0)
                    order.push(child.title);
            }
            compare(order.join(","), "Keyboard,Pointer Control");
            compare(motor.keyboardGroup.visible, true);
            compare(motor.pointerGroup.visible, true);
        }

        function test_every_rendered_motor_row_has_a_map_owner_and_key() {
            var shell = make();
            var motor = openMotor(shell);
            var rendered = [];
            collectRows(motor, rendered);
            compare(rendered.length, 3,
                    "the Motor sub-pane renders the two keyboard rows and the "
                    + "pointer row");

            for (var i = 0; i < rendered.length; ++i) {
                var row = rendered[i];
                verify(String(row.owner).length > 0,
                       "rendered row " + row.settingsId + " has no owner");
                verify(String(row.settingsKey).length > 0,
                       "rendered row " + row.settingsId + " has no key/API");
                var mapped = SettingsMap.rowFor("accessibility", row.settingsId);
                verify(mapped !== null,
                       "rendered row " + row.settingsId + " has no map row");
                compare(mapped.state, "shipped",
                        "rendered row " + row.settingsId + " must be shipped");
                verify(String(mapped.owner).length > 0 && mapped.owner !== "omit");
                verify(String(mapped.key).length > 0 && mapped.key !== "omit");
                compare(SettingsMap.subPaneForRow("accessibility", mapped.id),
                        "motor");
                compare(SettingsMap.rowSurface("accessibility", mapped.id),
                        "subpane");
                verify(SettingsMap.isRowLandable("accessibility", mapped.id));
            }

            var expected = SettingsMap.visibleRowsFor("accessibility")
                    .filter(function(entry) {
                        return SettingsMap.subPaneForRow("accessibility", entry.id)
                                === "motor";
                    })
                    .map(function(entry) { return String(entry.id); });
            var actual = rendered.map(function(row) { return String(row.settingsId); });
            compare(actual.sort().join(","), expected.sort().join(","));
        }

        function test_ownerless_motor_rows_are_omitted_with_a_reason() {
            var omitted = [
                "motor.voiceControl",
                "motor.keyboard.stickyKeys",
                "motor.keyboard.slowKeys",
                "motor.pointer.mouseKeys",
                "motor.pointer.ignoreTrackpad",
                "motor.switchControl"
            ];

            var shell = make();
            var motor = openMotor(shell);
            var ids = renderedIds(motor);
            for (var i = 0; i < omitted.length; ++i) {
                var id = omitted[i];
                var row = SettingsMap.rowFor("accessibility", id);
                verify(row !== null, id + " must stay in the map as an omission");
                compare(row.state, "omitted", id + " must not be shipped");
                compare(row.absence, "omitted", id + " must carry the omitted absence");
                verify(row.reason.length > 0, id + " must document its reason");
                compare(ids.indexOf(id), -1, id + " must not be rendered");
                compare(SettingsMap.isRowLandable("accessibility", id), false,
                        id + " must not be landable");
                verify(SettingsMap.visibleRowsFor("accessibility")
                       .map(function(entry) { return entry.id; }).indexOf(id) < 0,
                       id + " must not be in the search index");
            }

            // A stale mockup name never becomes a row result.
            var results = SettingsSearch.search("switch control");
            for (var r = 0; r < results.length; ++r) {
                verify(!(results[r].kind === "row"
                         && results[r].paneId === "accessibility"),
                       "an omitted motor row must never be a row result");
            }
        }

        function test_keyboard_navigation_is_the_shared_stored_key() {
            var shell = make();
            var motor = openMotor(shell);
            compare(motor.keyboardNavigation, false);
            compare(motor.keyboardNavigationToggle.checked, false);

            motor.keyboardNavigationToggle.toggle();
            compare(Settings.values["input.keyboardNavigation"], true);
            compare(motor.keyboardNavigation, true);

            // The map marks the durable preference stored-only: no compositor
            // consumer reads it yet.
            var row = SettingsMap.rowFor("accessibility",
                                         "motor.keyboard.fullKeyboardAccess");
            compare(row.storedOnly, true);
            compare(row.key, "input.keyboardNavigation");
            verify(row.absence !== "disabled-with-reason",
                   "a stored-only row is not a disabled row");
        }

        function test_on_screen_keyboard_delegate_matches_availability() {
            var shell = make();
            var motor = openMotor(shell);
            compare(motor.onScreenKeyboardAvailable, true);
            compare(motor.onScreenKeyboardRow.trailingActionEnabled, true);
            compare(motor.onScreenKeyboardRow.trailingAction, "Open");

            motor.onScreenKeyboardRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "on-screen-keyboard");

            // The disabled state is the same row with the reason in the
            // description (proven with the absent fixture in the T291b
            // absence test).
            compare(motor.onScreenKeyboardRow.trailingActionEnabled,
                    motor.onScreenKeyboardAvailable);
        }

        function test_double_click_speed_is_the_live_pointer_key() {
            var shell = make();
            var motor = openMotor(shell);
            compare(motor.doubleClickSpeed, 400);

            motor.doubleClickSpeedSlider.setValue(300);
            motor.doubleClickSpeedSlider.commit();
            compare(Settings.values["input.doubleClickSpeed"], 300);
            compare(motor.doubleClickSpeed, 300);

            compare(SettingsMap.rowFor("accessibility",
                                       "motor.pointer.doubleClickSpeed").absence,
                    "live-on-defaults");
        }

        // -- Search and deep links -------------------------------------------

        function test_a_search_result_lands_in_the_vision_sub_pane() {
            var results = SettingsSearch.search("reduce motion");
            var found = null;
            for (var i = 0; i < results.length; ++i) {
                if (results[i].kind === "row"
                        && results[i].paneId === "accessibility"
                        && results[i].targetRow === "vision.motion.reduceMotion")
                    found = results[i];
            }
            verify(found !== null,
                   "the Reduce motion row must be searchable by its synonym");
            compare(found.deepLink,
                    "settings://pane/accessibility/row/vision.motion.reduceMotion");

            // Start elsewhere so the accessibility body is cold; the shell
            // must queue the sub-pane until the pane body exists.
            var shell = make();
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            verify(shell.deepLink(found.deepLink));
            tryVerify(function() {
                return shell.currentPaneId === "accessibility"
                        && shell.paneBody.status === Loader.Ready;
            });
            var pane = paneOf(shell);
            tryVerify(function() {
                return pane.subPaneId === "vision"
                        && pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            compare(shell.paneTitle, "Vision");
            tryVerify(function() {
                return shell.currentRowId === "vision.motion.reduceMotion";
            });
        }

        function test_a_search_result_lands_in_the_motor_sub_pane() {
            var results = SettingsSearch.search("double click");
            var found = null;
            for (var i = 0; i < results.length; ++i) {
                if (results[i].kind === "row"
                        && results[i].paneId === "accessibility"
                        && results[i].targetRow === "motor.pointer.doubleClickSpeed")
                    found = results[i];
            }
            verify(found !== null,
                   "the Double-click speed row must be searchable");
            compare(found.deepLink,
                    "settings://pane/accessibility/row/motor.pointer.doubleClickSpeed");

            var shell = make();
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            verify(shell.deepLink(found.deepLink));
            tryVerify(function() {
                return shell.currentPaneId === "accessibility"
                        && shell.paneBody.status === Loader.Ready;
            });
            var pane = paneOf(shell);
            tryVerify(function() {
                return pane.subPaneId === "motor"
                        && pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            compare(shell.paneTitle, "Physical and Motor");
            tryVerify(function() {
                return shell.currentRowId === "motor.pointer.doubleClickSpeed";
            });
        }

        // -- Keyboard / AT-SPI ----------------------------------------------

        function test_the_live_controls_are_keyboard_operable() {
            var shell = make();
            var vision = openVision(shell);

            vision.reduceTransparencyToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["accessibility.reduceTransparency"], true);

            vision.increaseContrastToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["accessibility.increaseContrast"], true);

            vision.differentiateWithoutColorToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["accessibility.differentiateWithoutColor"], true);
        }

        function test_the_motor_controls_are_keyboard_operable() {
            var shell = make();
            var motor = openMotor(shell);

            motor.keyboardNavigationToggle.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(Settings.values["input.keyboardNavigation"], true);

            motor.doubleClickSpeedSlider.forceActiveFocus();
            keyClick(Qt.Key_Right);
            verify(Settings.values["input.doubleClickSpeed"] > 400,
                   "the slider must move with the keyboard");
        }

        // -- 740×820 ---------------------------------------------------------

        function test_the_pane_fits_the_measured_window() {
            var shell = make(740, 820);
            var pane = paneOf(shell);
            verify(pane.width > 0 && pane.width <= 740,
                   "the pane must not exceed the 740 px window");
            verify(pane.implicitHeight > 0);
            verify(pane.implicitWidth <= 740);

            var vision = openVision(shell);
            verify(vision.width > 0 && vision.width <= 740);
            verify(vision.implicitHeight > 0);

            pane.closeSubPane();
            waitForRendering(stage);
            var motor = openMotor(shell);
            verify(motor.width > 0 && motor.width <= 740);
            verify(motor.implicitHeight > 0);
        }
    }
}
