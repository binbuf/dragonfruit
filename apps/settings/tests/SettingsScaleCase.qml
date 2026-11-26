// SPDX-License-Identifier: GPL-3.0-or-later
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

        // T290g: the Appearance cards are the tallest and widest in the pane,
        // so a scale input that silently clips a tile, plate, card, or radio
        // group must fail here. Every check is in logical points; the device
        // pixel ratio is asserted separately above.
        function test_appearance_fit_at_scale() {
            var shell = createTemporaryObject(shellComponent, root,
                                              { width: 740, height: 820 });
            waitForRendering(root);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            shell.selectPane("appearance");
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                    && shell.loadedPaneId === "appearance";
            });
            waitForRendering(root);

            var pane = shell.paneBody.item;
            verify(pane, "the appearance body must load");
            verify(pane.width <= shell.detailContent.width + 0.5,
                   "the pane tracks the detail column at "
                   + root.expectedScale + "x");

            function fits(item, ancestor, what) {
                verify(item, what + " must exist");
                var origin = item.mapToItem(ancestor, 0, 0);
                verify(origin.x >= -0.5
                       && origin.x + item.width <= ancestor.width + 0.5,
                       what + " stays inside its column at "
                       + root.expectedScale + "x");
            }

            function fitsRow(row, what) {
                fits(row, pane, what);
                verify(row.control.x >= -0.5
                       && row.control.x + row.control.width
                          <= row.width + 0.5,
                       what + " control stays inside the row at "
                       + root.expectedScale + "x");
                verify(row.labelColumn.x + row.labelColumn.width
                       <= row.control.x + 0.5,
                       what + " label and control do not overlap at "
                       + root.expectedScale + "x");
            }

            // One card per mockup group, inside the detail column.
            var cards = [
                [pane.glassGroup, "Glass card"],
                [pane.themeGroup, "Theme card"],
                [pane.windowsGroup, "Windows card"],
                [pane.scrollBarsSection, "Show scroll bars card"],
                [pane.scrollBarClickSection, "Click in the scroll bar to card"]
            ];
            for (var c = 0; c < cards.length; ++c)
                fits(cards[c][0], pane, cards[c][1]);

            // The three mode tiles: all of them inside the Appearance row.
            compare(pane.schemeTileRepeater.count, 3);
            fitsRow(pane.schemeRow, "Appearance row");
            fits(pane.schemeTiles, pane.schemeRow, "mode tile row");
            for (var m = 0; m < pane.schemeTileRepeater.count; ++m) {
                var modeTile = pane.schemeTileRepeater.itemAt(m);
                fits(modeTile, pane.schemeRow, "mode tile " + m);
                verify(modeTile.x >= -0.5
                       && modeTile.x + modeTile.width
                          <= pane.schemeTiles.width + 0.5,
                       "mode tile " + m + " stays inside the tile row at "
                       + root.expectedScale + "x");
            }

            // The Glass preview, its resolver plate, and the tint slider.
            fits(pane.glassPreview, pane.glassGroup, "Glass preview");
            fits(pane.previewPlate, pane.glassPreview, "Glass plate");
            fitsRow(pane.transparencyRow, "Glass tint row");
            fits(pane.transparencySlider, pane.transparencyRow,
                 "Glass tint slider");

            // The Theme card: four rows, the accent swatches, and the four
            // icon-style tiles.
            fitsRow(pane.accentRow, "Accent color row");
            fitsRow(pane.highlightRow, "Text highlight row");
            fitsRow(pane.iconStyleRow, "Icon & widget style row");
            fitsRow(pane.folderColorRow, "Folder color row");
            for (var a = 0; a < pane.accentRepeater.count; ++a)
                fits(pane.accentRepeater.itemAt(a), pane.accentRow,
                     "accent swatch " + a);
            compare(pane.iconStyleTileRepeater.count, 4);
            fits(pane.iconStyleTiles, pane.iconStyleRow, "icon-style tile row");
            for (var t = 0; t < pane.iconStyleTileRepeater.count; ++t) {
                var styleTile = pane.iconStyleTileRepeater.itemAt(t);
                fits(styleTile, pane.iconStyleRow, "icon-style tile " + t);
                verify(styleTile.x >= -0.5
                       && styleTile.x + styleTile.width
                          <= pane.iconStyleTiles.width + 0.5,
                       "icon-style tile " + t + " stays inside the tile row at "
                       + root.expectedScale + "x");
            }

            // The Windows card rows.
            fitsRow(pane.sidebarIconSizeRow, "Sidebar icon size row");
            fitsRow(pane.wallpaperTintRow, "Wallpaper tint row");

            // Both stacked radio groups, and their option labels.
            fitsRow(pane.scrollBarsRow, "Show scroll bars row");
            fitsRow(pane.scrollBarClickRow, "Click in the scroll bar to row");
            fits(pane.scrollBarsGroup, pane.scrollBarsRow,
                 "Show scroll bars radios");
            fits(pane.scrollBarClickGroup, pane.scrollBarClickRow,
                 "Scroll-bar click radios");
            verify(pane.scrollBarsGroup.count === 3
                   && pane.scrollBarClickGroup.count === 2);
            for (var g = 0; g < pane.scrollBarsGroup.count; ++g) {
                var option = pane.scrollBarsGroup.repeater.itemAt(g);
                fits(option, pane.scrollBarsGroup,
                     "scroll-bar option " + g);
            }

            shell.destroy();
        }
    }
}
