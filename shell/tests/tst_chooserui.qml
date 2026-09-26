// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Screenshot

// FileChooser picker view tests (T-13.2b): the entry rows, selection, folder
// activation, the SaveFile name field, the accept-button enablement, and
// Escape cancellation. Runs headless on the offscreen platform; the bridge's
// D-Bus half is covered by tst_chooser.cpp.
Item {
    id: stage
    width: 640
    height: 440

    TestCase {
        id: testCase
        name: "FileChooser"
        when: windowShown

        Component { id: chooserComponent; FileChooser { } }
        SignalSpy { id: activatedSpy; signalName: "entryActivated" }
        SignalSpy { id: selectedSpy; signalName: "selectionChanged" }
        SignalSpy { id: acceptedSpy; signalName: "accepted" }
        SignalSpy { id: cancelledSpy; signalName: "cancelled" }
        SignalSpy { id: upSpy; signalName: "upRequested" }

        readonly property var sampleEntries: [
            { name: "alpha", uri: "file:///tmp/alpha", directory: true, size: -1, detail: "" },
            { name: "zeta.txt", uri: "file:///tmp/zeta.txt", directory: false, size: 12,
              detail: "12 B" }
        ]

        function make(props) {
            var chooser = createTemporaryObject(chooserComponent, stage, props || {});
            chooser.width = stage.width;
            chooser.height = stage.height;
            activatedSpy.target = chooser;
            selectedSpy.target = chooser;
            acceptedSpy.target = chooser;
            cancelledSpy.target = chooser;
            upSpy.target = chooser;
            activatedSpy.clear();
            selectedSpy.clear();
            acceptedSpy.clear();
            cancelledSpy.clear();
            upSpy.clear();
            waitForRendering(stage);
            return chooser;
        }

        function clickItem(item) {
            mouseClick(item, item.width / 2, item.height / 2);
        }

        function test_rows_render_and_selection_enables_accept() {
            var chooser = make({
                title: "Open File",
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                entries: sampleEntries
            });
            compare(chooser.entries.length, 2);
            verify(!chooser.canAccept, "no selection means no accept");

            var list = findChild(chooser, "chooserList");
            verify(list !== null, "entry list exists");
            compare(list.count, 2);

            var row = list.itemAtIndex(0);
            verify(row !== null, "first row exists");
            verify(row.width > 0, "row has width " + row.width);
            verify(row.height > 0, "row has height " + row.height);
            clickItem(row);
            tryCompare(selectedSpy, "count", 1);
            compare(selectedSpy.signalArguments[0][0], 0);

            // The shell pushes the bridge's selection back into the view; a
            // folder row still cannot be accepted in open mode.
            chooser.selectedIndex = 0;
            verify(!chooser.canAccept);

            var fileRow = list.itemAtIndex(1);
            clickItem(fileRow);
            tryCompare(selectedSpy, "count", 2);
            compare(selectedSpy.signalArguments[1][0], 1);
            chooser.selectedIndex = 1;
            verify(chooser.canAccept, "a selected file enables accept");

            var accept = findChild(chooser, "chooserAccept");
            verify(accept !== null);
            clickItem(accept);
            tryCompare(acceptedSpy, "count", 1);
        }

        function test_keyboard_activation_emits_the_row() {
            var chooser = make({
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                entries: sampleEntries
            });
            var list = findChild(chooser, "chooserList");
            list.forceActiveFocus();
            chooser.selectedIndex = 1;
            keyClick(Qt.Key_Return);
            tryCompare(activatedSpy, "count", 1);
            compare(activatedSpy.signalArguments[0][0], 1);
        }

        function test_save_mode_needs_a_name() {
            var chooser = make({
                title: "Save File",
                kind: "save",
                acceptLabel: "Save",
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                entries: sampleEntries
            });
            verify(chooser.saveMode);
            verify(!chooser.canAccept, "a save needs a name");
            chooser.saveName = "report.txt";
            tryCompare(chooser, "canAccept", true);

            var accept = findChild(chooser, "chooserAccept");
            clickItem(accept);
            tryCompare(acceptedSpy, "count", 1);
        }

        function test_up_is_offered_below_the_root() {
            var chooser = make({ currentUri: "file:///home/user", currentLabel: "/home/user" });
            verify(chooser.canGoUp);
            var up = findChild(chooser, "chooserUp");
            clickItem(up);
            tryCompare(upSpy, "count", 1);

            chooser.currentUri = "file:///";
            tryCompare(chooser, "canGoUp", false);
        }

        function test_escape_cancels() {
            var chooser = make({ title: "Open File", currentUri: "file:///tmp",
                                 currentLabel: "/tmp", entries: sampleEntries });
            chooser.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            tryCompare(cancelledSpy, "count", 1);
        }
    }
}