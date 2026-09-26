// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Screenshot

// ScreenCast source picker view tests (T-13.4a): the monitor/window sections,
// row selection, the accept-button enablement, the Share button, and Escape
// cancellation. Runs headless on the offscreen platform; the bridge's D-Bus
// half is covered by tst_screencast.cpp.
Item {
    id: stage
    width: 560
    height: 460

    TestCase {
        id: testCase
        name: "ScreenCastPicker"
        when: windowShown

        Component { id: pickerComponent; ScreenCastPicker { } }
        SignalSpy { id: toggledSpy; signalName: "sourceToggled" }
        SignalSpy { id: acceptedSpy; signalName: "accepted" }
        SignalSpy { id: cancelledSpy; signalName: "cancelled" }

        // A fresh copy per test: `sources` is a plain JS array and a test that
        // selects a row would otherwise mutate the shared fixture.
        function makeSources(firstSelected, secondSelected) {
            return [
                { id: "monitor:DP-1", kind: "monitor", label: "Built-in Display",
                  detail: "1920 × 1080", selected: firstSelected === true },
                { id: "window:7", kind: "window", label: "Files",
                  detail: "org.dragonfruit.Files", selected: secondSelected === true }
            ];
        }

        function make(props) {
            var picker = createTemporaryObject(pickerComponent, stage, props || {});
            picker.width = stage.width;
            picker.height = stage.height;
            toggledSpy.target = picker;
            acceptedSpy.target = picker;
            cancelledSpy.target = picker;
            toggledSpy.clear();
            acceptedSpy.clear();
            cancelledSpy.clear();
            waitForRendering(stage);
            return picker;
        }

        function clickItem(item) {
            mouseClick(item, item.width / 2, item.height / 2);
        }

        function test_sources_render_and_sections_are_present() {
            var picker = make({ sources: makeSources(), appId: "org.example.App" });
            compare(picker.sources.length, 2);
            verify(!picker.canAccept, "no selection means no accept");

            var list = findChild(picker, "screencastList");
            verify(list !== null, "source list exists");
            compare(list.count, 2);

            var row = list.itemAtIndex(0);
            verify(row !== null, "first row exists");
            verify(row.width > 0, "row has width " + row.width);
            verify(row.height > 0, "row has height " + row.height);
            clickItem(row);
            tryCompare(toggledSpy, "count", 1);
            compare(toggledSpy.signalArguments[0][0], "monitor:DP-1");
        }

        function test_a_selected_source_enables_share() {
            var picker = make({ sources: makeSources(true, false), multiple: true });
            compare(picker.selectedCount, 1);
            verify(picker.canAccept, "a selected source enables accept");

            var accept = findChild(picker, "screencastAccept");
            verify(accept !== null);
            clickItem(accept);
            tryCompare(acceptedSpy, "count", 1);
        }

        function test_multiple_counts_every_checked_source() {
            var picker = make({ sources: makeSources(true, true), multiple: true });
            compare(picker.selectedCount, 2);
        }

        function test_escape_cancels() {
            var picker = make({ sources: makeSources() });
            picker.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            tryCompare(cancelledSpy, "count", 1);
        }
    }
}