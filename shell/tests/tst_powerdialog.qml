// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Power

// Session power confirmation dialog view tests: the Shut Down and Restart
// copy, the icon, the 60-second countdown, the automatic expiry, and the
// confirm/cancel paths. Runs headless on the offscreen platform.
Item {
    id: stage
    width: 480
    height: 280

    TestCase {
        id: testCase
        name: "PowerDialog"
        when: windowShown

        Component { id: dialogComponent; PowerDialog { } }
        SignalSpy { id: confirmedSpy; signalName: "confirmed" }
        SignalSpy { id: cancelledSpy; signalName: "cancelled" }
        SignalSpy { id: expiredSpy; signalName: "expired" }

        function make(props) {
            var dialog = createTemporaryObject(dialogComponent, stage, props || {});
            dialog.width = stage.width;
            dialog.height = stage.height;
            confirmedSpy.target = dialog;
            cancelledSpy.target = dialog;
            expiredSpy.target = dialog;
            confirmedSpy.clear();
            cancelledSpy.clear();
            expiredSpy.clear();
            waitForRendering(stage);
            dialog.focusDefault();
            return dialog;
        }

        function clickItem(item) {
            mouseClick(item, item.width / 2, item.height / 2);
        }

        function test_shut_down_copy() {
            var dialog = make({ action: "shut-down", secondsRemaining: 60,
                                active: true });
            compare(findChild(dialog, "powerHeading").text, "Shut Down");
            compare(findChild(dialog, "powerQuestion").text,
                    "Are you sure you want to shut down your computer now?");
            compare(findChild(dialog, "powerCountdown").text,
                    "If you do nothing, the computer will shut down automatically in 60 seconds.");
            compare(findChild(dialog, "powerGlyph").name, "power");
            compare(findChild(dialog, "powerCancel").text, "Cancel");
            compare(findChild(dialog, "powerConfirm").text, "Shut Down");
        }

        function test_restart_copy() {
            var dialog = make({ action: "restart", secondsRemaining: 60,
                                active: true });
            compare(findChild(dialog, "powerHeading").text, "Restart");
            compare(findChild(dialog, "powerQuestion").text,
                    "Are you sure you want to restart your computer now?");
            compare(findChild(dialog, "powerCountdown").text,
                    "If you do nothing, the computer will restart automatically in 60 seconds.");
            compare(findChild(dialog, "powerGlyph").name, "arrows-clockwise");
            compare(findChild(dialog, "powerConfirm").text, "Restart");
        }

        function test_log_out_copy() {
            var dialog = make({ action: "log-out", secondsRemaining: 60,
                                active: true });
            compare(findChild(dialog, "powerHeading").text, "Log Out");
            compare(findChild(dialog, "powerQuestion").text,
                    "Are you sure you want to log out of your account now?");
            compare(findChild(dialog, "powerCountdown").text,
                    "If you do nothing, you will be logged out automatically in 60 seconds.");
            compare(findChild(dialog, "powerGlyph").name, "person");
            compare(findChild(dialog, "powerConfirm").text, "Log Out");
        }

        function test_countdown_ticks_to_zero_and_expires() {
            var dialog = make({ action: "restart", secondsRemaining: 2,
                                active: true });
            var countdown = findChild(dialog, "powerCountdown");
            compare(countdown.text,
                    "If you do nothing, the computer will restart automatically in 2 seconds.");
            dialog.advanceCountdown();
            compare(countdown.text,
                    "If you do nothing, the computer will restart automatically in 1 second.");
            dialog.advanceCountdown();
            compare(countdown.text,
                    "If you do nothing, the computer will restart automatically in 0 seconds.");
            tryCompare(expiredSpy, "count", 1);
        }

        function test_the_timer_stops_when_inactive() {
            var dialog = make({ action: "shut-down", active: true });
            var timer = findChild(dialog, "powerCountdownTimer");
            verify(timer.running, "the live dialog counts down");
            dialog.active = false;
            tryCompare(timer, "running", false);
        }

        function test_confirm_click() {
            var dialog = make({ action: "shut-down", active: true });
            clickItem(findChild(dialog, "powerConfirm"));
            tryCompare(confirmedSpy, "count", 1);
        }

        function test_cancel_click() {
            var dialog = make({ action: "restart", active: true });
            clickItem(findChild(dialog, "powerCancel"));
            tryCompare(cancelledSpy, "count", 1);
        }

        function test_escape_cancels() {
            var dialog = make({ action: "shut-down", active: true });
            dialog.focusDefault();
            keyClick(Qt.Key_Escape);
            tryCompare(cancelledSpy, "count", 1);
        }

        function test_return_confirms() {
            var dialog = make({ action: "restart", active: true });
            dialog.focusDefault();
            keyClick(Qt.Key_Return);
            tryCompare(confirmedSpy, "count", 1);
        }
    }
}
