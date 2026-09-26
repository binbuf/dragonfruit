// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Screenshot

// Screenshot selection overlay view tests (T-13.3a): the three selection modes
// (fullscreen click, region drag with a live rectangle, window click), the
// minimum-region guard, and Escape cancellation. Runs headless on the offscreen
// platform; the bridge's D-Bus half is covered by tst_screenshot.cpp.
Item {
    id: stage
    width: 400
    height: 300

    TestCase {
        id: testCase
        name: "SelectionOverlay"
        when: windowShown

        Component { id: overlayComponent; SelectionOverlay { } }
        SignalSpy { id: acceptedSpy; signalName: "accepted" }
        SignalSpy { id: cancelledSpy; signalName: "cancelled" }

        function make(props) {
            var overlay = createTemporaryObject(overlayComponent, stage, props || {});
            overlay.width = stage.width;
            overlay.height = stage.height;
            acceptedSpy.target = overlay;
            cancelledSpy.target = overlay;
            acceptedSpy.clear();
            cancelledSpy.clear();
            waitForRendering(stage);
            return overlay;
        }

        function test_fullscreen_click_accepts_the_whole_output() {
            var overlay = make({ mode: "fullscreen" });
            compare(overlay.fullscreenMode, true);
            mouseClick(overlay, 200, 150);
            tryCompare(acceptedSpy, "count", 1);
            var args = acceptedSpy.signalArguments[0];
            compare(args[0], 0);
            compare(args[1], 0);
            compare(args[2], 400);
            compare(args[3], 300);
        }

        function test_region_drag_accepts_the_selected_rect() {
            var overlay = make({ mode: "region" });
            mousePress(overlay, 50, 60);
            mouseMove(overlay, 250, 200);
            waitForRendering(stage);
            compare(overlay.hasSelection, true);
            compare(overlay.selectionX, 50);
            compare(overlay.selectionY, 60);
            compare(overlay.selectionWidth, 200);
            compare(overlay.selectionHeight, 140);
            mouseRelease(overlay, 250, 200);
            tryCompare(acceptedSpy, "count", 1);
            var args = acceptedSpy.signalArguments[0];
            compare(args[0], 50);
            compare(args[1], 60);
            compare(args[2], 200);
            compare(args[3], 140);
        }

        function test_a_stray_click_is_not_a_region() {
            var overlay = make({ mode: "region" });
            mousePress(overlay, 40, 40);
            mouseRelease(overlay, 41, 41);
            wait(50);
            compare(acceptedSpy.count, 0);
            verify(!overlay.hasSelection);
        }

        function test_window_click_accepts_at_the_pointer() {
            var overlay = make({ mode: "window" });
            mouseClick(overlay, 120, 90);
            tryCompare(acceptedSpy, "count", 1);
            var args = acceptedSpy.signalArguments[0];
            compare(args[0], 120);
            compare(args[1], 90);
        }

        function test_escape_cancels() {
            var overlay = make({ mode: "region" });
            overlay.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            tryCompare(cancelledSpy, "count", 1);
        }

        function test_the_mode_label_follows_the_mode() {
            var overlay = make({ mode: "region" });
            var label = findChild(overlay, "modeLabel");
            verify(label !== null, "mode label exists");
            verify(label.text.indexOf("Region") >= 0);
            overlay.mode = "fullscreen";
            verify(label.text.indexOf("Fullscreen") >= 0);
        }
    }
}