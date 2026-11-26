// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Screenshot

// FileChooser picker view tests (T-13.2b): the entry rows, selection and
// multi-selection, folder activation, the SaveFile name field, the overwrite
// confirmation, the SaveFiles note, the favorites sidebar and breadcrumbs,
// New Folder, and Escape cancellation. Runs headless on the offscreen
// platform; the bridge's D-Bus half is covered by tst_chooser.cpp.
Item {
    id: stage
    width: 760
    height: 520

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
        SignalSpy { id: browseSpy; signalName: "browseRequested" }
        SignalSpy { id: newFolderSpy; signalName: "newFolderRequested" }
        SignalSpy { id: selectAllSpy; signalName: "selectAllRequested" }
        SignalSpy { id: replaceSpy; signalName: "replaceAccepted" }
        SignalSpy { id: dismissSpy; signalName: "replaceDismissed" }

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
            browseSpy.target = chooser;
            newFolderSpy.target = chooser;
            selectAllSpy.target = chooser;
            replaceSpy.target = chooser;
            dismissSpy.target = chooser;
            activatedSpy.clear();
            selectedSpy.clear();
            acceptedSpy.clear();
            cancelledSpy.clear();
            upSpy.clear();
            browseSpy.clear();
            newFolderSpy.clear();
            selectAllSpy.clear();
            replaceSpy.clear();
            dismissSpy.clear();
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

            // The shell pushes the bridge's selection back into the view.
            chooser.selectedIndices = [0];
            chooser.selectedIndex = 0;
            // A folder row is a valid Open selection: it navigates.
            verify(chooser.canAccept, "a selected folder can be opened");

            var fileRow = list.itemAtIndex(1);
            clickItem(fileRow);
            tryCompare(selectedSpy, "count", 2);
            compare(selectedSpy.signalArguments[1][0], 1);
            chooser.selectedIndices = [1];
            chooser.selectedIndex = 1;
            verify(chooser.canAccept, "a selected file enables accept");

            var accept = findChild(chooser, "chooserAccept");
            verify(accept !== null);
            clickItem(accept);
            tryCompare(acceptedSpy, "count", 1);
        }

        function test_keyboard_navigation_selects_the_next_row() {
            var chooser = make({
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                entries: sampleEntries
            });
            chooser.selectedIndices = [0];
            chooser.selectedIndex = 0;
            var list = findChild(chooser, "chooserList");
            list.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Down);
            tryCompare(selectedSpy, "count", 1);
            compare(selectedSpy.signalArguments[0][0], 1);
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
            verify(findChild(chooser, "chooserSaveField").visible);
            chooser.saveName = "report.txt";
            tryCompare(chooser, "canAccept", true);

            var accept = findChild(chooser, "chooserAccept");
            clickItem(accept);
            tryCompare(acceptedSpy, "count", 1);
        }

        function test_return_in_the_save_field_accepts() {
            var chooser = make({
                title: "Save File",
                kind: "save",
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                saveName: "report.txt"
            });
            var input = findChild(chooser, "chooserSaveInput");
            input.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Return);
            tryCompare(acceptedSpy, "count", 1);
        }

        function test_save_files_mode_accepts_the_folder() {
            var chooser = make({
                title: "Save Files",
                kind: "save-files",
                acceptLabel: "Save Here",
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                entries: sampleEntries,
                saveFiles: ["one.txt", "two.txt"]
            });
            verify(chooser.saveFilesMode);
            verify(!chooser.saveMode);
            verify(chooser.canAccept, "SaveFiles accepts the current folder");
            verify(findChild(chooser, "chooserSaveFilesNote").visible);
            verify(!findChild(chooser, "chooserSaveRow").visible);
        }

        function test_replace_confirmation_replaces_or_dismisses() {
            var chooser = make({
                title: "Save File",
                kind: "save",
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                entries: sampleEntries,
                saveName: "zeta.txt"
            });
            verify(chooser.canAccept);
            chooser.confirmReplace = true;
            chooser.replaceName = "zeta.txt";
            waitForRendering(stage);
            verify(findChild(chooser, "chooserReplaceBanner").visible);
            verify(!chooser.canAccept, "the normal accept is disabled while confirming");

            clickItem(findChild(chooser, "chooserReplace"));
            tryCompare(replaceSpy, "count", 1);

            clickItem(findChild(chooser, "chooserKeepEditing"));
            tryCompare(dismissSpy, "count", 1);
        }

        function test_the_sidebar_browses_a_favorite() {
            var chooser = make({
                currentUri: "file:///tmp",
                favorites: [{ label: "Home", icon: "home", uri: "file:///home/user" }]
            });
            var favorite = findChild(chooser, "chooserFavorite");
            verify(favorite !== null);
            clickItem(favorite);
            tryCompare(browseSpy, "count", 1);
            compare(browseSpy.signalArguments[0][0], "file:///home/user");
        }

        function test_the_breadcrumbs_browse_an_ancestor() {
            var chooser = make({
                currentUri: "file:///tmp/docs",
                breadcrumbs: [
                    { label: "Computer", uri: "file:///" },
                    { label: "tmp", uri: "file:///tmp" },
                    { label: "docs", uri: "file:///tmp/docs" }
                ]
            });
            var crumb = findChild(chooser, "chooserCrumb");
            verify(crumb !== null);
            clickItem(crumb);
            tryCompare(browseSpy, "count", 1);
            compare(browseSpy.signalArguments[0][0], "file:///");
        }

        function test_new_folder_is_offered() {
            var chooser = make({ currentUri: "file:///tmp", currentLabel: "/tmp" });
            var button = findChild(chooser, "chooserNewFolder");
            verify(button !== null);
            clickItem(button);
            tryCompare(newFolderSpy, "count", 1);
        }

        function test_ctrl_a_selects_all() {
            var chooser = make({
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                multiple: true,
                entries: sampleEntries
            });
            var list = findChild(chooser, "chooserList");
            list.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_A, Qt.ControlModifier);
            tryCompare(selectAllSpy, "count", 1);
        }

        function test_folder_mode_explains_the_destination() {
            var chooser = make({
                title: "Choose Folder",
                directoryMode: true,
                currentUri: "file:///home/user/Documents",
                breadcrumbs: [
                    { label: "Computer", uri: "file:///" },
                    { label: "home", uri: "file:///home" },
                    { label: "user", uri: "file:///home/user" },
                    { label: "Documents", uri: "file:///home/user/Documents" }
                ]
            });
            var note = findChild(chooser, "chooserChooseFolderNote");
            verify(note.visible);
            verify(note.text.indexOf("Documents") >= 0, "names the folder: " + note.text);
            verify(chooser.canAccept, "the current folder is choosable");
        }

        function test_escape_dismisses_the_replace_confirmation() {
            var chooser = make({
                title: "Save File",
                kind: "save",
                currentUri: "file:///tmp",
                currentLabel: "/tmp",
                saveName: "zeta.txt",
                confirmReplace: true,
                replaceName: "zeta.txt"
            });
            chooser.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            tryCompare(dismissSpy, "count", 1);
            compare(cancelledSpy.count, 0, "the request stays open");
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

        function test_escape_from_the_list_cancels() {
            var chooser = make({ title: "Open File", currentUri: "file:///tmp",
                                 currentLabel: "/tmp", entries: sampleEntries });
            var list = findChild(chooser, "chooserList");
            list.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            tryCompare(cancelledSpy, "count", 1);
        }
    }
}
