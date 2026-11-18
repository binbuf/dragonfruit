// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T267 scale case. The input files set `expectedScale` to 1, 1.5, or 2 and
// ctest runs the same QML under QT_SCALE_FACTOR. The layout is checked in
// logical points at 740x820 and 720x480; the device pixel ratio is asserted
// separately so a run that silently ignored the factor cannot pass.
Item {
    id: root
    width: 1000
    height: 900
    property real expectedScale: 1.0

    Component { id: shellComponent; SettingsShell { } }

    function collectRows(item, out) {
        if (item === null || item === undefined)
            return;
        if (item.labelText !== undefined && item.control !== undefined
                && item.labelColumn !== undefined && item.label !== undefined)
            out.push(item);
        var children = item.children;
        for (var i = 0; i < children.length; ++i)
            root.collectRows(children[i], out);
    }

    TestCase {
        name: "SettingsScale"
        when: windowShown

        function init() {
            Settings.set("settings.lastPane", "");
        }

        function test_device_pixel_ratio() {
            fuzzyCompare(Screen.devicePixelRatio, root.expectedScale, 0.01);
        }

        function test_layout_at_reference_sizes() {
            var sizes = [[740, 820], [720, 480]];
            for (var s = 0; s < sizes.length; ++s) {
                var shell = createTemporaryObject(shellComponent, root,
                                                  { width: sizes[s][0],
                                                    height: sizes[s][1] });
                waitForRendering(root);
                tryVerify(function() { return shell.paneBody.status === Loader.Ready; });

                compare(shell.sidebarColumn.width + shell.detailColumn.width,
                        shell.width);
                // T286: the mockup geometry holds in logical points at every
                // device pixel ratio.
                compare(shell.searchField.width,
                        Theme.controls.settingsSidebar.searchWidth);
                compare(shell.searchField.height,
                        Theme.controls.settingsSidebar.searchHeight);
                compare(shell.searchField.x,
                        Theme.controls.settingsSidebar.searchInset);
                compare(shell.searchField.y,
                        Theme.controls.settingsSidebar.searchTop);
                compare(shell.sidebar.rowHeight,
                        Theme.controls.settingsSidebar.rowHeight);
                compare(shell.navPill.width,
                        Theme.controls.settingsWindow.navPillWidth);
                compare(shell.navPill.height,
                        Theme.controls.settingsWindow.navPillHeight);
                compare(shell.detailContent.x,
                        Theme.controls.settingsWindow.contentInset);
                compare(shell.detailContent.y,
                        Theme.controls.settingsWindow.contentTopInset);
                compare(shell.helpButton.width,
                        Theme.controls.settingsWindow.helpButtonSize);
                verify(shell.searchField.x >= 0
                       && shell.searchField.x + shell.searchField.width
                          <= shell.sidebarColumn.width + 0.5,
                       "search field fits the sidebar at "
                       + sizes[s][0] + "x" + sizes[s][1]);
                verify(shell.titleBar.x >= 0
                       && shell.titleBar.x + shell.titleBar.width
                          <= shell.detailColumn.width + 0.5);
                verify(shell.detailScroll.height > 0);
                verify(shell.detailScroll.cornerInset > 0,
                       "scrollbars stop short of the resize corner");
                verify(shell.resizeEdges.cornerAvoidance > 0,
                       "resize grips avoid the scrollbar corner");

                // Appearance has rows with toggles, sliders, selects, and
                // disclosure controls, so it exercises every control slot.
                shell.selectPane("appearance");
                tryVerify(function() {
                    return shell.paneBody.status === Loader.Ready
                        && shell.loadedPaneId === "appearance";
                });
                waitForRendering(root);
                var rows = [];
                root.collectRows(shell.paneBody.item, rows);
                verify(rows.length > 0, "the pane must render rows");
                for (var r = 0; r < rows.length; ++r) {
                    var row = rows[r];
                    verify(row.width > 0 && row.height > 0);
                    verify(row.labelText.width >= 0);
                    verify(row.control.x >= -0.5
                           && row.control.x + row.control.width
                              <= row.width + 0.5,
                           "control stays inside the row at "
                           + sizes[s][0] + "x" + sizes[s][1]);
                    verify(row.labelColumn.x + row.labelColumn.width
                           <= row.control.x + 0.5,
                           "label and control do not overlap at "
                           + sizes[s][0] + "x" + sizes[s][1]);
                    verify(row.labelText.elide !== Text.ElideNone
                           || row.labelText.contentWidth <= row.labelText.width + 0.5,
                           "label text is elided or fits");
                    verify(row.descriptionText.wrapMode !== Text.NoWrap
                           || row.descriptionText.contentWidth
                              <= row.descriptionText.width + 0.5,
                           "description wraps or fits");
                }
                shell.destroy();
            }
        }
    }
}
