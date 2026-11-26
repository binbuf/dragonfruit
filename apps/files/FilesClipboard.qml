// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick

// The Files clipboard model (T330): what Copy/Cut marked, and the mode a
// Paste will use. It holds no filesystem logic and no copy loop — the paste
// itself runs through `FilesDirectoryModel` / files-core. The model lives in
// the window shell, not the directory session, so a copy survives navigation
// (copy in one folder, paste in another).
//
// Each item is `{ uri, name, isDir }`; `isDir` is the kind hint the optimistic
// row needs without a stat. Cut marks but removes nothing until a paste, as
// the Files design requires.
QtObject {
    id: root

    // `""` (empty), `"copy"`, or `"cut"`.
    property string mode: ""
    // The marked items in selection order.
    property var items: []
    readonly property int count: items.length
    readonly property bool empty: items.length === 0

    signal changed()

    function copy(marked) {
        root.items = (marked || []).slice();
        root.mode = root.items.length > 0 ? "copy" : "";
        root.changed();
    }

    function cut(marked) {
        root.items = (marked || []).slice();
        root.mode = root.items.length > 0 ? "cut" : "";
        root.changed();
    }

    function clear() {
        if (root.mode === "" && root.items.length === 0)
            return;
        root.mode = "";
        root.items = [];
        root.changed();
    }
}
