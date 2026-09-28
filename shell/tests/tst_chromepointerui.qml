// SPDX-License-Identifier: MIT
import QtQuick
import QtTest

// T-16.12: the shared chrome pointer injection must deliver a stationary tap
// beside a `DragHandler`. This mirrors the Dock's `DockInject` regression on a
// non-Dock surface: a zero timestamp makes `DragHandler` measure a bogus move
// on the press, take the exclusive grab, and the sibling `TapHandler` never
// taps. `ChromeInject` drives the production `ChromePointer` sequence.
Item {
    id: stage
    width: 320
    height: 240

    Rectangle {
        id: control
        anchors.fill: parent
        color: "#8a8a8a"
        property int taps: 0
        property int drags: 0

        TapHandler {
            id: tap
            onTapped: control.taps++
        }
        DragHandler {
            id: drag
            onActiveChanged: if (active) control.drags++
        }
    }

    TestCase {
        name: "ChromePointer"
        when: windowShown

        function centerOf(item) {
            return item.mapToItem(null, item.width / 2, item.height / 2);
        }

        function test_stationary_injected_tap_taps_beside_a_drag_handler() {
            control.taps = 0;
            control.drags = 0;
            var win = stage.Window.window;
            var p = centerOf(control);
            ChromeInject.reset();
            ChromeInject.move(win, p.x, p.y);
            ChromeInject.button(win, p.x, p.y, Qt.LeftButton, true);
            ChromeInject.button(win, p.x, p.y, Qt.LeftButton, false);
            waitForRendering(stage);
            compare(control.taps, 1);
            compare(control.drags, 0);
        }

        function test_injected_slop_drag_lifts_instead_of_tapping() {
            control.taps = 0;
            control.drags = 0;
            var win = stage.Window.window;
            var p = centerOf(control);
            ChromeInject.reset();
            ChromeInject.move(win, p.x, p.y);
            ChromeInject.button(win, p.x, p.y, Qt.LeftButton, true);
            ChromeInject.move(win, p.x + 32, p.y);
            ChromeInject.button(win, p.x + 32, p.y, Qt.LeftButton, false);
            waitForRendering(stage);
            compare(control.taps, 0);
            verify(control.drags > 0);
        }
    }
}