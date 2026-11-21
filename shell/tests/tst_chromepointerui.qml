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
    property int highlightedRow: -1

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

    // The shell renders a content-sized popup by translating the root item so
    // the popup origin becomes the view origin. Qt flushes a frame-synchronous
    // hover at the last injected pointer position during polish, so the shell
    // must rebase that pointer with the item. These rows model the menu rows
    // that used to jump to a neighbour when the render target changed.
    Item {
        id: translatedHost
        objectName: "translatedHost"
        x: 200
        y: 20
        width: 100
        height: 5 * 30

        Column {
            id: translatedColumn
            Repeater {
                id: translatedRepeater
                model: 5
                Rectangle {
                    required property int index
                    width: translatedHost.width
                    height: 30
                    color: stage.highlightedRow === index ? "#4c8bf5" : "#c8c8c8"

                    HoverHandler {
                        onHoveredChanged: if (hovered)
                                              stage.highlightedRow = index
                    }
                    TapHandler {}
                }
            }
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

        // A frame-synchronous hover is evaluated against the translated tree.
        // Without rebasing the injected pointer, moving the host two rows makes
        // the row under the old pointer win, not the row the user is over.
        function test_translated_host_rebases_the_injected_pointer() {
            var win = stage.Window.window;
            var row = translatedRepeater.itemAt(2);
            verify(row !== null, "the third row exists");
            var p = row.mapToItem(null, row.width / 2, row.height / 2);
            ChromeInject.reset();
            ChromeInject.move(win, p.x, p.y);
            waitForRendering(stage);
            compare(stage.highlightedRow, 2, "the pointed row starts selected");

            var shift = 2 * row.height;
            translatedHost.y -= shift;
            ChromeInject.move(win, p.x, p.y - shift);
            waitForRendering(stage);
            compare(stage.highlightedRow, 2,
                    "the rebased pointer keeps the same row selected");

            translatedHost.y += shift;
            ChromeInject.move(win, p.x, p.y);
            waitForRendering(stage);
            compare(stage.highlightedRow, 2,
                    "restoring the host keeps the same row selected");
        }

        // A press schedules a render too. Rebasing only while no button is held
        // left the pressed row's highlight on a neighbour during that render.
        function test_translated_host_rebases_while_pressed() {
            var win = stage.Window.window;
            var row = translatedRepeater.itemAt(2);
            verify(row !== null, "the third row exists");
            var p = row.mapToItem(null, row.width / 2, row.height / 2);
            ChromeInject.reset();
            ChromeInject.move(win, p.x, p.y);
            waitForRendering(stage);
            compare(stage.highlightedRow, 2, "the pointed row starts selected");

            ChromeInject.button(win, p.x, p.y, Qt.LeftButton, true);
            var shift = 2 * row.height;
            translatedHost.y -= shift;
            ChromeInject.move(win, p.x, p.y - shift);
            waitForRendering(stage);
            compare(stage.highlightedRow, 2,
                    "the pressed row stays selected while the host is translated");

            translatedHost.y += shift;
            ChromeInject.move(win, p.x, p.y);
            waitForRendering(stage);
            compare(stage.highlightedRow, 2,
                    "the pressed row stays selected after the host is restored");
            ChromeInject.button(win, p.x, p.y, Qt.LeftButton, false);
        }
    }
}