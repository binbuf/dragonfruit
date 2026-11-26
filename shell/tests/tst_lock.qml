// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Lock

// Lock-screen tests (T-12.3a): the first-party lock view renders the wallpaper,
// date/time, account portrait, password field, and circular enter control the
// shell controller drives it with, and exposes the locked session to assistive
// technology. Runs headless on the offscreen platform.
Item {
    id: stage
    width: 1280
    height: 720

    TestCase {
        id: testCase
        name: "LockScreen"
        when: windowShown

        Component { id: lockComponent; LockScreen { } }

        function make(props) {
            var lock = createTemporaryObject(lockComponent, stage, props || {});
            lock.width = 1280;
            lock.height = 720;
            waitForRendering(stage);
            return lock;
        }

        function test_renders_the_account_clock_and_password_controls() {
            var lock = make({
                userName: "ada",
                timeText: "9:41",
                dateText: "Tue Nov 17",
                message: ""
            });

            compare(findChild(lock, "lockClock").text, "9:41");
            compare(findChild(lock, "lockDate").text, "Tue Nov 17");
            compare(findChild(lock, "lockUserName").text, "ada");
            verify(findChild(lock, "lockPasswordField") !== null);
            verify(findChild(lock, "lockEnterButton") !== null);
            compare(findChild(lock, "lockEnterGlyph").text, "\u2192");
        }

        // The reference layout puts the date above the time, both horizontally
        // centred at the top of the output.
        function test_clock_is_date_above_time_at_the_top_center() {
            var lock = make({ timeText: "9:41", dateText: "Tue Nov 17" });
            var date = findChild(lock, "lockDate");
            var clock = findChild(lock, "lockClock");

            verify(date.y + date.height <= clock.y + 0.5);
            verify(date.mapToItem(lock, 0, 0).y < lock.height * 0.35);
            verify(Math.abs(date.mapToItem(lock, date.width / 2, 0).x
                            - lock.width / 2) <= 1);
            verify(Math.abs(clock.mapToItem(lock, clock.width / 2, 0).x
                            - lock.width / 2) <= 1);
        }

        // The portrait and password field form the bottom-centre block; the
        // field follows the portrait and the enter control sits inside its
        // right edge.
        function test_portrait_and_password_field_are_at_the_bottom_center() {
            var lock = make({ userName: "ada" });
            var avatar = findChild(lock, "lockAvatar");
            var field = findChild(lock, "lockPasswordField");
            var button = findChild(lock, "lockEnterButton");

            verify(Math.abs(avatar.mapToItem(lock, avatar.width / 2, 0).x
                            - lock.width / 2) <= 1);
            verify(Math.abs(field.mapToItem(lock, field.width / 2, 0).x
                            - lock.width / 2) <= 1);
            verify(avatar.mapToItem(lock, 0, avatar.height).y
                   <= field.mapToItem(lock, 0, 0).y + 0.5);
            verify(field.mapToItem(lock, 0, field.height).y > lock.height * 0.6);

            var buttonRect = button.mapToItem(lock, 0, 0);
            var fieldRect = field.mapToItem(lock, 0, 0);
            verify(buttonRect.x >= fieldRect.x);
            verify(buttonRect.y >= fieldRect.y);
            verify(buttonRect.x + button.width <= fieldRect.x + field.width);
            verify(buttonRect.y + button.height <= fieldRect.y + field.height);
            // Fitted to the inside of the right edge, not floating outside.
            verify(fieldRect.x + field.width - (buttonRect.x + button.width)
                   <= button.width / 2);
        }

        function test_enter_control_submits_the_captured_password() {
            var lock = make({ authEnabled: true, passwordLength: 2 });
            var seen = 0;
            lock.submitPasswordRequested.connect(function() { seen += 1; });

            findChild(lock, "lockEnterButton").clicked();
            compare(seen, 1);

            // The control is inert while PAM is busy or the field is empty.
            lock.authBusy = true;
            findChild(lock, "lockEnterButton").clicked();
            compare(seen, 1);

            lock.authBusy = false;
            lock.passwordLength = 0;
            findChild(lock, "lockEnterButton").clicked();
            compare(seen, 1);
        }

        // The opaque lock surface draws the session's own wallpaper instead of
        // leaving the desktop visible underneath it.
        function test_wallpaper_is_the_lock_background() {
            var lock = make({ wallpaperColor: "#336699", wallpaperFit: "fill" });
            var colorFill = findChild(lock, "lockWallpaperColor");
            var image = findChild(lock, "lockWallpaper");
            verify(colorFill.visible);
            verify(Math.abs(colorFill.color.r - 0.2) < 0.01);
            verify(Math.abs(colorFill.color.g - 0.4) < 0.01);
            verify(Math.abs(colorFill.color.b - 0.6) < 0.01);
            compare(image.fillMode, Image.PreserveAspectCrop);

            lock.wallpaperFit = "fit";
            compare(image.fillMode, Image.PreserveAspectFit);
            lock.wallpaperFit = "stretch";
            compare(image.fillMode, Image.Stretch);
            lock.wallpaperFit = "center";
            compare(image.fillMode, Image.Pad);
        }

        // T227: the Screen Saver pane's clock options drive the lock scene.
        function test_screen_saver_clock_options_hide_the_clock_and_date() {
            var lock = make({ timeText: "09:41", dateText: "Monday 1 January",
                              showClock: true, showDate: true });
            var clock = findChild(lock, "lockClock");
            var date = findChild(lock, "lockDate");
            verify(clock.parent.visible);
            verify(date.visible);

            lock.showDate = false;
            verify(clock.parent.visible);
            verify(!date.visible);

            lock.showClock = false;
            verify(!clock.parent.visible);
        }

        function test_password_placeholder_reflects_auth_state() {
            var lock = make({ authEnabled: false, authBusy: false });
            compare(findChild(lock, "lockPasswordPlaceholder").text,
                    "Press Enter to unlock");

            lock.authEnabled = true;
            compare(findChild(lock, "lockPasswordPlaceholder").text,
                    "Enter Password");

            lock.authBusy = true;
            compare(findChild(lock, "lockPasswordPlaceholder").text,
                    "Authenticating…");
        }

        function test_password_caret_blinks_only_while_editable() {
            var lock = make({ authEnabled: true, authBusy: false, active: true });
            var caret = findChild(lock, "lockCaret");
            verify(caret !== null, "the field draws a text caret");
            verify(caret.visible, "the caret is visible while the field is editable");

            lock.authBusy = true;
            verify(!caret.visible, "the caret is hidden while PAM verifies");

            lock.authBusy = false;
            lock.active = false;
            verify(!caret.visible, "the caret stops with the lock scene");
        }

        function test_password_mask_reflects_captured_keys() {
            var lock = make({ authEnabled: true, passwordLength: 0 });
            var placeholder = findChild(lock, "lockPasswordPlaceholder");
            var mask = findChild(lock, "lockPasswordMask");
            verify(placeholder.visible);
            verify(!mask.visible);

            // Captured keys are drawn as bullets, never echoed as text.
            lock.passwordLength = 3;
            verify(!placeholder.visible);
            verify(mask.visible);
            compare(mask.text, "\u2022\u2022\u2022");

            // Backspacing to empty restores the placeholder.
            lock.passwordLength = 0;
            verify(placeholder.visible);
            verify(!mask.visible);
        }

        function test_message_is_shown_only_when_set() {
            var lock = make({ message: "" });
            verify(!findChild(lock, "lockMessage").visible);

            lock.message = "Authentication failed";
            verify(findChild(lock, "lockMessage").visible);
            compare(findChild(lock, "lockMessage").text, "Authentication failed");
        }

        function test_locked_session_is_announced() {
            var lock = make({ message: "" });
            compare(lock.Accessible.role, Accessible.Pane);
            compare(lock.Accessible.name, "Lock screen");
            compare(lock.Accessible.description, "Session locked.");

            lock.message = "Webcam in use";
            compare(lock.Accessible.description, "Webcam in use");
        }

        // T229a: the Lock Screen pane's `lock.*` keys are pushed onto the
        // scene by the shell; the scene only draws the resulting state.
        function test_user_name_and_photo_toggle_hides_the_identity() {
            var lock = make({ userName: "ada", showUserNameAndPhoto: true });
            verify(findChild(lock, "lockAvatar").visible);
            verify(findChild(lock, "lockUserName").visible);

            lock.showUserNameAndPhoto = false;
            verify(!findChild(lock, "lockAvatar").visible);
            verify(!findChild(lock, "lockUserName").visible);
        }

        function test_password_hint_and_lock_message_are_gated() {
            var lock = make({ showPasswordHints: false, passwordHint: "cat",
                              showLockMessage: false,
                              lockMessageText: "Back at 3." });
            verify(!findChild(lock, "lockPasswordHint").visible);
            verify(!findChild(lock, "lockScreenMessage").visible);

            lock.showPasswordHints = true;
            verify(findChild(lock, "lockPasswordHint").visible);
            compare(findChild(lock, "lockPasswordHint").text, "cat");

            lock.showLockMessage = true;
            verify(findChild(lock, "lockScreenMessage").visible);
            compare(findChild(lock, "lockScreenMessage").text, "Back at 3.");
        }

        function test_power_buttons_are_optional_and_report_intent() {
            var lock = make({ showPowerButtons: true });
            var row = findChild(lock, "lockPowerRow");
            verify(row.visible);
            compare(findChild(lock, "lockSleepButton").text, "Sleep");
            compare(findChild(lock, "lockRestartButton").text, "Restart");
            compare(findChild(lock, "lockShutDownButton").text, "Shut Down");

            var seen = [];
            lock.powerActionRequested.connect(function(action) {
                seen.push(action);
            });
            findChild(lock, "lockSleepButton").clicked();
            findChild(lock, "lockShutDownButton").clicked();
            compare(seen.join(","), "sleep,shut-down");

            lock.showPowerButtons = false;
            verify(!row.visible);
        }
    }
}