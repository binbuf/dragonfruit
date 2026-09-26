// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Lock

// Lock-screen tests (T-12.3a): the first-party lock view renders the account,
// clock, lock glyph, and password-field placeholder the shell controller drives
// it with, and exposes the locked session to assistive technology. Runs
// headless on the offscreen platform.
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

        function test_renders_the_account_clock_and_lock_glyph() {
            var lock = make({
                userName: "ada",
                timeText: "09:41",
                dateText: "Monday 1 January",
                message: ""
            });

            compare(findChild(lock, "lockClock").text, "09:41");
            compare(findChild(lock, "lockDate").text, "Monday 1 January");
            compare(findChild(lock, "lockUserName").text, "ada");
            verify(findChild(lock, "lockCard") !== null);
            verify(findChild(lock, "lockGlyph") !== null);
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
    }
}